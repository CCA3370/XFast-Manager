export default function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const tag = String(req.query.tag || '').trim()

  const target = tag
    ? `https://github.com/${owner}/${repo}/releases/tag/${encodeURIComponent(tag)}`
    : `https://github.com/${owner}/${repo}/releases`

  return res.redirect(307, target)
}
