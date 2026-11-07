import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DoctorNavdataReport } from '@/types'
import { createDoctorCheckDefinitions, type DoctorCheckContext } from './doctorChecks'

const tauri = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke }))
vi.mock('@/services/storage', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/storage')>()),
  getItem: vi.fn().mockResolvedValue(null),
}))

function navdataCheck() {
  const context: DoctorCheckContext = {
    xplanePath: '/xplane',
    mode: 'quick',
    crashAnalysisDmpEnabled: false,
    crashAnalysisIgnoreDateCheck: false,
    installationId: '',
    xplaneRunning: false,
    environment: null,
    xfast: null,
    log: null,
    system: null,
  }
  return createDoctorCheckDefinitions(context).find((check) => check.id === 'navdata')!
}

function logCheck(xplaneRunning: boolean | null) {
  const context: DoctorCheckContext = {
    xplanePath: '/xplane',
    mode: 'full',
    crashAnalysisDmpEnabled: true,
    crashAnalysisIgnoreDateCheck: false,
    installationId: '',
    xplaneRunning,
    environment: null,
    xfast: null,
    log: null,
    system: null,
  }
  return createDoctorCheckDefinitions(context).find((check) => check.id === 'log')!
}

describe('Gateway diagnostics', () => {
  beforeEach(() => vi.clearAllMocks())

  it('skips the network when no managed airports are installed', async () => {
    tauri.invoke.mockResolvedValue([])
    const check = createDoctorCheckDefinitions({
      xplanePath: '/xplane',
      mode: 'full',
    } as DoctorCheckContext).find((item) => item.id === 'gateway_updates')!
    expect(await check.run()).toEqual([expect.objectContaining({ outcome: 'notApplicable' })])
    expect(tauri.invoke).toHaveBeenCalledTimes(1)
  })

  it('reports partial upstream failures separately rather than passing unchecked airports', async () => {
    const installed = [
      { id: 1, airportIcao: 'ZBAA', sceneryId: 12 },
      { id: 2, airportIcao: 'ZSPD', sceneryId: 13 },
      { id: 3, airportIcao: 'ZGGG', sceneryId: 14 },
    ]
    tauri.invoke.mockResolvedValueOnce(installed).mockResolvedValueOnce([
      { ...installed[0], updateAvailable: null },
      { ...installed[1], updateAvailable: true, latestSceneryId: 20 },
    ])
    const check = createDoctorCheckDefinitions({
      xplanePath: '/xplane',
      mode: 'full',
    } as DoctorCheckContext).find((item) => item.id === 'gateway_updates')!
    expect(await check.run()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'updates.gateway_unavailable',
          outcome: 'unavailable',
          params: { count: 2 },
        }),
        expect.objectContaining({
          id: 'updates.gateway',
          outcome: 'info',
          params: { installed: 3, checked: 1, count: 1 },
        }),
      ]),
    )
  })
})

describe('add-on update diagnostics', () => {
  function check() {
    return createDoctorCheckDefinitions({
      xplanePath: '/xplane',
      mode: 'full',
    } as DoctorCheckContext).find((item) => item.id === 'addon_updates')!
  }

  it('does not query external updaters or report them as up to date', async () => {
    tauri.invoke.mockReset().mockResolvedValue({
      entries: [{ folderName: 'External', updateUrl: 'x-updater:External' }],
    })
    expect(await check().run()).toEqual([
      expect.objectContaining({
        outcome: 'notApplicable',
        params: { count: 0, installed: 0, checked: 0, external: 3 },
      }),
    ])
    expect(tauri.invoke.mock.calls.map(([command]) => command)).toEqual([
      'scan_aircraft',
      'scan_plugins',
      'get_scenery_manager_data',
    ])
  })

  it('retains confirmed updates when another provider or inventory cannot be checked', async () => {
    const item = (folderName: string) => ({
      folderName,
      updateUrl: 'https://example.test',
      version: '1',
    })
    tauri.invoke.mockImplementation(async (command: string) => {
      switch (command) {
        case 'scan_aircraft':
          return { entries: [item('Current'), item('Update'), item('Offline'), item('Omitted')] }
        case 'scan_plugins':
          throw new Error('plugin inventory unavailable')
        case 'get_scenery_manager_data':
          return { entries: [] }
        case 'check_aircraft_updates':
          return [
            { ...item('Current'), latestVersion: '1', hasUpdate: false },
            { ...item('Update'), latestVersion: '2', hasUpdate: true },
            { ...item('Offline'), latestVersion: null, hasUpdate: false },
          ]
        default:
          throw new Error(command)
      }
    })
    expect(await check().run()).toEqual([
      expect.objectContaining({
        outcome: 'unavailable',
        params: { installed: 4, checked: 2, count: 1, external: 0 },
        evidence: expect.arrayContaining([
          expect.objectContaining({ value: expect.stringContaining('Update') }),
          expect.objectContaining({ value: expect.stringContaining('Offline') }),
          expect.objectContaining({ value: expect.stringContaining('Omitted') }),
          expect.objectContaining({
            value: expect.stringContaining('plugin inventory unavailable'),
          }),
        ]),
        remediation: expect.objectContaining({ id: 'manage_updates' }),
      }),
    ])
  })
})

it('keeps the stale index reason visible when verified CSL updates exceed the evidence limit', async () => {
  tauri.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'csl_scan_packages') {
      return {
        packages: Array.from({ length: 25 }, (_, index) => ({
          name: `Model ${index + 1}`,
          status: 'needs_update',
          files_to_update: 2,
        })),
      }
    }
    if (command === 'altitude_scan_packages') {
      return { packages: [], index_warning: 'Using cached index' }
    }
    throw new Error(command)
  })
  const check = createDoctorCheckDefinitions({
    xplanePath: '/xplane',
    mode: 'full',
  } as DoctorCheckContext).find((item) => item.id === 'csl_updates')!
  const [result] = await check.run()
  expect(result).toMatchObject({
    id: 'updates.csl',
    outcome: 'unavailable',
    params: { count: 25, checked: 25 },
  })
  expect(result.evidence?.[0]?.value).toBe('ALTITUDE: Using cached index')
  expect(result.evidence?.[1]?.value).toBe('X-CSL · Model 1: 2 files')
  expect(result.evidence).toHaveLength(20)
})

function report(overrides: Partial<DoctorNavdataReport> = {}): DoctorNavdataReport {
  const absent = { present: false, problems: [], errors: [], headers: [] }
  return {
    bundled: { ...absent, present: true },
    overrideData: absent,
    supplemental: absent,
    cycles: [],
    metadataErrors: [],
    ...overrides,
  }
}

describe('navdata diagnostics', () => {
  beforeEach(() => vi.clearAllMocks())

  it('checks bundled files even with no custom data', async () => {
    tauri.invoke.mockResolvedValue(
      report({
        bundled: {
          present: true,
          problems: ['Resources/default data/earth_fix.dat: missing'],
          errors: [],
          headers: [],
        },
      }),
    )
    const checks = await navdataCheck().run()
    expect(checks.find((check) => check.id === 'navdata.bundled')).toMatchObject({
      outcome: 'warning',
      evidence: [{ kind: 'path', value: 'Resources/default data/earth_fix.dat: missing' }],
    })
    expect(checks.find((check) => check.id === 'navdata.integrity')?.outcome).toBe('notApplicable')
  })

  it('reports confirmed defects alongside incomplete reads', async () => {
    tauri.invoke.mockResolvedValue(
      report({
        overrideData: {
          present: true,
          problems: ['Custom Data/earth_nav.dat: empty file'],
          errors: ['Custom Data/earth_fix.dat: access denied'],
          headers: [],
        },
        metadataErrors: ['Custom Data/cycle.json: malformed JSON'],
      }),
    )
    const checks = await navdataCheck().run()
    expect(checks.find((check) => check.id === 'navdata.integrity')?.outcome).toBe('warning')
    expect(checks.find((check) => check.id === 'navdata.cycles')?.outcome).toBe('unavailable')
    expect(checks.find((check) => check.id === 'navdata.scan_access')).toMatchObject({
      outcome: 'unavailable',
      evidence: [
        { value: 'Custom Data/earth_fix.dat: access denied' },
        { value: 'Custom Data/cycle.json: malformed JSON' },
      ],
    })
  })
})

describe('live-session stability diagnostics', () => {
  beforeEach(() => vi.clearAllMocks())

  it('does not interpret the last session when the live process state is unknown', async () => {
    tauri.invoke.mockResolvedValue({
      log_path: '/xplane/Log.txt',
      is_xplane_log: true,
      session_end: 'crash',
      crash_info: 'This application has crashed',
      issues: [],
      system_info: { xplane_version: '12.4.4', gpu_model: null, gpu_driver: null },
    })
    const checks = await logCheck(null).run()
    expect(checks.find((check) => check.id === 'stability.last_session')).toMatchObject({
      outcome: 'unavailable',
      evidence: undefined,
    })
  })

  it('keeps an unfinished log inconclusive rather than reporting a crash', async () => {
    tauri.invoke.mockResolvedValue({
      log_path: '/xplane/Log.txt',
      is_xplane_log: true,
      crash_detected: false,
      crash_info: null,
      session_end: 'unknown',
      log_tail: '[NSC] [INFO] UiService initialized',
      issues: [],
      system_info: { xplane_version: '12.1.4-r1', gpu_model: null, gpu_driver: null },
    })
    const checks = await logCheck(false).run()
    expect(checks.find((check) => check.id === 'stability.last_session')).toMatchObject({
      outcome: 'info',
      params: { sessionEnd: 'unknown' },
    })
    expect(tauri.invoke).toHaveBeenCalledTimes(1)
  })

  it('does not classify an actively written Log.txt as a completed crash', async () => {
    tauri.invoke.mockImplementation(async (command: string) => {
      if (command === 'analyze_xplane_log') {
        return {
          log_path: '/xplane/Log.txt',
          is_xplane_log: true,
          crash_detected: true,
          crash_info: 'missing normal shutdown ending',
          issues: [],
          system_info: {
            xplane_version: '12.1.4-r1',
            gpu_model: null,
            gpu_driver: null,
          },
        }
      }
      if (command === 'analyze_crash_report') {
        throw new Error('deep crash analysis must not run for a live session')
      }
      return null
    })

    const checks = await logCheck(true).run()

    expect(checks.find((check) => check.id === 'stability.last_session')).toMatchObject({
      outcome: 'notApplicable',
      evidence: undefined,
    })
    expect(tauri.invoke).not.toHaveBeenCalledWith('analyze_crash_report', expect.anything())
  })
})
