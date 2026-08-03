import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { SceneryCategory, type SceneryManagerData, type SceneryManagerEntry } from '@/types'

const tauri = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))

const management = vi.hoisted(() => ({
  loadAddonUpdateOptions: vi.fn(async () => {}),
  checkItemUpdates: vi.fn(async () => ({ failed: false, checked: true, updateCount: 0 })),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke }))
vi.mock('./management', () => ({ useManagementStore: () => management }))

import { useAppStore } from './app'
import { useSceneryStore } from './scenery'

function createEntry(): SceneryManagerEntry {
  return {
    folderName: 'Unknown Scenery',
    category: SceneryCategory.Unrecognized,
    originalCategory: SceneryCategory.Unrecognized,
    subPriority: 0,
    enabled: false,
    sortOrder: 0,
    hasUpdate: false,
    missingLibraries: [],
    requiredLibraries: [],
    duplicateTiles: [],
    duplicateAirports: [],
    flattenAvailable: false,
    flattened: false,
  }
}

function createData(): SceneryManagerData {
  return {
    entries: [createEntry()],
    totalCount: 1,
    enabledCount: 0,
    missingDepsCount: 0,
    duplicateTilesCount: 0,
    duplicateAirportsCount: 0,
    needsSync: false,
    tileOverlaps: {},
  }
}

describe('Scenery store category changes', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'get_scenery_index_status') {
        return { indexExists: true, totalPackages: 1 }
      }
      if (command === 'get_scenery_manager_data') return createData()
      return undefined
    })
  })

  it('stages manual classification so Reset can discard it', async () => {
    const appStore = useAppStore()
    appStore.xplanePath = '/xplane'
    const store = useSceneryStore()
    await store.loadData()

    await store.updateCategory('Unknown Scenery', SceneryCategory.Library)
    expect(store.entries[0]?.category).toBe(SceneryCategory.Library)
    expect(store.hasLocalChanges).toBe(true)
    expect(tauri.invoke).not.toHaveBeenCalledWith('update_scenery_entry', expect.anything())

    store.resetChanges()
    expect(store.entries[0]?.category).toBe(SceneryCategory.Unrecognized)
    expect(store.hasLocalChanges).toBe(false)
  })

  it('persists the selected category together with the applied order', async () => {
    const appStore = useAppStore()
    appStore.xplanePath = '/xplane'
    const store = useSceneryStore()
    await store.loadData()

    await store.updateCategory('Unknown Scenery', SceneryCategory.Mesh)
    await store.applyChanges()

    expect(tauri.invoke).toHaveBeenCalledWith('apply_scenery_changes', {
      xplanePath: '/xplane',
      entries: [
        {
          folderName: 'Unknown Scenery',
          enabled: false,
          sortOrder: 0,
          category: SceneryCategory.Mesh,
        },
      ],
    })
    expect(store.hasLocalChanges).toBe(false)
  })
})
