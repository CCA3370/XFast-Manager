import { createPinia, setActivePinia } from 'pinia'
import { reactive, watch } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  getItem: vi.fn(),
  setItem: vi.fn(),
  locks: vi.fn(),
}))
const app = reactive({ xplanePath: '/A' })
let generation = 0
watch(
  () => app.xplanePath,
  () => {
    generation += 1
  },
  { flush: 'sync' },
)
vi.mock('./app', () => ({
  useAppStore: () =>
    Object.assign(app, {
      get xplaneGeneration() {
        return generation
      },
      captureXplaneContext: () => {
        const captured = generation
        return { path: app.xplanePath, isCurrent: () => captured === generation }
      },
    }),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn(), open: vi.fn() }))
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
vi.mock('./toast', () => ({ useToastStore: () => ({ error: vi.fn(), info: vi.fn() }) }))
vi.mock('./lock', () => ({
  useLockStore: () => ({
    isLocked: () => false,
    setLocked: vi.fn(),
    applyLockState: mocks.locks,
    exportLockState: () => ({ aircraft: [], plugin: [], scenery: [], lua: [], navdata: [] }),
  }),
}))
vi.mock('@/services/logger', () => ({ logError: vi.fn() }))
vi.mock('@/utils/validation', () => ({ validateXPlanePath: () => !!app.xplanePath }))
vi.mock('@/services/storage', () => ({
  getItem: mocks.getItem,
  setItem: mocks.setItem,
  STORAGE_KEYS: { ADDON_UPDATE_ITEM_BETA_PREFERENCES: 'betaItems' },
}))

import { useDiskUsageStore } from './diskUsage'
import { useManagementStore } from './management'
import { usePresetsStore } from './presets'
import { useActivityLogStore } from './activityLog'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

beforeEach(() => {
  setActivePinia(createPinia())
  app.xplanePath = '/A'
  vi.clearAllMocks()
  mocks.getItem.mockResolvedValue(null)
  mocks.setItem.mockResolvedValue(undefined)
})

describe('installation-bound store operations', () => {
  it('discards a disk scan after A → B → A', async () => {
    const pending = deferred<unknown>()
    mocks.invoke.mockReturnValue(pending.promise)
    const store = useDiskUsageStore()
    const scan = store.scan()
    app.xplanePath = '/B'
    app.xplanePath = '/A'
    pending.resolve({ totalBytes: 123, categories: [], scanDurationMs: 1 })
    await scan
    expect(store.report).toBeNull()
    expect(store.error).toBeNull()
    expect(store.isScanning).toBe(false)
  })

  it('does not start an update for B after waiting for A settings', async () => {
    const pending = deferred<null>()
    mocks.getItem.mockReturnValue(pending.promise)
    const store = useManagementStore()
    const operation = store.executeAddonUpdate('aircraft', 'same')
    const rejected = expect(operation).rejects.toThrow('directory changed')
    app.xplanePath = '/B'
    pending.resolve(null)
    await rejected
    expect(mocks.invoke).not.toHaveBeenCalled()
  })

  it('isolates per-item beta preferences without changing global update options', async () => {
    const store = useManagementStore()
    await store.setAddonUpdateItemBetaPreference('aircraft', 'same', true)
    expect(store.isAddonUpdateBetaEnabled('aircraft', 'same')).toBe(true)
    app.xplanePath = '/B'
    await store.loadAddonUpdateOptions()
    expect(store.isAddonUpdateBetaEnabled('aircraft', 'same')).toBe(false)
    const scopedKeys = mocks.getItem.mock.calls
      .map(([key]) => key)
      .filter((key) => typeof key === 'string' && key.includes('betaItems'))
    expect(new Set(scopedKeys).size).toBe(2)
  })

  it('does not populate the new root with an old aircraft scan', async () => {
    const pending = deferred<unknown>()
    mocks.invoke.mockReturnValue(pending.promise)
    const store = useManagementStore()
    const scan = store.loadAircraft()
    app.xplanePath = '/B'
    pending.resolve({
      entries: [{ folderName: 'same', displayName: 'old' }],
      totalCount: 1,
      enabledCount: 1,
    })
    await scan
    expect(store.aircraft).toEqual([])
    expect(store.aircraftTotalCount).toBe(0)
    expect(store.error).toBeNull()
  })

  it('stops a batch delete before sending remaining names to a new root', async () => {
    const pending = deferred<unknown>()
    mocks.invoke.mockReturnValue(pending.promise)
    const store = useManagementStore()
    const batch = store.batchDeleteItems('plugin', ['one', 'two'])
    const rejected = expect(batch).rejects.toThrow('directory changed')
    app.xplanePath = '/B'
    pending.resolve(undefined)
    await rejected
    expect(mocks.invoke).toHaveBeenCalledTimes(1)
    expect(mocks.invoke).toHaveBeenCalledWith(
      'delete_management_item',
      expect.objectContaining({ xplanePath: '/A', folderName: 'one' }),
    )
  })

  it('does not apply an old preset lock snapshot to a replacement root', async () => {
    const pending = deferred<unknown>()
    mocks.invoke.mockReturnValue(pending.promise)
    const store = usePresetsStore()
    const applying = store.applyPreset(1)
    const rejected = expect(applying).rejects.toThrow('directory changed')
    app.xplanePath = '/B'
    pending.resolve({
      changesMade: 1,
      errors: [],
      missingItems: [],
      lockState: { aircraft: ['same'] },
    })
    await rejected
    expect(mocks.locks).not.toHaveBeenCalled()
  })

  it('passes root for preset and activity queries, and does not query an empty root', async () => {
    mocks.invoke.mockImplementation(async (command: string) =>
      command === 'list_presets' ? [] : { entries: [], totalCount: 0 },
    )
    const presets = usePresetsStore()
    const activity = useActivityLogStore()
    await presets.loadPresets()
    await activity.loadRecent()
    expect(mocks.invoke).toHaveBeenCalledWith('list_presets', { xplanePath: '/A' })
    expect(mocks.invoke).toHaveBeenCalledWith(
      'get_activity_log',
      expect.objectContaining({ xplanePath: '/A' }),
    )
    mocks.invoke.mockClear()
    app.xplanePath = ''
    await presets.loadPresets()
    await activity.loadRecent()
    expect(mocks.invoke).not.toHaveBeenCalled()
  })
})
