import {
  buildGitHubHeaders,
  getProxyOrigin,
  parseIncludePreRelease,
  selectRelease,
} from './_lib/github-release.js'

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

  const includePreRelease = parseIncludePreRelease(req)
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const token = process.env.XFAST_GITHUB_TOKEN || ''
  const headers = buildGitHubHeaders(token, 'XFast-Manager-Update-Proxy')
  const selected = await selectRelease(owner, repo, headers, includePreRelease)

  if (!selected.ok) {
    return res
      .status(selected.status)
      .json({ error: selected.error, detail: selected.detail ?? null })
  }

  return res.status(200).json(normalizeRelease(selected.release, req))
}
