function buildGitHubHeaders(token) {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'XFast-Manager-Update-Proxy',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  return headers
}

function parseIncludePreRelease(value) {
  const normalized = String(value ?? '').trim().toLowerCase()
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on'
}

function getProxyOrigin(req) {
  const host = req.headers['x-forwarded-host'] || req.headers.host || 'x-fast-manager.vercel.app'
  const proto = req.headers['x-forwarded-proto'] || 'https'
  return `${proto}://${host}`
}

function normalizeRelease(release, req) {
  const tag = String(release?.tag_name || '').trim()
  const origin = getProxyOrigin(req)
  const proxyReleaseUrl = tag
    ? `${origin}/api/release-redirect?tag=${encodeURIComponent(tag)}`
    : `${origin}/api/release-redirect`

  return {
    tag_name: tag,
    name: String(release?.name || tag || ''),
    body: release?.body ?? '',
    prerelease: Boolean(release?.prerelease),
    published_at: String(release?.published_at || ''),
    html_url: proxyReleaseUrl,
  }
}

export default async function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const includePreRelease = parseIncludePreRelease(req.query.includePreRelease)
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const token = process.env.XFAST_GITHUB_TOKEN || ''
  const headers = buildGitHubHeaders(token)

  async function fetchAllReleases() {
    const url = `https://api.github.com/repos/${owner}/${repo}/releases?per_page=30`
    const response = await fetch(url, { headers })
    const data = await response.json().catch(() => [])
    if (!response.ok) {
      return { ok: false, status: response.status, data }
    }
    return { ok: true, status: 200, data: Array.isArray(data) ? data : [] }
  }

  if (includePreRelease) {
    const all = await fetchAllReleases()
    if (!all.ok) {
      return res.status(all.status).json({ error: 'Failed to fetch releases', detail: all.data })
    }
    const latest = all.data[0]
    if (!latest) {
      return res.status(404).json({ error: 'No releases found' })
    }
    return res.status(200).json(normalizeRelease(latest, req))
  }

  const latestStableResponse = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/releases/latest`,
    { headers },
  )

  if (latestStableResponse.ok) {
    const latestStable = await latestStableResponse.json().catch(() => ({}))
    return res.status(200).json(normalizeRelease(latestStable, req))
  }

  if (latestStableResponse.status !== 404) {
    const data = await latestStableResponse.json().catch(() => ({}))
    return res
      .status(latestStableResponse.status)
      .json({ error: 'Failed to fetch latest release', detail: data })
  }

  const all = await fetchAllReleases()
  if (!all.ok) {
    return res.status(all.status).json({ error: 'Failed to fetch releases', detail: all.data })
  }

  const stable = all.data.find((release) => !release?.prerelease) || all.data[0]
  if (!stable) {
    return res.status(404).json({ error: 'No releases found' })
  }

  return res.status(200).json(normalizeRelease(stable, req))
}
