import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  AircraftInfo,
  PluginInfo,
  NavdataManagerInfo,
  ManagementData,
  ManagementTab,
  ManagementItemType
} from '@/types'
import { useAppStore } from './app'
import { getNavdataCycleStatus } from '@/utils/airac'

// Cache duration: 1 hour in milliseconds
const UPDATE_CACHE_DURATION = 60 * 60 * 1000

// Cache structure for update check results
// Only cache latestVersion, hasUpdate should be recalculated based on current local version
interface UpdateCacheEntry {
  latestVersion: string | null
  timestamp: number
}

// Update cache: key is updateUrl, value is cache entry
const updateCache = new Map<string, UpdateCacheEntry>()

export const useManagementStore = defineStore('management', () => {
  const appStore = useAppStore()

  // State
  const aircraft = ref<AircraftInfo[]>([])
  const plugins = ref<PluginInfo[]>([])
  const navdata = ref<NavdataManagerInfo[]>([])
  const activeTab = ref<ManagementTab>('aircraft')
  const isLoading = ref(false)
  const isCheckingUpdates = ref(false)
  const error = ref<string | null>(null)

  // Counts
  const aircraftTotalCount = ref(0)
  const aircraftEnabledCount = ref(0)
  const pluginsTotalCount = ref(0)
  const pluginsEnabledCount = ref(0)
  const navdataTotalCount = ref(0)
  const navdataEnabledCount = ref(0)

  // Computed properties
  const sortedAircraft = computed(() => {
    return [...aircraft.value].sort((a, b) =>
      a.displayName.toLowerCase().localeCompare(b.displayName.toLowerCase())
    )
  })

  const sortedPlugins = computed(() => {
    return [...plugins.value].sort((a, b) =>
      a.displayName.toLowerCase().localeCompare(b.displayName.toLowerCase())
    )
  })

  const sortedNavdata = computed(() => {
    return [...navdata.value].sort((a, b) =>
      a.providerName.toLowerCase().localeCompare(b.providerName.toLowerCase())
    )
  })

  // Update counts
  const aircraftUpdateCount = computed(() => {
    return aircraft.value.filter(a => a.hasUpdate).length
  })

  const pluginsUpdateCount = computed(() => {
    return plugins.value.filter(p => p.hasUpdate).length
  })

  // Navdata outdated count
  const navdataOutdatedCount = computed(() => {
    return navdata.value.filter(n => {
      const cycleText = n.cycle || n.airac
      return getNavdataCycleStatus(cycleText) === 'outdated'
    }).length
  })

  // Helper function to check if cache is valid
  function isCacheValid(url: string): boolean {
    const cached = updateCache.get(url)
    if (!cached) return false
    return Date.now() - cached.timestamp < UPDATE_CACHE_DURATION
  }

  // Apply cached update info to items
  // hasUpdate is recalculated based on current local version vs cached remote version
  function applyCachedUpdates<T extends { updateUrl?: string; version?: string; latestVersion?: string; hasUpdate: boolean }>(
    items: T[]
  ): T[] {
    return items.map(item => {
      if (item.updateUrl && isCacheValid(item.updateUrl)) {
        const cached = updateCache.get(item.updateUrl)!
        const latestVersion = cached.latestVersion ?? undefined
        // Recalculate hasUpdate based on current local version
        const hasUpdate = latestVersion != null && latestVersion !== (item.version || '')
        return {
          ...item,
          latestVersion,
          hasUpdate
        }
      }
      return item
    })
  }

  // Get items that need update check (no valid cache)
  function getItemsNeedingUpdateCheck<T extends { updateUrl?: string }>(items: T[]): T[] {
    return items.filter(item => item.updateUrl && !isCacheValid(item.updateUrl))
  }

  // Load aircraft data
  async function loadAircraft() {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      return
    }

    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<ManagementData<AircraftInfo>>('scan_aircraft', {
        xplanePath: appStore.xplanePath
      })
      
      // Apply cached update info first
      aircraft.value = applyCachedUpdates(result.entries)
      aircraftTotalCount.value = result.totalCount
      aircraftEnabledCount.value = result.enabledCount

      // Then check for updates for items without valid cache
      checkAircraftUpdates()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load aircraft:', e)
    } finally {
      isLoading.value = false
    }
  }

  // Check for aircraft updates (only items without valid cache)
  async function checkAircraftUpdates() {
    if (aircraft.value.length === 0) return

    // Only check aircraft that have update URLs and no valid cache
    const aircraftToCheck = getItemsNeedingUpdateCheck(aircraft.value)
    if (aircraftToCheck.length === 0) return

    isCheckingUpdates.value = true

    try {
      // Send only items needing check to backend
      const updated = await invoke<AircraftInfo[]>('check_aircraft_updates', {
        aircraft: aircraftToCheck
      })
      
      // Update cache with results (only store latestVersion, not hasUpdate)
      for (const item of updated) {
        if (item.updateUrl) {
          updateCache.set(item.updateUrl, {
            latestVersion: item.latestVersion ?? null,
            timestamp: Date.now()
          })
        }
      }
      
      // Merge updated items back into aircraft list
      const updatedMap = new Map(updated.map(a => [a.folderName, a]))
      aircraft.value = aircraft.value.map(a => {
        const updatedItem = updatedMap.get(a.folderName)
        if (updatedItem) {
          return { ...a, latestVersion: updatedItem.latestVersion, hasUpdate: updatedItem.hasUpdate }
        }
        return a
      })
    } catch (e) {
      console.error('Failed to check aircraft updates:', e)
      // Don't set error.value here as this is a background operation
    } finally {
      isCheckingUpdates.value = false
    }
  }

  // Load plugins data
  async function loadPlugins() {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      return
    }

    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<ManagementData<PluginInfo>>('scan_plugins', {
        xplanePath: appStore.xplanePath
      })
      
      // Apply cached update info first
      plugins.value = applyCachedUpdates(result.entries)
      pluginsTotalCount.value = result.totalCount
      pluginsEnabledCount.value = result.enabledCount

      // Then check for updates for items without valid cache
      checkPluginsUpdates()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load plugins:', e)
    } finally {
      isLoading.value = false
    }
  }

  // Check for plugin updates (only items without valid cache)
  async function checkPluginsUpdates() {
    if (plugins.value.length === 0) return

    // Only check plugins that have update URLs and no valid cache
    const pluginsToCheck = getItemsNeedingUpdateCheck(plugins.value)
    if (pluginsToCheck.length === 0) return

    isCheckingUpdates.value = true

    try {
      // Send only items needing check to backend
      const updated = await invoke<PluginInfo[]>('check_plugins_updates', {
        plugins: pluginsToCheck
      })
      
      // Update cache with results (only store latestVersion, not hasUpdate)
      for (const item of updated) {
        if (item.updateUrl) {
          updateCache.set(item.updateUrl, {
            latestVersion: item.latestVersion ?? null,
            timestamp: Date.now()
          })
        }
      }
      
      // Merge updated items back into plugins list
      const updatedMap = new Map(updated.map(p => [p.folderName, p]))
      plugins.value = plugins.value.map(p => {
        const updatedItem = updatedMap.get(p.folderName)
        if (updatedItem) {
          return { ...p, latestVersion: updatedItem.latestVersion, hasUpdate: updatedItem.hasUpdate }
        }
        return p
      })
    } catch (e) {
      console.error('Failed to check plugin updates:', e)
      // Don't set error.value here as this is a background operation
    } finally {
      isCheckingUpdates.value = false
    }
  }

  // Load navdata
  async function loadNavdata() {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      return
    }

    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<ManagementData<NavdataManagerInfo>>('scan_navdata', {
        xplanePath: appStore.xplanePath
      })
      navdata.value = result.entries
      navdataTotalCount.value = result.totalCount
      navdataEnabledCount.value = result.enabledCount
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load navdata:', e)
    } finally {
      isLoading.value = false
    }
  }

  // Load data for current tab
  async function loadCurrentTabData() {
    switch (activeTab.value) {
      case 'aircraft':
        await loadAircraft()
        break
      case 'plugin':
        await loadPlugins()
        break
      case 'navdata':
        await loadNavdata()
        break
      // scenery is handled by sceneryStore
    }
  }

  // Toggle enabled state
  async function toggleEnabled(itemType: ManagementItemType, folderName: string) {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      return
    }

    try {
      const newEnabled = await invoke<boolean>('toggle_management_item', {
        xplanePath: appStore.xplanePath,
        itemType,
        folderName
      })

      // Update local state
      switch (itemType) {
        case 'aircraft': {
          // Aircraft: folder name stays the same, only enabled state changes
          const item = aircraft.value.find(a => a.folderName === folderName)
          if (item) {
            item.enabled = newEnabled
            aircraftEnabledCount.value = aircraft.value.filter(a => a.enabled).length
          }
          break
        }
        case 'plugin': {
          // Plugin: folder name stays the same, only enabled state changes
          const item = plugins.value.find(p => p.folderName === folderName)
          if (item) {
            item.enabled = newEnabled
            pluginsEnabledCount.value = plugins.value.filter(p => p.enabled).length
          }
          break
        }
        case 'navdata': {
          // Navdata: folder name stays the same, only enabled state changes
          const item = navdata.value.find(n => n.folderName === folderName)
          if (item) {
            item.enabled = newEnabled
            navdataEnabledCount.value = navdata.value.filter(n => n.enabled).length
          }
          break
        }
      }
    } catch (e) {
      error.value = String(e)
      console.error('Failed to toggle enabled:', e)
      throw e
    }
  }

  // Delete item
  async function deleteItem(itemType: ManagementItemType, folderName: string) {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      throw new Error(error.value)
    }

    try {
      await invoke('delete_management_item', {
        xplanePath: appStore.xplanePath,
        itemType,
        folderName
      })

      // Remove from local state
      switch (itemType) {
        case 'aircraft':
          aircraft.value = aircraft.value.filter(a => a.folderName !== folderName)
          aircraftTotalCount.value = aircraft.value.length
          aircraftEnabledCount.value = aircraft.value.filter(a => a.enabled).length
          break
        case 'plugin':
          plugins.value = plugins.value.filter(p => p.folderName !== folderName)
          pluginsTotalCount.value = plugins.value.length
          pluginsEnabledCount.value = plugins.value.filter(p => p.enabled).length
          break
        case 'navdata':
          navdata.value = navdata.value.filter(n => n.folderName !== folderName)
          navdataTotalCount.value = navdata.value.length
          navdataEnabledCount.value = navdata.value.filter(n => n.enabled).length
          break
      }
    } catch (e) {
      error.value = String(e)
      console.error('Failed to delete item:', e)
      throw e
    }
  }

  // Open folder
  async function openFolder(itemType: ManagementItemType, folderName: string) {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      throw new Error(error.value)
    }

    try {
      await invoke('open_management_folder', {
        xplanePath: appStore.xplanePath,
        itemType,
        folderName
      })
    } catch (e) {
      error.value = String(e)
      console.error('Failed to open folder:', e)
      throw e
    }
  }

  // Set active tab
  function setActiveTab(tab: ManagementTab) {
    activeTab.value = tab
  }

  // Clear store state
  function clear() {
    aircraft.value = []
    plugins.value = []
    navdata.value = []
    error.value = null
  }

  return {
    // State
    aircraft,
    plugins,
    navdata,
    activeTab,
    isLoading,
    isCheckingUpdates,
    error,

    // Counts
    aircraftTotalCount,
    aircraftEnabledCount,
    pluginsTotalCount,
    pluginsEnabledCount,
    navdataTotalCount,
    navdataEnabledCount,

    // Computed
    sortedAircraft,
    sortedPlugins,
    sortedNavdata,
    aircraftUpdateCount,
    pluginsUpdateCount,
    navdataOutdatedCount,

    // Actions
    loadAircraft,
    checkAircraftUpdates,
    loadPlugins,
    checkPluginsUpdates,
    loadNavdata,
    loadCurrentTabData,
    toggleEnabled,
    deleteItem,
    openFolder,
    setActiveTab,
    clear
  }
})
