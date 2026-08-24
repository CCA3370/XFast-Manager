import { describe, expect, it } from 'vitest'
import { formatDoctorDuration } from './doctorPresentation'

describe('Health presentation helpers', () => {
  it('normalizes rounded seconds instead of displaying 60 seconds in a minute', () => {
    expect(formatDoctorDuration(59_600)).toBe('1m 0s')
    expect(formatDoctorDuration(3_599_600)).toBe('60m 0s')
  })

  it('keeps millisecond and short-second values compact', () => {
    expect(formatDoctorDuration(850)).toBe('850 ms')
    expect(formatDoctorDuration(1_250)).toBe('1.3 s')
  })
})
