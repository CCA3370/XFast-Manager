import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DoctorEnvironmentReport, DoctorXfastHealthReport } from '@/types'
import type { DoctorRun } from '@/types/doctor'

const tauri = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))

const history = vi.hoisted(() => ({
  loadDoctorRunsForPath: vi.fn<(path: string) => Promise<DoctorRun[]>>(async () => []),
  saveDoctorRun: vi.fn<(path: string, run: DoctorRun) => Promise<DoctorRun[]>>(
    async (_path, run) => [run],
  ),
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
    installation: {
      rootUsable: true,
      core: { problems: [], errors: [] },
      aircraft: { problems: [], errors: [] },
      airfoils: { problems: [], errors: [] },
    },
    diskSpace: {
      availableBytes: 50 * 1024 * 1024 * 1024,
      totalBytes: 500 * 1024 * 1024 * 1024,
      error: null,
    },
    directoryAccess: { blocked: [], errors: [] },
    vulkanRuntime: null,
    inProgramFiles: false,
    isSteamInstall: false,
    readonlyCount: 0,
    readonlyScanCapped: false,
    readonlyScanErrors: [],
    readonlyScanPerformed: false,
    injectors: [],
    competingOrganizers: [],
    system: {
      os: 'linux',
      osVersion: 'test',
      architecture: 'x86_64',
      cpuModel: 'Test CPU',
      logicalCores: 8,
      physicalCores: 4,
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
    diskSpace: {
      availableBytes: 20 * 1024 * 1024 * 1024,
      totalBytes: 100 * 1024 * 1024 * 1024,
      error: null,
    },
    databaseOk: true,
    databaseDetails: ['ok'],
    sceneryIndexError: null,
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
    case 'doctor_inspect_installation':
      return {
        sceneryRegistration: 'readable',
        sceneryRegistrationError: null,
        globalAirportsEnabled: true,
        sceneryPathErrors: [],
        sceneryLinkErrors: [],
        sceneryLinksCapped: false,
        invalidSceneryEntries: [],
        missingSceneryPaths: [],
        duplicateSceneryPaths: [],
        brokenSceneryLinks: [],
        sceneryFiles: null,
        plugins: { checked: 0, problems: [], incompatible: [], errors: [], capped: false },
        shaderCachePresent: true,
        preferencesPresent: true,
        logAgeDays: 0,
      }
    case 'doctor_list_backups':
      return []
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
        session_end: 'normal',
        log_tail: '',
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
        bundled: { present: true, problems: [], errors: [], headers: [] },
        overrideData: { present: false, problems: [], errors: [], headers: [] },
        supplemental: { present: false, problems: [], errors: [], headers: [] },
        cycles: [],
        metadataErrors: [],
      }
    case 'get_activity_log':
      return { entries: [], totalCount: 0 }
    case 'scan_output_cleanup_items':
      return { items: [] }
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
    history.saveDoctorRun.mockImplementation(async (_path, run) => [run])
    storage.getItem.mockResolvedValue(null)
    installHealthyMocks()
    const appStore = useAppStore()
    appStore.xplanePath = '/xplane'
  })

  it('reports a scenery inspection limit separately from read errors', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      const response = await healthyResponse(command)
      if (command !== 'doctor_inspect_installation') return response
      return {
        ...(response as object),
        sceneryFiles: { checked: 25, problems: [], errors: [], limit: 'time' },
      }
    })
    const store = useDoctorStore()
    await store.runDiagnostics('full')
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'installation.scan_access',
          outcome: 'unavailable',
          params: { errors: 0, capped: 1 },
          evidence: [
            expect.objectContaining({ value: 'Enabled scenery files: time limit reached' }),
          ],
        }),
        expect.objectContaining({ id: 'scenery.files', outcome: 'unavailable' }),
      ]),
    )
  })

  it('continues checking a readable installation when the executable is missing', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      if (command !== 'doctor_scan_environment') return healthyResponse(command)
      const env = environment()
      env.installation.core.problems = ['X-Plane executable: missing']
      env.installation.aircraft.problems = ['Aircraft/**/Cirrus SR22.acf: missing']
      return env
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'installation.structure', outcome: 'critical' }),
        expect.objectContaining({ id: 'installation.stock_aircraft', outcome: 'warning' }),
        expect.objectContaining({ id: 'navdata.bundled', outcome: 'pass' }),
      ]),
    )
  })

  it('stops installation-dependent probes when the selected root cannot be opened', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      if (command !== 'doctor_scan_environment') return healthyResponse(command)
      const env = environment()
      env.installation.rootUsable = false
      env.installation.core.errors = ['Access denied']
      return env
    })
    const store = useDoctorStore()
    await store.runDiagnostics('full')
    expect(store.currentRun?.checks).toEqual([
      expect.objectContaining({ id: 'installation.structure', outcome: 'unavailable' }),
    ])
    expect(tauri.invoke).not.toHaveBeenCalledWith('doctor_navdata_status', expect.anything())
  })

  it('distinguishes unavailable host measurements from confirmed resource and permission problems', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      if (command !== 'doctor_scan_environment') return healthyResponse(command)
      const env = environment()
      env.diskSpace = { availableBytes: null, totalBytes: null, error: 'volume query failed' }
      env.directoryAccess.blocked = ['Output: permission denied']
      env.system.physicalCores = 2
      env.vulkanRuntime = { state: 'available', version: '1.2.0', meetsMinimum: false }
      return env
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'storage.xplane_space', outcome: 'unavailable' }),
        expect.objectContaining({ id: 'installation.write_access', outcome: 'warning' }),
        expect.objectContaining({ id: 'system.inventory', outcome: 'warning' }),
        expect.objectContaining({ id: 'system.vulkan_runtime', outcome: 'warning' }),
      ]),
    )
  })

  it('does not suggest rebuilding an index when its database could not be checked', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      if (command !== 'doctor_scan_xfast_health') return healthyResponse(command)
      return {
        ...xfast(),
        databaseOk: null,
        databaseDetails: ['database is locked'],
        sceneryIndex: null,
      }
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'xfast.database',
          outcome: 'unavailable',
          remediation: undefined,
        }),
        expect.objectContaining({
          id: 'xfast.scenery_index',
          outcome: 'unavailable',
          remediation: undefined,
        }),
      ]),
    )
  })

  it('reports the live Global Airports registration and excludes disabled scenery conflicts', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      const response = await healthyResponse(command)
      if (command === 'doctor_inspect_installation') {
        return { ...(response as object), globalAirportsEnabled: false }
      }
      if (command === 'get_scenery_manager_data') {
        return {
          ...(response as object),
          entries: [
            {
              folderName: 'Enabled',
              enabled: true,
              duplicateTiles: ['+01+002'],
              missingLibraries: [],
            },
            {
              folderName: 'Disabled',
              enabled: false,
              duplicateTiles: ['+01+002'],
              missingLibraries: ['Unused'],
            },
          ],
        }
      }
      if (command === 'airport_flatten_list_overrides') throw new Error('Flatten inspection failed')
      return response
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'scenery.global_airports', outcome: 'critical' }),
        expect.objectContaining({ id: 'scenery.dependencies', outcome: 'pass' }),
        expect.objectContaining({ id: 'scenery.overlaps', outcome: 'pass' }),
        expect.objectContaining({ id: 'diagnostic.scenery_flatten', outcome: 'unavailable' }),
      ]),
    )
  })

  it('keeps a confirmed simulator crash when the independent dump inspection fails', async () => {
    useAppStore().crashAnalysisDmpEnabled = true
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'analyze_xplane_log') {
        return {
          ...((await healthyResponse(command)) as object),
          session_end: 'crash',
          crash_info: 'This application has crashed',
        }
      }
      if (command === 'analyze_crash_report') throw new Error('Dump could not be parsed')
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.runDiagnostics('full')
    expect(store.currentRun?.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'stability.last_session', outcome: 'critical' }),
        expect.objectContaining({ id: 'diagnostic.crash_dump', outcome: 'unavailable' }),
      ]),
    )
  })

  it('keeps unreadable scenery registrations separate from readable link inventory', async () => {
    tauri.invoke.mockImplementation(async (command) => {
      const response = await healthyResponse(command)
      if (command !== 'doctor_inspect_installation') return response
      return {
        ...(response as object),
        sceneryRegistration: 'unavailable',
        sceneryRegistrationError: 'Custom Scenery/scenery_packs.ini: invalid UTF-8',
      }
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const checks = store.currentRun?.checks ?? []
    for (const id of [
      'scenery.registration',
      'scenery.invalid_entries',
      'scenery.missing_paths',
      'scenery.duplicate_paths',
    ]) {
      expect(checks.find((check) => check.id === id)).toMatchObject({
        outcome: 'unavailable',
        remediation: undefined,
      })
    }
    expect(checks.find((check) => check.id === 'scenery.broken_links')?.outcome).toBe('pass')
    expect(store.currentRun?.summary.completeness).toBe('partial')
  })

  it.each([
    {
      problems: [],
      pluginErrors: [],
      sceneryLinkErrors: ['Custom Scenery: denied'],
      expected: 'pass',
    },
    {
      problems: [],
      pluginErrors: ['plugins: denied'],
      sceneryLinkErrors: [],
      expected: 'unavailable',
    },
    {
      problems: ['Test.xpl: invalid PE signature'],
      pluginErrors: ['plugins: denied'],
      sceneryLinkErrors: [],
      expected: 'warning',
    },
  ])('reports plugin integrity as $expected based on plugin evidence', async (scenario) => {
    tauri.invoke.mockImplementation(async (command) => {
      const response = await healthyResponse(command)
      if (command !== 'doctor_inspect_installation') return response
      return {
        ...(response as object),
        sceneryLinkErrors: scenario.sceneryLinkErrors,
        plugins: {
          checked: 1,
          problems: scenario.problems,
          incompatible: [],
          errors: scenario.pluginErrors,
          capped: false,
        },
      }
    })

    const store = useDoctorStore()
    await store.runDiagnostics('full')
    const run = store.currentRun
    const pluginCheck = run?.checks.find((check) => check.id === 'addons.binary_integrity')
    expect(pluginCheck?.outcome).toBe(scenario.expected)
    expect(pluginCheck?.remediation?.id).toBe(
      scenario.problems.length ? 'manage_plugins' : undefined,
    )
    expect(run?.checks.find((check) => check.id === 'installation.scan_access')?.outcome).toBe(
      'unavailable',
    )
    expect(run?.summary.completeness).toBe('partial')
  })

  it.each(['disable_injectors', 'rebuild_shader_cache', 'reset_preferences'])(
    'backs up %s and rechecks the affected installation',
    async (action) => {
      const store = useDoctorStore()
      await store.runDiagnostics('quick')
      const scansBefore = tauri.invoke.mock.calls.filter(
        ([command]) => command === 'doctor_inspect_installation',
      ).length
      expect(await store.runMaintenance(action)).toBe(true)
      expect(store.currentRun?.system?.gpuModel).toBe('Test GPU')
      expect(tauri.invoke).toHaveBeenCalledWith('doctor_apply_repair', {
        xplanePath: '/xplane',
        repair: action,
      })
      expect(tauri.invoke).toHaveBeenCalledWith('doctor_list_backups', { xplanePath: '/xplane' })
      expect(
        tauri.invoke.mock.calls.filter(([command]) => command === 'doctor_inspect_installation'),
      ).toHaveLength(scansBefore + 1)
      expect(
        store.currentRun?.checks.filter((check) => check.id === 'scenery.registration'),
      ).toHaveLength(1)
    },
  )

  it('blocks maintenance when X-Plane starts after the check', async () => {
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    tauri.invoke.mockImplementation(async (command) =>
      command === 'is_xplane_running' ? true : healthyResponse(command),
    )
    expect(await store.runMaintenance('reset_preferences')).toBe(false)
    expect(store.error).toBe('xplane_running')
    expect(tauri.invoke.mock.calls.some(([command]) => command === 'doctor_apply_repair')).toBe(
      false,
    )
  })

  it('restores the selected backup and refreshes the available backups', async () => {
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    expect(await store.runMaintenance('restore_health_backup', 'selected-backup')).toBe(true)
    expect(tauri.invoke).toHaveBeenCalledWith('doctor_restore_backup', {
      xplanePath: '/xplane',
      backupId: 'selected-backup',
    })
  })

  it('writes scenery fixes to the simulator file after updating the index', async () => {
    useAppStore().autoSortScenery = true
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'get_scenery_manager_data')
        return {
          entries: [{ folderName: '*GLOBAL_AIRPORTS*', enabled: false }],
          needsSync: true,
          missingDepsCount: 0,
          duplicateTilesCount: 0,
          duplicateAirportsCount: 0,
        }
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const sort = store.currentRun!.checks.find((check) => check.id === 'scenery.load_order')!
    expect(await store.applyRemediation(sort)).toBe(true)
    const commands = tauri.invoke.mock.calls.map(([command]) => command)
    expect(commands.indexOf('apply_scenery_changes')).toBeGreaterThan(
      commands.indexOf('sort_scenery_packs'),
    )
    expect(tauri.invoke).toHaveBeenCalledWith('apply_scenery_changes', {
      xplanePath: '/xplane',
      entries: [],
    })
  })

  it('does not offer or execute an automatic scenery sort when the setting is off', async () => {
    useAppStore().autoSortScenery = false
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'get_scenery_manager_data')
        return {
          entries: [{ folderName: '*GLOBAL_AIRPORTS*', enabled: true }],
          needsSync: true,
          missingDepsCount: 0,
          duplicateTilesCount: 0,
          duplicateAirportsCount: 0,
        }
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const sort = store.currentRun!.checks.find((check) => check.id === 'scenery.load_order')!
    expect(
      store.displayedRun!.checks.find((check) => check.id === sort.id)!.remediation,
    ).toBeUndefined()
    expect(await store.applyRemediation(sort)).toBe(false)
    await store.applyAllSafeFixes()
    expect(tauri.invoke.mock.calls.some(([command]) => command === 'sort_scenery_packs')).toBe(
      false,
    )
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

  it('resolves X-Plane runtime state before interpreting the current log', async () => {
    const runtimeState = deferred<boolean>()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'is_xplane_running') return runtimeState.promise
      if (command === 'analyze_xplane_log') {
        return {
          log_path: '/xplane/Log.txt',
          is_xplane_log: true,
          crash_detected: true,
          crash_info: 'Log ended before a normal shutdown marker',
          issues: [],
          system_info: {
            xplane_version: '12.1.4-r1',
            gpu_model: 'Test GPU',
            gpu_driver: '1.0',
          },
        }
      }
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    const running = store.runDiagnostics('quick')

    await vi.waitFor(() => expect(tauri.invoke).toHaveBeenCalledWith('is_xplane_running'))
    expect(tauri.invoke.mock.calls.some(([command]) => command === 'analyze_xplane_log')).toBe(
      false,
    )

    runtimeState.resolve(true)
    await running

    expect(store.xplaneRunning).toBe(true)
    expect(
      store.currentRun?.checks.find((item) => item.id === 'stability.last_session')?.outcome,
    ).toBe('notApplicable')
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
        expect.objectContaining({ id: 'updates.csl', outcome: 'notApplicable' }),
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

  it.each([
    { batch: false, applied: 0 },
    { batch: false, applied: 2 },
    { batch: true, applied: 0 },
    { batch: true, applied: 2 },
  ])('reports flatten failures truthfully (%j)', async ({ batch, applied }) => {
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'airport_flatten_list_overrides') {
        return [{ icao: 'TEST', sourcePath: '/scenery/apt.dat', status: 'drifted' }]
      }
      if (command === 'airport_flatten_apply_all_drifted') {
        return {
          applied,
          skipped: 0,
          failed: [{ icao: 'TEST', sourcePath: '/scenery/apt.dat', error: 'Permission denied' }],
        }
      }
      return healthyResponse(command)
    })
    const store = useDoctorStore()
    await store.runDiagnostics('quick')
    const check = store.currentRun!.checks.find((item) => item.id === 'scenery.flatten')!
    expect(check.remediation?.id).toBe('apply_flatten')
    if (batch) {
      expect(await store.applyAllSafeFixes()).toEqual({ applied, failed: 1 })
    } else {
      expect(await store.applyRemediation(check)).toBe(false)
    }
    expect(store.error).toContain('TEST (/scenery/apt.dat): Permission denied')
    expect(store.currentRun!.checks.find((item) => item.id === 'scenery.flatten')?.outcome).toBe(
      'warning',
    )
    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'airport_flatten_list_overrides'),
    ).toHaveLength(2)
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

  it('clears the previous installation state before awaiting new history', async () => {
    const nextHistory = deferred<DoctorRun[]>()
    history.loadDoctorRunsForPath.mockImplementation((path: string) =>
      path === '/xplane-a'
        ? Promise.resolve([historyRun('run-a', 'installation-a')])
        : nextHistory.promise,
    )
    const appStore = useAppStore()
    const store = useDoctorStore()
    appStore.xplanePath = '/xplane-a'
    await store.loadHistory()
    store.error = 'old installation error'

    appStore.xplanePath = '/xplane-b'
    const loading = store.loadHistory()

    expect(store.currentRun).toBeNull()
    expect(store.history).toEqual([])
    expect(store.error).toBeNull()
    expect(store.isHistoryLoading).toBe(true)
    nextHistory.resolve([])
    await loading
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

  it('serializes repair preflight and blocks a competing diagnostic run', async () => {
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
    expect(check).toBeDefined()

    const safetyCheck = deferred<boolean>()
    tauri.invoke.mockClear()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'is_xplane_running') return safetyCheck.promise
      return healthyResponse(command)
    })

    const firstRepair = store.applyRemediation(check!)
    expect(store.fixingId).toBe(check?.id)
    expect(store.isBusy).toBe(true)

    const secondRepair = store.applyRemediation(check!)
    await store.runDiagnostics('full')
    expect(store.currentRun?.mode).toBe('quick')

    safetyCheck.resolve(false)
    expect(await secondRepair).toBe(false)
    expect(await firstRepair).toBe(true)
    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'is_xplane_running'),
    ).toHaveLength(1)
    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'get_app_version'),
    ).toHaveLength(0)
  })

  it('serializes safe batch repair preflight', async () => {
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

    const safetyCheck = deferred<boolean>()
    tauri.invoke.mockClear()
    tauri.invoke.mockImplementation(async (command) => {
      if (command === 'is_xplane_running') return safetyCheck.promise
      return healthyResponse(command)
    })

    const firstBatch = store.applyAllSafeFixes()
    expect(store.repairingAll).toBe(true)
    expect(store.isBusy).toBe(true)
    const secondBatch = store.applyAllSafeFixes()

    safetyCheck.resolve(false)
    expect(await secondBatch).toEqual({ applied: 0, failed: 0 })
    expect(await firstBatch).toEqual({ applied: 1, failed: 0 })
    expect(
      tauri.invoke.mock.calls.filter(([command]) => command === 'is_xplane_running'),
    ).toHaveLength(1)
  })

  it('adds active recheck time without counting idle time since the original scan', async () => {
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
    expect(check).toBeDefined()
    store.currentRun = {
      ...store.currentRun!,
      startedAt: 1,
      durationMs: 2_000,
    }

    expect(await store.applyRemediation(check!)).toBe(true)

    expect(store.currentRun?.durationMs).toBeGreaterThanOrEqual(2_000)
    expect(store.currentRun?.durationMs).toBeLessThan(10_000)
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
      if (command === 'doctor_inspect_installation') {
        return {
          ...((await healthyResponse(command)) as object),
          plugins: {
            checked: 1,
            problems: [],
            incompatible: ['Resources/plugins/Beta Plugin/win_x64/Beta Plugin.xpl'],
            errors: [],
            capped: false,
          },
        }
      }
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
