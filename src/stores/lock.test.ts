import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from './app'
import { useLockStore } from './lock'

const mocks = vi.hoisted(() => ({
  getItem: vi.fn<(key: string) => Promise<unknown>>(),
  setItem: vi.fn<(key: string, value: unknown) => Promise<void>>(),
  invoke: vi.fn(),
}))

vi.mock('@/services/storage', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/storage')>()),
  getItem: mocks.getItem,
  setItem: mocks.setItem,
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))

describe('installation-scoped locks', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetAllMocks()
    mocks.getItem.mockResolvedValue(null)
    mocks.setItem.mockResolvedValue(undefined)
  })

  it('isolates same-named scenery and checks target containment', async () => {
    const app = useAppStore()
    const locks = useLockStore()
    app.xplanePath = 'C:\\XP-A'
    await locks.setLocked('scenery', 'Shared', true)
    expect(locks.isPathLocked('c:/xp-a/Custom Scenery/Shared', app.xplanePath)).toBe(true)
    expect(locks.isPathLocked('c:/xp-ab/Custom Scenery/Shared', app.xplanePath)).toBe(false)
    app.xplanePath = 'D:/XP-B'
    expect(locks.getLockedItems('scenery')).toEqual([])
    expect(locks.isPathLocked('D:/XP-B/Custom Scenery/Shared', app.xplanePath)).toBe(false)
    app.xplanePath = 'c:/xp-a/'
    expect(locks.getLockedItems('scenery')).toEqual(['shared'])
  })

  it('does not assign legacy global locks to the first selected root', async () => {
    const app = useAppStore()
    const locks = useLockStore()
    mocks.getItem.mockImplementation(async (key) =>
      key === 'lockedItems' ? { scenery: ['shared'] } : null,
    )
    app.xplanePath = '/XP-B'
    await locks.initStore()
    expect(locks.getLockedItems('scenery')).toEqual([])
    expect(mocks.setItem).not.toHaveBeenCalled()
  })

  it('uses the initiating root when the switch happens during persistence', async () => {
    const app = useAppStore()
    const locks = useLockStore()
    app.xplanePath = '/XP-A'
    let release!: () => void
    mocks.setItem.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        }),
    )
    const pending = locks.toggleLock('plugin', 'Shared')
    await vi.waitFor(() => expect(mocks.setItem).toHaveBeenCalled())
    app.xplanePath = '/XP-B'
    release()
    await expect(pending).resolves.toBe(true)
    expect(mocks.invoke).toHaveBeenCalledWith('set_cfg_disabled', {
      xplanePath: '/XP-A',
      itemType: 'plugin',
      folderName: 'Shared',
      disabled: true,
    })
    expect(locks.isLocked('plugin', 'Shared')).toBe(false)
    expect(locks.isLocked('plugin', 'Shared', '/XP-A')).toBe(true)
  })
})
