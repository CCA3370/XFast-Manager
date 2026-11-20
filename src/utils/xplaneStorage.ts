/** Keep installation-specific settings separate from global preferences. */
export function rootStorageKey(baseKey: string, xplanePath: string): string {
  return `${baseKey}:${JSON.stringify(xplanePath)}`
}
