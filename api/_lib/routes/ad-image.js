import {
  UPSTREAM_TIMEOUT_MS,
  fetchAdCampaignFile,
  isAdImageFilename,
  isAdRevision,
  readBoundedBody,
} from '../ad-campaign-source.js'

const MAX_IMAGE_BYTES = 2 * 1024 * 1024
const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
const jpegSignature = Buffer.from([255, 216, 255])
const riffSignature = Buffer.from('RIFF')
const webpSignature = Buffer.from('WEBP')

function getImageParams(req) {
  if (!req.url) return req.query ?? {}

  const { searchParams } = new URL(req.url, 'https://x-fast-manager.vercel.app')
  const revision = searchParams.getAll('revision')
  const name = searchParams.getAll('name')
  return revision.length === 1 && name.length === 1 ? { revision: revision[0], name: name[0] } : {}
}

function isMatchingImage(bytes, type) {
  if (type === 'image/png') return bytes.subarray(0, pngSignature.length).equals(pngSignature)
  if (type === 'image/jpeg') return bytes.subarray(0, jpegSignature.length).equals(jpegSignature)
  return bytes.subarray(0, 4).equals(riffSignature) && bytes.subarray(8, 12).equals(webpSignature)
}

function getImageMimeType(filename) {
  const extension = filename.split('.').at(-1).toLowerCase()
  return extension === 'jpg' || extension === 'jpeg' ? 'image/jpeg' : `image/${extension}`
}

export default async function handler(req, res) {
  res.setHeader('Access-Control-Allow-Origin', '*')
  res.setHeader('Cache-Control', 'no-store')
  if (req.method !== 'GET') {
    res.setHeader('Allow', 'GET')
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const { revision, name } = getImageParams(req)
  if (!isAdRevision(revision) || !isAdImageFilename(name)) {
    return res.status(400).json({ error: 'Invalid ad image parameters' })
  }

  try {
    const signal = AbortSignal.timeout(UPSTREAM_TIMEOUT_MS)
    const response = await fetchAdCampaignFile(revision, `images/${name}`, signal)
    if (response.status === 404) {
      await response.body?.cancel()
      return res.status(404).json({ error: 'Ad image not found' })
    }

    const mimeType = getImageMimeType(name)
    const contentType = response.headers.get('content-type')?.split(';')[0].trim().toLowerCase()
    if (contentType !== mimeType) {
      await response.body?.cancel()
      throw new Error('Unexpected ad image content type')
    }

    const bytes = await readBoundedBody(response, MAX_IMAGE_BYTES)
    if (!isMatchingImage(bytes, mimeType)) throw new Error('Invalid ad image signature')

    res.setHeader('Content-Type', mimeType)
    res.setHeader('X-Content-Type-Options', 'nosniff')
    res.setHeader('Cache-Control', 'public, max-age=31536000, immutable')
    res.setHeader('Vercel-CDN-Cache-Control', 'public, s-maxage=31536000')
    return res.status(200).end(bytes)
  } catch {
    return res.status(502).json({ error: 'Ad image unavailable' })
  }
}
