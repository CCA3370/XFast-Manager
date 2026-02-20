import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'

export const useThemeStore = defineStore('theme', () => {
  // Initialize with system preference, will be overwritten by stored value after init
  const isDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)

  // Initialization flag
  const isInitialized = ref(false)

  const MAX_SYNC_RETRIES = 3
  const THEME_TRANSITION_DURATION_MS = 400
  let themeTransitionTimer: number | undefined
  let windowThemeTimer: number | undefined

  // Track listeners for cleanup (fixes memory leak)
  let visibilityHandler: (() => void) | null = null
  let syncIntervalId: number | null = null

  // Initialize store by loading saved theme
  async function initStore(): Promise<void> {
    if (isInitialized.value) return

    const savedTheme = await getItem<string>(STORAGE_KEYS.THEME)
    if (savedTheme === 'dark') {
      isDark.value = true
    } else if (savedTheme === 'light') {
      isDark.value = false
    }
    // If no saved theme, keep system preference

    isInitialized.value = true

    // Setup listeners after initialization
    setupListeners()
  }

  function toggleTheme() {
    isDark.value = !isDark.value
  }

  async function syncWindowTheme(retryCount = 0): Promise<void> {
    try {
      const appWindow = getCurrentWindow()
      await appWindow.setTheme(isDark.value ? 'dark' : 'light')
    } catch (e) {
      // Retry on failure (up to MAX_SYNC_RETRIES times)
      if (retryCount < MAX_SYNC_RETRIES) {
        console.debug(`Failed to sync window theme (attempt ${retryCount + 1}/${MAX_SYNC_RETRIES}), retrying...`, e)
        setTimeout(() => {
          syncWindowTheme(retryCount + 1)
        }, 100 * (retryCount + 1)) // Exponential backoff: 100ms, 200ms, 300ms
      } else {
        console.debug('Failed to set window theme after retries:', e)
      }
    }
  }

  async function applyTheme() {
    const transitionDurationMs = THEME_TRANSITION_DURATION_MS

    await syncWindowTheme()
    await applyThemeInstantly(transitionDurationMs)

    // Sync Tauri window theme with app theme (with retry logic)
    if (transitionDurationMs > 0) {
      syncWindowThemeWithDelay(transitionDurationMs)
    }
  }

  function scheduleThemeTransitionCleanup(transitionDurationMs: number) {
    if (themeTransitionTimer !== undefined) {
      window.clearTimeout(themeTransitionTimer)
    }
    themeTransitionTimer = window.setTimeout(() => {
      document.documentElement.classList.remove('theme-transitioning')
      themeTransitionTimer = undefined
    }, transitionDurationMs)
  }

  function syncWindowThemeWithDelay(delayMs: number) {
    if (windowThemeTimer !== undefined) {
      window.clearTimeout(windowThemeTimer)
      windowThemeTimer = undefined
    }
    if (delayMs <= 0) {
      void syncWindowTheme()
      return
    }
    windowThemeTimer = window.setTimeout(() => {
      void syncWindowTheme()
      windowThemeTimer = undefined
    }, delayMs)
  }

  async function applyThemeInstantly(transitionDurationMs: number) {
    document.documentElement.classList.add('theme-transitioning')
    document.documentElement.style.setProperty('--theme-transition-duration', `${transitionDurationMs}ms`)

    if (isDark.value) {
      document.documentElement.classList.add('dark')
      await setItem(STORAGE_KEYS.THEME, 'dark')
    } else {
      document.documentElement.classList.remove('dark')
      await setItem(STORAGE_KEYS.THEME, 'light')
    }

    scheduleThemeTransitionCleanup(transitionDurationMs)
  }

  // Force sync window theme (can be called manually if needed)
  async function forceSync() {
    await syncWindowTheme()
  }

  // Setup listeners (called after initialization)
  function setupListeners() {
    // Watch for changes and apply
    watch(isDark, applyTheme, { immediate: true })

    // Re-sync when window becomes visible (handles edge cases)
    if (typeof document !== 'undefined') {
      // Store reference to handler for cleanup
      visibilityHandler = () => {
        if (!document.hidden) {
          syncWindowTheme()
        }
      }
      document.addEventListener('visibilitychange', visibilityHandler)

      // Periodic sync every 5 seconds to ensure titlebar stays in sync
      // Store reference for cleanup
      syncIntervalId = window.setInterval(() => {
        syncWindowTheme()
      }, 5000)
    }
  }

  // Cleanup function to prevent memory leaks
  function cleanup() {
    if (visibilityHandler) {
      document.removeEventListener('visibilitychange', visibilityHandler)
      visibilityHandler = null
    }
    if (syncIntervalId !== null) {
      clearInterval(syncIntervalId)
      syncIntervalId = null
    }
    if (themeTransitionTimer !== undefined) {
      window.clearTimeout(themeTransitionTimer)
      themeTransitionTimer = undefined
    }
    if (windowThemeTimer !== undefined) {
      window.clearTimeout(windowThemeTimer)
      windowThemeTimer = undefined
    }
  }

  return {
    isDark,
    isInitialized,
    initStore,
    toggleTheme,
    forceSync,
    cleanup
  }
})
