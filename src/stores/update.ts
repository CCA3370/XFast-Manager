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
  const showPostUpdateChangelog = ref(false)
  const postUpdateVersion = ref('')
  const postUpdateReleaseNotes = ref('')
  const postUpdateReleaseUrl = ref('')

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
    // Select URL based on current language
    const currentLocale = i18n.global.locale.value
    const releaseUrl =
      currentLocale === 'zh'
        ? 'https://www.3370tech.cn/zh/products/xfast-manager'
        : 'https://forums.x-plane.org/files/file/98845-linwinmacxfast-managerdrag-drop-addon-installer-auto-scenery-sorting-more/'

    logBasic('User clicked download button, opening release URL', 'update')

    try {
      await invokeVoidCommand('open_url', { url: releaseUrl })
      logDebug(`Successfully opened URL: ${releaseUrl}`, 'update')
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

  function normalizeVersionTag(version: string): string {
    return version.trim().replace(/^v/i, '')
  }

  function extractChangelogSection(markdown: string, version: string): string {
    const normalized = normalizeVersionTag(version)
    const escapedVersion = normalized.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const sectionRegex = new RegExp(
      `^##\\s+\\[v?${escapedVersion}\\].*\\n([\\s\\S]*?)(?=^##\\s+\\[|\\Z)`,
      'm',
    )

    const match = markdown.match(sectionRegex)
    return match?.[1]?.trim() ?? ''
  }

  async function fetchDevChangelogSection(version: string): Promise<string> {
    const changelogUrl =
      'https://raw.githubusercontent.com/CCA3370/XFast-Manager/dev/CHANGELOG.md'

    try {
      const response = await fetch(changelogUrl)
      if (!response.ok) {
        logDebug(`Failed to fetch dev CHANGELOG.md, status: ${response.status}`, 'update')
        return ''
      }

      const markdown = await response.text()
      return extractChangelogSection(markdown, version)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      logError(`Failed to fetch dev CHANGELOG.md: ${message}`, 'update')
      return ''
    }
  }

  async function checkAndShowPostUpdateChangelog() {
    try {
      const currentVersion = await invoke<string>('get_app_version')
      const normalizedVersion = normalizeVersionTag(currentVersion)
      const shownVersion = await getItem<string>(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION)

      if (shownVersion && normalizeVersionTag(shownVersion) === normalizedVersion) {
        return
      }

      const releaseNotes = await fetchDevChangelogSection(normalizedVersion)
      if (!releaseNotes) {
        logDebug(`No changelog section found in dev branch for v${normalizedVersion}`, 'update')
        await setItem(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION, normalizedVersion)
        return
      }

      postUpdateVersion.value = normalizedVersion
      postUpdateReleaseNotes.value = releaseNotes
      postUpdateReleaseUrl.value = `https://github.com/CCA3370/XFast-Manager/releases/tag/v${normalizedVersion}`
      showPostUpdateChangelog.value = true

      await setItem(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION, normalizedVersion)
      logBasic(`Showing post-update changelog for v${normalizedVersion}`, 'update')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      logError(`Failed to check post-update changelog: ${message}`, 'update')
    }
  }

  function dismissPostUpdateChangelog() {
    showPostUpdateChangelog.value = false
  }

  return {
    updateInfo,
    showUpdateBanner,
    lastCheckTime,
    checkInProgress,
    autoCheckEnabled,
    includePreRelease,
    showPostUpdateChangelog,
    postUpdateVersion,
    postUpdateReleaseNotes,
    postUpdateReleaseUrl,
    isInitialized,
    initStore,
    checkForUpdates,
    checkAndShowPostUpdateChangelog,
    dismissUpdate,
    dismissPostUpdateChangelog,
    openReleaseUrl,
    toggleAutoCheck,
    toggleIncludePreRelease,
  }
})
