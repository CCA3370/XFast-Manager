import { describe, expect, it } from 'vitest'
import type { DoctorCheckResult, DoctorRun } from '@/types/doctor'
import {
  addDoctorHistoryRun,
  emptyDoctorHistory,
  getDoctorHistoryForPath,
  isDoctorRunStale,
  sortDoctorChecks,
  summarizeDoctorRun,
} from './doctor'

function check(id: string, outcome: DoctorCheckResult['outcome']): DoctorCheckResult {
  return { id, section: 'installation', outcome, durationMs: 1 }
}

describe('doctor run summary', () => {
  it('keeps check failures separate from health severity and marks coverage partial', () => {
    const summary = summarizeDoctorRun(
      [check('path', 'pass'), check('network', 'unavailable')],
      'completed',
    )

    expect(summary.severity).toBe('ok')
    expect(summary.completeness).toBe('partial')
    expect(summary.coveragePercent).toBe(50)
    expect(summary.unavailable).toBe(1)
  })

  it('excludes not-applicable checks from the coverage denominator', () => {
    const summary = summarizeDoctorRun(
      [check('path', 'pass'), check('dmp', 'notApplicable')],
      'completed',
    )

    expect(summary.eligible).toBe(1)
    expect(summary.coveragePercent).toBe(100)
    expect(summary.completeness).toBe('complete')
  })

  it('uses the highest issue severity without hiding cancellation', () => {
    const summary = summarizeDoctorRun(
      [check('warning', 'warning'), check('critical', 'critical'), check('late', 'cancelled')],
      'cancelled',
    )

    expect(summary.severity).toBe('critical')
    expect(summary.completeness).toBe('cancelled')
  })
})

describe('doctor check ordering', () => {
  it('places actionable failures ahead of passed checks inside a section', () => {
    const sorted = sortDoctorChecks([
      check('pass', 'pass'),
      check('warning', 'warning'),
      check('unavailable', 'unavailable'),
    ])

    expect(sorted.map((item) => item.id)).toEqual(['warning', 'unavailable', 'pass'])
  })
})

function run(id: string, installationId = 'install'): DoctorRun {
  const checks = [check('path', 'pass')]
  return {
    schemaVersion: 1,
    id,
    installationId,
    mode: 'quick',
    state: 'completed',
    startedAt: Number(id.replace(/\D/g, '')) || 1,
    completedAt: 1_000,
    durationMs: 10,
    appVersion: '1.0.0',
    checks,
    summary: summarizeDoctorRun(checks, 'completed'),
    system: null,
  }
}

describe('doctor history', () => {
  it('keeps histories isolated by installation and trims old runs', () => {
    let history = emptyDoctorHistory()
    for (let index = 1; index <= 22; index++) {
      history = addDoctorHistoryRun(history, '/xplane', run(`run-${index}`))
    }
    history = addDoctorHistoryRun(history, '/other', run('other-1', 'other'))

    expect(getDoctorHistoryForPath(history, '/xplane')).toHaveLength(20)
    expect(getDoctorHistoryForPath(history, '/xplane')[0]?.id).toBe('run-22')
    expect(getDoctorHistoryForPath(history, '/other').map((item) => item.id)).toEqual(['other-1'])
  })

  it('marks a result stale at 24 hours', () => {
    const item = run('run-1')
    item.completedAt = 1_000
    expect(isDoctorRunStale(item, 1_000 + 24 * 60 * 60 * 1000 - 1)).toBe(false)
    expect(isDoctorRunStale(item, 1_000 + 24 * 60 * 60 * 1000)).toBe(true)
  })
})
