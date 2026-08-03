import { createHash } from 'node:crypto'
import {
  isReportableError,
  normalizeErrorFingerprintInput,
} from '../../../shared/error-report-policy.js'

const AUTO_REPORTED_LABEL = 'auto-reported'
const MAX_DEDUP_PAGES = 3

export function createErrorFingerprint(report) {
  const normalized = normalizeErrorFingerprintInput({
    code: report.errorCode,
    origin: report.errorOrigin,
    operation: report.errorOperation,
    message: report.errorMessage,
  })
  return createHash('sha256').update(normalized).digest('hex')
}

function fingerprintMarker(fingerprint) {
  return `<!-- xfast-fingerprint:${fingerprint} -->`
}

async function findOpenIssueByFingerprint({ token, owner, repo, fingerprint }) {
  const marker = fingerprintMarker(fingerprint)

  for (let page = 1; page <= MAX_DEDUP_PAGES; page += 1) {
    const response = await fetch(
      `https://api.github.com/repos/${owner}/${repo}/issues?state=open&labels=${AUTO_REPORTED_LABEL}&per_page=100&page=${page}`,
      {
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: 'application/vnd.github+json',
          'User-Agent': 'XFast-Manager-Bug-Reporter',
        },
      },
    )

    if (!response.ok) {
      throw new Error(`GitHub deduplication lookup failed with status ${response.status}`)
    }

    const issues = await response.json()
    if (!Array.isArray(issues)) {
      throw new Error('GitHub deduplication lookup returned invalid data')
    }

    const match = issues.find(
      (issue) =>
        !issue.pull_request && typeof issue.body === 'string' && issue.body.includes(marker),
    )
    if (match) return match
    if (issues.length < 100) return null
  }

  return null
}

function proxyIssueUrl(req, issueNumber, fallbackUrl) {
  const forwardedHost =
    req.headers['x-forwarded-host'] || req.headers.host || 'x-fast-manager.vercel.app'
  const forwardedProto = req.headers['x-forwarded-proto'] || 'https'
  const origin = `${forwardedProto}://${forwardedHost}`
  return issueNumber > 0 ? `${origin}/api/issue-redirect?number=${issueNumber}` : fallbackUrl
}

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const {
    appVersion,
    os,
    arch,
    errorTitle,
    errorMessage,
    logs,
    category,
    errorCode,
    errorOrigin,
    errorOperation,
    reportable,
  } = req.body || {}

  if (!errorMessage) {
    return res.status(400).json({ error: 'errorMessage is required' })
  }

  if (
    !isReportableError({
      code: errorCode,
      origin: errorOrigin,
      operation: errorOperation,
      message: errorMessage,
      reportable,
    })
  ) {
    return res.status(422).json({
      code: 'bug_report_not_allowed',
      error: 'This operational error is not eligible for automatic bug reporting.',
    })
  }

  const token = process.env.XFAST_GITHUB_TOKEN
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  if (!token) {
    return res.status(500).json({ error: 'server token not configured' })
  }

  const fingerprint = createErrorFingerprint({
    errorCode,
    errorOrigin,
    errorOperation,
    errorMessage,
  })

  let existingIssue
  try {
    existingIssue = await findOpenIssueByFingerprint({ token, owner, repo, fingerprint })
  } catch {
    return res.status(503).json({
      code: 'bug_report_dedup_check_failed',
      error: 'Unable to verify whether this issue was already reported.',
    })
  }

  if (existingIssue) {
    const issueNumber = Number(existingIssue.number || 0)
    return res.status(200).json({
      issueUrl: proxyIssueUrl(req, issueNumber, existingIssue.html_url),
      issueNumber,
      deduplicated: true,
    })
  }

  const summary = String(errorTitle || errorMessage)
    .trim()
    .slice(0, 80)
  const issueTitle = `[Bug]: ${summary}`

  const issueBody = [
    fingerprintMarker(fingerprint),
    '### Bug Report (Auto-submitted)',
    '',
    '**Brief Description**',
    String(errorTitle || '(not provided)').trim(),
    '',
    '**Error Message**',
    '```',
    String(errorMessage).trim(),
    '```',
    '',
    '**Environment**',
    `- XFast Manager Version: \`${String(appVersion || 'unknown').trim()}\``,
    `- Operating System: \`${String(os || 'unknown').trim()}\``,
    `- CPU Architecture: \`${String(arch || 'unknown').trim()}\``,
    `- Category: ${String(category || 'Other').trim()}`,
    '',
    '**Logs**',
    '<details>',
    '<summary>Click to expand logs</summary>',
    '',
    '```',
    String(logs || '(no logs provided)')
      .trim()
      .slice(0, 50000),
    '```',
    '</details>',
    '',
    '---',
    '*This issue was auto-submitted from the XFast Manager error dialog.*',
  ].join('\n')

  const ghResponse = await fetch(`https://api.github.com/repos/${owner}/${repo}/issues`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'Content-Type': 'application/json',
      'User-Agent': 'XFast-Manager-Bug-Reporter',
    },
    body: JSON.stringify({
      title: issueTitle,
      body: issueBody,
      labels: ['bug', AUTO_REPORTED_LABEL],
    }),
  })

  const ghData = await ghResponse.json().catch(() => ({}))

  if (!ghResponse.ok) {
    return res.status(ghResponse.status).json({ error: ghData })
  }

  const issueNumber = Number(ghData.number || 0)
  return res.status(200).json({
    issueUrl: proxyIssueUrl(req, issueNumber, ghData.html_url),
    issueNumber,
    deduplicated: false,
  })
}
