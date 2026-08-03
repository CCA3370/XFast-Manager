import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DoctorEnvironmentReport, DoctorXfastHealthReport } from '@/types'

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

vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke }))
vi.mock('@/services/doctorHistory', () => history)
vi.mock('@/services/logger', () => ({ logError: vi.fn() }))
vi.mock('./lock', () => ({ useLockStore: () => locks }))

import { useAppStore } from './app'
import { useDoctorStore } from './doctor'

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
})
