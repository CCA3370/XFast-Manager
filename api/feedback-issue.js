const TYPE_LABEL_MAP = {
  bug: ['bug', 'feedback'],
  'feature-request': ['enhancement', 'feedback'],
  improvement: ['improvement', 'feedback'],
  other: ['feedback'],
}

const TYPE_TITLE_MAP = {
  bug: 'Bug',
  'feature-request': 'Feature Request',
  improvement: 'Improvement',
  other: 'Other',
}

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const { title, type, content, appVersion, os, arch } = req.body || {}
  const feedbackTitle = String(title || '').trim()
  const feedbackType = String(type || 'other').trim().toLowerCase()
  const feedbackContent = String(content || '').trim()

  if (!feedbackTitle) {
    return res.status(400).json({ error: 'title is required' })
  }
  if (!feedbackContent) {
    return res.status(400).json({ error: 'content is required' })
  }

  const token = process.env.XFAST_GITHUB_TOKEN
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  if (!token) {
    return res.status(500).json({ error: 'server token not configured' })
  }

  const normalizedType = TYPE_LABEL_MAP[feedbackType] ? feedbackType : 'other'
  const typeDisplay = TYPE_TITLE_MAP[normalizedType]
  const issueTitle = `[Feedback][${typeDisplay}] ${feedbackTitle.slice(0, 100)}`
  const labels = TYPE_LABEL_MAP[normalizedType]

  const issueBody = [
    '### User Feedback',
    '',
    '**Title**',
    feedbackTitle,
    '',
    '**Type**',
    typeDisplay,
    '',
    '**Content**',
    feedbackContent.slice(0, 50000),
    '',
    '**Environment**',
    `- XFast Manager Version: \`${String(appVersion || 'unknown').trim()}\``,
    `- Operating System: \`${String(os || 'unknown').trim()}\``,
    `- CPU Architecture: \`${String(arch || 'unknown').trim()}\``,
    '',
    '---',
    '*Submitted from XFast Manager Feedback Dialog.*',
  ].join('\n')

  const ghResponse = await fetch(`https://api.github.com/repos/${owner}/${repo}/issues`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'Content-Type': 'application/json',
      'User-Agent': 'XFast-Manager-Feedback-Proxy',
    },
    body: JSON.stringify({
      title: issueTitle,
      body: issueBody,
      labels,
    }),
  })

  const ghData = await ghResponse.json().catch(() => ({}))

  if (!ghResponse.ok) {
    return res.status(ghResponse.status).json({ error: ghData })
  }

  return res.status(200).json({
    issueUrl: ghData.html_url,
    issueNumber: ghData.number,
    issueTitle,
  })
}
