import 'fake-indexeddb/auto'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { loadMapTile } from './mapTiles'
const request = vi.fn()
const image = () =>
  new Response(new Blob(['map tile'], { type: 'image/png' }), {
    headers: { 'content-type': 'image/png', 'cache-control': 'max-age=604800' },
  })
const signal = () => new AbortController().signal
beforeEach(() => {
  vi.stubGlobal('fetch', request)
  vi.stubGlobal('navigator', { onLine: true })
  request.mockReset()
  request.mockImplementation(async () => image())
})
afterEach(() => vi.unstubAllGlobals())
describe('viewed map tile cache', () => {
  it('saves a viewed map image and reuses it offline without another network request', async () => {
    const first = await loadMapTile(4, 1, 1, signal())
    expect(first.source).toBe('online')
    first.revoke?.()
    vi.stubGlobal('navigator', { onLine: false })
    const offline = await loadMapTile(4, 1, 1, signal())
    expect(offline.source).toBe('cache')
    expect(request).toHaveBeenCalledTimes(1)
    offline.revoke?.()
  })
  it('reuses unexpired cached tiles online and honors the response cache policy', async () => {
    const first = await loadMapTile(4, 2, 2, signal())
    first.revoke?.()
    const second = await loadMapTile(4, 2, 2, signal())
    expect(second.source).toBe('cache')
    expect(request).toHaveBeenCalledTimes(1)
    second.revoke?.()
    expect(request.mock.calls[0]?.[1].cache).toBe('default')
  })
  it('does not persist tiles marked no-store', async () => {
    request.mockImplementation(
      async () =>
        new Response(new Blob(['tile'], { type: 'image/png' }), {
          headers: { 'cache-control': 'no-store' },
        }),
    )
    const first = await loadMapTile(4, 3, 3, signal())
    first.revoke?.()
    const second = await loadMapTile(4, 3, 3, signal())
    expect(second.source).toBe('online')
    expect(request).toHaveBeenCalledTimes(2)
    second.revoke?.()
  })
  it('reports uncached offline areas without inventing a geographic basemap', async () => {
    vi.stubGlobal('navigator', { onLine: false })
    expect((await loadMapTile(19, 50000, 50000, signal())).source).toBe('missing')
    expect(request).not.toHaveBeenCalled()
  })
  it('does not start a request after its map tile was cancelled', async () => {
    const controller = new AbortController()
    controller.abort()
    await loadMapTile(19, 51000, 51000, controller.signal)
    expect(request).not.toHaveBeenCalled()
  })
})
