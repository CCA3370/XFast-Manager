import { describe, expect, it } from 'vitest'
import { describeDoctorCheck, formatDoctorDuration } from './doctorPresentation'

describe('Health presentation helpers', () => {
  it('does not claim Gateway updates when the check does not apply', () => {
    const check = {
      id: 'updates.gateway',
      section: 'updates',
      outcome: 'notApplicable',
      durationMs: 0,
    } as const
    for (const locale of ['zh', 'en']) {
      const text = describeDoctorCheck(check, locale)
      expect(text.description).not.toMatch(/have updates|有更新|更新的地景/)
      expect(text.suggestion).toBe('')
    }
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
