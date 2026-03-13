import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import type { CslPackageInfo, CslPath, CslScanResult, CslProgress } from '@/types'
import { useAppStore } from './app'
import { useManagementStore } from './management'
import { useToastStore } from './toast'
import { logError } from '@/services/logger'

export const useCslStore = defineStore('csl', () => {
  const appStore = useAppStore()
  const managementStore = useManagementStore()
  const toast = useToastStore()
  const { t } = useI18n()

  // ========== CSL State ==========
  const packages = ref<CslPackageInfo[]>([])
  const paths = ref<CslPath[]>([])
  const customPaths = ref<string[]>([])
  const serverVersion = ref('')
  const isLoading = ref(false)
  const installingPackages = ref<string[]>([])
  const progressMap = ref<Record<string, CslProgress>>({})
  const error = ref<string | null>(null)
  const searchQuery = ref('')

  // Track pending installs for deferred rescan
  let pendingInstalls = 0
  let recentlyInstalled: string[] = []

  // ========== ALTITUDE State ==========
  const altitudePackages = ref<CslPackageInfo[]>([])
  const altitudeLoading = ref(false)
  const altitudeInstallingPackages = ref<string[]>([])
  const altitudeProgressMap = ref<Record<string, CslProgress>>({})

  // Track pending altitude installs for deferred rescan
  let altitudePendingInstalls = 0

  // ========== CSL Computed ==========
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

  // True when both CSL and ALTITUDE scans have finished
  const allScansDone = computed(() => !isLoading.value && !altitudeLoading.value)

  // ========== ALTITUDE Computed ==========
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

  // ========== CSL Actions ==========
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
        const idx = packages.value.findIndex((p) => p.name === upd.name)
        if (idx !== -1) {
          // Preserve description from original (rescan doesn't re-fetch descriptions)
          upd.description = packages.value[idx].description
          packages.value[idx] = upd
        }
      }
    } catch (e) {
      logError(`CSL partial rescan failed: ${e}`, 'csl')
    }
  }

  async function installPackage(packageName: string) {
    if (installingPackages.value.includes(packageName)) return

    installingPackages.value = [...installingPackages.value, packageName]
    pendingInstalls++

    const parallelDownloads = managementStore.addonUpdateOptions.totalThreads ?? 32

    try {
      await invoke('csl_install_package', {
        packageName,
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
        parallelDownloads,
      })

      toast.success(t('csl.installSuccess', { name: packageName }))

      // Optimistically update local status so button doesn't reappear before rescan
      const pkg = packages.value.find((p) => p.name === packageName)
      if (pkg) {
        pkg.status = 'up_to_date'
        pkg.files_to_update = 0
        pkg.update_size_bytes = 0
      }

      recentlyInstalled.push(packageName)
    } catch (e) {
      logError(`CSL install failed for ${packageName}: ${e}`, 'csl')
      toast.error(t('csl.installError', { name: packageName }))
    } finally {
      installingPackages.value = installingPackages.value.filter((n) => n !== packageName)
      const newMap = { ...progressMap.value }
      delete newMap[packageName]
      progressMap.value = newMap
      pendingInstalls--

      // Partial rescan only when all installs are done
      if (pendingInstalls === 0 && recentlyInstalled.length > 0) {
        const toRescan = [...recentlyInstalled]
        recentlyInstalled = []
        await rescanPackages(toRescan)
      }
    }
  }

  async function installAll() {
    const toInstall = packages.value.filter((p) => p.status !== 'up_to_date')
    for (const pkg of toInstall) {
      installPackage(pkg.name) // fire without await — run concurrently
    }
  }

  async function uninstallPackage(packageName: string) {
    try {
      await invoke('csl_uninstall_package', {
        packageName,
        xplanePath: appStore.xplanePath,
        customPaths: customPaths.value,
      })

      toast.success(t('csl.uninstallSuccess', { name: packageName }))

      // Optimistically update local state — no rescan needed
      const pkg = packages.value.find((p) => p.name === packageName)
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

  function addCustomPath(path: string) {
    if (!customPaths.value.includes(path)) {
      customPaths.value.push(path)
    }
  }

  function removeCustomPath(path: string) {
    customPaths.value = customPaths.value.filter((p) => p !== path)
  }

  function updateProgress(prog: CslProgress) {
    progressMap.value = { ...progressMap.value, [prog.package_name]: prog }
  }

  // ========== ALTITUDE Actions ==========
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

  async function installAltitudePackage() {
    if (!appStore.xplanePath) return
    if (altitudeInstallingPackages.value.includes('ALTITUDE')) return

    altitudeInstallingPackages.value = [...altitudeInstallingPackages.value, 'ALTITUDE']
    altitudePendingInstalls++

    const parallelDownloads = managementStore.addonUpdateOptions.totalThreads ?? 32

    try {
      await invoke('altitude_install_package', {
        xplanePath: appStore.xplanePath,
        parallelDownloads,
      })

      toast.success(t('altitude.installSuccess', { name: 'ALTITUDE' }))

      for (const pkg of altitudePackages.value) {
        pkg.status = 'up_to_date'
        pkg.files_to_update = 0
        pkg.update_size_bytes = 0
      }
    } catch (e) {
      logError(`ALTITUDE install failed: ${e}`, 'altitude')
      toast.error(t('altitude.installError', { name: 'ALTITUDE' }))
    } finally {
      altitudeInstallingPackages.value = altitudeInstallingPackages.value.filter((n) => n !== 'ALTITUDE')
      const newMap = { ...altitudeProgressMap.value }
      delete newMap['ALTITUDE']
      altitudeProgressMap.value = newMap
      altitudePendingInstalls--
    }
  }

  async function installAllAltitude() {
    const hasNotUpToDate = altitudePackages.value.some((p) => p.status !== 'up_to_date')
    if (hasNotUpToDate) {
      installAltitudePackage()
    }
  }

  async function uninstallAltitudePackage() {
    if (!appStore.xplanePath) return

    try {
      await invoke('altitude_uninstall_package', {
        xplanePath: appStore.xplanePath,
      })

      toast.success(t('altitude.uninstallSuccess', { name: 'ALTITUDE' }))

      // Optimistically update local state — no rescan needed
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
    // CSL State
    packages,
    paths,
    customPaths,
    serverVersion,
    isLoading,
    installingPackages,
    progressMap,
    error,
    searchQuery,

    // CSL Computed
    totalPackages,
    installedCount,
    updatesCount,
    notUpToDateCount,
    allPaths,
    allScansDone,

    // CSL Actions
    scanPackages,
    installPackage,
    installAll,
    uninstallPackage,
    addCustomPath,
    removeCustomPath,
    updateProgress,

    // ALTITUDE State
    altitudePackages,
    altitudeLoading,
    altitudeInstallingPackages,
    altitudeProgressMap,

    // ALTITUDE Computed
    altitudeTotalPackages,
    altitudeInstalledCount,
    altitudeUpdatesCount,
    altitudeNotUpToDateCount,

    // ALTITUDE Actions
    scanAltitudePackages,
    installAltitudePackage,
    installAllAltitude,
    uninstallAltitudePackage,
    updateAltitudeProgress,
  }
})
