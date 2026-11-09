/** AIRAC effective dates repeat every 28 days; numbering restarts each year. */
const REFERENCE = Date.UTC(2026, 0, 22)
const PERIOD = 28 * 86400000
const first = (year: number) =>
  REFERENCE + Math.ceil((Date.UTC(year, 0, 1) - REFERENCE) / PERIOD) * PERIOD
export function getCurrentAiracCycle(date: Date = new Date()): string {
  const effective = REFERENCE + Math.floor((date.getTime() - REFERENCE) / PERIOD) * PERIOD
  const year = new Date(effective).getUTCFullYear()
  return (
    String(year % 100).padStart(2, '0') +
    String(Math.round((effective - first(year)) / PERIOD) + 1).padStart(2, '0')
  )
}
export function parseAiracCycle(value: string): { year: number; cycle: number } | null {
  if (!/^\d{4}$/.test(value)) return null
  const year = 2000 + Number(value.slice(0, 2)),
    cycle = Number(value.slice(2))
  const count = Math.round((first(year + 1) - first(year)) / PERIOD)
  return cycle >= 1 && cycle <= count ? { year, cycle } : null
}
export function compareAiracCycles(a: string, b: string): number {
  const x = parseAiracCycle(a),
    y = parseAiracCycle(b)
  return x && y ? x.year - y.year || x.cycle - y.cycle : 0
}
export function getNavdataCycleStatus(
  value: string | null | undefined,
  date: Date = new Date(),
): 'current' | 'outdated' | 'future' | null {
  const cycle = value?.match(/\b\d{4}\b/)?.[0]
  if (!cycle || !parseAiracCycle(cycle)) return null
  const comparison = compareAiracCycles(cycle, getCurrentAiracCycle(date))
  return comparison === 0 ? 'current' : comparison < 0 ? 'outdated' : 'future'
}
