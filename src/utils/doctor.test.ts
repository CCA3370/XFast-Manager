import { describe, expect, it } from 'vitest'
import type { DoctorCheckResult } from '@/types/doctor'
import { sortDoctorChecks, summarizeDoctorRun } from './doctor'

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
