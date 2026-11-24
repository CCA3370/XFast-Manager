import { createPinia, setActivePinia } from 'pinia'
import { reactive, ref, watch } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  getItem: vi.fn(),
  setItem: vi.fn(),
  isLocked: vi.fn(),
  setLocked: vi.fn(),
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
    isLocked: mocks.isLocked,
    setLocked: mocks.setLocked,
    applyLockState: mocks.locks,
    exportLockState: () => ({ aircraft: [], plugin: [], scenery: [], lua: [], navdata: [] }),
    initStore: vi.fn().mockResolvedValue(undefined),
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
  mocks.isLocked.mockReturnValue(false)
  mocks.setLocked.mockResolvedValue(undefined)
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

  it('discards an aircraft update check whose settings load outlives the selected root', async () => {
    const pending = deferred<null>()
    mocks.getItem.mockReturnValue(pending.promise)
    mocks.invoke.mockResolvedValueOnce({
      entries: [{ folderName: 'same', updateUrl: 'https://example.test/main', hasUpdate: false }],
      totalCount: 1,
      enabledCount: 1,
    })
    const store = useManagementStore()
    const check = store.checkAircraftUpdates(true, true)
    const finished = expect(check).resolves.toBeUndefined()
    await vi.waitFor(() => expect(mocks.getItem).toHaveBeenCalled())
    app.xplanePath = '/B'
    pending.resolve(null)
    await finished
    expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith('scan_aircraft', { xplanePath: '/A' })
    expect(store.isCheckingUpdates).toBe(false)
  })

  it('keeps a manual plugin lock in B when its same-named copy was auto-locked in A', async () => {
    const store = useManagementStore()
    mocks.invoke.mockImplementation(
      async (command: string, args: { xplanePath: string }) =>
        command === 'toggle_management_item' && args.xplanePath === '/B',
    )
    await store.toggleEnabled('plugin', 'same')
    expect(mocks.setLocked).toHaveBeenCalledExactlyOnceWith('plugin', 'same', true, '/A')

    app.xplanePath = '/B'
    mocks.isLocked.mockReturnValue(true)
    await store.toggleEnabled('plugin', 'same')
    expect(mocks.setLocked).toHaveBeenCalledTimes(1)

    app.xplanePath = '/A'
    mocks.invoke.mockResolvedValue(true)
    await store.toggleEnabled('plugin', 'same')
    expect(mocks.setLocked).toHaveBeenLastCalledWith('plugin', 'same', false, '/A')
  })

  it('isolates per-item beta preferences without changing global update options', async () => {
    mocks.invoke.mockResolvedValue({})
    const store = useManagementStore()
    await store.setAddonUpdateItemBetaPreference('aircraft', 'same', true)
    await store.buildAddonUpdatePlan('aircraft', 'same')
    expect(mocks.invoke).toHaveBeenLastCalledWith(
      'build_addon_update_plan',
      expect.objectContaining({
        options: expect.objectContaining({ useBeta: true }),
      }),
    )
    app.xplanePath = '/B'
    await store.buildAddonUpdatePlan('aircraft', 'same')
    expect(mocks.invoke.mock.lastCall?.[1].options.useBeta).toBeUndefined()
    await store.setAddonUpdateItemBetaPreference('aircraft', 'same', false)
    await store.buildAddonUpdatePlan('aircraft', 'same')
    expect(mocks.invoke).toHaveBeenLastCalledWith(
      'build_addon_update_plan',
      expect.objectContaining({
        options: expect.objectContaining({ useBeta: false }),
      }),
    )
    const scopedKeys = mocks.getItem.mock.calls
      .map(([key]) => key)
      .filter((key) => typeof key === 'string' && key.includes('betaItems'))
    expect(new Set(scopedKeys).size).toBe(2)
  })

  it('preserves both legacy channel choices when preferences are saved concurrently', async () => {
    const store = useManagementStore()
    const pending = deferred<undefined>()
    mocks.setItem.mockReturnValueOnce(pending.promise)
    const first = store.setAddonUpdateItemBetaPreference('aircraft', 'first', true)
    const second = store.setAddonUpdateItemBetaPreference('plugin', 'second', false)
    await vi.waitFor(() => expect(mocks.setItem).toHaveBeenCalled())
    expect(mocks.setItem).toHaveBeenCalledTimes(1)

    pending.resolve(undefined)
    await Promise.all([first, second])
    expect(mocks.setItem.mock.lastCall?.[1]).toEqual({
      'aircraft:first': true,
      'plugin:second': false,
    })
  })

  it('rechecks SkunkCrafts replicas that share a Main URL and follow different channels', async () => {
    const store = useManagementStore()
    const items = ref([
      {
        folderName: 'Beta copy',
        updateUrl: 'https://example.test/main',
        version: '2',
        hasUpdate: false,
      },
      {
        folderName: 'Stable copy',
        updateUrl: 'https://example.test/main',
        version: '1',
        hasUpdate: false,
      },
    ])
    const check = () =>
      store.checkItemUpdates({
        itemsRef: items,
        checkCommand: 'check_aircraft_updates',
        checkParamName: 'aircraft',
        itemType: 'aircraft',
        logName: 'aircraft',
      })
    mocks.invoke
      .mockResolvedValueOnce(
        items.value.map((item) => ({
          ...item,
          latestVersion: item.version,
        })),
      )
      .mockResolvedValueOnce(
        items.value.map((item) => ({
          ...item,
          latestVersion: item.folderName === 'Beta copy' ? '3' : '1',
          hasUpdate: item.folderName === 'Beta copy',
        })),
      )

    await check()
    await check()

    expect(mocks.invoke).toHaveBeenCalledTimes(2)
    expect(items.value.map((item) => item.hasUpdate)).toEqual([true, false])
  })

  it('discards a superseded SkunkCrafts check when the new channel is unavailable', async () => {
    const store = useManagementStore()
    const item = {
      folderName: 'Aircraft',
      updateUrl: 'https://example.test/main',
      version: '2',
      hasUpdate: false,
    }
    const items = ref([item])
    const check = () =>
      store.checkItemUpdates({
        itemsRef: items,
        checkCommand: 'check_aircraft_updates',
        checkParamName: 'aircraft',
        itemType: 'aircraft',
        logName: 'aircraft',
      })
    const pending = deferred<unknown>()
    mocks.invoke.mockReturnValueOnce(pending.promise).mockResolvedValueOnce([item])

    const oldCheck = check()
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(1))
    expect(await check()).toEqual({ checked: true, updateCount: 0, failed: true })
    pending.resolve([{ ...item, latestVersion: '1', hasUpdate: true }])
    await oldCheck

    expect(items.value[0].hasUpdate).toBe(false)
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

  it('finishes a submitted preset on its starting installation after a switch', async () => {
    const pending = deferred<unknown>()
    mocks.invoke.mockReturnValue(pending.promise)
    const store = usePresetsStore()
    const applying = store.applyPreset(1)
    app.xplanePath = '/B'
    expect(store.isApplying).toBe(false)
    app.xplanePath = '/A/'
    expect(store.isApplying).toBe(true)
    await expect(store.applyPreset(2)).rejects.toThrow('already being applied')
    app.xplanePath = '/B'
    pending.resolve({
      changesMade: 1,
      errors: [],
      missingItems: [],
      lockState: { aircraft: ['same'] },
    })
    await applying
    expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith('apply_preset', {
      xplanePath: '/A',
      presetId: 1,
    })
    expect(mocks.locks).toHaveBeenCalledExactlyOnceWith({ aircraft: ['same'], navdata: [] }, '/A')
    app.xplanePath = '/A'
    expect(store.isApplying).toBe(false)
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

  it('discards results if the installation changes before their consumers resume', async () => {
    const diskResponse = deferred<unknown>()
    const presetResponse = deferred<unknown>()
    const activityResponse = deferred<unknown>()
    mocks.invoke
      .mockReturnValueOnce(diskResponse.promise)
      .mockReturnValueOnce(presetResponse.promise)
      .mockReturnValueOnce(activityResponse.promise)

    const disk = useDiskUsageStore()
    const presets = usePresetsStore()
    const activity = useActivityLogStore()
    const pending = [disk.scan(), presets.loadPresets(), activity.loadRecent()]
    diskResponse.resolve({ totalBytes: 123, categories: [], scanDurationMs: 1 })
    presetResponse.resolve([{ id: 1, name: 'Preset from A' }])
    activityResponse.resolve({ entries: [{ id: 1, itemName: 'Add-on from A' }], totalCount: 1 })
    // Let the scoped IPC requests resolve before the stores consume their results.
    await Promise.resolve()
    app.xplanePath = '/B'
    await Promise.all(pending)

    expect(disk.report).toBeNull()
    expect(presets.presets).toEqual([])
    expect(activity.entries).toEqual([])
  })
})
