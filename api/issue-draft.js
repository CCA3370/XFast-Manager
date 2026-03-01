function buildIssueNewUrl({ owner, repo, template, labels, title, body }) {
  const issueUrl = new URL(`https://github.com/${owner}/${repo}/issues/new`)
  if (template) {
    issueUrl.searchParams.set('template', template)
  }
  if (labels) {
    issueUrl.searchParams.set('labels', labels)
  }
  if (title) {
    issueUrl.searchParams.set('title', title)
  }
  if (body) {
    issueUrl.searchParams.set('body', body)
  }
  return issueUrl.toString()
}

export default function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  const template = String(req.query.template || '').trim()
  const labels = String(req.query.labels || '').trim()
  const title = String(req.query.title || '').trim()
  const body = String(req.query.body || '').trim()
  const mode = String(req.query.mode || '').trim().toLowerCase()

  const url = buildIssueNewUrl({
    owner,
    repo,
    template,
    labels,
    title,
    body,
  })

  if (mode === 'json') {
    return res.status(200).json({ url })
  }

  return res.redirect(307, url)
}
