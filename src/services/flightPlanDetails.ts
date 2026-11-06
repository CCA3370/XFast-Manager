import { invoke } from '@tauri-apps/api/core'

export interface Waypoint {
  id: string
  kind: number
  via: string
  altitude: number | null
  lat: number | null
  lon: number | null
  source?: string
  name?: string
}
export interface PlanDetails {
  raw: string
  version: number
  fields: Record<string, string>
  points: Waypoint[]
  issues: string[]
}
export interface NavPoint {
  kind: number
  id: string
  latitude: number
  longitude: number
  name: string
  source: string
}
export interface NavLookup {
  points: NavPoint[]
  sources: string[]
  warnings: string[]
}
const finite = (value: string | undefined) => {
  if (!value?.trim()) return null
  const n = Number(value)
  return Number.isFinite(n) ? n : null
}
function parseSimBrief(raw: string): PlanDetails {
  const result: PlanDetails = { raw, version: 0, fields: {}, points: [], issues: [] }
  if (/<!DOCTYPE|<!ENTITY/i.test(raw)) {
    result.issues.push('format')
    return result
  }
  const doc = new DOMParser().parseFromString(raw, 'application/xml')
  if (
    doc.getElementsByTagName('parsererror').length > 0 ||
    doc.documentElement?.tagName !== 'OFP'
  ) {
    result.issues.push('format')
    return result
  }
  const child = (parent: Element | null, key: string) =>
    Array.from(parent?.children ?? []).find((e) => e.localName === key) ?? null
  const value = (parent: Element | null, key: string) =>
    child(parent, key)?.textContent?.trim() ?? ''
  const root = doc.documentElement
  const origin = child(root, 'origin'),
    destination = child(root, 'destination')
  for (const [key, text] of Object.entries({
    ADEP: value(origin, 'icao_code'),
    ADES: value(destination, 'icao_code'),
    DEPRWY: value(origin, 'plan_rwy'),
    DESRWY: value(destination, 'plan_rwy'),
    CYCLE: value(child(root, 'params'), 'airac'),
  }))
    if (text) result.fields[key] = text
  const point = (element: Element, endpoint = false): Waypoint => {
    let lat = finite(value(element, 'pos_lat')),
      lon = finite(value(element, 'pos_long'))
    if (lat === null || lon === null || Math.abs(lat) > 90 || Math.abs(lon) > 180) {
      lat = lon = null
      result.issues.push(`coordinates:${result.points.length + 1}`)
    }
    const type = value(element, 'type')
    return {
      id: value(element, endpoint ? 'icao_code' : 'ident') || '?',
      kind: endpoint || type === 'apt' ? 1 : type === 'vor' ? 3 : type === 'ndb' ? 2 : 11,
      via: endpoint ? 'DRCT' : value(element, 'via_airway') || 'DRCT',
      altitude: finite(value(element, endpoint ? 'elevation' : 'altitude_feet')) || null,
      lat,
      lon,
      name: value(element, 'name'),
    }
  }
  for (const fix of Array.from(child(root, 'navlog')?.children ?? []).filter(
    (e) => e.localName === 'fix',
  )) {
    if (result.points.length >= 10000) {
      result.issues.push('limit')
      break
    }
    result.points.push(point(fix))
  }
  if (!result.points.length) {
    result.issues.push('empty')
    return result
  }
  // OFP navlogs can begin after takeoff and end before the destination.
  if (origin && result.points[0]?.id !== result.fields.ADEP)
    result.points.unshift(point(origin, true))
  if (destination && result.points.at(-1)?.id !== result.fields.ADES)
    result.points.push(point(destination, true))
  return result
}
export function parsePlan(raw: string): PlanDetails {
  if (raw.trimStart().startsWith('<')) return parseSimBrief(raw)
  const lines = raw.replace(/^\uFEFF/, '').split(/\r?\n/)
  const version = Number(lines[1]?.trim().split(/\s+/)[0])
  const result: PlanDetails = { raw, version, fields: {}, points: [], issues: [] }
  if (!['I', 'A'].includes(lines[0]?.trim() ?? '') || ![3, 1100].includes(version)) {
    result.issues.push('format')
    return result
  }
  let expected: number | null = null
  let route = version === 3
  for (let i = 2; i < lines.length; i++) {
    const cols = lines[i]!.trim().split(/\s+/)
    if (!cols[0]) continue
    if (cols[0] === 'NUMENR') {
      expected = finite(cols[1])
      route = true
      continue
    }
    if (version === 3 && i < 4 && cols.length === 1) {
      if (i === 3) expected = finite(cols[0])
      continue
    }
    if (!/^\d+$/.test(cols[0]) && !route) {
      result.fields[cols[0]] = cols.slice(1).join(' ')
      continue
    }
    if (version === 3 && cols.length === 1 && cols[0] === '0') continue
    if (result.points.length >= 10000) {
      result.issues.push('limit')
      break
    }
    const modern = version === 1100
    const kind = Number(cols[0])
    const altitude = finite(cols[modern ? 3 : 2])
    let lat = finite(cols[modern ? 4 : 3])
    let lon = finite(cols[modern ? 5 : 4])
    if (lat === null || lon === null || Math.abs(lat) > 90 || Math.abs(lon) > 180) {
      lat = lon = null
      result.issues.push(`coordinates:${i + 1}`)
    }
    if (![1, 2, 3, 11, 28].includes(kind) || cols.length !== (modern ? 6 : 5))
      result.issues.push(`row:${i + 1}`)
    result.points.push({
      id: cols[1] ?? '?',
      kind,
      via: modern ? (cols[2] ?? '') : 'DRCT',
      altitude: altitude === 0 ? null : altitude,
      lat,
      lon,
    })
  }
  if (expected !== null && expected !== result.points.length) result.issues.push('count')
  if (!route && version === 1100) result.issues.push('count')
  if (!result.points.length) result.issues.push('empty')
  return result
}
export function distance(a: Waypoint, b: Waypoint): number | null {
  if (a.lat === null || a.lon === null || b.lat === null || b.lon === null) return null
  const rad = Math.PI / 180
  const h =
    Math.sin(((b.lat - a.lat) * rad) / 2) ** 2 +
    Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(((b.lon - a.lon) * rad) / 2) ** 2
  return 3440.065 * 2 * Math.asin(Math.sqrt(Math.min(1, Math.max(0, h))))
}
export function legs(points: Waypoint[]) {
  let cumulative: number | null = 0
  return points.slice(1).map((point, index) => {
    const previous = points[index]!
    const nm = distance(previous, point)
    cumulative = cumulative === null || nm === null ? null : cumulative + nm
    return { from: previous.id, to: point.id, nm, cumulative }
  })
}
export function enrichPlan(plan: PlanDetails, lookup: NavLookup): PlanDetails {
  const index = new Map<string, NavPoint[]>()
  for (const point of lookup.points) {
    const key = `${point.kind}:${point.id}`
    const group = index.get(key) ?? []
    group.push(point)
    index.set(key, group)
  }
  return {
    ...plan,
    points: plan.points.map((point) => {
      const matches = index.get(`${point.kind}:${point.id}`) ?? []
      const near = matches.filter((p) => {
        const nm = distance(point, { ...point, lat: p.latitude, lon: p.longitude })
        return nm === null || nm <= 5
      })
      // Repeated identifiers in different regions are never guessed.
      if (near.length !== 1) return point
      const match = near[0]!
      return {
        ...point,
        lat: point.lat ?? match.latitude,
        lon: point.lon ?? match.longitude,
        source: match.source,
        name: match.name,
      }
    }),
  }
}
export async function readPlan(root: string, name: string) {
  return parsePlan(await invoke<string>('read_flight_plan', { xplanePath: root, name }))
}

/** Unwrap adjacent longitudes so dateline crossings follow the short route. */
export function mapCoordinates(points: Waypoint[]): ([number, number] | null)[] {
  let previous: number | null = null
  return points.map((point) => {
    if (point.lat === null || point.lon === null) return null
    let lon = point.lon
    if (previous !== null) {
      while (lon - previous > 180) lon -= 360
      while (lon - previous < -180) lon += 360
    }
    previous = lon
    return [point.lat, lon]
  })
}
