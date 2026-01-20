import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

export const useThemeStore = defineStore('theme', () => {
  // Initialize from localStorage or system preference
  const isDark = ref(localStorage.getItem('theme') === 'dark' ||
    (!('theme' in localStorage) && window.matchMedia('(prefers-color-scheme: dark)').matches))

  let syncRetryCount = 0
  const MAX_SYNC_RETRIES = 3

  function toggleTheme() {
    isDark.value = !isDark.value
  }

  async function syncWindowTheme(retryCount = 0): Promise<void> {
    try {
      const appWindow = getCurrentWindow()
      await appWindow.setTheme(isDark.value ? 'dark' : 'light')
      syncRetryCount = 0 // Reset on success
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
    // Check if there are many expanded scenery entries (performance optimization)
    const expandedEntries = document.querySelectorAll('[data-scenery-index]')
    const isLargeList = expandedEntries.length > 100

    if (isLargeList) {
      // For large lists, use progressive theme switching
      await applyThemeProgressively(expandedEntries)
    } else {
      // For small lists, use instant theme switching
      await applyThemeInstantly()
    }

    // Sync Tauri window theme with app theme (with retry logic)
    await syncWindowTheme()
  }

  async function applyThemeInstantly() {
    // Disable transitions during theme change to prevent lag
    document.documentElement.classList.add('theme-transitioning')
    document.documentElement.style.setProperty('--theme-transition-duration', '50ms')

    if (isDark.value) {
      document.documentElement.classList.add('dark')
      localStorage.setItem('theme', 'dark')
    } else {
      document.documentElement.classList.remove('dark')
      localStorage.setItem('theme', 'light')
    }

    setTimeout(() => {
      document.documentElement.classList.remove('theme-transitioning')
    }, 50)
  }

  async function applyThemeProgressively(entries: NodeListOf<Element>) {
    // Apply theme to root first (for navbar, background, etc.)
    if (isDark.value) {
      document.documentElement.classList.add('dark')
      localStorage.setItem('theme', 'dark')
    } else {
      document.documentElement.classList.remove('dark')
      localStorage.setItem('theme', 'light')
    }

    // Disable transitions globally
    document.documentElement.classList.add('theme-transitioning')

    // Get viewport bounds
    const viewportHeight = window.innerHeight
    const scrollTop = document.documentElement.scrollTop || document.body.scrollTop

    // Categorize entries: visible, above viewport, below viewport
    const visibleEntries: Element[] = []
    const aboveEntries: Element[] = []
    const belowEntries: Element[] = []

    entries.forEach(entry => {
      const rect = entry.getBoundingClientRect()
      const absoluteTop = rect.top + scrollTop

      if (rect.top < viewportHeight && rect.bottom > 0) {
        // Visible in viewport
        visibleEntries.push(entry)
      } else if (absoluteTop < scrollTop) {
        // Above viewport
        aboveEntries.push(entry)
      } else {
        // Below viewport
        belowEntries.push(entry)
      }
    })

    // Process in batches: visible first, then others
    const batchSize = 20
    const allBatches = [
      ...chunkArray(visibleEntries, batchSize),
      ...chunkArray(aboveEntries, batchSize),
      ...chunkArray(belowEntries, batchSize)
    ]

    // Process batches with small delays
    for (let i = 0; i < allBatches.length; i++) {
      const batch = allBatches[i]
      batch.forEach(entry => {
        // Force a reflow for this entry to apply theme
        entry.classList.add('theme-batch-update')
        // Trigger reflow
        void entry.clientHeight
        entry.classList.remove('theme-batch-update')
      })

      // Small delay between batches (except for the last one)
      if (i < allBatches.length - 1) {
        await new Promise(resolve => setTimeout(resolve, 16)) // ~1 frame
      }
    }

    // Re-enable transitions after all batches are done
    setTimeout(() => {
      document.documentElement.classList.remove('theme-transitioning')
    }, 50)
  }

  function chunkArray<T>(array: T[], size: number): T[][] {
    const chunks: T[][] = []
    for (let i = 0; i < array.length; i += size) {
      chunks.push(array.slice(i, i + size))
    }
    return chunks
  }

  // Force sync window theme (can be called manually if needed)
  async function forceSync() {
    await syncWindowTheme()
  }

  // Watch for changes and apply
  watch(isDark, applyTheme, { immediate: true })

  // Re-sync when window becomes visible (handles edge cases)
  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', () => {
      if (!document.hidden) {
        syncWindowTheme()
      }
    })

    // Periodic sync every 5 seconds to ensure titlebar stays in sync
    setInterval(() => {
      syncWindowTheme()
    }, 5000)
  }

  return {
    isDark,
    toggleTheme,
    forceSync
  }
})
