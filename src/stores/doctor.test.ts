import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DoctorEnvironmentReport, DoctorXfastHealthReport } from '@/types'
import type { DoctorRun } from '@/types/doctor'

const tauri = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))

const history = vi.hoisted(() => ({
  loadDoctorRunsForPath: vi.fn(async () => []),
  saveDoctorRun: vi.fn(async (_path: string, run: unknown) => [run]),
}))

const locks = vi.hoisted(() => ({
  isInitialized: true,
  initStore: vi.fn(async () => {}),
  getLockedItems: vi.fn(() => ['Locked Scenery']),
}))

const storage = vi.hoisted(() => ({
  getItem: vi.fn<(key: string) => Promise<unknown>>(async () => null),
  setItem: vi.fn(async () => {}),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke }))
vi.mock('@/services/doctorHistory', () => history)
vi.mock('@/services/logger', () => ({ logError: vi.fn() }))
vi.mock('@/services/storage', () => ({
  getItem: storage.getItem,
  setItem: storage.setItem,
  STORAGE_KEYS: {
    INCLUDE_PRE_RELEASE: 'includePreRelease',
    ADDON_UPDATE_ITEM_BETA_PREFERENCES: 'addonUpdateItemBetaPreferences',
    CSL_CUSTOM_PATHS: 'cslCustomPaths',
    CSL_INSTALL_LOCATION: 'cslInstallLocation',
    CSL_ACTIVE_SERVER_BASE_URL: 'cslActiveServerBaseUrl',
  },
}))
vi.mock('./lock', () => ({ useLockStore: () => locks }))

import { useAppStore } from './app'
import { useDoctorStore } from './doctor'

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}

function historyRun(id: string, installationId: string): DoctorRun {
  return {
    schemaVersion: 1,
    id,
    installationId,
    mode: 'quick',
    state: 'completed',
    startedAt: 1,
    completedAt: 2,
    durationMs: 1,
    appVersion: '1.2.5',
    checks: [],
    summary: {
      severity: 'ok',
      completeness: 'complete',
      total: 0,
      eligible: 0,
      covered: 0,
      coveragePercent: 100,
      pass: 0,
      info: 0,
      warning: 0,
      critical: 0,
      unavailable: 0,
      notApplicable: 0,
      cancelled: 0,
    },
    system: null,
  }
}

function environment(): DoctorEnvironmentReport {
  return {
    installationId: 'installation-1',
    rootExists: true,
    executablePresent: true,
    logPresent: true,
    freeBytes: 50 * 1024 * 1024 * 1024,
    totalBytes: 500 * 1024 * 1024 * 1024,
    inProgramFiles: false,
    isSteamInstall: false,
    missingCoreDirs: [],
    readonlyCount: 0,
    readonlyScanCapped: false,
    readonlyScanPerformed: false,
    injectors: [],
    competingOrganizers: [],
    system: {
      os: 'linux',
      osVersion: 'test',
      architecture: 'x86_64',
      cpuModel: 'Test CPU',
      logicalCores: 8,
      totalMemoryBytes: 32 * 1024 * 1024 * 1024,
      availableMemoryBytes: 16 * 1024 * 1024 * 1024,
    },
  }
}

function xfast(indexMissing = false): DoctorXfastHealthReport {
  return {
    appDataDir: '/data/xfast',
    appDataWritable: true,
    appDataWriteError: null,
    appDataFreeBytes: 20 * 1024 * 1024 * 1024,
    appDataTotalBytes: 100 * 1024 * 1024 * 1024,
    databaseOk: true,
    databaseDetail: 'ok',
    schemaCompatible: true,
    sceneryIndex: {
      indexExists: true,
      indexedCount: indexMissing ? 1 : 2,
      filesystemCount: 2,
      missingFromIndex: indexMissing ? ['New Scenery'] : [],
      missingFromDisk: [],
    },
  }
}

async function healthyResponse(command: string): Promise<unknown> {
  switch (command) {
    case 'get_app_version':
      return '1.2.5'
    case 'validate_xplane_path':
      return true
    case 'doctor_scan_environment':
      return environment()
    case 'is_xplane_running':
      return false
    case 'doctor_scan_xfast_health':
      return xfast()
    case 'analyze_xplane_log':
      return {
        log_path: '/xplane/Log.txt',
        is_xplane_log: true,
        crash_detected: false,
        crash_info: null,
        issues: [],
        system_info: {
          xplane_version: '12.1.4-r1',
          gpu_model: 'Test GPU',
          gpu_driver: '1.0',
        },
      }
    case 'get_scenery_manager_data':
      return {
        entries: [],
        totalCount: 0,
        enabledCount: 0,
        missingDepsCount: 0,
        duplicateTilesCount: 0,
        duplicateAirportsCount: 0,
        needsSync: false,
        tileOverlaps: {},
      }
    case 'airport_flatten_list_overrides':
      return []
    case 'doctor_navdata_status':
      return {
        cycles: [],
        customDataExists: false,
        cifpPresent: false,
        earthDatMissing: [],
      }
    case 'get_activity_log':
      return { entries: [], totalCount: 0 }
    case 'scan_output_cleanup_items':
      return { totalBytes: 0, totalFiles: 0 }
    case 'get_platform':
      return 'linux'
    case 'scan_aircraft':
    case 'scan_plugins':
      return { entries: [] }
    case 'gateway_list_installed':
      return []
    case 'check_for_updates':
      return { isUpdateAvailable: false, latestVersion: '1.2.5' }
    case 'csl_scan_packages':
    case 'altitude_scan_packages':
      return { packages: [], paths: [], server_version: '', index_warning: null }
    case 'quick_scan_scenery_index':
      return { indexExists: true, added: [], removed: [], updated: [] }
    default:
      return undefined
  }
}

function installHealthyMocks() {
  tauri.invoke.mockImplementation(async (command) => healthyResponse(command))
}

describe('Health diagnostic store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    history.loadDoctorRunsForPath.mockResolvedValue([])
    history.saveDoctorRun.mockImplementation(async (_path: string, run: unknown) => [run])
    storage.getItem.mockResolvedValue(null)
    installHealthyMocks()
    const appStore = useAppStore()
    appStore.xplanePath = '/xplane'
  })

  it('completes a quick run with full coverage and persists it by installation', async () => {
    const store = useDoctorStore()

    await store.runDiagnostics('quick')

    expect(store.currentRun?.state).toBe('completed')
    expect(store.currentRun?.installationId).toBe('installation-1')
    expect(store.currentRun?.summary.completeness).toBe('complete')
    expect(store.currentRun?.summary.coveragePercent).toBe(100)
    expect(store.currentRun?.summary.unavailable).toBe(0)
    expect(history.saveDoctorRun).toHaveBeenCalledOnce()

    const runId = store.currentRun?.id
    await store.loadHistory()
    expect(store.currentRun?.id).toBe(runId)
    expect(store.currentRunIsLive).toBe(true)
  })

  it('marks a failed probe unavailable instead of reporting an all-clear result', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'doctor_scan_xfast_health') throw new Error('database locked')
      return healthyResponse(command)
    })
    const store = useDoctorStore()

    await store.runDiagnostics('quick')

    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'diagnostic.xfast', outcome: 'unavailable' }),
      ]),
    )
    expect(store.currentRun?.summary.completeness).toBe('partial')
    expect(store.currentRun?.summary.coveragePercent).toBeLessThan(100)
  })

  it('runs the full local and network inventory with complete coverage', async () => {
    const store = useDoctorStore()

    await store.runDiagnostics('full')

    expect(store.currentRun?.mode).toBe('full')
    expect(store.currentRun?.summary.completeness).toBe('complete')
    expect(store.currentRun?.summary.unavailable).toBe(0)
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'addons.inventory', outcome: 'pass' }),
        expect.objectContaining({ id: 'storage.cleanable', outcome: 'pass' }),
        expect.objectContaining({ id: 'updates.app', outcome: 'pass' }),
        expect.objectContaining({ id: 'updates.csl', outcome: 'pass' }),
      ]),
    )
  })

  it('cancels pending work and does not save an incomplete run to history', async () => {
    const neverFinishes = new Promise<DoctorEnvironmentReport>(() => {})
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'get_app_version') return '1.2.5'
      if (command === 'validate_xplane_path') return true
      if (command === 'doctor_scan_environment') return neverFinishes
      return undefined
    })
    const store = useDoctorStore()
    const running = store.runDiagnostics('quick')
    await vi.waitFor(() => expect(store.isRunning).toBe(true))

    store.cancelRun()
    await running

    expect(store.currentRun?.state).toBe('cancelled')
    expect(store.currentRun?.summary.completeness).toBe('cancelled')
    expect(store.currentRun?.summary.cancelled).toBeGreaterThan(0)
    expect(history.saveDoctorRun).not.toHaveBeenCalled()
  })

  it('rechecks only the affected XFast checks after a safe index refresh', async () => {
    let xfastScanCount = 0
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'doctor_scan_xfast_health') {
        xfastScanCount += 1
        return xfast(xfastScanCount === 1)
      }
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const check = store.currentRun?.checks.find((item) => item.id === 'xfast.scenery_index')
    expect(check?.outcome).toBe('warning')

    const applied = check ? await store.applyRemediation(check) : false

    expect(applied).toBe(true)
    expect(tauri.invoke).toHaveBeenCalledWith('quick_scan_scenery_index', {
      xplanePath: '/xplane',
      lockedFolderNames: ['Locked Scenery'],
    })
    expect(
      store.currentRun?.checks.find((item) => item.id === 'xfast.scenery_index')?.outcome,
    ).toBe('pass')
  })

  it('locks the scan synchronously so repeated clicks cannot start competing runs', async () => {
    const version = deferred<string>()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'get_app_version') return version.promise
      return healthyResponse(command)
    })
    const store = useDoctorStore()

    const firstRun = store.runDiagnostics('quick')
    expect(store.isRunning).toBe(true)

    await store.runDiagnostics('full')
    version.resolve('1.2.5')
    await firstRun

    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'get_app_version'),
    ).toHaveLength(1)
    expect(store.currentRun?.mode).toBe('quick')
    expect(store.currentRun?.state).toBe('completed')
  })

  it('invalidates an active scan when the configured installation changes', async () => {
    const environmentResult = deferred<DoctorEnvironmentReport>()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'doctor_scan_environment') return environmentResult.promise
      return healthyResponse(command)
    })
    const appStore = useAppStore()
    appStore.xplanePath = '/xplane-a'
    const store = useDoctorStore()
    const running = store.runDiagnostics('quick')
    await vi.waitFor(() =>
      expect(tauri.invoke).toHaveBeenCalledWith('doctor_scan_environment', {
        xplanePath: '/xplane-a',
        depth: 'quick',
      }),
    )

    appStore.xplanePath = '/xplane-b'
    await store.loadHistory()
    environmentResult.resolve(environment())
    await running

    expect(store.currentRun).toBeNull()
    expect(store.checkRuntime).toEqual([])
    expect(store.currentRunIsLive).toBe(false)
    expect(history.saveDoctorRun).not.toHaveBeenCalled()
    expect(history.loadDoctorRunsForPath).toHaveBeenCalledWith('/xplane-b')
  })

  it('ignores a stale history response after switching installations', async () => {
    const firstLoad = deferred<DoctorRun[]>()
    const secondLoad = deferred<DoctorRun[]>()
    history.loadDoctorRunsForPath.mockImplementation((path: string) =>
      path === '/xplane-a' ? firstLoad.promise : secondLoad.promise,
    )
    const appStore = useAppStore()
    const store = useDoctorStore()

    appStore.xplanePath = '/xplane-a'
    const loadingA = store.loadHistory()
    appStore.xplanePath = '/xplane-b'
    const loadingB = store.loadHistory()
    secondLoad.resolve([historyRun('run-b', 'installation-b')])
    await loadingB
    firstLoad.resolve([historyRun('run-a', 'installation-a')])
    await loadingA

    expect(store.history.map((run) => run.id)).toEqual(['run-b'])
    expect(store.currentRun?.id).toBe('run-b')
    expect(store.isHistoryLoading).toBe(false)
  })

  it('keeps the active run visible while a scan is in progress', async () => {
    history.loadDoctorRunsForPath.mockResolvedValue([historyRun('saved-run', 'saved-installation')])
    const environmentResult = deferred<DoctorEnvironmentReport>()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'doctor_scan_environment') return environmentResult.promise
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.loadHistory()
    const running = store.runDiagnostics('quick')
    await vi.waitFor(() => expect(store.isRunning).toBe(true))

    store.selectHistoryRun('saved-run')

    expect(store.selectedRunId).toBeNull()
    expect(store.displayedRun?.state).toBe('running')
    store.cancelRun()
    await running
  })

  it('refuses repairs for a selected history result or a different installation', async () => {
    let xfastScanCount = 0
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'doctor_scan_xfast_health') {
        xfastScanCount += 1
        return xfast(xfastScanCount === 1)
      }
      return healthyResponse(command)
    })
    const appStore = useAppStore()
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const check = store.currentRun?.checks.find((item) => item.id === 'xfast.scenery_index')
    expect(check?.remediation?.kind).toBe('automatic')

    store.selectHistoryRun(store.currentRun?.id ?? null)
    expect(check ? await store.applyRemediation(check) : true).toBe(false)

    store.selectHistoryRun(null)
    appStore.xplanePath = '/different-installation'
    expect(check ? await store.applyRemediation(check) : true).toBe(false)
    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'quick_scan_scenery_index'),
    ).toHaveLength(0)
  })

  it('keeps a cancelled repair recheck out of completed history', async () => {
    const recheck = deferred<DoctorXfastHealthReport>()
    let xfastScanCount = 0
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'doctor_scan_xfast_health') {
        xfastScanCount += 1
        return xfastScanCount === 1 ? xfast(true) : recheck.promise
      }
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const check = store.currentRun?.checks.find((item) => item.id === 'xfast.scenery_index')

    const repairing = check ? store.applyRemediation(check) : Promise.resolve(false)
    await vi.waitFor(() => expect(xfastScanCount).toBe(2))
    store.cancelRun()
    await repairing

    expect(store.currentRun?.state).toBe('cancelled')
    expect(store.currentRun?.summary.completeness).toBe('cancelled')
    expect(store.canRepairCurrentRun).toBe(false)
    expect(history.saveDoctorRun).toHaveBeenCalledTimes(1)
  })

  it('uses saved update channels and actionable remediation routes in a full diagnosis', async () => {
    storage.getItem.mockImplementation(async (key: string) => {
      if (key === 'includePreRelease') return true
      if (key === 'addonUpdateItemBetaPreferences') {
        return {
          'aircraft:Beta Aircraft': true,
          'plugin:Beta Plugin': true,
          'scenery:Beta Scenery': true,
        }
      }
      return null
    })
    tauri.invoke.mockImplementation(async (command, args) => {
      if (command === 'scan_aircraft') {
        return {
          entries: [
            {
              folderName: 'Beta Aircraft',
              displayName: 'Beta Aircraft',
              updateUrl: 'https://example.com/aircraft',
              hasUpdate: false,
            },
          ],
        }
      }
      if (command === 'scan_plugins') {
        return {
          entries: [
            {
              folderName: 'Beta Plugin',
              displayName: 'Beta Plugin',
              enabled: true,
              platform: 'win',
              updateUrl: 'https://example.com/plugin',
              hasUpdate: false,
            },
          ],
        }
      }
      if (command === 'get_scenery_manager_data') {
        return {
          entries: [
            {
              folderName: 'Beta Scenery',
              displayName: 'Beta Scenery',
              enabled: true,
              updateUrl: 'https://example.com/scenery',
              missingLibraries: [],
              hasUpdate: false,
            },
          ],
          totalCount: 1,
          enabledCount: 1,
          missingDepsCount: 0,
          duplicateTilesCount: 0,
          duplicateAirportsCount: 0,
          needsSync: false,
          tileOverlaps: {},
        }
      }
      if (command === 'check_aircraft_updates') return args?.aircraft ?? []
      if (command === 'check_plugins_updates') return args?.plugins ?? []
      if (command === 'check_scenery_updates') return args?.scenery ?? []
      if (command === 'check_for_updates') {
        return { isUpdateAvailable: true, latestVersion: '2.0.0' }
      }
      return healthyResponse(command)
    })
    const store = useDoctorStore()

    await store.runDiagnostics('full')

    expect(tauri.invoke).toHaveBeenCalledWith(
      'check_aircraft_updates',
      expect.objectContaining({ betaFolders: ['Beta Aircraft'] }),
    )
    expect(tauri.invoke).toHaveBeenCalledWith(
      'check_plugins_updates',
      expect.objectContaining({ betaFolders: ['Beta Plugin'] }),
    )
    expect(tauri.invoke).toHaveBeenCalledWith(
      'check_scenery_updates',
      expect.objectContaining({ betaFolders: ['Beta Scenery'] }),
    )
    expect(tauri.invoke).toHaveBeenCalledWith('check_for_updates', {
      manual: false,
      includePreRelease: true,
    })
    expect(
      store.currentRun?.checks.find((item) => item.id === 'addons.platform')?.remediation?.route,
    ).toBe('/management?tab=plugin')
    expect(
      store.currentRun?.checks.find((item) => item.id === 'updates.app')?.remediation?.route,
    ).toBe('/settings')
  })
})
