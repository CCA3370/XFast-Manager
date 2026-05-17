import { SceneryCategory, type SceneryManagerEntry } from '@/types'

export type SmartSceneryGroupKind = 'simheaven' | 'ortho' | 'airport' | 'product'

export interface SmartSceneryGroup {
  id: string
  kind: SmartSceneryGroupKind
  title: string
  entries: SceneryManagerEntry[]
  enabledCount: number
  totalCount: number
}

export type SmartSceneryRow =
  | { rowType: 'group'; group: SmartSceneryGroup }
  | { rowType: 'entry'; entry: SceneryManagerEntry }

interface GroupCandidate {
  key: string
  kind: SmartSceneryGroupKind
  title: string
}

type GroupDetector = (entry: SceneryManagerEntry) => GroupCandidate | null

const AIRPORT_CATEGORIES = new Set<SceneryCategory>([
  SceneryCategory.Airport,
  SceneryCategory.Overlay,
  SceneryCategory.AirportMesh,
])

function titleCaseToken(token: string): string {
  if (!token) return token
  if (/^[A-Z0-9]{3,}$/.test(token)) return token
  return token.slice(0, 1).toUpperCase() + token.slice(1).toLowerCase()
}

function titleCaseWords(value: string): string {
  return value
    .split(/[\s_-]+/)
    .filter(Boolean)
    .map(titleCaseToken)
    .join(' ')
}

function normalizeFolderName(folderName: string): string {
  return folderName.toLowerCase().replace(/[._]+/g, '-').replace(/\s+/g, '-')
}

function detectSimHeavenGroup(entry: SceneryManagerEntry): GroupCandidate | null {
  const name = entry.folderName
  const normalized = normalizeFolderName(name)

  if (!normalized.includes('simheaven') && !normalized.includes('x-world')) return null

  const xWorldMatch = name.match(/(?:simheaven[\s_-]*)?x[\s_-]?world[\s_-]+([a-z][a-z0-9]*)/i)
  const legacyMatch = name.match(/simheaven[\s_-]*x[\s_-]+([a-z][a-z0-9]*)/i)
  const region = xWorldMatch?.[1] ?? legacyMatch?.[1] ?? 'global'
  const titleRegion = region === 'global' ? 'Global' : titleCaseWords(region)

  return {
    key: `simheaven:${region.toLowerCase()}`,
    kind: 'simheaven',
    title: `SimHeaven X-World ${titleRegion}`,
  }
}

function detectOrthoGroup(entry: SceneryManagerEntry): GroupCandidate | null {
  const tile = entry.folderName.match(/[+-]\d{2}[+-]\d{3}/)?.[0]
  if (!tile) return null

  const normalized = normalizeFolderName(entry.folderName)
  const isLikelyOrtho =
    normalized.includes('ortho') ||
    normalized.includes('tile') ||
    entry.category === SceneryCategory.Mesh ||
    entry.category === SceneryCategory.AirportMesh

  if (!isLikelyOrtho) return null

  return {
    key: `ortho:${tile}`,
    kind: 'ortho',
    title: `Ortho Tile ${tile}`,
  }
}

function detectAirportGroup(entry: SceneryManagerEntry): GroupCandidate | null {
  if (!AIRPORT_CATEGORIES.has(entry.category)) return null

  const icao =
    entry.airportId?.match(/^[A-Z0-9]{3,5}$/i)?.[0] ??
    entry.folderName.match(/(?:^|[\s_.-])([A-Z][A-Z0-9]{2,4})(?:[\s_.-]|$)/)?.[1]

  if (!icao) return null

  const normalizedIcao = icao.toUpperCase()
  return {
    key: `airport:${normalizedIcao}`,
    kind: 'airport',
    title: `Airport ${normalizedIcao}`,
  }
}

function detectProductGroup(entry: SceneryManagerEntry): GroupCandidate | null {
  const withoutTile = entry.folderName.replace(/[+-]\d{2}[+-]\d{3}/g, ' ')
  const tokens = withoutTile
    .toLowerCase()
    .split(/[\s_.-]+/)
    .filter(Boolean)
    .filter((token) => !/^(v?\d+(\.\d+)*|xp\d*|x-plane|mesh|overlay|hd|uhd|library)$/.test(token))

  const prefixTokens: string[] = []
  for (const token of tokens) {
    if (/^\d+$/.test(token)) break
    prefixTokens.push(token)
    if (prefixTokens.length === 3) break
  }

  if (prefixTokens.length < 2) return null

  const key = prefixTokens.join('-')
  if (key.length < 5) return null

  return {
    key: `product:${key}`,
    kind: 'product',
    title: titleCaseWords(key),
  }
}

function buildGroupsForDetector(
  entries: SceneryManagerEntry[],
  assigned: Set<string>,
  detector: GroupDetector,
): SmartSceneryGroup[] {
  const buckets = new Map<string, { candidate: GroupCandidate; entries: SceneryManagerEntry[] }>()

  for (const entry of entries) {
    if (assigned.has(entry.folderName)) continue

    const candidate = detector(entry)
    if (!candidate) continue

    const bucket = buckets.get(candidate.key)
    if (bucket) {
      bucket.entries.push(entry)
    } else {
      buckets.set(candidate.key, { candidate, entries: [entry] })
    }
  }

  const groups: SmartSceneryGroup[] = []
  for (const bucket of buckets.values()) {
    if (bucket.entries.length < 2) continue

    for (const entry of bucket.entries) {
      assigned.add(entry.folderName)
    }

    groups.push({
      id: bucket.candidate.key,
      kind: bucket.candidate.kind,
      title: bucket.candidate.title,
      entries: bucket.entries,
      enabledCount: bucket.entries.filter((entry) => entry.enabled).length,
      totalCount: bucket.entries.length,
    })
  }

  return groups
}

export function buildSmartSceneryRows(entries: SceneryManagerEntry[]): SmartSceneryRow[] {
  const assigned = new Set<string>()
  const groups = [
    ...buildGroupsForDetector(entries, assigned, detectSimHeavenGroup),
    ...buildGroupsForDetector(entries, assigned, detectOrthoGroup),
    ...buildGroupsForDetector(entries, assigned, detectAirportGroup),
    ...buildGroupsForDetector(entries, assigned, detectProductGroup),
  ]

  const groupByEntry = new Map<string, SmartSceneryGroup>()
  for (const group of groups) {
    for (const entry of group.entries) {
      groupByEntry.set(entry.folderName, group)
    }
  }

  const emittedGroups = new Set<string>()
  const rows: SmartSceneryRow[] = []

  for (const entry of entries) {
    const group = groupByEntry.get(entry.folderName)
    if (!group) {
      rows.push({ rowType: 'entry', entry })
      continue
    }

    if (emittedGroups.has(group.id)) continue

    emittedGroups.add(group.id)
    rows.push({ rowType: 'group', group })
  }

  return rows
}
