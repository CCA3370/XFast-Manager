import { onBeforeUnmount } from 'vue'
import { createXplaneScope } from '@/utils/xplaneContext'

// Root-specific views are remounted by the app when the installation changes.
// Bind their requests to the installation that supplied their displayed data.
export function useXplaneViewRoot() {
  const root = createXplaneScope()()
  let disposed = false
  onBeforeUnmount(() => {
    disposed = true
  })
  return {
    xplanePath: root.path,
    isCurrentRoot: () => !disposed && !!root.path && root.isCurrent(),
  }
}
