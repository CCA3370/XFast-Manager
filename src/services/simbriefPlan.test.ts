// @vitest-environment happy-dom
import { describe, expect, it, vi } from 'vitest'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
import { parsePlan, legs } from './flightPlanDetails'
const xml = `<?xml version="1.0"?><OFP><params><airac>2611</airac></params><origin><icao_code>ZSWZ</icao_code><plan_rwy>03</plan_rwy><pos_lat>27.91</pos_lat><pos_long>120.85</pos_long><elevation>13</elevation></origin><destination><icao_code>ZSSS</icao_code><plan_rwy>36R</plan_rwy><pos_lat>31.19</pos_lat><pos_long>121.33</pos_long></destination><navlog><fix><ident>DER03</ident><type>wpt</type><pos_lat>27.92</pos_lat><pos_long>120.86</pos_long><via_airway>REMM9S</via_airway><altitude_feet>2700</altitude_feet></fix><fix><ident>ZSSS</ident><type>apt</type><pos_lat>31.19</pos_lat><pos_long>121.33</pos_long><altitude_feet>2500</altitude_feet></fix></navlog></OFP>`
describe('SimBrief OFP XML', () => {
  it('reads route metadata and navlog heights, and includes the origin when omitted from the navlog', () => {
    const plan = parsePlan(xml)
    expect(plan.issues).toEqual([])
    expect(plan.fields).toMatchObject({ ADEP: 'ZSWZ', ADES: 'ZSSS', CYCLE: '2611', DEPRWY: '03' })
    expect(plan.points.map((p) => p.id)).toEqual(['ZSWZ', 'DER03', 'ZSSS'])
    expect(plan.points[1]?.altitude).toBe(2700)
    expect(plan.points[1]?.via).toBe('REMM9S')
    expect(legs(plan.points).at(-1)?.cumulative).toBeGreaterThan(100)
    expect(plan.raw).toBe(xml)
  })
  it('preserves missing coordinates without coercing them to zero', () => {
    const plan = parsePlan(xml.replace('<pos_lat>27.92</pos_lat>', '<pos_lat/>'))
    expect(plan.points[1]?.lat).toBeNull()
    expect(plan.issues.length).toBeGreaterThan(0)
  })
  it('rejects unrelated XML and XML with external entity declarations', () => {
    expect(parsePlan('<settings/>').issues).toContain('format')
    expect(
      parsePlan('<!DOCTYPE OFP SYSTEM "https://invalid.example/data">' + xml).issues,
    ).toContain('format')
  })
})
