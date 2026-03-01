function buildGitHubHeaders(token) {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'XFast-Manager-Livery-Patterns-Proxy',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  return headers
}

async function fetchJsonFileFromRepo(path, owner, repo, token, ref = 'dev') {
  const headers = buildGitHubHeaders(token)
  const metaUrl = `https://api.github.com/repos/${owner}/${repo}/contents/${path}?ref=${encodeURIComponent(ref)}`
  const metaResponse = await fetch(metaUrl, { headers })
  const metaData = await metaResponse.json().catch(() => ({}))
  if (!metaResponse.ok) {
    throw Object.assign(new Error('Failed to fetch file metadata'), {
      status: metaResponse.status,
      detail: metaData,
    })
  }

  if (metaData?.content) {
    const content = Buffer.from(String(metaData.content).replace(/\n/g, ''), 'base64').toString('utf8')
    return JSON.parse(content)
  }

  if (metaData?.download_url) {
    const fileResponse = await fetch(metaData.download_url, { headers })
    const text = await fileResponse.text()
    if (!fileResponse.ok) {
      throw Object.assign(new Error('Failed to download file'), {
        status: fileResponse.status,
        detail: text,
      })
    }
    return JSON.parse(text)
  }

  throw Object.assign(new Error('Unsupported metadata response'), {
    status: 500,
    detail: metaData,
  })
}

export default async function handler(req, res) {
  if (req.method !== 'GET') {
    return res.status(405).json({ error: 'Method not allowed' })
  }

  const owner = process.env.GITHUB_OWNER || 'CCA3370'
  const repo = process.env.GITHUB_REPO || 'XFast-Manager'
  const token = process.env.XFAST_GITHUB_TOKEN || ''
  const ref = String(req.query.ref || process.env.GITHUB_REF || 'dev').trim() || 'dev'

  try {
    const payload = await fetchJsonFileFromRepo('data/livery_patterns.json', owner, repo, token, ref)
    res.setHeader('Cache-Control', 's-maxage=600, stale-while-revalidate=3600')
    return res.status(200).json(payload)
  } catch (error) {
    return res.status(error?.status || 500).json({
      error: error?.message || 'Failed to fetch livery patterns data',
      detail: error?.detail || null,
    })
  }
}
