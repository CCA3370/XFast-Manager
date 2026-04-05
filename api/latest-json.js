import {
  buildGitHubHeaders,
  parseIncludePreRelease,
  selectRelease,
} from './_lib/github-release.js'

export default async function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const includePreRelease = parseIncludePreRelease(req)
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const token = process.env.XFAST_GITHUB_TOKEN || ''
  const headers = buildGitHubHeaders(token, 'XFast-Manager-Updater-Latest-Proxy')

  const selected = await selectRelease(owner, repo, headers, includePreRelease)
  if (!selected.ok) {
    return res
      .status(selected.status)
      .json({ error: selected.error, detail: selected.detail ?? null })
  }

  const release = selected.release
  const asset = Array.isArray(release?.assets)
    ? release.assets.find((entry) => entry?.name === 'latest.json')
    : null

  if (!asset?.browser_download_url) {
    return res.status(404).json({
      error: 'Updater manifest asset not found',
      detail: { tag: release?.tag_name ?? '', asset: 'latest.json' },
    })
  }

  const manifestResponse = await fetch(asset.browser_download_url, { headers })
  const manifestText = await manifestResponse.text()

  if (!manifestResponse.ok) {
    return res.status(manifestResponse.status).json({
      error: 'Failed to fetch updater manifest',
      detail: manifestText.slice(0, 1000),
    })
  }

  let manifest
  try {
    manifest = JSON.parse(manifestText)
  } catch {
    return res.status(502).json({
      error: 'Updater manifest is not valid JSON',
      detail: manifestText.slice(0, 1000),
    })
  }

  res.setHeader('Cache-Control', 'public, max-age=300, s-maxage=300')
  return res.status(200).json(manifest)
}
