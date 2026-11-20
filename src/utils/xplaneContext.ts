import { watch } from 'vue'
import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'

/** An epoch also rejects A → B → A completions, not just mismatched path strings. */
export function createXplaneScope(onChange: () => void = () => {}) {
  const app = useAppStore()
  watch(() => app.xplanePath, onChange, { flush: 'sync' })
  return () => {
    const { path, isCurrent } = app.captureXplaneContext()
    const assertCurrent = () => {
      if (!isCurrent()) throw new Error('X-Plane directory changed; retry the operation')
    }
    const invoke = async <T = unknown>(
      command: string,
      args?: Record<string, unknown>,
    ): Promise<T> => {
      assertCurrent()
      if (!path) throw new Error('Select an X-Plane directory first')
      const result = await tauriInvoke<T>(command, { ...args, xplanePath: path })
      assertCurrent()
      return result
    }
    return { path, isCurrent, assertCurrent, invoke }
  }
}
