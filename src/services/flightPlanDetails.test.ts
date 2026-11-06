import { describe, expect, it, vi } from 'vitest'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
import { parsePlan, distance, legs, enrichPlan } from './flightPlanDetails'
const modern =
  'I\n1100 Version\nCYCLE 2611\nADEP KAAA\nADES KBBB\nSID TEST1\nNUMENR 3\n1 KAAA ADEP 100 0 179\n11 FIX DRCT 30000 0 -179\n1 KBBB ADES 200 1 -178\n'
describe('FMS inspection', () => {
  it('parses current FMS metadata, heights, airway and BOM/CRLF', () => {
    const p = parsePlan('\uFEFF' + modern.replaceAll('\n', '\r\n'))
    expect(p.version).toBe(1100)
    expect(p.fields.SID).toBe('TEST1')
    expect(p.points).toHaveLength(3)
    expect(p.points[1]?.altitude).toBe(30000)
    expect(p.issues).toEqual([])
  })
  it('parses v3 without treating numeric headers as waypoints', () => {
    const p = parsePlan('A\n3 version\n1\n2\n1 KAAA 0 10 20\n28 LATLON 15000 11 21\n0\n')
    expect(p.points).toHaveLength(2)
    expect(p.points[0]?.altitude).toBeNull()
    expect(p.issues).toEqual([])
  })
  it('uses short great-circle distance across the dateline', () => {
    const p = parsePlan(modern)
    expect(distance(p.points[0]!, p.points[1]!)).toBeCloseTo(120.08, 1)
    expect(legs(p.points)[1]?.cumulative).toBeGreaterThan(200)
  })
  it('preserves raw contents and missing-coordinate gaps without inventing distances', () => {
    const p = parsePlan(modern.replace('0 -179', 'NaN 999'))
    expect(p.raw).toContain('NaN')
    expect(p.points[1]?.lat).toBeNull()
    expect(legs(p.points).map((l) => l.nm)).toEqual([null, null])
    expect(p.issues.length).toBeGreaterThan(0)
  })
  it('reports invalid format and count mismatch without throwing away raw text', () => {
    expect(parsePlan('not a plan').issues).toContain('format')
    expect(parsePlan(modern.replace('NUMENR 3', 'NUMENR 4')).issues).toContain('count')
  })
  it('fills only unambiguous missing coordinates and never replaces plan coordinates', () => {
    const p = parsePlan(modern.replace('0 -179', 'x y'))
    const match = {
      kind: 11,
      id: 'FIX',
      latitude: 0,
      longitude: -179,
      name: 'Fix',
      source: 'user_fix.dat',
    }
    expect(enrichPlan(p, { points: [match], sources: [], warnings: [] }).points[1]?.lon).toBe(-179)
    expect(
      enrichPlan(p, { points: [match, { ...match, latitude: 40 }], sources: [], warnings: [] })
        .points[1]?.lon,
    ).toBeNull()
    expect(
      enrichPlan(parsePlan(modern), {
        points: [{ ...match, longitude: -178.99 }],
        sources: [],
        warnings: [],
      }).points[1]?.lon,
    ).toBe(-179)
  })
})
