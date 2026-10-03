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

function report(overrides: Partial<DoctorNavdataReport> = {}): DoctorNavdataReport {
  return {
    customDataExists: true,
    simulatorOverridePresent: false,
    cifpPresent: false,
    earthDatMissing: [],
    cycles: [],
    ...overrides,
  }
}

describe('navdata diagnostic applicability', () => {
  beforeEach(() => vi.clearAllMocks())

  it('does not demand simulator CIFP for independently installed add-on data', async () => {
    tauri.invoke.mockResolvedValue(
      report({
        cycles: [
          {
            folderName: 'GNS430/navdata',
            providerName: 'Navigraph',
            cycle: '2610',
            airac: null,
            effectiveDate: null,
            expiryDate: null,
            daysRemaining: null,
            status: 'ok',
            source: 'none',
          },
        ],
      }),
    )
    const checks = await navdataCheck().run()
    expect(checks.find((check) => check.id === 'navdata.cycles')?.outcome).toBe('pass')
    expect(checks.find((check) => check.id === 'navdata.cifp')?.outcome).toBe('notApplicable')
    expect(checks.find((check) => check.id === 'navdata.integrity')?.outcome).toBe('notApplicable')
    expect(checks.some((check) => check.outcome === 'warning')).toBe(false)
  })

  it('reports incomplete simulator overrides even without readable cycle metadata', async () => {
    tauri.invoke.mockResolvedValue(
      report({
        simulatorOverridePresent: true,
        earthDatMissing: ['earth_fix.dat'],
      }),
    )
    const checks = await navdataCheck().run()
    expect(checks.find((check) => check.id === 'navdata.cycles')?.outcome).toBe('info')
    expect(checks.find((check) => check.id === 'navdata.cifp')?.outcome).toBe('warning')
    expect(checks.find((check) => check.id === 'navdata.integrity')).toMatchObject({
      outcome: 'warning',
      evidence: [{ kind: 'path', value: 'earth_fix.dat', sensitive: true }],
    })
  })

  it('keeps an empty Custom Data directory not applicable', async () => {
    tauri.invoke.mockResolvedValue(report())
    expect(await navdataCheck().run()).toEqual([
      expect.objectContaining({ id: 'navdata.custom_data', outcome: 'notApplicable' }),
    ])
  })
})


describe('live-session stability diagnostics', () => {
  beforeEach(() => vi.clearAllMocks())

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
    expect(tauri.invoke).not.toHaveBeenCalledWith(
      'analyze_crash_report',
      expect.anything(),
    )
  })
})
