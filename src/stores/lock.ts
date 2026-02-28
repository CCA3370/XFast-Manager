import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ManagementItemType } from '@/types'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'
import { useAppStore } from './app'

interface LockedItemsData {
  aircraft: string[]
  plugin: string[]
  navdata: string[]
  scenery: string[]
  lua: string[]
}

type LockItemType = ManagementItemType | 'scenery' | 'lua'

export const useLockStore = defineStore('lock', () => {
  // Internal state using Sets for efficient lookup
  const aircraft = ref<Set<string>>(new Set())
  const plugin = ref<Set<string>>(new Set())
  const navdata = ref<Set<string>>(new Set())
  const scenery = ref<Set<string>>(new Set())
  const lua = ref<Set<string>>(new Set())

  // Initialization flag
  const isInitialized = ref(false)

  // Load from storage on initialization
  async function initStore(): Promise<void> {
    if (isInitialized.value) return

    try {
      const data = await getItem<LockedItemsData>(STORAGE_KEYS.LOCKED_ITEMS)
      if (data) {
        if (Array.isArray(data.aircraft))
          aircraft.value = new Set(data.aircraft.map((s) => s.toLowerCase()))
        if (Array.isArray(data.plugin))
          plugin.value = new Set(data.plugin.map((s) => s.toLowerCase()))
        if (Array.isArray(data.navdata))
          navdata.value = new Set(data.navdata.map((s) => s.toLowerCase()))
        if (Array.isArray(data.scenery))
          scenery.value = new Set(data.scenery.map((s) => s.toLowerCase()))
        if (Array.isArray(data.lua))
          lua.value = new Set(data.lua.map((s) => s.toLowerCase()))
      }
    } catch (e) {
      console.error('Failed to load locked items from storage:', e)
    }

    isInitialized.value = true
  }

  // Save to storage
  async function saveToStorage() {
    const data: LockedItemsData = {
      aircraft: Array.from(aircraft.value),
      plugin: Array.from(plugin.value),
      navdata: Array.from(navdata.value),
      scenery: Array.from(scenery.value),
      lua: Array.from(lua.value),
    }
    await setItem(STORAGE_KEYS.LOCKED_ITEMS, data)
  }

  // Get the set for a specific type
  function getSetForType(type: LockItemType): Set<string> {
    switch (type) {
      case 'aircraft':
        return aircraft.value
      case 'plugin':
        return plugin.value
      case 'navdata':
        return navdata.value
      case 'scenery':
        return scenery.value
      case 'lua':
        return lua.value
      default:
        return new Set()
    }
  }

  // Check if an item is locked
  function isLocked(type: LockItemType, folderName: string): boolean {
    return getSetForType(type).has(folderName.toLowerCase())
  }

  // Toggle lock state
  async function toggleLock(type: LockItemType, folderName: string): Promise<boolean> {
    const set = getSetForType(type)
    const key = folderName.toLowerCase()
    const wasLocked = set.has(key)

    if (wasLocked) {
      set.delete(key)
    } else {
      set.add(key)
    }

    await saveToStorage()
    const newLocked = !wasLocked

    // Sync to cfg file for aircraft and plugins
    if (type === 'aircraft' || type === 'plugin') {
      const appStore = useAppStore()
      if (appStore.xplanePath) {
        try {
          await invoke('set_cfg_disabled', {
            xplanePath: appStore.xplanePath,
            itemType: type,
            folderName,
            disabled: newLocked,
          })
        } catch (e) {
          // Log but don't fail - the app lock state is the source of truth
          console.warn('Failed to sync lock state to cfg file:', e)
        }
      }
    }

    return newLocked
  }

  // Set lock state explicitly
  async function setLocked(
    type: LockItemType,
    folderName: string,
    locked: boolean,
  ) {
    const set = getSetForType(type)
    const key = folderName.toLowerCase()

    if (locked) {
      set.add(key)
    } else {
      set.delete(key)
    }

    await saveToStorage()
  }

  // Check if an install target path is locked
  // targetPath: the full path where the addon will be installed
  // xplanePath: the X-Plane root path
  function isPathLocked(targetPath: string, xplanePath: string): boolean {
    if (!targetPath || !xplanePath) return false

    // Normalize paths for comparison
    const normalizedTarget = targetPath.replace(/\\/g, '/').toLowerCase()
    const normalizedXplane = xplanePath.replace(/\\/g, '/').toLowerCase()

    // Extract the folder name from the target path
    const relativePath = normalizedTarget.startsWith(normalizedXplane)
      ? normalizedTarget.substring(normalizedXplane.length)
      : normalizedTarget

    // Remove leading slash
    const cleanPath = relativePath.replace(/^\/+/, '')

    // Determine the type and folder name based on the path
    // Aircraft: Aircraft/folderName
    // Plugin: Resources/plugins/folderName
    // Lua script: Resources/plugins/FlyWithLua/Scripts/script.lua
    // Navdata: Custom Data/folderName or similar
    // Scenery: Custom Scenery/folderName

    if (cleanPath.startsWith('aircraft/')) {
      const folderName = cleanPath.split('/')[1]
      if (folderName) return aircraft.value.has(folderName)
    } else if (cleanPath.startsWith('resources/plugins/flywithlua/scripts/')) {
      // Lua scripts and companions should be controlled by per-script Lua locks only.
      // Do NOT inherit FlyWithLua plugin lock for installation conflict checks.
      const scriptEntry = cleanPath.split('/')[4]
      if (!scriptEntry) return false

      // Lock key for Lua scripts is display name (file stem, lower-cased).
      const scriptStem = scriptEntry.includes('.')
        ? scriptEntry.substring(0, scriptEntry.lastIndexOf('.'))
        : scriptEntry
      return lua.value.has(scriptStem)
    } else if (cleanPath.startsWith('resources/plugins/')) {
      const folderName = cleanPath.split('/')[2]
      if (folderName) return plugin.value.has(folderName)
    } else if (cleanPath.startsWith('custom scenery/')) {
      const folderName = cleanPath.split('/')[2]
      if (folderName) return scenery.value.has(folderName)
    } else if (cleanPath.startsWith('custom data/')) {
      const folderName = cleanPath.split('/')[2]
      if (folderName) return navdata.value.has(folderName)
    }

    return false
  }

  // Computed counts
  const lockedAircraftCount = computed(() => aircraft.value.size)
  const lockedPluginCount = computed(() => plugin.value.size)
  const lockedNavdataCount = computed(() => navdata.value.size)
  const lockedSceneryCount = computed(() => scenery.value.size)
  const lockedLuaCount = computed(() => lua.value.size)
  const totalLockedCount = computed(
    () =>
      aircraft.value.size +
      plugin.value.size +
      navdata.value.size +
      scenery.value.size +
      lua.value.size,
  )

  return {
    // State (readonly computed for external access)
    lockedAircraftCount,
    lockedPluginCount,
    lockedNavdataCount,
    lockedSceneryCount,
    lockedLuaCount,
    totalLockedCount,
    isInitialized,

    // Actions
    initStore,
    isLocked,
    toggleLock,
    setLocked,
    isPathLocked,
  }
})
