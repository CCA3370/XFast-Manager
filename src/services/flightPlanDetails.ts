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
export function parsePlan(raw: string): PlanDetails {
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
