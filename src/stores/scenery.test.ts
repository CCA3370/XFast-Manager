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
      reorder: true,
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

  it('keeps file order when applying only enable changes with automatic sorting off', async () => {
    const app = useAppStore()
    app.xplanePath = '/xplane'
    app.autoSortScenery = false
    const store = useSceneryStore()
    await store.loadData()
    const first = {
      ...createEntry(),
      folderName: 'Mesh first',
      category: SceneryCategory.Mesh,
      sortOrder: 0,
    }
    const second = {
      ...createEntry(),
      folderName: 'Airport second',
      category: SceneryCategory.Airport,
      sortOrder: 1,
    }
    store.data!.entries = [first, second]
    store.toggleEnabled(first.folderName)
    await store.applyChanges([second, first]) // Category-grouped display is not a manual reorder.
    expect(tauri.invoke).toHaveBeenCalledWith('apply_scenery_changes', {
      xplanePath: '/xplane',
      reorder: false,
      entries: [
        {
          folderName: first.folderName,
          enabled: true,
          sortOrder: 0,
          category: SceneryCategory.Mesh,
        },
        {
          folderName: second.folderName,
          enabled: false,
          sortOrder: 1,
          category: SceneryCategory.Airport,
        },
      ],
    })
  })

  it('applies an explicitly dragged order while automatic sorting is off', async () => {
    const app = useAppStore()
    app.xplanePath = '/xplane'
    app.autoSortScenery = false
    const store = useSceneryStore()
    await store.loadData()
    const first = { ...createEntry(), folderName: 'First', sortOrder: 0 }
    const second = { ...createEntry(), folderName: 'Second', sortOrder: 1 }
    store.data!.entries = [first, second]
    await store.reorderEntries([second, first])
    await store.applyChanges()
    const payload = tauri.invoke.mock.calls
      .slice()
      .reverse()
      .find(([command]) => command === 'apply_scenery_changes')![1]!
    expect(payload.reorder).toBe(true)
    expect(
      (payload.entries as Array<{ folderName: string }>).map((entry) => entry.folderName),
    ).toEqual(['Second', 'First'])
    await store.applyChanges()
    expect(
      tauri.invoke.mock.calls
        .slice()
        .reverse()
        .find(([command]) => command === 'apply_scenery_changes')![1]!.reorder,
    ).toBe(false)
  })
})

describe('Scenery store installation isolation', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  function dataFor(folderName: string): SceneryManagerData {
    return { ...createData(), entries: [{ ...createEntry(), folderName }] }
  }

  it('drops an old load after switching A → B → A, even though the path matches', async () => {
    const app = useAppStore()
    app.xplanePath = '/xp/A'
    const store = useSceneryStore()
    let finishOld!: (data: SceneryManagerData) => void
    let oldRequest!: () => void
    const oldInvoked = new Promise<void>((resolve) => {
      oldRequest = resolve
    })
    let aLoads = 0
    tauri.invoke.mockImplementation(async (command, args) => {
      if (command === 'get_scenery_index_status') return { indexExists: true }
      if (command !== 'get_scenery_manager_data') return undefined
      if (args?.xplanePath === '/xp/B') return dataFor('B')
      if (++aLoads > 1) return dataFor('A fresh')
      return new Promise((resolve) => {
        finishOld = resolve
        oldRequest()
      })
    })
    const oldLoad = store.loadData()
    await oldInvoked
    app.xplanePath = '/xp/B'
    expect(store.indexExists).toBe(false)
    expect(store.entries).toEqual([])
    await store.loadData()
    expect(store.entries.map(({ folderName }) => folderName)).toEqual(['B'])
    app.xplanePath = '/xp/A'
    await store.loadData()
    finishOld(dataFor('A stale'))
    await oldLoad
    expect(store.entries.map(({ folderName }) => folderName)).toEqual(['A fresh'])
    expect(store.hasLocalChanges).toBe(false)
  })

  it('clears staged edits and ignores completion of a save to the prior root', async () => {
    const app = useAppStore()
    app.xplanePath = '/xp/A'
    const store = useSceneryStore()
    let finishSave!: () => void
    tauri.invoke.mockImplementation(async (command, args) => {
      if (command === 'get_scenery_index_status') return { indexExists: true }
      if (command === 'get_scenery_manager_data') {
        return { ...dataFor('same-folder'), needsSync: args?.xplanePath === '/xp/B' }
      }
      if (command === 'apply_scenery_changes') {
        return new Promise<void>((resolve) => {
          finishSave = resolve
        })
      }
    })
    await store.loadData()
    store.toggleEnabled('same-folder')
    expect(store.hasLocalChanges).toBe(true)
    const saving = store.applyChanges()
    app.xplanePath = '/xp/B'
    expect(store.hasLocalChanges).toBe(false)
    await store.loadData()
    finishSave()
    await saving
    expect(store.data?.needsSync).toBe(true)
    expect(store.entries[0]?.enabled).toBe(false)
    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'apply_scenery_changes'),
    ).toEqual([['apply_scenery_changes', expect.objectContaining({ xplanePath: '/xp/A' })]])
  })

  it('does not send a scenery data request if its index status returned after a root switch', async () => {
    const app = useAppStore()
    app.xplanePath = '/xp/A'
    const store = useSceneryStore()
    let finishStatus!: (data: { indexExists: boolean }) => void
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'get_scenery_index_status')
        return new Promise((resolve) => {
          finishStatus = resolve
        })
    })
    const load = store.loadData()
    app.xplanePath = '/xp/B'
    finishStatus({ indexExists: true })
    await load
    expect(store.indexExists).toBe(false)
    expect(tauri.invoke).not.toHaveBeenCalledWith('get_scenery_manager_data', expect.anything())
  })
})
