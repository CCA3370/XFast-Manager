function parseBooleanFlag(value) {
  const normalized = String(value ?? '').trim().toLowerCase()
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on'
}

export function buildGitHubHeaders(token, userAgent) {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': userAgent,
  }

  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  return headers
}

export function parseIncludePreRelease(req) {
  return (
    parseBooleanFlag(req.query?.includePreRelease) ||
    parseBooleanFlag(req.headers?.['x-include-prerelease'])
  )
}

export function getProxyOrigin(req) {
  const host = req.headers['x-forwarded-host'] || req.headers.host || 'x-fast-manager.vercel.app'
  const proto = req.headers['x-forwarded-proto'] || 'https'
  return `${proto}://${host}`
}

export async function fetchAllReleases(owner, repo, headers) {
  const url = `https://api.github.com/repos/${owner}/${repo}/releases?per_page=30`
  const response = await fetch(url, { headers })
  const data = await response.json().catch(() => [])

  if (!response.ok) {
    return { ok: false, status: response.status, data }
  }

  return { ok: true, status: 200, data: Array.isArray(data) ? data : [] }
}

export async function selectRelease(owner, repo, headers, includePreRelease) {
  if (includePreRelease) {
    const all = await fetchAllReleases(owner, repo, headers)
    if (!all.ok) {
      return { ok: false, status: all.status, error: 'Failed to fetch releases', detail: all.data }
    }

    const latest = all.data.find((release) => !release?.draft)
    if (!latest) {
      return { ok: false, status: 404, error: 'No releases found' }
    }

    return { ok: true, status: 200, release: latest }
  }

  const latestStableResponse = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/releases/latest`,
    { headers },
  )

  if (latestStableResponse.ok) {
    const latestStable = await latestStableResponse.json().catch(() => ({}))
    return { ok: true, status: 200, release: latestStable }
  }

  if (latestStableResponse.status !== 404) {
    const detail = await latestStableResponse.json().catch(() => ({}))
    return {
      ok: false,
      status: latestStableResponse.status,
      error: 'Failed to fetch latest release',
      detail,
    }
  }

  const all = await fetchAllReleases(owner, repo, headers)
  if (!all.ok) {
    return { ok: false, status: all.status, error: 'Failed to fetch releases', detail: all.data }
  }

  const stable = all.data.find((release) => !release?.draft && !release?.prerelease)
  if (stable) {
    return { ok: true, status: 200, release: stable }
  }

  const fallback = all.data.find((release) => !release?.draft)
  if (!fallback) {
    return { ok: false, status: 404, error: 'No releases found' }
  }

  return { ok: true, status: 200, release: fallback }
}
