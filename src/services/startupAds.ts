import { getItem, saveSettings, STORAGE_KEYS } from '@/services/storage'
import { buildVercelApiUrl } from '@/services/vercelApi'

const MAX_MANIFEST_BYTES = 256 * 1024
const MAX_IMAGE_BYTES = 2 * 1024 * 1024
const IMAGE_TIMEOUT_MS = 2000
const CAMPAIGN_ID = /^[a-z0-9][a-z0-9_-]{0,63}$/i
const IMAGE_NAME = /^[a-z0-9][a-z0-9_-]{0,95}\.(?:png|jpe?g|webp)$/i
const REVISION = /^[a-f0-9]{40}$/
const UTC_DATE = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,3}))?Z$/
const IMAGE_CONTENT_TYPES = ['image/png', 'image/jpeg', 'image/webp']

interface Creative {
  image: string
  alt: string
}

export interface StartupAd {
  id: string
  firstParty?: true
  imageUrl: string
  alt: string
  url: string
  endsAt?: number
}

interface LoadStartupAdOptions {
  locale: string
  seenIds: readonly string[]
  signal: AbortSignal
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function parseStartupAdUrl(value: unknown): string | null {
  if (typeof value !== 'string' || !/^https:\/\//i.test(value) || /[\s\\]/.test(value)) return null

  try {
    const url = new URL(value)
    return url.protocol === 'https:' && url.hostname && !url.username && !url.password
      ? url.href
      : null
  } catch {
    return null
  }
}

function parseCreative(value: unknown): Creative | null {
  if (!isObject(value) || typeof value.image !== 'string' || !IMAGE_NAME.test(value.image)) {
    return null
  }

  const alt = typeof value.alt === 'string' ? value.alt.trim() : ''
  return alt && alt.length <= 300 ? { image: value.image, alt } : null
}

function parseUtcDate(value: unknown): number {
  if (typeof value !== 'string') return NaN
  const parts = UTC_DATE.exec(value)
  const date = new Date(value)
  if (!parts || !Number.isFinite(date.getTime())) return NaN

  const canonical = `${parts[1]}.${(parts[2] ?? '').padEnd(3, '0')}Z`
  return date.toISOString() === canonical ? date.getTime() : NaN
}

function isActive(start: number, end: number): boolean {
  const now = Date.now()
  return start <= now && now < end
}

function fetchAdResource(url: string, signal: AbortSignal) {
  return fetch(url, {
    signal,
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
    redirect: 'error',
  })
}

async function readBoundedBody(
  response: Response,
  limit: number,
): Promise<Uint8Array<ArrayBuffer>> {
  const declaredSize = response.headers.get('content-length')
  if (!response.body || (declaredSize !== null && Number(declaredSize) > limit)) {
    await response.body?.cancel()
    throw new Error('Invalid startup ad response size')
  }

  const reader = response.body.getReader()
  const chunks: Uint8Array[] = []
  let size = 0

  try {
    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      size += value.byteLength
      if (size > limit) {
        await reader.cancel()
        throw new Error('Invalid startup ad response size')
      }
      chunks.push(value)
    }
  } finally {
    reader.releaseLock()
  }

  const bytes = new Uint8Array(size)
  let offset = 0
  for (const chunk of chunks) {
    bytes.set(chunk, offset)
    offset += chunk.byteLength
  }
  return bytes
}

function decodeImage(src: string, signal: AbortSignal): Promise<void> {
  if (signal.aborted) return Promise.reject(signal.reason)

  const image = new Image()
  return new Promise((resolve, reject) => {
    const abort = () => {
      signal.removeEventListener('abort', abort)
      image.removeAttribute('src')
      reject(signal.reason)
    }

    signal.addEventListener('abort', abort, { once: true })
    image.src = src
    void image.decode().then(
      () => {
        signal.removeEventListener('abort', abort)
        resolve()
      },
      (error: unknown) => {
        signal.removeEventListener('abort', abort)
        reject(error)
      },
    )
  })
}

async function loadImage(revision: string, name: string, signal: AbortSignal): Promise<string> {
  if (signal.aborted) throw signal.reason

  const controller = new AbortController()
  const abort = () => controller.abort(signal.reason)
  const timeout = setTimeout(() => controller.abort(), IMAGE_TIMEOUT_MS)
  signal.addEventListener('abort', abort, { once: true })

  const query = new URLSearchParams({ revision, name })
  try {
    const response = await fetchAdResource(
      `${buildVercelApiUrl('ad-image')}?${query}`,
      controller.signal,
    )
    const type = response.headers.get('content-type')?.split(';', 1)[0]?.trim().toLowerCase()
    if (!response.ok || !type || !IMAGE_CONTENT_TYPES.includes(type)) {
      await response.body?.cancel()
      throw new Error('Invalid startup ad image response')
    }

    const bytes = await readBoundedBody(response, MAX_IMAGE_BYTES)
    const imageUrl = URL.createObjectURL(new Blob([bytes], { type }))
    try {
      await decodeImage(imageUrl, controller.signal)
      return imageUrl
    } catch (error) {
      URL.revokeObjectURL(imageUrl)
      throw error
    }
  } finally {
    clearTimeout(timeout)
    signal.removeEventListener('abort', abort)
  }
}

export async function loadStartupAd({
  locale,
  seenIds,
  signal,
}: LoadStartupAdOptions): Promise<StartupAd | null> {
  if (signal.aborted) return null

  try {
    const response = await fetchAdResource(buildVercelApiUrl('ad-campaigns'), signal)
    if (!response.ok) {
      await response.body?.cancel()
      return null
    }

    const bytes = await readBoundedBody(response, MAX_MANIFEST_BYTES)
    const manifest: unknown = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes))
    if (
      !isObject(manifest) ||
      manifest.schemaVersion !== 1 ||
      typeof manifest.revision !== 'string' ||
      !REVISION.test(manifest.revision) ||
      !Array.isArray(manifest.campaigns)
    ) {
      return null
    }

    const seen = new Set(seenIds)
    for (const campaign of manifest.campaigns) {
      if (signal.aborted) break
      if (
        !isObject(campaign) ||
        typeof campaign.id !== 'string' ||
        !CAMPAIGN_ID.test(campaign.id) ||
        seen.has(campaign.id)
      ) {
        continue
      }

      const url = parseStartupAdUrl(campaign.url)
      const base = parseCreative(campaign.creative)
      const start = campaign.startsAt === undefined ? -Infinity : parseUtcDate(campaign.startsAt)
      const end = campaign.endsAt === undefined ? Infinity : parseUtcDate(campaign.endsAt)
      if (!url || !base || !isActive(start, end)) continue

      const local = isObject(campaign.localizations)
        ? parseCreative(campaign.localizations[locale])
        : null
      for (const creative of local && local.image !== base.image
        ? [local, base]
        : [local ?? base]) {
        try {
          const imageUrl = await loadImage(manifest.revision, creative.image, signal)
          if (!signal.aborted && isActive(start, end)) {
            return {
              id: campaign.id,
              ...(campaign.firstParty === true && { firstParty: true }),
              imageUrl,
              alt: creative.alt,
              url,
              endsAt: Number.isFinite(end) ? end : undefined,
            }
          }
          URL.revokeObjectURL(imageUrl)
          break
        } catch {
          if (signal.aborted) return null
        }
      }
    }
  } catch {
    // Startup advertising is optional. Leave the application available on any feed failure.
  }

  return null
}

export async function loadSeenStartupAdIds(): Promise<readonly string[]> {
  const saved = await getItem<unknown>(STORAGE_KEYS.HIDDEN_STARTUP_ADS)
  return Array.isArray(saved)
    ? saved.filter((id): id is string => typeof id === 'string' && CAMPAIGN_ID.test(id))
    : []
}

let preferenceWrite: Promise<void> = Promise.resolve()

export function setStartupAdSeen(id: string, seen: boolean): Promise<void> {
  const write = preferenceWrite.then(async () => {
    const saved = new Set(await loadSeenStartupAdIds())
    if (seen) saved.add(id)
    else saved.delete(id)
    await saveSettings({ [STORAGE_KEYS.HIDDEN_STARTUP_ADS]: [...saved] })
  })
  preferenceWrite = write.catch(() => {})
  return write
}
