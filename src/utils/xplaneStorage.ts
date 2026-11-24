/** Keep installation-specific settings separate from global preferences. */
export function rootStorageKey(baseKey: string, xplanePath: string): string {
  return `${baseKey}:${JSON.stringify(xplanePath)}`
}

export function xplanePathKey(path: string): string {
  const normalized = path.trim().replaceAll('\\', '/').replace(/\/+$/, '')
  return /^[A-Za-z]:|^\/\//.test(normalized) ? normalized.toLowerCase() : normalized
}
