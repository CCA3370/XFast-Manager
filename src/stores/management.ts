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

export const useManagementStore = defineStore('management', () => {
  const appStore = useAppStore()

  // State
  const aircraft = ref<AircraftInfo[]>([])
  const plugins = ref<PluginInfo[]>([])
  const navdata = ref<NavdataManagerInfo[]>([])
  const activeTab = ref<ManagementTab>('aircraft')
  const isLoading = ref(false)
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
      aircraft.value = result.entries
      aircraftTotalCount.value = result.totalCount
      aircraftEnabledCount.value = result.enabledCount
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load aircraft:', e)
    } finally {
      isLoading.value = false
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
      plugins.value = result.entries
      pluginsTotalCount.value = result.totalCount
      pluginsEnabledCount.value = result.enabledCount
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load plugins:', e)
    } finally {
      isLoading.value = false
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
          // Navdata: folder name changes with - prefix
          const item = navdata.value.find(n => n.folderName === folderName)
          if (item) {
            item.enabled = newEnabled
            // Update folder name (add or remove - prefix)
            const baseName = item.folderName.startsWith('-')
              ? item.folderName.slice(1)
              : item.folderName
            item.folderName = newEnabled ? baseName : `-${baseName}`
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

    // Actions
    loadAircraft,
    loadPlugins,
    loadNavdata,
    loadCurrentTabData,
    toggleEnabled,
    deleteItem,
    openFolder,
    setActiveTab,
    clear
  }
})
