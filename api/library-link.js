export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const { libraryName, downloadUrl, referencedBy } = req.body || {}

  if (!libraryName || !downloadUrl) {
    return res.status(400).json({ error: 'libraryName/downloadUrl required' })
  }

  let parsed
  try {
    parsed = new URL(downloadUrl)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      throw new Error('invalid protocol')
    }
  } catch {
    return res.status(400).json({ error: 'invalid downloadUrl' })
  }

  const token = process.env.XFAST_GITHUB_TOKEN
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  if (!token) {
    return res.status(500).json({ error: 'server token not configured' })
  }

  const issueTitle = `[Library Link] ${String(libraryName).trim()}`
  const issueBody = [
    '### Library Link Submission',
    '',
    `- Library Name: \`${String(libraryName).trim()}\``,
    `- Download URL: ${String(downloadUrl).trim()}`,
    `- Referenced By Scenery: \`${(referencedBy || '(not provided)').toString().trim()}\``,
    '',
    'Please review this link. If valid, add the `approved-link` label to trigger auto-update for `data/library_links.json` on `dev`.'
  ].join('\n')

  const ghResponse = await fetch(`https://api.github.com/repos/${owner}/${repo}/issues`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'Content-Type': 'application/json',
      'User-Agent': 'XFast-Manager-Link-Proxy'
    },
    body: JSON.stringify({
      title: issueTitle,
      body: issueBody,
      labels: ['library-link']
    })
  })

  const ghData = await ghResponse.json().catch(() => ({}))

  if (!ghResponse.ok) {
    return res.status(ghResponse.status).json({ error: ghData })
  }

  const forwardedHost = req.headers['x-forwarded-host'] || req.headers.host || 'x-fast-manager.vercel.app'
  const forwardedProto = req.headers['x-forwarded-proto'] || 'https'
  const origin = `${forwardedProto}://${forwardedHost}`
  const issueNumber = Number(ghData.number || 0)
  const proxyIssueUrl = issueNumber > 0
    ? `${origin}/api/issue-redirect?number=${issueNumber}`
    : ''

  return res.status(200).json({
    issueUrl: proxyIssueUrl || ghData.html_url,
    issueNumber,
  })
}
