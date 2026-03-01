function toPositiveInt(value, fallback = 0) {
  const parsed = Number(value)
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback
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
  const page = Math.max(1, toPositiveInt(req.query.page, 1))
  const perPage = Math.min(100, Math.max(1, toPositiveInt(req.query.perPage || req.query.per_page, 30)))
  if (!issueNumber) {
    return res.status(400).json({ error: 'issueNumber must be a positive integer' })
  }

  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const token = process.env.XFAST_GITHUB_TOKEN || ''
  const headers = buildGitHubHeaders(token, 'XFast-Manager-Issue-Detail-Proxy')
  const forwardedHost = req.headers['x-forwarded-host'] || req.headers.host || 'x-fast-manager.vercel.app'
  const forwardedProto = req.headers['x-forwarded-proto'] || 'https'
  const origin = `${forwardedProto}://${forwardedHost}`

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
  commentsUrl.searchParams.set('page', String(page))
  commentsUrl.searchParams.set('per_page', String(perPage))
  const commentsResponse = await fetch(commentsUrl.toString(), { headers })
  const commentsData = await commentsResponse.json().catch(() => [])
  if (!commentsResponse.ok) {
    return res.status(commentsResponse.status).json({
      error: 'Failed to fetch issue comments',
      detail: commentsData,
    })
  }

  const normalizedComments = Array.isArray(commentsData)
    ? commentsData.map((comment) => ({
        id: Number(comment?.id || 0),
        author: comment?.user?.login || 'unknown',
        body: comment?.body || '',
        created_at: comment?.created_at || '',
        updated_at: comment?.updated_at || '',
      }))
    : []

  return res.status(200).json({
    issue: {
      number: Number(issueData?.number || issueNumber),
      title: issueData?.title || '',
      state: issueData?.state || 'open',
      html_url: `${origin}/api/issue-redirect?number=${Number(issueData?.number || issueNumber)}`,
      comments: Number(issueData?.comments || 0),
      created_at: issueData?.created_at || '',
      updated_at: issueData?.updated_at || '',
    },
    comments: normalizedComments,
    page,
    per_page: perPage,
    has_more: normalizedComments.length >= perPage,
  })
}
