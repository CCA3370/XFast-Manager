import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import {
  getErrorMessage,
  type CslPackageInfo,
  type CslPath,
  type CslScanResult,
  type CslProgress,
} from '@/types'
import { useAppStore } from './app'
import { useManagementStore } from './management'
import { useToastStore } from './toast'
import { logError } from '@/services/logger'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'

type InstallSource = 'csl' | 'altitude'

interface QueuedInstallTask {
  source: InstallSource
  name: string
}

const CSL_MAX_PARALLEL_DOWNLOADS = 12
const DEFAULT_CSL_SERVER_BASE_URL = 'http://x-csl.ru'
const MAX_CSL_SERVER_BASE_URLS = 4
const DESCRIPTION_LOAD_BATCH_SIZE = 12
const DESCRIPTION_LOAD_QUEUE_DELAY_MS = 80

export const useCslStore = defineStore('csl', () => {
  const appStore = useAppStore()
  const managementStore = useManagementStore()
  const toast = useToastStore()
  const { t } = useI18n()

  const packages = ref<CslPackageInfo[]>([])
  const paths = ref<CslPath[]>([])
  const customPaths = ref<string[]>([])
  const serverVersion = ref('')
  const serverBaseUrls = ref<string[]>([DEFAULT_CSL_SERVER_BASE_URL])
  const activeServerBaseUrl = ref(DEFAULT_CSL_SERVER_BASE_URL)
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
  const isServerConfigLoaded = ref(false)
  const pendingDescriptionPackages = ref<string[]>([])
  const loadingDescriptionPackages = ref<string[]>([])

  let installQueueRunning = false
  let descriptionLoadTimer: ReturnType<typeof setTimeout> | null = null
  let descriptionLoadRunning = false
  let descriptionLoadGeneration = 0
  const descriptionLoadQueue = new Set<string>()

  const totalPackages = computed(() => packages.value.length)
  const installedCount = computed(
    () => packages.value.filter((p) => p.status !== 'not_installed').length,
  )
  const updatesCount = computed(
    () => packages.value.filter((p) => p.status === 'needs_update').length,
  )
  const notUpToDateCount = computed(
    () => packages.value.filter((p) => p.status !== 'up_to_date').length,
  )

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
  const altitudeInstalledCount = computed(
    () => altitudePackages.value.filter((p) => p.status !== 'not_installed').length,
  )
  const altitudeUpdatesCount = computed(
    () => altitudePackages.value.filter((p) => p.status === 'needs_update').length,
  )
  const altitudeNotUpToDateCount = computed(
    () => altitudePackages.value.filter((p) => p.status !== 'up_to_date').length,
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
    return Math.min(
      managementStore.addonUpdateOptions.totalThreads ?? 32,
      CSL_MAX_PARALLEL_DOWNLOADS,
    )
  }

  function normalizeServerBaseUrl(url: string): string {
    const trimmed = url.trim()

    if (!trimmed) {
      throw new Error(t('csl.serverUrlRequired'))
    }

    let parsed: URL
    try {
      parsed = new URL(trimmed)
    } catch {
      throw new Error(t('csl.serverUrlInvalid'))
    }

    if (!['http:', 'https:'].includes(parsed.protocol)) {
      throw new Error(t('csl.serverUrlInvalid'))
    }

    const pathname = parsed.pathname || '/'
    const hasOnlyTrailingSlashes = /^\/+$/.test(pathname)

    if (!hasOnlyTrailingSlashes || parsed.search || parsed.hash) {
      throw new Error(t('csl.serverUrlNoPath'))
    }

    return parsed.origin
  }

  function collectServerBaseUrls(
    urls: string[],
    options: { ignoreInvalid?: boolean; ignoreDuplicates?: boolean } = {},
  ): string[] {
    const normalized: string[] = []

    for (const rawUrl of urls) {
      const trimmed = rawUrl.trim()
      if (!trimmed) {
        continue
      }

      try {
        const normalizedUrl = normalizeServerBaseUrl(trimmed)

        if (normalized.includes(normalizedUrl)) {
          if (options.ignoreDuplicates) {
            continue
          }

          throw new Error(t('csl.serverUrlDuplicate'))
        }

        normalized.push(normalizedUrl)
      } catch (error) {
        if (options.ignoreInvalid) {
          continue
        }

        throw error
      }
    }

    return normalized
  }

  async function ensureServerConfigLoaded() {
    if (isServerConfigLoaded.value) {
      return
    }

    const savedUrls = await getItem<string[]>(STORAGE_KEYS.CSL_SERVER_BASE_URLS)
    const savedActiveUrl = await getItem<string>(STORAGE_KEYS.CSL_ACTIVE_SERVER_BASE_URL)

    const nextUrls = Array.isArray(savedUrls)
      ? collectServerBaseUrls(savedUrls, {
          ignoreInvalid: true,
          ignoreDuplicates: true,
        }).slice(0, MAX_CSL_SERVER_BASE_URLS)
      : []

    if (nextUrls.length === 0) {
      nextUrls.push(DEFAULT_CSL_SERVER_BASE_URL)
    }

    let nextActiveUrl = nextUrls[0]

    if (typeof savedActiveUrl === 'string') {
      try {
        const normalizedActiveUrl = normalizeServerBaseUrl(savedActiveUrl)
        if (nextUrls.includes(normalizedActiveUrl)) {
          nextActiveUrl = normalizedActiveUrl
        }
      } catch {
        nextActiveUrl = nextUrls[0]
      }
    }

    serverBaseUrls.value = nextUrls
    activeServerBaseUrl.value = nextActiveUrl
    isServerConfigLoaded.value = true
  }

  async function saveServerConfig(urls: string[], selectedUrl: string) {
    const nextUrls = collectServerBaseUrls(urls)

    if (nextUrls.length === 0) {
      throw new Error(t('csl.serverUrlRequired'))
    }

    if (nextUrls.length > MAX_CSL_SERVER_BASE_URLS) {
      throw new Error(
        t('csl.serverLimitReached', {
          max: MAX_CSL_SERVER_BASE_URLS,
        }),
      )
    }

    const normalizedSelectedUrl = normalizeServerBaseUrl(selectedUrl)
    if (!nextUrls.includes(normalizedSelectedUrl)) {
      throw new Error(t('csl.serverSelectionRequired'))
    }

    serverBaseUrls.value = nextUrls
    activeServerBaseUrl.value = normalizedSelectedUrl
    isServerConfigLoaded.value = true

    await setItem(STORAGE_KEYS.CSL_SERVER_BASE_URLS, nextUrls)
    await setItem(STORAGE_KEYS.CSL_ACTIVE_SERVER_BASE_URL, normalizedSelectedUrl)
  }

  function resetDescriptionLoadState(packageNames: string[]) {
    descriptionLoadGeneration += 1
    pendingDescriptionPackages.value = [...new Set(packageNames)]
    loadingDescriptionPackages.value = []

    descriptionLoadQueue.clear()
    if (descriptionLoadTimer !== null) {
      clearTimeout(descriptionLoadTimer)
      descriptionLoadTimer = null
    }
  }

  function isDescriptionPending(packageName: string): boolean {
    return pendingDescriptionPackages.value.includes(packageName)
  }

  function isDescriptionLoading(packageName: string): boolean {
    return loadingDescriptionPackages.value.includes(packageName)
  }

  function queuePackageDescriptions(packageNames: string[]) {
    const nextNames = packageNames.filter(
      (name) => isDescriptionPending(name) && !isDescriptionLoading(name),
    )

    if (nextNames.length === 0) {
      return
    }

    for (const name of nextNames) {
      descriptionLoadQueue.add(name)
    }

    if (descriptionLoadTimer !== null) {
      return
    }

    descriptionLoadTimer = setTimeout(() => {
      descriptionLoadTimer = null
      void pumpDescriptionQueue(descriptionLoadGeneration)
    }, DESCRIPTION_LOAD_QUEUE_DELAY_MS)
  }

  async function pumpDescriptionQueue(generation: number) {
    if (descriptionLoadRunning || generation !== descriptionLoadGeneration) {
      return
    }

    descriptionLoadRunning = true

    try {
      while (descriptionLoadQueue.size > 0 && generation === descriptionLoadGeneration) {
        const batch = Array.from(descriptionLoadQueue)
          .filter((name) => isDescriptionPending(name) && !isDescriptionLoading(name))
          .slice(0, DESCRIPTION_LOAD_BATCH_SIZE)

        if (batch.length === 0) {
          break
        }

        for (const name of batch) {
          descriptionLoadQueue.delete(name)
        }

        loadingDescriptionPackages.value = [
          ...new Set([...loadingDescriptionPackages.value, ...batch]),
        ]

        let generationChanged = false

        try {
          await ensureServerConfigLoaded()
          const descriptions = await invoke<Record<string, string>>(
            'csl_fetch_package_descriptions',
            {
              packageNames: batch,
              serverBaseUrl: activeServerBaseUrl.value,
            },
          )

          if (generation !== descriptionLoadGeneration) {
            generationChanged = true
          } else {
            for (const pkg of packages.value) {
              const description = descriptions[pkg.name]
              if (typeof description === 'string' && description.trim()) {
                pkg.description = description
              }
            }
          }
        } catch (e) {
          logError(`CSL description fetch failed: ${e}`, 'csl')
        } finally {
          if (!generationChanged && generation === descriptionLoadGeneration) {
            loadingDescriptionPackages.value = loadingDescriptionPackages.value.filter(
              (name) => !batch.includes(name),
            )
            pendingDescriptionPackages.value = pendingDescriptionPackages.value.filter(
              (name) => !batch.includes(name),
            )
          }
        }

        if (generationChanged || generation !== descriptionLoadGeneration) {
          break
        }
      }
    } finally {
      descriptionLoadRunning = false

      if (generation === descriptionLoadGeneration && descriptionLoadQueue.size > 0) {
        void pumpDescriptionQueue(generation)
      }
    }
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
    await ensureServerConfigLoaded()

    const parallelDownloads = getParallelDownloads()

    if (task.source === 'altitude') {
      try {
        await invoke('altitude_install_package', {
          xplanePath: appStore.xplanePath,
          parallelDownloads,
          serverBaseUrl: activeServerBaseUrl.value,
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
        serverBaseUrl: activeServerBaseUrl.value,
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
          cancellingTaskKeys.value = cancellingTaskKeys.value.filter(
            (key) => key !== taskKey(nextTask),
          )
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

    await ensureServerConfigLoaded()
    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<CslScanResult>('csl_scan_packages', {
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
        serverBaseUrl: activeServerBaseUrl.value,
      })

      packages.value = result.packages
      paths.value = result.paths
      serverVersion.value = result.server_version
      resetDescriptionLoadState(
        result.packages.filter((pkg) => !pkg.description).map((pkg) => pkg.name),
      )
      void syncLinks()
    } catch (e) {
      resetDescriptionLoadState([])
      error.value = String(e)
      logError(`CSL scan failed: ${e}`, 'csl')
      toast.error(t('csl.fetchError'))
    } finally {
      isLoading.value = false
    }
  }

  async function rescanPackages(packageNames: string[]) {
    try {
      await ensureServerConfigLoaded()
      const updated = await invoke<CslPackageInfo[]>('csl_rescan_packages', {
        xplanePath: appStore.xplanePath,
        packageNames,
        serverBaseUrl: activeServerBaseUrl.value,
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

    await ensureServerConfigLoaded()
    altitudeLoading.value = true

    try {
      const result = await invoke<CslScanResult>('altitude_scan_packages', {
        xplanePath: appStore.xplanePath,
        serverBaseUrl: activeServerBaseUrl.value,
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
      await ensureServerConfigLoaded()
      await invoke('altitude_uninstall_package', {
        xplanePath: appStore.xplanePath,
        serverBaseUrl: activeServerBaseUrl.value,
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
    serverBaseUrls,
    activeServerBaseUrl,
    isLoading,
    installingPackages,
    queuedPackages,
    cancellingPackages,
    progressMap,
    error,
    searchQuery,
    pendingDescriptionPackages,
    loadingDescriptionPackages,

    totalPackages,
    installedCount,
    updatesCount,
    notUpToDateCount,
    allPaths,
    allScansDone,
    hasPendingInstalls,

    scanPackages,
    ensureServerConfigLoaded,
    saveServerConfig,
    queuePackageDescriptions,
    isDescriptionPending,
    isDescriptionLoading,
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
