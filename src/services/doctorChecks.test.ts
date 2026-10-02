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
