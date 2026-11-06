import { describe, expect, it } from 'vitest'
import { getCurrentAiracCycle, getNavdataCycleStatus, parseAiracCycle } from './airac'
describe('AIRAC calendar', () => {
  it('matches the published 2026 effective dates', () => {
    expect(getCurrentAiracCycle(new Date('2026-10-28T23:59:59Z'))).toBe('2610')
    expect(getCurrentAiracCycle(new Date('2026-10-29T00:00:00Z'))).toBe('2611')
    expect(getCurrentAiracCycle(new Date('2026-11-26T00:00:00Z'))).toBe('2612')
  })
  it('handles fourteen-cycle years and the year boundary', () => {
    expect(parseAiracCycle('2014')).not.toBeNull()
    expect(getCurrentAiracCycle(new Date('2020-12-31T00:00:00Z'))).toBe('2014')
    expect(getCurrentAiracCycle(new Date('2021-01-01T00:00:00Z'))).toBe('2014')
    expect(parseAiracCycle('2614')).toBeNull()
  })
  it('never treats invalid metadata as current', () => {
    const date = new Date('2026-11-06T00:00:00Z')
    expect(getNavdataCycleStatus('2611', date)).toBe('current')
    expect(getNavdataCycleStatus('2610', date)).toBe('outdated')
    expect(getNavdataCycleStatus('2612', date)).toBe('future')
    expect(getNavdataCycleStatus('2699', date)).toBeNull()
    expect(parseAiracCycle('26x1')).toBeNull()
  })
})
