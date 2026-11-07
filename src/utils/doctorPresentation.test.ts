import { describe, expect, it } from 'vitest'
import { describeDoctorCheck, formatDoctorDuration } from './doctorPresentation'

describe('Health presentation helpers', () => {
  it('explains that there are no managed Gateway airports to check', () => {
    const check = {
      id: 'updates.gateway',
      section: 'updates',
      outcome: 'notApplicable',
      durationMs: 0,
    } as const
    expect(describeDoctorCheck(check, 'en').description).toContain('No Gateway airports managed')
    expect(describeDoctorCheck(check, 'zh').description).toContain('没有由 XFast Manager 管理')
  })

  it('does not mistake unavailable checks for known failures', () => {
    const text = describeDoctorCheck({
      id: 'addons.binary_integrity',
      section: 'addons',
      outcome: 'unavailable',
      durationMs: 0,
    })
    expect(text.description).toContain('could not produce')
    expect(text.description).not.toContain('invalid executable')
  })
  it('normalizes rounded seconds instead of displaying 60 seconds in a minute', () => {
    expect(formatDoctorDuration(59_600)).toBe('1m 0s')
    expect(formatDoctorDuration(3_599_600)).toBe('60m 0s')
  })

  it('keeps millisecond and short-second values compact', () => {
    expect(formatDoctorDuration(850)).toBe('850 ms')
    expect(formatDoctorDuration(1_250)).toBe('1.3 s')
  })
})
