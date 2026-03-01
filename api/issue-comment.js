export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const { issueNumber, commentBody } = req.body || {}
  const number = Number(issueNumber)
  const body = String(commentBody || '').trim()

  if (!Number.isInteger(number) || number <= 0) {
    return res.status(400).json({ error: 'issueNumber must be a positive integer' })
  }
  if (!body) {
    return res.status(400).json({ error: 'commentBody is required' })
  }

  const token = process.env.XFAST_GITHUB_TOKEN
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  if (!token) {
    return res.status(500).json({ error: 'server token not configured' })
  }

  const prefixedBody = ['User Comment:', '', body].join('\n')

  const ghResponse = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/issues/${number}/comments`,
    {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/vnd.github+json',
        'Content-Type': 'application/json',
        'User-Agent': 'XFast-Manager-Issue-Comment-Proxy',
      },
      body: JSON.stringify({ body: prefixedBody }),
    },
  )

  const ghData = await ghResponse.json().catch(() => ({}))

  if (!ghResponse.ok) {
    return res.status(ghResponse.status).json({ error: ghData })
  }

  return res.status(200).json({ ok: true, commentUrl: ghData.html_url })
}
