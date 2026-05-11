// Whether an item exposes a SkunkCrafts/Zibo addon-update affordance that
// the AddonUpdateDrawer can handle. Items routed through `x-updater:` tagged
// URLs (browser-only manual download) are excluded.
export function isDrawerUpdatable(item: {
  updateUrl?: string
  updateProvider?: string
}): boolean {
  if (item.updateProvider === 'x-updater') return false
  if (item.updateProvider === 'zibo') return true
  const value = (item.updateUrl || '').trim().toLowerCase()
  return !!value && !value.startsWith('x-updater:')
}
