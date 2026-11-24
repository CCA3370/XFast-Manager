import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { setTimeout as delay } from 'node:timers/promises'

const [mode = 'active', rawPort = '1432'] = process.argv.slice(2)
const port = Number(rawPort)
if (
  !['active', 'empty', 'slow', 'missing-image'].includes(mode) ||
  !Number.isInteger(port) ||
  port < 1 ||
  port > 65535
) {
  throw new Error(
    'Usage: node docs/testing/startup-ads-fixture.mjs [active|empty|slow|missing-image] [port]',
  )
}

const revision = 'a'.repeat(40)
const assets = new Map([
  ['poster.png', new URL('../../screenshots/Poster.png', import.meta.url)],
  ['installer.png', new URL('../../screenshots/feature-1.png', import.meta.url)],
  ['icon.png', new URL('../../public/icon.png', import.meta.url)],
])
const primary = {
  id: 'smoke-primary',
  firstParty: true,
  creative: { image: 'poster.png', alt: 'XFast Manager test poster' },
  localizations: { zh: { image: 'installer.png', alt: 'XFast Manager 安装界面测试图' } },
  url: 'https://example.com/',
}
const secondary = {
  id: 'smoke-secondary',
  creative: { image: 'icon.png', alt: 'XFast Manager test icon' },
  url: 'https://example.com/',
}
const missingImage = {
  id: 'smoke-missing-image',
  creative: { image: 'missing.png', alt: 'Missing test image' },
  url: 'https://example.com/',
}
const campaigns =
  mode === 'empty' ? [] : [mode === 'missing-image' ? missingImage : primary, secondary]

const server = createServer(async (req, res) => {
  const request = new URL(req.url, 'http://localhost')
  res.setHeader('Access-Control-Allow-Origin', '*')
  res.setHeader('Cache-Control', 'no-store')

  if (req.method !== 'GET') {
    res.writeHead(405, { Allow: 'GET' }).end()
    return
  }
  if (request.pathname === '/api/ad-campaigns') {
    if (mode === 'slow') await delay(6500)
    if (res.destroyed) return
    res.writeHead(200, { 'Content-Type': 'application/json' })
    res.end(JSON.stringify({ schemaVersion: 1, revision, campaigns }))
    return
  }
  if (request.pathname === '/api/ad-image' && request.searchParams.get('revision') === revision) {
    const asset = assets.get(request.searchParams.get('name'))
    if (asset) {
      try {
        const bytes = await readFile(asset)
        res.writeHead(200, { 'Content-Type': 'image/png', 'Content-Length': bytes.length })
        res.end(bytes)
      } catch {
        res.writeHead(500).end()
      }
      return
    }
  }
  res.writeHead(404).end()
})

server.listen(port, 'localhost', () => {
  console.log(`Startup ad fixture (${mode}) listening on http://localhost:${port}/api`)
})
