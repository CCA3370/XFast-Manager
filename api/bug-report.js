export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const { appVersion, os, arch, errorTitle, errorMessage, logs, category } = req.body || {}

  if (!errorMessage) {
    return res.status(400).json({ error: 'errorMessage is required' })
  }

  const token = process.env.XFAST_GITHUB_TOKEN
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  if (!token) {
    return res.status(500).json({ error: 'server token not configured' })
  }

  const summary = String(errorTitle || errorMessage).trim().slice(0, 80)
  const issueTitle = `[Bug]: ${summary}`

  const issueBody = [
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
    String(logs || '(no logs provided)').trim().slice(0, 50000),
    '```',
    '</details>',
    '',
    '---',
    '*This issue was auto-submitted from the XFast Manager error dialog.*'
  ].join('\n')

  const ghResponse = await fetch(`https://api.github.com/repos/${owner}/${repo}/issues`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'Content-Type': 'application/json',
      'User-Agent': 'XFast-Manager-Bug-Reporter'
    },
    body: JSON.stringify({
      title: issueTitle,
      body: issueBody,
      labels: ['bug', 'auto-reported']
    })
  })

  const ghData = await ghResponse.json().catch(() => ({}))

  if (!ghResponse.ok) {
    return res.status(ghResponse.status).json({ error: ghData })
  }

  return res.status(200).json({ issueUrl: ghData.html_url })
}
