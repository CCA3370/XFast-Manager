import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DoctorNavdataReport } from '@/types'
import { createDoctorCheckDefinitions, type DoctorCheckContext } from './doctorChecks'

const tauri = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke }))

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

function logCheck(xplaneRunning: boolean) {
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
    tauri.invoke.mockResolvedValueOnce([{}, {}]).mockResolvedValueOnce([
      { airportIcao: 'ZBAA', updateAvailable: null },
      { airportIcao: 'ZSPD', updateAvailable: false },
    ])
    const check = createDoctorCheckDefinitions({
      xplanePath: '/xplane',
      mode: 'full',
    } as DoctorCheckContext).find((item) => item.id === 'gateway_updates')!
    expect(await check.run()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'updates.gateway_unavailable', outcome: 'unavailable' }),
      ]),
    )
  })
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
