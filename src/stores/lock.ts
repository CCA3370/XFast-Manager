import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ManagementData, ManagementItemType } from '@/types'
import { getItem, setItem } from '@/services/storage'
import { xplanePathKey } from '@/utils/xplaneStorage'
import { useAppStore } from './app'

export interface LockedItemsData {
  aircraft: string[]
  plugin: string[]
  navdata: string[]
  scenery: string[]
  lua: string[]
}

const ROOT_LOCKS_KEY = 'lockedItemsByXplanePath'

type LockItemType = ManagementItemType | 'scenery' | 'lua'

export const useLockStore = defineStore('lock', () => {
  type LockSets = Record<LockItemType, Set<string>>
  const roots = ref(new Map<string, LockSets>())
  const empty = new Set<string>()
  const isInitialized = ref(false)
  let initialization: Promise<void> | undefined
  let writeQueue = Promise.resolve()

  function normalizeKeys(items?: string[] | null): string[] {
    if (!Array.isArray(items)) return []

    const seen = new Set<string>()
    const normalized: string[] = []

    for (const item of items) {
      if (typeof item !== 'string') continue
      const key = item.trim().toLowerCase()
      if (!key || seen.has(key)) continue
      seen.add(key)
      normalized.push(key)
    }

    return normalized
  }

  function normalizeSnapshot(snapshot?: Partial<LockedItemsData> | null): LockedItemsData {
    return {
      aircraft: normalizeKeys(snapshot?.aircraft),
      plugin: normalizeKeys(snapshot?.plugin),
      navdata: normalizeKeys(snapshot?.navdata),
      scenery: normalizeKeys(snapshot?.scenery),
      lua: normalizeKeys(snapshot?.lua),
    }
  }

  function toSets(snapshot?: Partial<LockedItemsData> | null): LockSets {
    const normalized = normalizeSnapshot(snapshot)
    return {
      aircraft: new Set(normalized.aircraft),
      plugin: new Set(normalized.plugin),
      navdata: new Set(normalized.navdata),
      scenery: new Set(normalized.scenery),
      lua: new Set(normalized.lua),
    }
  }

  function snapshot(sets?: LockSets): LockedItemsData {
    return {
      aircraft: [...(sets?.aircraft ?? empty)],
      plugin: [...(sets?.plugin ?? empty)],
      navdata: [...(sets?.navdata ?? empty)],
      scenery: [...(sets?.scenery ?? empty)],
      lua: [...(sets?.lua ?? empty)],
    }
  }

  function exportLockState(xplanePath = useAppStore().xplanePath): LockedItemsData {
    return snapshot(roots.value.get(xplanePathKey(xplanePath)))
  }

  function initStore(): Promise<void> {
    return (initialization ??= (async () => {
      // Old global locks have no installation provenance. Leave them on disk, but
      // do not inherit them into an arbitrary installation.
      const data = await getItem<Record<string, LockedItemsData>>(ROOT_LOCKS_KEY)
      if (data && typeof data === 'object') {
        for (const [key, state] of Object.entries(data)) {
          if (key) roots.value.set(xplanePathKey(key), toSets(state))
        }
      }
      isInitialized.value = true
    })().catch((error: unknown) => {
      initialization = undefined
      throw error
    }))
  }

  function saveToStorage() {
    const data = Object.fromEntries([...roots.value].map(([key, sets]) => [key, snapshot(sets)]))
    const write = writeQueue.then(() => setItem(ROOT_LOCKS_KEY, data))
    writeQueue = write.catch(() => {})
    return write
  }

  function getSetForType(type: LockItemType, xplanePath = useAppStore().xplanePath) {
    return roots.value.get(xplanePathKey(xplanePath))?.[type] ?? empty
  }

  function isLocked(type: LockItemType, folderName: string, xplanePath?: string): boolean {
    return getSetForType(type, xplanePath).has(folderName.trim().toLowerCase())
  }

  function getLockedItems(type: LockItemType, xplanePath?: string): string[] {
    return [...getSetForType(type, xplanePath)]
  }

  async function toggleLock(
    type: LockItemType,
    folderName: string,
    xplanePath = useAppStore().xplanePath,
  ): Promise<boolean> {
    await initStore()
    const key = xplanePathKey(xplanePath)
    const name = folderName.trim().toLowerCase()
    if (!key || !name) return false
    if (!roots.value.has(key)) roots.value.set(key, toSets())
    const set = getSetForType(type, xplanePath)
    const locked = !set.has(name)
    if (locked) set.add(name)
    else set.delete(name)
    await saveToStorage()
    if (type === 'aircraft' || type === 'plugin') {
      try {
        await invoke('set_cfg_disabled', {
          xplanePath,
          itemType: type,
          folderName,
          disabled: locked,
        })
      } catch (error) {
        console.warn('Failed to sync lock state to cfg file:', error)
      }
    }
    return locked
  }

  async function setLocked(
    type: LockItemType,
    folderName: string,
    locked: boolean,
    xplanePath = useAppStore().xplanePath,
  ) {
    const key = xplanePathKey(xplanePath)
    const name = folderName.trim().toLowerCase()
    if (!key || !name) return
    await initStore()
    if (!roots.value.has(key)) roots.value.set(key, toSets())
    const set = getSetForType(type, xplanePath)
    if (locked) set.add(name)
    else set.delete(name)
    await saveToStorage()
  }

  async function syncCfgDisabledState(
    type: 'aircraft' | 'plugin',
    xplanePath: string,
    desiredLocked: Set<string>,
  ) {
    const scanCommand = type === 'aircraft' ? 'scan_aircraft' : 'scan_plugins'
    const result = await invoke<ManagementData<{ folderName: string }>>(scanCommand, {
      xplanePath,
    })

    for (const item of result.entries) {
      await invoke('set_cfg_disabled', {
        xplanePath,
        itemType: type,
        folderName: item.folderName,
        disabled: desiredLocked.has(item.folderName.toLowerCase()),
      })
    }
  }

  async function applyLockState(
    state?: Partial<LockedItemsData> | null,
    xplanePath = useAppStore().xplanePath,
  ) {
    const key = xplanePathKey(xplanePath)
    if (!key) return
    await initStore()
    const sets = toSets(state)
    roots.value.set(key, sets)
    await saveToStorage()
    await syncCfgDisabledState('aircraft', xplanePath, sets.aircraft)
    await syncCfgDisabledState('plugin', xplanePath, sets.plugin)
  }

  function isPathLocked(targetPath: string, xplanePath: string): boolean {
    if (!targetPath || !xplanePath) return false
    const root = xplanePathKey(xplanePath)
    const normalizedTarget = xplanePathKey(targetPath)
    if (!normalizedTarget.startsWith(root + '/')) return false
    const cleanPath = normalizedTarget.slice(root.length + 1).toLowerCase()

    // Determine the type and folder name based on the path
    // Aircraft: Aircraft/folderName
    // Plugin: Resources/plugins/folderName
    // Lua script: Resources/plugins/FlyWithLua/Scripts/script.lua
    // Navdata: Custom Data/folderName or similar
    // Scenery: Custom Scenery/folderName

    if (cleanPath.startsWith('aircraft/')) {
      const folderName = cleanPath.split('/')[1]
      if (folderName) return getSetForType('aircraft', xplanePath).has(folderName)
    } else if (cleanPath.startsWith('resources/plugins/flywithlua/scripts/')) {
      // Lua scripts and companions should be controlled by per-script Lua locks only.
      // Do NOT inherit FlyWithLua plugin lock for installation conflict checks.
      const scriptEntry = cleanPath.split('/')[4]
      if (!scriptEntry) return false

      // Lock key for Lua scripts is display name (file stem, lower-cased).
      const scriptStem = scriptEntry.includes('.')
        ? scriptEntry.substring(0, scriptEntry.lastIndexOf('.'))
        : scriptEntry
      return getSetForType('lua', xplanePath).has(scriptStem)
    } else if (cleanPath.startsWith('resources/plugins/')) {
      const folderName = cleanPath.split('/')[2]
      if (folderName) return getSetForType('plugin', xplanePath).has(folderName)
    } else if (cleanPath.startsWith('custom scenery/')) {
      const folderName = cleanPath.split('/')[1]
      if (folderName) return getSetForType('scenery', xplanePath).has(folderName)
    } else if (cleanPath.startsWith('custom data/')) {
      const folderName = cleanPath.split('/')[1]
      if (folderName) return getSetForType('navdata', xplanePath).has(folderName)
    }

    return false
  }

  // Computed counts
  const lockedAircraftCount = computed(() => getSetForType('aircraft').size)
  const lockedPluginCount = computed(() => getSetForType('plugin').size)
  const lockedNavdataCount = computed(() => getSetForType('navdata').size)
  const lockedSceneryCount = computed(() => getSetForType('scenery').size)
  const lockedLuaCount = computed(() => getSetForType('lua').size)
  const totalLockedCount = computed(
    () =>
      getSetForType('aircraft').size +
      getSetForType('plugin').size +
      getSetForType('navdata').size +
      getSetForType('scenery').size +
      getSetForType('lua').size,
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
    getLockedItems,
    toggleLock,
    setLocked,
    exportLockState,
    applyLockState,
    isPathLocked,
  }
})
