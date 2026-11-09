import { canAutoReport, rejectOperationalReport } from '../bug-report-policy.js'

const AUTO_REPORT_BODY =
  /^### Bug Report \(Auto-submitted\)\r?\n\r?\n\*\*Error Message\*\*\r?\n```\r?\n([\s\S]+?)\r?\n```(?:\r?\n|$)/

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
  const mode = String(req.query.mode || '')
    .trim()
    .toLowerCase()

  // Older desktop clients open this fallback even when the submission API rejects a report.
  const autoReport = template === 'bug_report.yml' ? body.match(AUTO_REPORT_BODY) : null
  if (autoReport) {
    const errorMessage = autoReport[1]
    const summary = title.replace(/^\[Bug\]: /, '')
    const errorTitle = summary === errorMessage.slice(0, 80) ? '' : summary
    const logs = body.slice(autoReport[0].length)
    if (!canAutoReport({ errorTitle, errorMessage, logs })) {
      return rejectOperationalReport(res)
    }
  }

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
