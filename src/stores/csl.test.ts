import { createPinia, setActivePinia } from 'pinia'
import { reactive } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { rootStorageKey } from '@/utils/xplaneStorage'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
  getItem: vi.fn<(key: string) => Promise<unknown>>(),
  setItem: vi.fn(async (_key: string, _value: unknown) => {}),
}))

const app = reactive({ xplanePath: '/XP/A' })
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
vi.mock('./app', () => ({ useAppStore: () => app }))
vi.mock('./management', () => ({
  useManagementStore: () => ({ addonUpdateOptions: {}, loadAddonUpdateOptions: async () => {} }),
}))
vi.mock('./toast', () => ({
  useToastStore: () => ({ info: vi.fn(), error: vi.fn(), warning: vi.fn(), success: vi.fn() }),
}))
vi.mock('@/services/logger', () => ({ logDebug: vi.fn(), logError: vi.fn() }))
vi.mock('@/services/storage', () => ({
  getItem: mocks.getItem,
  setItem: mocks.setItem,
  STORAGE_KEYS: {
    CSL_CUSTOM_PATHS: 'cslCustomPaths',
    CSL_INSTALL_LOCATION: 'cslInstallLocation',
    CSL_SERVER_BASE_URLS: 'cslServerBaseUrls',
    CSL_ACTIVE_SERVER_BASE_URL: 'cslActiveServerBaseUrl',
  },
}))

import { useCslStore } from './csl'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise
  })
  return { promise, resolve }
}

beforeEach(() => {
  app.xplanePath = '/XP/A'
  setActivePinia(createPinia())
  vi.clearAllMocks()
  mocks.getItem.mockResolvedValue(null)
  mocks.invoke.mockResolvedValue(undefined)
})

describe('CSL X-Plane root isolation', () => {
  it('ignores old-root CSL scan results even after switching back', async () => {
    const first = deferred<unknown>()
    mocks.invoke.mockImplementation((command) => {
      if (command === 'csl_scan_packages') return first.promise
      return Promise.resolve(undefined)
    })
    const store = useCslStore()
    const pending = store.scanPackages()
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        'csl_scan_packages',
        expect.objectContaining({ xplanePath: '/XP/A' }),
      ),
    )
    app.xplanePath = '/XP/B'
    app.xplanePath = '/XP/A'
    first.resolve({
      packages: [{ name: 'old' }],
      paths: [{ source: 'auto', path: '/XP/A/old' }],
      server_version: 'old',
    })
    await pending
    expect(store.packages).toEqual([])
    expect(store.paths).toEqual([])
    expect(store.serverVersion).toBe('')
    expect(store.lastScannedXplanePath).toBe('')
  })

  it('isolates scoped path preferences and discards a late preference read', async () => {
    const first = deferred<string[]>()
    mocks.getItem.mockImplementation((key) => {
      if (key === rootStorageKey('cslCustomPaths', '/XP/A')) return first.promise
      if (key === rootStorageKey('cslCustomPaths', '/XP/B')) return Promise.resolve(['/XP/B/custom'])
      if (key === rootStorageKey('cslInstallLocation', '/XP/B'))
        return Promise.resolve('/XP/B/install')
      return Promise.resolve(null)
    })
    const store = useCslStore()
    const pending = store.ensureCustomPathsLoaded()
    app.xplanePath = '/XP/B'
    await Promise.all([store.ensureCustomPathsLoaded(), store.ensureInstallLocationLoaded()])
    first.resolve(['/XP/A/late'])
    await pending
    expect(store.customPaths).toEqual(['/XP/B/custom'])
    expect(store.resolvedInstallLocation).toBe('/XP/B/install')
    await store.setInstallLocation('/XP/B/next')
    expect(mocks.setItem).toHaveBeenCalledWith(
      rootStorageKey('cslInstallLocation', '/XP/B'),
      '/XP/B/next',
    )
  })

  it('drops queued old-root work and never rescans the new root after the old install', async () => {
    const first = deferred<unknown>()
    mocks.invoke.mockImplementation((command, args) => {
      if (command === 'csl_install_package' && args?.xplanePath === '/XP/A') return first.promise
      if (command === 'csl_rescan_packages') return Promise.resolve([])
      return Promise.resolve(undefined)
    })
    const store = useCslStore()
    store.installPackage('old-active')
    store.installPackage('old-queued')
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        'csl_install_package',
        expect.objectContaining({ xplanePath: '/XP/A', packageName: 'old-active' }),
      ),
    )
    app.xplanePath = '/XP/B'
    store.installPackage('new')
    first.resolve(undefined)
    await vi.waitFor(() => expect(store.hasPendingInstalls).toBe(false))
    const installs = mocks.invoke.mock.calls.filter(([command]) => command === 'csl_install_package')
    expect(installs.map(([, args]) => [args?.xplanePath, args?.packageName])).toEqual([
      ['/XP/A', 'old-active'],
      ['/XP/B', 'new'],
    ])
    expect(mocks.invoke).not.toHaveBeenCalledWith(
      'csl_rescan_packages',
      expect.objectContaining({ packageNames: ['old-active'] }),
    )
  })

  it('ignores progress from a previous install of the same package', async () => {
    const a = deferred<unknown>()
    const b = deferred<unknown>()
    mocks.invoke.mockImplementation((command, args) => {
      if (command === 'csl_install_package') {
        return args?.xplanePath === '/XP/A' ? a.promise : b.promise
      }
      if (command === 'csl_rescan_packages') return Promise.resolve([])
      return Promise.resolve(undefined)
    })
    const store = useCslStore()
    const installerCalls = () =>
      mocks.invoke.mock.calls.filter(([command]) => command === 'csl_install_package')
    store.installPackage('same')
    await vi.waitFor(() => expect(installerCalls()).toHaveLength(1))
    app.xplanePath = '/XP/B'
    store.installPackage('same')
    a.resolve(undefined)
    await vi.waitFor(() => expect(installerCalls()).toHaveLength(2))
    const progress = {
      package_name: 'same',
      current_file: 1,
      total_files: 2,
      current_file_name: 'part',
      bytes_downloaded: 4,
      total_bytes: 8,
    }
    store.updateProgress({ ...progress, request_id: String(installerCalls()[0][1]?.requestId) })
    expect(store.progressMap).toEqual({})
    store.updateProgress({ ...progress, request_id: String(installerCalls()[1][1]?.requestId) })
    expect(store.progressMap.same).toMatchObject(progress)
    b.resolve(undefined)
    await vi.waitFor(() => expect(store.hasPendingInstalls).toBe(false))
  })
})
