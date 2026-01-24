import { defineStore } from 'pinia'
import { ref, computed, type Ref } from 'vue'
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
import { logError } from '@/services/logger'

// Cache duration: 1 hour in milliseconds
const UPDATE_CACHE_DURATION = 60 * 60 * 1000

// Maximum number of entries in update cache to prevent unbounded memory growth
const MAX_UPDATE_CACHE_SIZE = 500

// Cache structure for update check results
// Only cache latestVersion, hasUpdate should be recalculated based on current local version
interface UpdateCacheEntry {
  latestVersion: string | null
  timestamp: number
}

// Update cache: key is updateUrl, value is cache entry
const updateCache = new Map<string, UpdateCacheEntry>()

// Evict expired entries and oldest entries if cache is too large
// Uses batch eviction for O(1) amortized complexity
function evictUpdateCacheIfNeeded() {
  const now = Date.now()

  // First, remove expired entries
  for (const [key, entry] of updateCache.entries()) {
    if (now - entry.timestamp >= UPDATE_CACHE_DURATION) {
      updateCache.delete(key)
    }
  }

  // If still over capacity, batch remove oldest entries (10% or at least 10)
  if (updateCache.size >= MAX_UPDATE_CACHE_SIZE) {
    const entriesToRemove = Math.max(Math.floor(MAX_UPDATE_CACHE_SIZE / 10), 10)
    const targetAge = UPDATE_CACHE_DURATION / 2

    // Collect keys to remove (prioritize older entries)
    const keysToRemove: string[] = []
    for (const [key, entry] of updateCache.entries()) {
      if (now - entry.timestamp > targetAge || keysToRemove.length < entriesToRemove) {
        keysToRemove.push(key)
        if (keysToRemove.length >= entriesToRemove) break
      }
    }

    for (const key of keysToRemove) {
      updateCache.delete(key)
    }
  }
}

// Helper function to set cache entry with eviction
function setCacheEntry(url: string, entry: UpdateCacheEntry) {
  evictUpdateCacheIfNeeded()
  updateCache.set(url, entry)
}

// Type for items that can have update info
interface UpdatableItem {
  updateUrl?: string
  version?: string
  latestVersion?: string
  hasUpdate: boolean
  folderName: string
}

// Base type for any loadable management item
interface LoadableItem {
  folderName: string
}

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

  // ========================================
  // Generic helper functions to reduce code duplication
  // ========================================

  // Helper function to check if cache is valid
  function isCacheValid(url: string): boolean {
    const cached = updateCache.get(url)
    if (!cached) return false
    return Date.now() - cached.timestamp < UPDATE_CACHE_DURATION
  }

  // Apply cached update info to items
  // hasUpdate is recalculated based on current local version vs cached remote version
  function applyCachedUpdates<T extends UpdatableItem>(items: T[]): T[] {
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

  // Generic load function for management items
  interface LoadConfig<T> {
    scanCommand: string
    itemsRef: Ref<T[]>
    totalCountRef: Ref<number>
    enabledCountRef: Ref<number>
    applyCache?: boolean
    afterLoad?: () => void
    logName: string
  }

  async function loadItems<T extends LoadableItem>(config: LoadConfig<T>) {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      return
    }

    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<ManagementData<T>>(config.scanCommand, {
        xplanePath: appStore.xplanePath
      })

      // Apply cached update info if applicable (only for UpdatableItem types)
      if (config.applyCache) {
        config.itemsRef.value = applyCachedUpdates(result.entries as unknown as UpdatableItem[]) as unknown as T[]
      } else {
        config.itemsRef.value = result.entries
      }
      config.totalCountRef.value = result.totalCount
      config.enabledCountRef.value = result.enabledCount

      // Run post-load callback (e.g., start update check)
      if (config.afterLoad) {
        config.afterLoad()
      }
    } catch (e) {
      error.value = String(e)
      logError(`Failed to load ${config.logName}: ${e}`, 'management')
    } finally {
      isLoading.value = false
    }
  }

  // Generic update check function
  interface UpdateCheckConfig<T extends UpdatableItem> {
    itemsRef: Ref<T[]>
    checkCommand: string
    checkParamName: string
    logName: string
  }

  async function checkItemUpdates<T extends UpdatableItem>(config: UpdateCheckConfig<T>) {
    if (config.itemsRef.value.length === 0) return

    // Only check items that have update URLs and no valid cache
    const itemsToCheck = getItemsNeedingUpdateCheck(config.itemsRef.value)
    if (itemsToCheck.length === 0) return

    isCheckingUpdates.value = true

    try {
      // Send only items needing check to backend
      const updated = await invoke<T[]>(config.checkCommand, {
        [config.checkParamName]: itemsToCheck
      })

      // Update cache with results (only store latestVersion, not hasUpdate)
      for (const item of updated) {
        if (item.updateUrl) {
          setCacheEntry(item.updateUrl, {
            latestVersion: item.latestVersion ?? null,
            timestamp: Date.now()
          })
        }
      }

      // Merge updated items back into list
      const updatedMap = new Map(updated.map(item => [item.folderName, item]))
      config.itemsRef.value = config.itemsRef.value.map(item => {
        const updatedItem = updatedMap.get(item.folderName)
        if (updatedItem) {
          return { ...item, latestVersion: updatedItem.latestVersion, hasUpdate: updatedItem.hasUpdate }
        }
        return item
      })
    } catch (e) {
      logError(`Failed to check ${config.logName} updates: ${e}`, 'management')
      // Don't set error.value here as this is a background operation
    } finally {
      isCheckingUpdates.value = false
    }
  }

  // ========================================
  // Load functions using generic helpers
  // ========================================

  // Load aircraft data
  async function loadAircraft() {
    await loadItems<AircraftInfo>({
      scanCommand: 'scan_aircraft',
      itemsRef: aircraft,
      totalCountRef: aircraftTotalCount,
      enabledCountRef: aircraftEnabledCount,
      applyCache: true,
      afterLoad: () => checkAircraftUpdates(),
      logName: 'aircraft'
    })
  }

  // Check for aircraft updates
  async function checkAircraftUpdates() {
    await checkItemUpdates<AircraftInfo>({
      itemsRef: aircraft,
      checkCommand: 'check_aircraft_updates',
      checkParamName: 'aircraft',
      logName: 'aircraft'
    })
  }

  // Load plugins data
  async function loadPlugins() {
    await loadItems<PluginInfo>({
      scanCommand: 'scan_plugins',
      itemsRef: plugins,
      totalCountRef: pluginsTotalCount,
      enabledCountRef: pluginsEnabledCount,
      applyCache: true,
      afterLoad: () => checkPluginsUpdates(),
      logName: 'plugins'
    })
  }

  // Check for plugin updates
  async function checkPluginsUpdates() {
    await checkItemUpdates<PluginInfo>({
      itemsRef: plugins,
      checkCommand: 'check_plugins_updates',
      checkParamName: 'plugins',
      logName: 'plugins'
    })
  }

  // Load navdata (no update cache needed)
  async function loadNavdata() {
    await loadItems<NavdataManagerInfo>({
      scanCommand: 'scan_navdata',
      itemsRef: navdata,
      totalCountRef: navdataTotalCount,
      enabledCountRef: navdataEnabledCount,
      applyCache: false,
      logName: 'navdata'
    })
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
      logError(`Failed to toggle enabled: ${e}`, 'management')
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
      logError(`Failed to delete item: ${e}`, 'management')
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
      logError(`Failed to open folder: ${e}`, 'management')
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
