import { afterEach, describe, expect, it, vi } from 'vitest'
import rootEntrypoint from '../../[...route].js'
import apiEntrypoint from '../../api/[...route].js'

const upstreamRef = 'https://api.github.com/repos/CCA3370/XFast-Manager/git/ref/heads/ad-campaigns'
const upstreamRaw = 'https://raw.githubusercontent.com/CCA3370/XFast-Manager'
const revision = 'a'.repeat(40)
const nextRevision = 'b'.repeat(40)
const manifestPath = 'ad-campaigns/manifest.json'
const imagePath = 'ad-campaigns/images/banner-1.png'
const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
const empty = { schemaVersion: 1, revision: null, campaigns: [] }

function makeResponse() {
  return {
    headers: new Headers(),
    statusCode: 200,
    body: undefined,
    setHeader(name, value) {
      this.headers.set(name, value)
    },
    status(code) {
      this.statusCode = code
      return this
    },
    json(body) {
      this.body = body
    },
    end(body) {
      this.body = body
    },
  }
}

async function request(entrypoint, route, method = 'GET') {
  const url = new URL(route, 'https://x-fast-manager.vercel.app')
  const query = Object.fromEntries(url.searchParams)
  const res = makeResponse()
  await entrypoint({ method, url: `${url.pathname}${url.search}`, query }, res)
  return res
}

function refResponse(sha = revision) {
  return Response.json({ object: { type: 'commit', sha } })
}

function campaign(overrides = {}) {
  return {
    id: 'startup-sponsor',
    enabled: true,
    creative: { image: 'banner-1.png', alt: 'Sponsor artwork' },
    url: 'https://example.org/offer',
    ...overrides,
  }
}

function source(campaigns = [], overrides = {}) {
  return { schemaVersion: 1, enabled: true, campaigns, ...overrides }
}

function mockFetch(...responses) {
  const fetchMock = vi.fn()
  for (const response of responses) fetchMock.mockResolvedValueOnce(response)
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('ad campaigns route', () => {
  it('serves enabled campaigns in source order with normalized text and valid localizations', async () => {
    const startsAt = '2026-12-01T00:00:00.1Z'
    const endsAt = '2026-12-02T00:00:00Z'
    const fetchMock = mockFetch(
      refResponse(),
      Response.json(
        source([
          campaign({ id: 'first', url: 'file:///tmp/launch' }),
          campaign({ id: 'disabled', enabled: false }),
          campaign({
            id: 'first',
            startsAt,
            endsAt,
            creative: { image: 'banner-1.png', alt: ' Available now ' },
            localizations: {
              zh: { image: 'banner_zh.png', alt: '中文广告' },
              en: { image: '../invalid.png', alt: 'Skipped artwork' },
            },
          }),
          campaign({ id: 'first', url: 'https://example.org/duplicate' }),
          campaign({ id: 'second', url: 'https://example.org' }),
        ]),
      ),
    )

    const res = await request(rootEntrypoint, '/api/ad-campaigns?ref=untrusted')

    expect(res.statusCode).toBe(200)
    expect(res.body).toEqual({
      schemaVersion: 1,
      revision,
      campaigns: [
        {
          id: 'first',
          startsAt,
          endsAt,
          creative: { image: 'banner-1.png', alt: 'Available now' },
          localizations: { zh: { image: 'banner_zh.png', alt: '中文广告' } },
          url: 'https://example.org/offer',
        },
        {
          id: 'second',
          creative: { image: 'banner-1.png', alt: 'Sponsor artwork' },
          url: 'https://example.org/',
        },
      ],
    })
    expect(res.headers.get('access-control-allow-origin')).toBe('*')
    expect(res.headers.has('access-control-allow-credentials')).toBe(false)
    expect(res.headers.get('cache-control')).toBe('no-store')
    expect(res.headers.get('vercel-cdn-cache-control')).toBe('public, s-maxage=60')
    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      upstreamRef,
      `${upstreamRaw}/${revision}/${manifestPath}`,
    ])
    expect(fetchMock.mock.calls[1][1].redirect).toBe('error')
  })

  it('skips invalid schedules, IDs, destinations, and image filenames', async () => {
    mockFetch(
      refResponse(),
      Response.json(
        source([
          campaign({ id: 'bad/date', startsAt: '2026-12-01T00:00:00Z' }),
          campaign({ id: 'invalid-day', startsAt: '2026-02-29T00:00:00Z' }),
          campaign({ id: 'offset', startsAt: '2026-12-01T00:00:00+00:00' }),
          campaign({
            id: 'reversed',
            startsAt: '2026-12-02T00:00:00Z',
            endsAt: '2026-12-01T00:00:00Z',
          }),
          campaign({ id: 'credentials', url: 'https://user:pass@example.org/' }),
          campaign({ id: 'whitespace', url: 'https://example.org/promo\ncode' }),
          campaign({ id: 'backslash', url: 'https://example.org\\path' }),
          campaign({ id: 'not-an-image', creative: { image: 'inline.svg', alt: 'SVG' } }),
          campaign({ id: 'valid', endsAt: '2028-02-29T00:00:00Z' }),
        ]),
      ),
    )

    const res = await request(apiEntrypoint, '/api/ad-campaigns')

    expect(res.statusCode).toBe(200)
    expect(res.body.campaigns.map(({ id }) => id)).toEqual(['valid'])
  })

  it('marks only campaigns with literal first-party opt-in', async () => {
    mockFetch(
      refResponse(),
      Response.json(
        source([
          campaign({ id: 'ours', firstParty: true }),
          campaign({ id: 'external', firstParty: false }),
          campaign({ id: 'legacy' }),
          campaign({ id: 'invalid-flag', firstParty: 'true' }),
        ]),
      ),
    )

    const res = await request(apiEntrypoint, '/api/ad-campaigns')

    expect(res.statusCode).toBe(200)
    expect(res.body.campaigns.map(({ id }) => id)).toEqual([
      'ours',
      'external',
      'legacy',
      'invalid-flag',
    ])
    expect(res.body.campaigns[0]).toHaveProperty('firstParty', true)
    for (const ordinary of res.body.campaigns.slice(1)) {
      expect(ordinary).not.toHaveProperty('firstParty')
    }
  })

  it.each(['ref', 'manifest', 'disabled'])('returns an empty document for %s', async (kind) => {
    const missing = new Response(null, { status: 404 })
    if (kind === 'ref') mockFetch(missing)
    if (kind === 'manifest') mockFetch(refResponse(), missing)
    if (kind === 'disabled')
      mockFetch(refResponse(), Response.json(source([campaign()], { enabled: false })))

    const res = await request(apiEntrypoint, '/api/ad-campaigns')

    expect(res.statusCode).toBe(200)
    expect(res.body).toEqual(empty)
  })

  it.each(['upstream status', 'malformed revision', 'malformed schema', 'oversized source'])(
    'fails closed without caching %s',
    async (kind) => {
      if (kind === 'upstream status') mockFetch(new Response(null, { status: 503 }))
      if (kind === 'malformed revision') mockFetch(refResponse('../not-a-sha'))
      if (kind === 'malformed schema')
        mockFetch(refResponse(), Response.json(source([], { schemaVersion: 2 })))
      if (kind === 'oversized source')
        mockFetch(refResponse(), new Response(' '.repeat(256 * 1024 + 1)))

      const res = await request(apiEntrypoint, '/api/ad-campaigns')

      expect(res.statusCode).toBe(502)
      expect(res.headers.get('cache-control')).toBe('no-store')
      expect(res.headers.has('vercel-cdn-cache-control')).toBe(false)
      expect(res.body).toEqual({ error: 'Ad campaigns unavailable' })
    },
  )

  it('fails closed when the shared upstream deadline aborts a streamed manifest', async () => {
    const controller = new AbortController()
    vi.spyOn(AbortSignal, 'timeout').mockReturnValue(controller.signal)
    const fetchMock = mockFetch(refResponse())
    fetchMock.mockImplementationOnce(async (_url, { signal }) => {
      const body = new ReadableStream({
        start(stream) {
          signal.addEventListener('abort', () => stream.error(signal.reason), { once: true })
        },
      })
      controller.abort()
      return new Response(body)
    })

    const res = await request(rootEntrypoint, '/api/ad-campaigns')

    expect(res.statusCode).toBe(502)
    expect(res.headers.get('cache-control')).toBe('no-store')
    expect(fetchMock.mock.calls.map(([, options]) => options.signal)).toEqual([
      controller.signal,
      controller.signal,
    ])
  })
})

describe('ad image route', () => {
  it('uses the manifest revision independently of later changes to the campaign branch', async () => {
    const fetchMock = mockFetch(
      refResponse(),
      Response.json(source([campaign()])),
      refResponse(nextRevision),
      Response.json(source([campaign()])),
      new Response(pngSignature, { headers: { 'Content-Type': 'image/png' } }),
    )

    const first = await request(apiEntrypoint, '/api/ad-campaigns')
    const second = await request(apiEntrypoint, '/api/ad-campaigns')
    const image = await request(
      rootEntrypoint,
      `/api/ad-image?revision=${first.body.revision}&name=${first.body.campaigns[0].creative.image}`,
    )

    expect(second.body.revision).toBe(nextRevision)
    expect(fetchMock.mock.calls.at(-1)[0]).toBe(`${upstreamRaw}/${revision}/${imagePath}`)
    expect(image.statusCode).toBe(200)
    expect(image.body).toEqual(pngSignature)
    expect(image.headers.get('content-type')).toBe('image/png')
    expect(image.headers.get('x-content-type-options')).toBe('nosniff')
    expect(image.headers.get('cache-control')).toContain('immutable')
    expect(image.headers.get('access-control-allow-origin')).toBe('*')
  })

  it.each([
    `revision=${revision}&name=${encodeURIComponent('../banner-1.png')}`,
    `revision=${revision}&name=${encodeURIComponent('..\\banner-1.png')}`,
    `revision=${revision}&name=advertiser.svg`,
    `revision=${revision}&name=orig.png&name=other.png`,
    'revision=main&name=banner-1.png',
  ])('rejects invalid image parameters %s without making a request', async (query) => {
    const fetchMock = mockFetch()
    const res = await request(rootEntrypoint, `/api/ad-image?${query}`)

    expect(res.statusCode).toBe(400)
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it.each([
    ['webp', 'image/webp', Buffer.from([82, 73, 70, 70, 4, 0, 0, 0, 87, 69, 66, 80])],
    ['jpg', 'image/jpeg', Buffer.from([255, 216, 255, 224])],
  ])('allows supported %s raster assets', async (extension, type, bytes) => {
    mockFetch(new Response(bytes, { headers: { 'Content-Type': type } }))
    const res = await request(
      apiEntrypoint,
      `/api/ad-image?revision=${revision}&name=asset.${extension}`,
    )

    expect(res.statusCode).toBe(200)
    expect(res.body).toEqual(bytes)
    expect(res.headers.get('content-type')).toBe(type)
  })

  it('checks exact WebP signature bytes', async () => {
    const bogusWebpSignature = Buffer.from([210, 201, 198, 198, 4, 0, 0, 0, 215, 197, 194, 208])
    mockFetch(new Response(bogusWebpSignature, { headers: { 'Content-Type': 'image/webp' } }))

    const res = await request(apiEntrypoint, `/api/ad-image?revision=${revision}&name=asset.webp`)

    expect(res.statusCode).toBe(502)
    expect(res.headers.get('cache-control')).toBe('no-store')
  })

  it.each([
    [
      'disallowed MIME type',
      new Response(pngSignature, { headers: { 'Content-Type': 'text/html' } }),
      502,
    ],
    [
      'file signature mismatch',
      new Response(Buffer.from([255, 216, 255, 224]), { headers: { 'Content-Type': 'image/png' } }),
      502,
    ],
    [
      'oversized streamed body',
      new Response(Buffer.alloc(2 * 1024 * 1024 + 1), { headers: { 'Content-Type': 'image/png' } }),
      502,
    ],
    [
      'redirect',
      new Response(null, { status: 302, headers: { location: 'https://example.org' } }),
      502,
    ],
    ['missing image', new Response(null, { status: 404 }), 404],
  ])('fails closed without caching a %s', async (_description, response, status) => {
    mockFetch(response)
    const res = await request(apiEntrypoint, `/api/ad-image?revision=${revision}&name=banner-1.png`)

    expect(res.statusCode).toBe(status)
    expect(res.headers.get('cache-control')).toBe('no-store')
  })

  it.each(['ad-campaigns', 'ad-image'])('offers only GET access to %s', async (route) => {
    const fetchMock = mockFetch()
    const res = await request(apiEntrypoint, `/api/${route}`, 'POST')

    expect(res.statusCode).toBe(405)
    expect(res.headers.get('allow')).toBe('GET')
    expect(fetchMock).not.toHaveBeenCalled()
  })
})
