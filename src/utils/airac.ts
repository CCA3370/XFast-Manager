/**
 * AIRAC (Aeronautical Information Regulation And Control) cycle utilities
 *
 * AIRAC cycles are 28 days long. The cycle format is YYNN where:
 * - YY = last two digits of year
 * - NN = cycle number within that year (01-13)
 *
 * Reference: Cycle 2601 starts on 2026-01-22
 */

// Reference point: Cycle 2601 starts on 2026-01-22
const REFERENCE_DATE = new Date(Date.UTC(2026, 0, 22)) // Jan 22, 2026
const REFERENCE_YEAR = 2026
const REFERENCE_CYCLE = 1
const CYCLE_DAYS = 28

/**
 * Get the current AIRAC cycle based on the current date
 * @returns The current cycle in YYNN format (e.g., "2601")
 */
export function getCurrentAiracCycle(date: Date = new Date()): string {
  // Convert to UTC to avoid timezone issues
  const utcDate = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()))

  // Calculate days difference from reference
  const diffMs = utcDate.getTime() - REFERENCE_DATE.getTime()
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  // Calculate cycle offset (can be negative for dates before reference)
  const cycleOffset = Math.floor(diffDays / CYCLE_DAYS)

  // Calculate absolute cycle number from reference
  // Reference is cycle 1 of 2026
  let year = REFERENCE_YEAR
  let cycle = REFERENCE_CYCLE + cycleOffset

  // Normalize: each year has 13 cycles
  while (cycle > 13) {
    cycle -= 13
    year += 1
  }
  while (cycle < 1) {
    cycle += 13
    year -= 1
  }

  // Format as YYNN
  const yearPart = (year % 100).toString().padStart(2, '0')
  const cyclePart = cycle.toString().padStart(2, '0')

  return `${yearPart}${cyclePart}`
}

/**
 * Parse an AIRAC cycle string to year and cycle number
 * @param cycleStr The cycle string (e.g., "2601" or "2513")
 * @returns Object with year (full year) and cycle number, or null if invalid
 */
export function parseAiracCycle(cycleStr: string): { year: number; cycle: number } | null {
  if (!cycleStr || cycleStr.length !== 4) return null

  const yearPart = parseInt(cycleStr.substring(0, 2), 10)
  const cyclePart = parseInt(cycleStr.substring(2, 4), 10)

  if (isNaN(yearPart) || isNaN(cyclePart)) return null
  if (cyclePart < 1 || cyclePart > 13) return null

  // Convert 2-digit year to full year (assume 2000s for now)
  const fullYear = 2000 + yearPart

  return { year: fullYear, cycle: cyclePart }
}

/**
 * Compare two AIRAC cycles
 * @returns negative if a < b, 0 if equal, positive if a > b
 */
export function compareAiracCycles(a: string, b: string): number {
  const parsedA = parseAiracCycle(a)
  const parsedB = parseAiracCycle(b)

  if (!parsedA || !parsedB) return 0

  if (parsedA.year !== parsedB.year) {
    return parsedA.year - parsedB.year
  }
  return parsedA.cycle - parsedB.cycle
}

/**
 * Check if a navdata cycle is current (matches the current AIRAC cycle)
 * @param cycleStr The cycle string from navdata (e.g., "2601")
 * @returns 'current' if matches current cycle, 'outdated' if older, 'future' if newer, null if invalid
 */
export function getNavdataCycleStatus(
  cycleStr: string | null | undefined,
): 'current' | 'outdated' | 'future' | null {
  if (!cycleStr) return null

  // Normalize the cycle string - extract 4-digit cycle if present
  const match = cycleStr.match(/(\d{4})/)
  if (!match) return null

  const normalizedCycle = match[1]
  const currentCycle = getCurrentAiracCycle()

  const comparison = compareAiracCycles(normalizedCycle, currentCycle)

  if (comparison === 0) return 'current'
  if (comparison < 0) return 'outdated'
  return 'future'
}
