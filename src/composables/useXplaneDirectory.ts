import { computed, onScopeDispose, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { logError } from '@/services/logger'
import { useAppStore } from '@/stores/app'

export function useXplaneDirectory(completeOnboarding: boolean) {
  const store = useAppStore()
  const draft = ref(store.xplanePath)
  const errorKey = ref('')
  const status = ref<'idle' | 'validating' | 'saving' | 'saved'>('idle')
  let attempt = 0

  onScopeDispose(() => {
    attempt += 1
  })

  const pathInput = computed({
    get: () => draft.value,
    set: (value: string) => {
      if (status.value === 'saving') return
      attempt += 1
      draft.value = value
      errorKey.value = ''
      status.value = 'idle'
    },
  })

  async function submit(clear = false) {
    if (status.value === 'saving' || status.value === 'validating') return false
    const path = clear ? '' : draft.value.trim()
    const id = ++attempt
    errorKey.value = ''
    status.value = 'validating'
    try {
      if (!path && !clear) {
        errorKey.value = 'settings.pathError'
        return false
      }
      if (path) {
        const valid = await invoke<boolean>('validate_xplane_path', { path })
        if (id !== attempt) return false
        if (!valid) {
          const exists = await invoke<boolean>('check_path_exists', { path })
          if (id !== attempt) return false
          errorKey.value = exists ? 'settings.notValidXplanePath' : 'settings.pathNotExist'
          return false
        }
      }
      status.value = 'saving'
      await store.setXplanePath(path, completeOnboarding)
      if (id !== attempt) return false
      draft.value = path
      status.value = 'saved'
      return true
    } catch (error) {
      if (id === attempt) {
        errorKey.value =
          status.value === 'saving' ? 'settingsSetup.saveFailed' : 'settingsSetup.validationFailed'
        logError(`X-Plane directory could not be applied: ${error}`, 'settings')
      }
      return false
    } finally {
      if (id === attempt && status.value !== 'saved') status.value = 'idle'
    }
  }

  return { pathInput, errorKey, status, submit }
}
