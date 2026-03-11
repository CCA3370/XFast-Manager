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

  // State
  const packages = ref<CslPackageInfo[]>([])
  const paths = ref<CslPath[]>([])
  const customPaths = ref<string[]>([])
  const serverVersion = ref('')
  const isLoading = ref(false)
  const installingPackages = ref<string[]>([])
  const progressMap = ref<Record<string, CslProgress>>({})
  const error = ref<string | null>(null)

  // Track pending installs for deferred rescan
  let pendingInstalls = 0

  // Computed
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

  // Actions
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

  async function installPackage(packageName: string, targetPath: string) {
    if (installingPackages.value.includes(packageName)) return

    installingPackages.value = [...installingPackages.value, packageName]
    pendingInstalls++

    const parallelDownloads = managementStore.addonUpdateOptions.totalThreads ?? 32

    try {
      await invoke('csl_install_package', {
        packageName,
        targetPath,
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
    } catch (e) {
      logError(`CSL install failed for ${packageName}: ${e}`, 'csl')
      toast.error(t('csl.installError', { name: packageName }))
    } finally {
      installingPackages.value = installingPackages.value.filter((n) => n !== packageName)
      const newMap = { ...progressMap.value }
      delete newMap[packageName]
      progressMap.value = newMap
      pendingInstalls--

      // Rescan only when all installs are done
      if (pendingInstalls === 0) {
        await scanPackages()
      }
    }
  }

  async function installAll(targetPath: string) {
    const toInstall = packages.value.filter((p) => p.status !== 'up_to_date')
    for (const pkg of toInstall) {
      installPackage(pkg.name, targetPath) // fire without await — run concurrently
    }
  }

  async function uninstallPackage(packageName: string) {
    try {
      await invoke('csl_uninstall_package', {
        packageName,
        paths: allPaths.value,
      })

      toast.success(t('csl.uninstallSuccess', { name: packageName }))
      // Rescan to update status
      await scanPackages()
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

  return {
    // State
    packages,
    paths,
    customPaths,
    serverVersion,
    isLoading,
    installingPackages,
    progressMap,
    error,

    // Computed
    totalPackages,
    installedCount,
    updatesCount,
    notUpToDateCount,
    allPaths,

    // Actions
    scanPackages,
    installPackage,
    installAll,
    uninstallPackage,
    addCustomPath,
    removeCustomPath,
    updateProgress,
  }
})
