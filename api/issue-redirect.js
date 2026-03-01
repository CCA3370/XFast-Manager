export default function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const number = Number(req.query.number || req.query.issueNumber || 0)
  if (!Number.isInteger(number) || number <= 0) {
    return res.status(400).json({ error: 'number must be a positive integer' })
  }

  const target = `https://github.com/${owner}/${repo}/issues/${number}`
  return res.redirect(307, target)
}
