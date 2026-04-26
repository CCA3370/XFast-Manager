const DEFAULT_VERCEL_API_BASE_URL = 'https://x-fast-manager.vercel.app/api'

function normalizeApiBaseUrl(rawValue?: string): string {
  const trimmed = rawValue?.trim()
  const base = trimmed && trimmed.length > 0 ? trimmed : DEFAULT_VERCEL_API_BASE_URL
  const normalized = base.replace(/\/+$/, '')

  return normalized.endsWith('/api') ? normalized : `${normalized}/api`
}

function normalizeRoute(route: string): string {
  return route.trim().replace(/^\/+/, '')
}

export function buildVercelApiUrl(route: string, overrideUrl?: string): string {
  const override = overrideUrl?.trim()
  if (override) {
    return override
  }

  const baseUrl = normalizeApiBaseUrl(import.meta.env.VITE_XFAST_VERCEL_API_BASE_URL)
  return `${baseUrl}/${normalizeRoute(route)}`
}
