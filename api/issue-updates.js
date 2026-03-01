function toPositiveInt(value) {
  const parsed = Number(value)
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 0
}

function buildGitHubHeaders(token, userAgent) {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': userAgent,
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  return headers
}

export default async function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const issueNumber = toPositiveInt(req.query.issueNumber || req.query.issue_number)
  const since = String(req.query.since || '').trim()
  if (!issueNumber) {
    return res.status(400).json({ error: 'issueNumber must be a positive integer' })
  }

  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const token = process.env.XFAST_GITHUB_TOKEN || ''
  const headers = buildGitHubHeaders(token, 'XFast-Manager-Issue-Updates-Proxy')

  const issueResponse = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/issues/${issueNumber}`,
    { headers },
  )
  const issueData = await issueResponse.json().catch(() => ({}))
  if (!issueResponse.ok) {
    return res.status(issueResponse.status).json({
      error: 'Failed to fetch issue',
      detail: issueData,
    })
  }

  const commentsUrl = new URL(
    `https://api.github.com/repos/${owner}/${repo}/issues/${issueNumber}/comments`,
  )
  commentsUrl.searchParams.set('per_page', '20')
  if (since) {
    commentsUrl.searchParams.set('since', since)
  }

  const commentsResponse = await fetch(commentsUrl.toString(), { headers })
  const commentsData = commentsResponse.ok
    ? await commentsResponse.json().catch(() => [])
    : []

  const newComments = Array.isArray(commentsData)
    ? commentsData.map((comment) => ({
        author: comment?.user?.login || 'unknown',
        body: comment?.body || '',
        created_at: comment?.created_at || '',
      }))
    : []

  return res.status(200).json({
    state: issueData?.state || 'open',
    total_comments: Number(issueData?.comments || 0),
    new_comments: newComments,
  })
}
