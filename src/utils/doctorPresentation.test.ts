import { describe, expect, it } from 'vitest'
import type { DoctorCheckResult } from '@/types/doctor'
import { describeDoctorCheck, formatDoctorDuration, suggestDoctorCheck } from './doctorPresentation'

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

describe('Health next steps', () => {
  it('selects the repair tool for the installation channel', () => {
    const missingFile: DoctorCheckResult = {
      id: 'installation.structure',
      section: 'installation',
      outcome: 'critical',
      durationMs: 0,
    }
    const location: DoctorCheckResult = {
      id: 'installation.location',
      section: 'installation',
      outcome: 'pass',
      durationMs: 0,
      params: { steam: 1 },
    }
    expect(suggestDoctorCheck(missingFile, 'en', [location])).toContain('Steam’s Verify')
    location.params = { steam: 0 }
    expect(suggestDoctorCheck(missingFile, 'en', [location])).toContain(
      'official X-Plane installer',
    )
  })

  it('separates a scan limit from actual read errors', () => {
    const limited: DoctorCheckResult = {
      id: 'installation.scan_access',
      section: 'installation',
      outcome: 'unavailable',
      durationMs: 0,
      params: { errors: 0, capped: 1 },
    }
    expect(suggestDoctorCheck(limited, 'en')).toContain('only part of the library')
    expect(suggestDoctorCheck(limited, 'en')).not.toContain('readable from your account')
    limited.params = { errors: 2, capped: 1 }
    expect(suggestDoctorCheck(limited, 'en')).toContain('readable from your account')
    expect(suggestDoctorCheck(limited, 'en')).toContain('only part of the library')
  })

  it('keeps manual updates, external sources and unverified items distinct', () => {
    const mixed: DoctorCheckResult = {
      id: 'updates.addons',
      section: 'updates',
      outcome: 'unavailable',
      durationMs: 0,
      params: { count: 2, manual: 2, external: 1, installed: 4, checked: 3 },
    }
    const advice = suggestDoctorCheck(mixed, 'en')
    expect(advice).toContain('manual upgrades')
    expect(advice).toContain('external updater')
    expect(advice).toContain('could not be verified')
    expect(advice).not.toContain('confirmed updates in Management')
    mixed.params = { ...mixed.params, manual: 1 }
    expect(suggestDoctorCheck(mixed, 'en')).toContain('confirmed updates in Management')
  })

  it('explains future, expired and mismatched cycles in one result', () => {
    const check: DoctorCheckResult = {
      id: 'navdata.cycles',
      section: 'navdata',
      outcome: 'warning',
      durationMs: 0,
      params: { future: 1, expired: 1, conflicts: 1, expiring: 0, unknown: 0 },
    }
    const advice = suggestDoctorCheck(check, 'en')
    expect(advice).toContain('already effective cycle')
    expect(advice).toContain('expired sources')
    expect(advice).toContain('matching files and metadata')
  })

  it('leaves successful checks and Gateway without managed airports free of update advice', () => {
    const check: DoctorCheckResult = {
      id: 'updates.gateway',
      section: 'updates',
      outcome: 'notApplicable',
      durationMs: 0,
    }
    expect(suggestDoctorCheck(check, 'en')).toBe('')
    expect(suggestDoctorCheck({ ...check, outcome: 'pass' }, 'en')).toBe('')
  })
})
