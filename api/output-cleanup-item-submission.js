const LEVEL_LABELS = {
  recommended: 'Recommended cleanup',
  cleanable: 'Cleanable',
  cautious: 'Cautious cleanup',
}

function normalizeLevel(input) {
  const value = String(input || '').trim().toLowerCase()
  return LEVEL_LABELS[value] ? value : 'cleanable'
}

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const {
    folderName,
    relativePath,
    description,
    expectedLevel,
    sizeBytes,
    fileCount,
    appVersion,
    os,
    arch,
  } = req.body || {}

  const normalizedFolderName = String(folderName || '').trim()
  const normalizedRelativePath = String(relativePath || '').trim()
  const normalizedDescription = String(description || '').trim()
  const normalizedLevel = normalizeLevel(expectedLevel)

  if (!normalizedFolderName) {
    return res.status(400).json({ error: 'folderName is required' })
  }
  if (!normalizedRelativePath) {
    return res.status(400).json({ error: 'relativePath is required' })
  }
  if (!normalizedDescription) {
    return res.status(400).json({ error: 'description is required' })
  }

  const token = process.env.XFAST_GITHUB_TOKEN
  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'

  if (!token) {
    return res.status(500).json({ error: 'server token not configured' })
  }

  const issueTitle = `[Output Cleanup] ${normalizedFolderName.slice(0, 100)}`
  const issueBody = [
    '### Output Cleanup Item Submission',
    '',
    `- Folder Name: \`${normalizedFolderName}\``,
    `- Relative Path: \`${normalizedRelativePath}\``,
    `- Expected Level: \`${normalizedLevel}\` (${LEVEL_LABELS[normalizedLevel]})`,
    `- Current Size: \`${Number(sizeBytes || 0)} bytes\``,
    `- Current File Count: \`${Number(fileCount || 0)}\``,
    '',
    '### User Description',
    '',
    normalizedDescription.slice(0, 10000),
    '',
    '### Environment',
    '',
    `- XFast Manager Version: \`${String(appVersion || 'unknown').trim()}\``,
    `- Operating System: \`${String(os || 'unknown').trim()}\``,
    `- CPU Architecture: \`${String(arch || 'unknown').trim()}\``,
    '',
    'Please review this folder. If accepted, update `data/output_cleanup_items.json` on `dev`.',
  ].join('\n')

  const ghResponse = await fetch(`https://api.github.com/repos/${owner}/${repo}/issues`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'Content-Type': 'application/json',
      'User-Agent': 'XFast-Manager-Output-Cleanup-Proxy',
    },
    body: JSON.stringify({
      title: issueTitle,
      body: issueBody,
      labels: ['output-cleanup-item'],
    }),
  })

  const ghData = await ghResponse.json().catch(() => ({}))

  if (!ghResponse.ok) {
    return res.status(ghResponse.status).json({ error: ghData })
  }

  const forwardedHost = req.headers['x-forwarded-host'] || req.headers.host || 'x-fast-manager.vercel.app'
  const forwardedProto = req.headers['x-forwarded-proto'] || 'https'
  const origin = `${forwardedProto}://${forwardedHost}`
  const issueNumber = Number(ghData.number || 0)
  const proxyIssueUrl =
    issueNumber > 0 ? `${origin}/api/issue-redirect?number=${issueNumber}` : ''

  return res.status(200).json({
    issueUrl: proxyIssueUrl || ghData.html_url,
    issueNumber,
    issueTitle,
  })
}
