interface CachedTile {
  key: string
  blob: Blob
  expires: number
  saved: number
}
let database: Promise<IDBDatabase> | undefined
function db() {
  return (database ??= new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open('xfast-map-tiles', 1)
    request.onupgradeneeded = () => {
      const store = request.result.createObjectStore('tiles', { keyPath: 'key' })
      store.createIndex('saved', 'saved')
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  }))
}
async function cached(key: string): Promise<CachedTile | undefined> {
  try {
    return await new Promise((resolve, reject) => {
      void db().then((d) => {
        const r = d.transaction('tiles').objectStore('tiles').get(key)
        r.onsuccess = () => resolve(r.result)
        r.onerror = () => reject(r.error)
      }, reject)
    })
  } catch {
    return undefined
  }
}
async function save(tile: CachedTile) {
  try {
    const d = await db()
    const tx = d.transaction('tiles', 'readwrite'),
      store = tx.objectStore('tiles')
    store.put(tile)
    const count = store.count()
    count.onsuccess = () => {
      let excess = count.result - 2000
      if (excess > 0) {
        const cursor = store.index('saved').openCursor()
        cursor.onsuccess = () => {
          const entry = cursor.result
          if (entry && excess-- > 0) {
            entry.delete()
            entry.continue()
          }
        }
      }
    }
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve()
      tx.onerror = () => reject(tx.error)
    })
  } catch {
    /* Cache quota or private mode must not prevent the online map from rendering. */
  }
}
export interface TileResult {
  url: string
  source: 'online' | 'cache' | 'missing'
  revoke?: () => void
}
function blobResult(blob: Blob, source: TileResult['source']): TileResult {
  const url = URL.createObjectURL(blob)
  return { url, source, revoke: () => URL.revokeObjectURL(url) }
}
export async function loadMapTile(
  z: number,
  x: number,
  y: number,
  signal: AbortSignal,
): Promise<TileResult> {
  const key = `${z}/${x}/${y}`,
    stored = await cached(key)
  if (stored && (stored.expires > Date.now() || !navigator.onLine))
    return blobResult(stored.blob, 'cache')
  if (navigator.onLine && !signal.aborted) {
    const controller = new AbortController()
    const abort = () => controller.abort()
    signal.addEventListener('abort', abort, { once: true })
    const timer = setTimeout(abort, 6000)
    try {
      const response = await fetch(`https://tile.openstreetmap.org/${key}.png`, {
        signal: controller.signal,
        cache: 'default',
      })
      if (!response.ok) throw new Error('Tile unavailable')
      const blob = await response.blob()
      if (!/^image\/(png|jpeg|webp)/.test(blob.type)) throw new Error('Not a map image')
      const policy = response.headers.get('cache-control') ?? ''
      const maxAge = policy.match(/max-age=(\d+)/)?.[1]
      const age = /no-cache/.test(policy) ? 0 : Number(maxAge ?? 604800)
      const expires =
        !maxAge && response.headers.has('expires')
          ? Date.parse(response.headers.get('expires')!)
          : Date.now() + Math.max(0, age - Number(response.headers.get('age') ?? 0)) * 1000
      if (!/no-store/.test(policy))
        await save({
          key,
          blob,
          expires: Number.isFinite(expires) ? expires : Date.now(),
          saved: Date.now(),
        })
      return blobResult(blob, 'online')
    } catch {
      /* Fall back to the last saved copy of this tile. */
    } finally {
      clearTimeout(timer)
      signal.removeEventListener('abort', abort)
    }
  }
  if (stored) return blobResult(stored.blob, 'cache')
  // Reuse a previously viewed lower-resolution tile when the exact zoom is absent.
  for (let levels = 1; levels <= Math.min(z, 4); levels++) {
    if (signal.aborted) break
    const factor = 2 ** levels,
      parent = await cached(`${z - levels}/${Math.floor(x / factor)}/${Math.floor(y / factor)}`)
    if (!parent) continue
    try {
      const image = await createImageBitmap(parent.blob)
      const canvas = document.createElement('canvas')
      canvas.width = canvas.height = 256
      const context = canvas.getContext('2d')
      if (!context) {
        image.close()
        break
      }
      context.drawImage(
        image,
        ((x % factor) * image.width) / factor,
        ((y % factor) * image.height) / factor,
        image.width / factor,
        image.height / factor,
        0,
        0,
        256,
        256,
      )
      image.close()
      const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
      if (blob) return blobResult(blob, 'cache')
    } catch {
      /* Try the next saved parent level. */
    }
  }
  return {
    url: 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" width="256" height="256"/%3E',
    source: 'missing',
  }
}
