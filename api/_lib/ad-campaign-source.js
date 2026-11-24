const repository = 'CCA3370/XFast-Manager'
const refUrl = `https://api.github.com/repos/${repository}/git/ref/heads/ad-campaigns`
const rawBaseUrl = `https://raw.githubusercontent.com/${repository}`
const revisionPattern = /^[a-f0-9]{40}$/
const filenamePattern = /^[a-z0-9][a-z0-9_-]{0,95}\.(?:png|jpe?g|webp)$/i

export const UPSTREAM_TIMEOUT_MS = 4000

export function isAdRevision(value) {
  return typeof value === 'string' && revisionPattern.test(value)
}

export function isAdImageFilename(value) {
  return typeof value === 'string' && filenamePattern.test(value)
}

export async function fetchAdCampaignRef(signal) {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'XFast-Manager-Ad-Campaigns',
  }
  if (process.env.XFAST_GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${process.env.XFAST_GITHUB_TOKEN}`
  }

  const response = await fetch(refUrl, { headers, redirect: 'error', signal })
  if (response.status === 404) {
    await response.body?.cancel()
    return null
  }

  const { object } = await readJson(response, 16 * 1024)
  if (object?.type !== 'commit' || !isAdRevision(object.sha)) {
    throw new Error('Invalid ad campaign revision')
  }
  return object.sha
}

export function fetchAdCampaignFile(revision, path, signal) {
  return fetch(`${rawBaseUrl}/${revision}/ad-campaigns/${path}`, { redirect: 'error', signal })
}

export async function readJson(response, maxBytes) {
  const bytes = await readBoundedBody(response, maxBytes)
  return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes))
}

export async function readBoundedBody(response, maxBytes) {
  if (!response.ok || !response.body || Number(response.headers.get('content-length')) > maxBytes) {
    await response.body?.cancel()
    throw new Error('Invalid ad campaign response')
  }

  const reader = response.body.getReader()
  const chunks = []
  let size = 0
  try {
    for (;;) {
      const { value, done } = await reader.read()
      if (done) return Buffer.concat(chunks, size)

      size += value.byteLength
      if (size > maxBytes) {
        await reader.cancel()
        throw new Error('Ad campaign response exceeds the size limit')
      }
      chunks.push(value)
    }
  } finally {
    reader.releaseLock()
  }
}
