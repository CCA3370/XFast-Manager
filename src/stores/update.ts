import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { UpdateInfo } from '@/types'
import { useToastStore } from './toast'
import { useModalStore } from './modal'
import { i18n } from '@/i18n'
import { logError, logDebug, logBasic } from '@/services/logger'
import { invokeVoidCommand, CommandError } from '@/services/api'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'

const t = i18n.global.t

export const useUpdateStore = defineStore('update', () => {
  const toast = useToastStore()
  const modal = useModalStore()

  const updateInfo = ref<UpdateInfo | null>(null)
  const showUpdateBanner = ref(false)
  const lastCheckTime = ref<number | null>(null)
  const checkInProgress = ref(false)
  const autoCheckEnabled = ref(true)
  const includePreRelease = ref(false)

  // Initialization flag
  const isInitialized = ref(false)

  // Initialize store by loading saved settings
  async function initStore(): Promise<void> {
    if (isInitialized.value) return

    const savedAutoCheck = await getItem<boolean>(STORAGE_KEYS.AUTO_CHECK_ENABLED)
    if (typeof savedAutoCheck === 'boolean') {
      autoCheckEnabled.value = savedAutoCheck
    }

    const savedIncludePreRelease = await getItem<boolean>(STORAGE_KEYS.INCLUDE_PRE_RELEASE)
    if (typeof savedIncludePreRelease === 'boolean') {
      includePreRelease.value = savedIncludePreRelease
    }

    const savedLastCheckTime = await getItem<string>(STORAGE_KEYS.LAST_CHECK_TIME)
    if (savedLastCheckTime) {
      lastCheckTime.value = parseInt(savedLastCheckTime, 10)
    }

    isInitialized.value = true
  }

  async function checkForUpdates(manual = false) {
    if (checkInProgress.value) return

    checkInProgress.value = true

    // Log check start
    if (manual) {
      logBasic('User manually checking for updates', 'update')
    } else {
      logDebug('Auto-checking for updates', 'update')
    }

    try {
      const result = await invoke<UpdateInfo>('check_for_updates', {
        manual,
        includePreRelease: includePreRelease.value,
      })

      // Show update banner if an update is available
      if (result.isUpdateAvailable) {
        updateInfo.value = result
        showUpdateBanner.value = true

        // Log update found
        logBasic(`Update available: ${result.currentVersion} -> ${result.latestVersion}`, 'update')

        // Show notification on manual check
        if (manual) {
          toast.success(t('update.updateAvailableNotification', { version: result.latestVersion }))
        }
      } else {
        // No update available
        logDebug('No update available', 'update')

        if (manual) {
          // Show notification when manually checked and already up to date
          toast.success(t('update.upToDate'))
        }
      }

      lastCheckTime.value = Date.now()
      await setItem(STORAGE_KEYS.LAST_CHECK_TIME, lastCheckTime.value.toString())
    } catch (error) {
      const errorMessage =
        typeof error === 'string' ? error : ((error as Error)?.message ?? String(error))

      if (errorMessage.includes('Cache not expired')) {
        logDebug('Update check skipped (cache not expired)', 'update')
        return
      }

      // Log the error
      logError(`Update check failed: ${errorMessage}`, 'update')

      if (manual) {
        // Show error on manual check
        modal.showError(t('update.checkFailed'))
      }
      // Silently fail on auto-check (already logged)
    } finally {
      checkInProgress.value = false
    }
  }

  function dismissUpdate() {
    // Temporarily dismiss the banner without recording the dismissed version
    // Will show again on next check if a new version is still available
    logDebug('User dismissed update banner', 'update')
    showUpdateBanner.value = false
  }

  async function openReleaseUrl() {
    const forumUrl = 'https://github.com/CCA3370/XFast-Manager/releases'

    logBasic('User clicked download button, opening forum URL', 'update')

    try {
      await invokeVoidCommand('open_url', { url: forumUrl })
      logDebug(`Successfully opened URL: ${forumUrl}`, 'update')
    } catch (error) {
      const message = error instanceof CommandError ? error.message : String(error)
      logError(`Failed to open download URL: ${message}`, 'update')
      modal.showError(t('common.error'))
    }
  }

  async function toggleAutoCheck() {
    autoCheckEnabled.value = !autoCheckEnabled.value
    await setItem(STORAGE_KEYS.AUTO_CHECK_ENABLED, autoCheckEnabled.value)
    logBasic(`Auto-check updates ${autoCheckEnabled.value ? 'enabled' : 'disabled'}`, 'update')
  }

  async function toggleIncludePreRelease() {
    includePreRelease.value = !includePreRelease.value
    await setItem(STORAGE_KEYS.INCLUDE_PRE_RELEASE, includePreRelease.value)
    logBasic(`Include pre-release ${includePreRelease.value ? 'enabled' : 'disabled'}`, 'update')
  }

  return {
    updateInfo,
    showUpdateBanner,
    lastCheckTime,
    checkInProgress,
    autoCheckEnabled,
    includePreRelease,
    isInitialized,
    initStore,
    checkForUpdates,
    dismissUpdate,
    openReleaseUrl,
    toggleAutoCheck,
    toggleIncludePreRelease,
  }
})
