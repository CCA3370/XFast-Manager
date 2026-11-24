import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getItem, saveSettings, STORAGE_KEYS } from '@/services/storage'
import { loadSeenStartupAdIds, loadStartupAd, setStartupAdSeen } from './startupAds'

vi.mock('@/services/storage', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/storage')>()),
  getItem: vi.fn(),
  saveSettings: vi.fn(),
}))

const revision = 'a'.repeat(40)
const fetchResource = vi.fn<typeof fetch>()
const decode = vi.fn<() => Promise<void>>()
const campaign = {
  id: 'campaign-one',
  creative: { image: 'default.png', alt: 'Default creative' },
  url: 'https://example.test/promotion',
}

function serveCampaigns(campaigns: unknown[]) {
  fetchResource.mockResolvedValueOnce(Response.json({ schemaVersion: 1, revision, campaigns }))
}

function imageResponse(type = 'image/png') {
  return new Response('test-image', { headers: { 'Content-Type': type } })
}

function load(seenIds: string[] = [], signal = new AbortController().signal) {
  return loadStartupAd({ locale: 'zh', seenIds, signal })
}

function requestedImages() {
  return fetchResource.mock.calls
    .slice(1)
    .map(([url]) => new URL(String(url)).searchParams.get('name'))
}

beforeEach(() => {
  fetchResource.mockReset()
  decode.mockReset().mockResolvedValue(undefined)
  vi.stubGlobal('fetch', fetchResource)
  vi.stubGlobal(
    'Image',
    class {
      decode = decode
      removeAttribute() {}
    },
  )
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:test-image')
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  vi.mocked(getItem).mockReset()
  vi.mocked(saveSettings).mockReset()
})

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('startup ad delivery', () => {
  it('selects the first unseen active creative for the current language', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-11-24T12:00:00Z'))
    serveCampaigns([
      { ...campaign, id: 'seen' },
      { ...campaign, id: 'future', startsAt: '2026-11-24T12:00:01Z' },
      { ...campaign, id: 'expired', endsAt: '2026-11-24T12:00:00Z' },
      { ...campaign, id: 'invalid-date', endsAt: '2026-11-31T00:00:00Z' },
      {
        ...campaign,
        startsAt: '2026-11-24T12:00:00Z',
        endsAt: '2026-11-24T12:01:00Z',
        localizations: { zh: { image: 'localized.webp', alt: '本地化广告' } },
      },
      { ...campaign, id: 'next' },
    ])
    fetchResource.mockResolvedValueOnce(imageResponse('image/webp'))

    await expect(load(['seen'])).resolves.toEqual({
      id: 'campaign-one',
      imageUrl: 'blob:test-image',
      alt: '本地化广告',
      url: campaign.url,
      endsAt: Date.parse('2026-11-24T12:01:00Z'),
    })
    expect(requestedImages()).toEqual(['localized.webp'])
    expect(new URL(String(fetchResource.mock.calls[1]?.[0])).searchParams.get('revision')).toBe(
      revision,
    )
    for (const [, options] of fetchResource.mock.calls) {
      expect(options).toMatchObject({
        credentials: 'omit',
        referrerPolicy: 'no-referrer',
        redirect: 'error',
      })
    }
  })

  it.each([true, false, undefined, 'true'])(
    'honors first-party opt-in only for literal true (%s)',
    async (firstParty) => {
      serveCampaigns([{ ...campaign, firstParty }])
      fetchResource.mockResolvedValueOnce(imageResponse())

      const ad = await load()

      expect(ad?.id).toBe(campaign.id)
      if (firstParty === true) expect(ad).toHaveProperty('firstParty', true)
      else expect(ad).not.toHaveProperty('firstParty')
    },
  )

  it('skips unsafe destinations and filenames and falls through a creative that cannot decode', async () => {
    serveCampaigns([
      { ...campaign, url: 'file:///etc/passwd' },
      { ...campaign, url: 'https://user:password@example.test' },
      { ...campaign, creative: { image: '../default.png', alt: 'invalid image' } },
      {
        ...campaign,
        localizations: { zh: { image: 'invalid.webp', alt: '本地化广告' } },
      },
      { ...campaign, id: 'backup', creative: { image: 'backup.png', alt: 'Backup' } },
    ])
    fetchResource.mockResolvedValueOnce(imageResponse('image/webp'))
    fetchResource.mockResolvedValueOnce(imageResponse('image/svg+xml'))
    fetchResource.mockResolvedValueOnce(imageResponse())
    decode.mockRejectedValueOnce(new Error('Image cannot decode'))

    await expect(load()).resolves.toMatchObject({ id: 'backup', alt: 'Backup' })
    expect(requestedImages()).toEqual(['invalid.webp', 'default.png', 'backup.png'])
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(1)
  })

  it('skips a stalled image and aborts decoding when the startup opportunity ends', async () => {
    vi.useFakeTimers()
    serveCampaigns([
      campaign,
      { ...campaign, id: 'backup', creative: { image: 'backup.png', alt: 'Backup' } },
    ])
    fetchResource.mockImplementationOnce(
      (_url, options) =>
        new Promise((_resolve, reject) => {
          options?.signal?.addEventListener('abort', () => reject(options.signal?.reason))
        }),
    )
    fetchResource.mockResolvedValueOnce(imageResponse())
    decode.mockImplementationOnce(() => new Promise(() => {}))
    const controller = new AbortController()
    const result = load([], controller.signal)

    await vi.advanceTimersByTimeAsync(2100)
    expect(requestedImages()).toEqual(['default.png', 'backup.png'])
    expect(decode).toHaveBeenCalledTimes(1)
    controller.abort()

    await expect(result).resolves.toBeNull()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:test-image')
  })

  it('returns no ad for an unsupported manifest or an oversized streamed image', async () => {
    fetchResource.mockResolvedValueOnce(
      Response.json({ schemaVersion: 2, revision, campaigns: [campaign] }),
    )
    await expect(load()).resolves.toBeNull()
    expect(fetchResource).toHaveBeenCalledTimes(1)

    serveCampaigns([campaign])
    fetchResource.mockResolvedValueOnce(
      new Response(new Uint8Array(2 * 1024 * 1024 + 1), {
        headers: { 'Content-Type': 'image/png' },
      }),
    )
    await expect(load()).resolves.toBeNull()
    expect(decode).not.toHaveBeenCalled()
  })
})

describe('local startup ad preferences', () => {
  it('serializes immediate reversals and retains other campaign IDs', async () => {
    const key = STORAGE_KEYS.HIDDEN_STARTUP_ADS
    let saved: unknown = ['other', '..', 45]
    vi.mocked(getItem).mockImplementation(async () => saved as never)
    vi.mocked(saveSettings).mockImplementation(async (values) => {
      saved = values[key]
    })

    await expect(loadSeenStartupAdIds()).resolves.toEqual(['other'])
    await Promise.all([setStartupAdSeen(campaign.id, true), setStartupAdSeen(campaign.id, false)])

    expect(saveSettings).toHaveBeenNthCalledWith(1, { [key]: ['other', campaign.id] })
    expect(saveSettings).toHaveBeenNthCalledWith(2, { [key]: ['other'] })
    await expect(loadSeenStartupAdIds()).resolves.toEqual(['other'])
  })
})
