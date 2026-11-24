import {
  UPSTREAM_TIMEOUT_MS,
  fetchAdCampaignFile,
  fetchAdCampaignRef,
  isAdImageFilename,
  readJson,
} from '../ad-campaign-source.js'

const MAX_MANIFEST_BYTES = 256 * 1024
const campaignIdPattern = /^[a-z0-9][a-z0-9_-]{0,63}$/i
const datePattern = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,3}))?Z$/
const emptyManifest = { schemaVersion: 1, revision: null, campaigns: [] }

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function parseCampaignDate(value) {
  if (value === undefined) return undefined
  if (typeof value !== 'string') return null

  const match = datePattern.exec(value)
  if (!match) return null

  const expected = `${match[1]}.${(match[2] || '').padEnd(3, '0')}Z`
  const date = new Date(expected)
  return Number.isNaN(date.getTime()) || date.toISOString() !== expected ? null : date.getTime()
}

function normalizeDestination(value) {
  if (
    typeof value !== 'string' ||
    value.length > 2048 ||
    !/^https:\/\//i.test(value) ||
    /[\s\\]/.test(value)
  ) {
    return null
  }

  try {
    const url = new URL(value)
    if (url.protocol !== 'https:' || !url.hostname || url.username || url.password) return null
    return url.href.length <= 2048 ? url.href : null
  } catch {
    return null
  }
}

function normalizeCreative(value) {
  if (!isRecord(value) || !isAdImageFilename(value.image) || typeof value.alt !== 'string') {
    return null
  }
  const alt = value.alt.trim()
  return alt.length > 0 && alt.length <= 300 ? { image: value.image, alt } : null
}

function normalizeLocalizations(value) {
  const localizations = {}
  if (!isRecord(value)) return localizations

  for (const [locale, rawCreative] of Object.entries(value)) {
    const creative = normalizeCreative(rawCreative)
    if (!creative) continue

    try {
      const [canonicalLocale] = Intl.getCanonicalLocales(locale)
      if (!Object.hasOwn(localizations, canonicalLocale)) {
        localizations[canonicalLocale] = creative
      }
    } catch {
      // Invalid locale tags leave the default creative available.
    }
  }
  return localizations
}

function normalizeManifest(raw, revision) {
  if (
    !isRecord(raw) ||
    raw.schemaVersion !== 1 ||
    typeof raw.enabled !== 'boolean' ||
    !Array.isArray(raw.campaigns)
  ) {
    throw new Error('Invalid ad campaign manifest')
  }
  if (!raw.enabled) return emptyManifest

  const campaigns = new Map()
  for (const row of raw.campaigns) {
    if (
      !isRecord(row) ||
      row.enabled !== true ||
      typeof row.id !== 'string' ||
      !campaignIdPattern.test(row.id) ||
      campaigns.has(row.id)
    ) {
      continue
    }

    const creative = normalizeCreative(row.creative)
    const url = normalizeDestination(row.url)
    const start = parseCampaignDate(row.startsAt)
    const end = parseCampaignDate(row.endsAt)
    if (
      !creative ||
      !url ||
      start === null ||
      end === null ||
      (start !== undefined && end !== undefined && end <= start)
    ) {
      continue
    }

    const localizations = normalizeLocalizations(row.localizations)
    campaigns.set(row.id, {
      id: row.id,
      ...(row.firstParty === true && { firstParty: true }),
      startsAt: row.startsAt,
      endsAt: row.endsAt,
      creative,
      ...(Object.keys(localizations).length > 0 && { localizations }),
      url,
    })
  }

  return { schemaVersion: 1, revision, campaigns: [...campaigns.values()] }
}

async function loadManifest() {
  const signal = AbortSignal.timeout(UPSTREAM_TIMEOUT_MS)
  const revision = await fetchAdCampaignRef(signal)
  if (!revision) return emptyManifest

  const response = await fetchAdCampaignFile(revision, 'manifest.json', signal)
  if (response.status === 404) {
    await response.body?.cancel()
    return emptyManifest
  }

  const manifest = normalizeManifest(await readJson(response, MAX_MANIFEST_BYTES), revision)
  if (Buffer.byteLength(JSON.stringify(manifest)) > MAX_MANIFEST_BYTES) {
    throw new Error('Ad campaign manifest exceeds the size limit')
  }
  return manifest
}

export default async function handler(req, res) {
  res.setHeader('Access-Control-Allow-Origin', '*')
  res.setHeader('Cache-Control', 'no-store')
  if (req.method !== 'GET') {
    res.setHeader('Allow', 'GET')
    return res.status(405).json({ error: 'Method not allowed' })
  }

  try {
    const manifest = await loadManifest()
    res.setHeader('Vercel-CDN-Cache-Control', 'public, s-maxage=60')
    return res.status(200).json(manifest)
  } catch {
    return res.status(502).json({ error: 'Ad campaigns unavailable' })
  }
}
