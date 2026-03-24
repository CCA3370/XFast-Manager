import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { getErrorMessage, type CslPackageInfo, type CslPath, type CslScanResult, type CslProgress } from '@/types'
import { useAppStore } from './app'
import { useManagementStore } from './management'
import { useToastStore } from './toast'
import { logError } from '@/services/logger'

type InstallSource = 'csl' | 'altitude'

interface QueuedInstallTask {
  source: InstallSource
  name: string
}

const CSL_MAX_PARALLEL_DOWNLOADS = 12

export const useCslStore = defineStore('csl', () => {
  const appStore = useAppStore()
  const managementStore = useManagementStore()
  const toast = useToastStore()
  const { t } = useI18n()

  const packages = ref<CslPackageInfo[]>([])
  const paths = ref<CslPath[]>([])
  const customPaths = ref<string[]>([])
  const serverVersion = ref('')
  const isLoading = ref(false)
  const progressMap = ref<Record<string, CslProgress>>({})
  const error = ref<string | null>(null)
  const searchQuery = ref('')

  const altitudePackages = ref<CslPackageInfo[]>([])
  const altitudeLoading = ref(false)
  const altitudeProgressMap = ref<Record<string, CslProgress>>({})

  const installQueue = ref<QueuedInstallTask[]>([])
  const activeInstallTask = ref<QueuedInstallTask | null>(null)
  const cancellingTaskKeys = ref<string[]>([])

  let installQueueRunning = false

  const totalPackages = computed(() => packages.value.length)
  const installedCount = computed(() => packages.value.filter((p) => p.status !== 'not_installed').length)
  const updatesCount = computed(() => packages.value.filter((p) => p.status === 'needs_update').length)
  const notUpToDateCount = computed(() => packages.value.filter((p) => p.status !== 'up_to_date').length)

  const allPaths = computed(() => {
    const pathStrings = paths.value.map((p) => p.path)
    for (const cp of customPaths.value) {
      if (!pathStrings.includes(cp)) {
        pathStrings.push(cp)
      }
    }
    return pathStrings
  })

  const allScansDone = computed(() => !isLoading.value && !altitudeLoading.value)

  const altitudeTotalPackages = computed(() => altitudePackages.value.length)
  const altitudeInstalledCount = computed(() =>
    altitudePackages.value.filter((p) => p.status !== 'not_installed').length,
  )
  const altitudeUpdatesCount = computed(() =>
    altitudePackages.value.filter((p) => p.status === 'needs_update').length,
  )
  const altitudeNotUpToDateCount = computed(() =>
    altitudePackages.value.filter((p) => p.status !== 'up_to_date').length,
  )

  const queuedPackages = computed(() =>
    installQueue.value.filter((task) => task.source === 'csl').map((task) => task.name),
  )
  const queuedAltitudePackages = computed(() =>
    installQueue.value.filter((task) => task.source === 'altitude').map((task) => task.name),
  )
  const installingPackages = computed(() =>
    activeInstallTask.value?.source === 'csl' ? [activeInstallTask.value.name] : [],
  )
  const altitudeInstallingPackages = computed(() =>
    activeInstallTask.value?.source === 'altitude' ? [activeInstallTask.value.name] : [],
  )
  const cancellingPackages = computed(() =>
    cancellingTaskKeys.value
      .filter((key) => key.startsWith('csl:'))
      .map((key) => key.slice('csl:'.length)),
  )
  const cancellingAltitudePackages = computed(() =>
    cancellingTaskKeys.value
      .filter((key) => key.startsWith('altitude:'))
      .map((key) => key.slice('altitude:'.length)),
  )
  const hasPendingInstalls = computed(
    () => activeInstallTask.value !== null || installQueue.value.length > 0,
  )

  function taskKey(task: QueuedInstallTask): string {
    return `${task.source}:${task.name}`
  }

  function isTaskActive(task: QueuedInstallTask): boolean {
    return activeInstallTask.value != null && taskKey(activeInstallTask.value) === taskKey(task)
  }

  function isTaskQueued(task: QueuedInstallTask): boolean {
    return installQueue.value.some((queued) => taskKey(queued) === taskKey(task))
  }

  function isCancelledInstallError(error: unknown): boolean {
    const message = getErrorMessage(error).toLowerCase()
    return message.includes('cancelled') || message.includes('canceled')
  }

  function getParallelDownloads(): number {
    return Math.min(managementStore.addonUpdateOptions.totalThreads ?? 32, CSL_MAX_PARALLEL_DOWNLOADS)
  }

  async function syncLinks(packageNames?: string[], cleanupPaths?: string[]) {
    if (!appStore.xplanePath) {
      return
    }

    try {
      await invoke('csl_sync_links', {
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
        packageNames: packageNames && packageNames.length > 0 ? packageNames : null,
        cleanupPaths: cleanupPaths && cleanupPaths.length > 0 ? cleanupPaths : null,
      })
    } catch (e) {
      logError(`CSL link sync failed: ${e}`, 'csl')
    }
  }

  function clearProgressForTask(task: QueuedInstallTask) {
    if (task.source === 'altitude') {
      const nextMap = { ...altitudeProgressMap.value }
      delete nextMap[task.name]
      altitudeProgressMap.value = nextMap
      return
    }

    const nextMap = { ...progressMap.value }
    delete nextMap[task.name]
    progressMap.value = nextMap
  }

  function enqueueInstallTask(task: QueuedInstallTask) {
    if (isTaskActive(task) || isTaskQueued(task)) {
      return
    }

    installQueue.value = [...installQueue.value, task]
    void pumpInstallQueue()
  }

  function enqueueInstallTasks(tasks: QueuedInstallTask[]) {
    let nextQueue = installQueue.value
    let changed = false

    for (const task of tasks) {
      if (isTaskActive(task) || nextQueue.some((queued) => taskKey(queued) === taskKey(task))) {
        continue
      }
      nextQueue = [...nextQueue, task]
      changed = true
    }

    if (!changed) {
      return
    }

    installQueue.value = nextQueue
    void pumpInstallQueue()
  }

  async function runInstallTask(task: QueuedInstallTask): Promise<boolean> {
    await managementStore.loadAddonUpdateOptions()

    const parallelDownloads = getParallelDownloads()

    if (task.source === 'altitude') {
      try {
        await invoke('altitude_install_package', {
          xplanePath: appStore.xplanePath,
          parallelDownloads,
        })

        toast.success(t('altitude.installSuccess', { name: task.name }))

        for (const pkg of altitudePackages.value) {
          pkg.status = 'up_to_date'
          pkg.files_to_update = 0
          pkg.update_size_bytes = 0
        }

        return false
      } catch (e) {
        if (isCancelledInstallError(e)) {
          toast.info(t('altitude.cancelled', { name: task.name }))
          return false
        }

        logError(`ALTITUDE install failed: ${e}`, 'altitude')
        toast.error(t('altitude.installError', { name: task.name }))
        return false
      }
    }

    try {
      await invoke('csl_install_package', {
        packageName: task.name,
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
        parallelDownloads,
      })

      toast.success(t('csl.installSuccess', { name: task.name }))

      const pkg = packages.value.find((item) => item.name === task.name)
      if (pkg) {
        pkg.status = 'up_to_date'
        pkg.files_to_update = 0
        pkg.update_size_bytes = 0
      }
    } catch (e) {
      if (isCancelledInstallError(e)) {
        toast.info(t('csl.cancelled', { name: task.name }))
        return true
      }

      logError(`CSL install failed for ${task.name}: ${e}`, 'csl')
      toast.error(t('csl.installError', { name: task.name }))
      return true
    }

    return true
  }

  async function pumpInstallQueue() {
    if (installQueueRunning) {
      return
    }

    installQueueRunning = true

    try {
      while (true) {
        const nextTask = installQueue.value[0]

        if (!nextTask) {
          break
        }

        installQueue.value = installQueue.value.slice(1)
        activeInstallTask.value = nextTask

        try {
          const shouldRescan = await runInstallTask(nextTask)
          if (shouldRescan && nextTask.source === 'csl') {
            await rescanPackages([nextTask.name])
          }
        } finally {
          cancellingTaskKeys.value = cancellingTaskKeys.value.filter((key) => key !== taskKey(nextTask))
          activeInstallTask.value = null
          clearProgressForTask(nextTask)
        }
      }
    } finally {
      installQueueRunning = false

      if (installQueue.value.length > 0) {
        void pumpInstallQueue()
      }
    }
  }

  async function scanPackages() {
    if (!appStore.xplanePath) {
      error.value = t('csl.noPathsDetected')
      return
    }

    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<CslScanResult>('csl_scan_packages', {
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
      })

      packages.value = result.packages
      paths.value = result.paths
      serverVersion.value = result.server_version
      await syncLinks()
    } catch (e) {
      error.value = String(e)
      logError(`CSL scan failed: ${e}`, 'csl')
      toast.error(t('csl.fetchError'))
    } finally {
      isLoading.value = false
    }
  }

  async function rescanPackages(packageNames: string[]) {
    try {
      const updated = await invoke<CslPackageInfo[]>('csl_rescan_packages', {
        xplanePath: appStore.xplanePath,
        packageNames,
      })

      for (const upd of updated) {
        const index = packages.value.findIndex((p) => p.name === upd.name)
        if (index !== -1) {
          upd.description = packages.value[index].description
          packages.value[index] = upd
        }
      }

      await syncLinks(packageNames)
    } catch (e) {
      logError(`CSL partial rescan failed: ${e}`, 'csl')
    }
  }

  function installPackage(packageName: string) {
    enqueueInstallTask({ source: 'csl', name: packageName })
  }

  function installAll() {
    enqueueInstallTasks(
      packages.value
        .filter((pkg) => pkg.status !== 'up_to_date')
        .map((pkg) => ({ source: 'csl' as const, name: pkg.name })),
    )
  }

  async function uninstallPackage(packageName: string) {
    try {
      await invoke('csl_uninstall_package', {
        packageName,
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
      })

      toast.success(t('csl.uninstallSuccess', { name: packageName }))

      const pkg = packages.value.find((item) => item.name === packageName)
      if (pkg) {
        pkg.status = 'not_installed'
        pkg.files_to_update = pkg.file_count
        pkg.update_size_bytes = pkg.total_size_bytes
      }
    } catch (e) {
      logError(`CSL uninstall failed for ${packageName}: ${e}`, 'csl')
      toast.error(t('csl.uninstallError', { name: packageName }))
    }
  }

  async function addCustomPath(path: string) {
    if (!customPaths.value.includes(path)) {
      customPaths.value.push(path)
      await syncLinks()
    }
  }

  async function removeCustomPath(path: string) {
    const hadPath = customPaths.value.includes(path)
    customPaths.value = customPaths.value.filter((item) => item !== path)
    if (hadPath) {
      await syncLinks(undefined, [path])
    }
  }

  function updateProgress(prog: CslProgress) {
    progressMap.value = { ...progressMap.value, [prog.package_name]: prog }
  }

  async function scanAltitudePackages() {
    if (!appStore.xplanePath) return

    altitudeLoading.value = true

    try {
      const result = await invoke<CslScanResult>('altitude_scan_packages', {
        xplanePath: appStore.xplanePath,
      })

      altitudePackages.value = result.packages
    } catch (e) {
      logError(`ALTITUDE scan failed: ${e}`, 'altitude')
      toast.error(t('altitude.fetchError'))
    } finally {
      altitudeLoading.value = false
    }
  }

  function installAltitudePackage() {
    enqueueInstallTask({ source: 'altitude', name: 'ALTITUDE' })
  }

  function installAllAltitude() {
    if (altitudePackages.value.some((pkg) => pkg.status !== 'up_to_date')) {
      installAltitudePackage()
    }
  }

  function installAllCombined(tasks: QueuedInstallTask[]) {
    enqueueInstallTasks(tasks)
  }

  async function cancelInstall(source: InstallSource, packageName: string) {
    const task = { source, name: packageName }
    const key = taskKey(task)

    if (isTaskActive(task)) {
      if (cancellingTaskKeys.value.includes(key)) {
        return
      }

      cancellingTaskKeys.value = [...cancellingTaskKeys.value, key]

      try {
        await invoke('csl_cancel_install', {
          source,
          packageName,
        })
      } catch (e) {
        cancellingTaskKeys.value = cancellingTaskKeys.value.filter((item) => item !== key)
        logError(`Failed to cancel ${key}: ${e}`, 'csl')
        toast.error(
          source === 'altitude'
            ? t('altitude.cancelError', { name: packageName })
            : t('csl.cancelError', { name: packageName }),
        )
      }

      return
    }

    installQueue.value = installQueue.value.filter((queued) => taskKey(queued) !== key)
    clearProgressForTask(task)
  }

  async function uninstallAltitudePackage() {
    if (!appStore.xplanePath) return

    try {
      await invoke('altitude_uninstall_package', {
        xplanePath: appStore.xplanePath,
      })

      toast.success(t('altitude.uninstallSuccess', { name: 'ALTITUDE' }))

      for (const pkg of altitudePackages.value) {
        pkg.status = 'not_installed'
        pkg.files_to_update = pkg.file_count
        pkg.update_size_bytes = pkg.total_size_bytes
      }
    } catch (e) {
      logError(`ALTITUDE uninstall failed: ${e}`, 'altitude')
      toast.error(t('altitude.uninstallError', { name: 'ALTITUDE' }))
    }
  }

  function updateAltitudeProgress(prog: CslProgress) {
    altitudeProgressMap.value = { ...altitudeProgressMap.value, [prog.package_name]: prog }
  }

  return {
    packages,
    paths,
    customPaths,
    serverVersion,
    isLoading,
    installingPackages,
    queuedPackages,
    cancellingPackages,
    progressMap,
    error,
    searchQuery,

    totalPackages,
    installedCount,
    updatesCount,
    notUpToDateCount,
    allPaths,
    allScansDone,
    hasPendingInstalls,

    scanPackages,
    installPackage,
    installAll,
    installAllCombined,
    cancelInstall,
    uninstallPackage,
    addCustomPath,
    removeCustomPath,
    updateProgress,

    altitudePackages,
    altitudeLoading,
    altitudeInstallingPackages,
    queuedAltitudePackages,
    cancellingAltitudePackages,
    altitudeProgressMap,

    altitudeTotalPackages,
    altitudeInstalledCount,
    altitudeUpdatesCount,
    altitudeNotUpToDateCount,

    scanAltitudePackages,
    installAltitudePackage,
    installAllAltitude,
    uninstallAltitudePackage,
    updateAltitudeProgress,
  }
})
