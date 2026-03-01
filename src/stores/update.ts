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
import bundledChangelog from '@/generated/changelog'

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
    const withoutPrefix = version.trim().replace(/^v/i, '')
    const semverCore = withoutPrefix.split('+')[0]?.split('-')[0] ?? withoutPrefix
    const match = semverCore.match(/\d+(?:\.\d+){0,3}/)
    if (!match) return semverCore.trim()

    const segments = match[0].split('.').map((segment) => {
      const normalized = Number.parseInt(segment, 10)
      return Number.isFinite(normalized) ? String(normalized) : segment
    })

    // Normalize "1.2.3.0" to "1.2.3" for release tag matching.
    if (segments.length > 3 && segments[segments.length - 1] === '0') {
      segments.pop()
    }

    return segments.join('.').trim()
  }

  type ChangelogEntry = {
    version: string
    normalizedVersion: string
    section: string
  }

  function parseChangelogEntries(markdown: string): ChangelogEntry[] {
    const headers: Array<{ version: string; headerEnd: number; headerStart: number }> = []
    const headerRegex = /^##\s+\[v?([^\]]+)\].*$/gm
    let match: RegExpExecArray | null

    while (true) {
      match = headerRegex.exec(markdown)
      if (!match) break
      headers.push({
        version: match[1]?.trim() ?? '',
        headerStart: match.index,
        headerEnd: headerRegex.lastIndex,
      })
    }

    return headers.map((header, index) => {
      const sectionEnd = index + 1 < headers.length ? headers[index + 1].headerStart : markdown.length
      const section = markdown.slice(header.headerEnd, sectionEnd).trim()
      return {
        version: header.version,
        normalizedVersion: normalizeVersionTag(header.version),
        section,
      }
    })
  }

  const bundledChangelogEntries = parseChangelogEntries(bundledChangelog)

  function readBundledChangelogSection(version: string): string {
    const normalized = normalizeVersionTag(version)
    return bundledChangelogEntries.find((entry) => entry.normalizedVersion === normalized)?.section ?? ''
  }

  function getLatestBundledChangelogEntry(): ChangelogEntry | null {
    return bundledChangelogEntries[0] ?? null
  }

  function getBundledChangelogVersionsPreview(limit = 8): string {
    return bundledChangelogEntries
      .slice(0, limit)
      .map((entry) => entry.normalizedVersion)
      .join(', ')
  }

  function applyChangelog(version: string, releaseNotes: string) {
    postUpdateVersion.value = version
    postUpdateReleaseNotes.value = releaseNotes
    postUpdateReleaseUrl.value = `https://github.com/CCA3370/XFast-Manager/releases/tag/v${version}`
    showPostUpdateChangelog.value = true
  }

  async function checkAndShowPostUpdateChangelog() {
    try {
      const currentVersion = await invoke<string>('get_app_version')
      const normalizedVersion = normalizeVersionTag(currentVersion)
      const shownVersion = await getItem<string>(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION)

      logDebug(
        `Post-update changelog check rawVersion='${currentVersion}' normalized='${normalizedVersion}' bundledEntries=${bundledChangelogEntries.length} bundledLength=${bundledChangelog.length}`,
        'update'
      )

      if (shownVersion && normalizeVersionTag(shownVersion) === normalizedVersion) {
        return
      }

      const releaseNotes = readBundledChangelogSection(normalizedVersion)
      if (!releaseNotes) {
        const latestEntry = getLatestBundledChangelogEntry()
        logDebug(
          `No bundled changelog section for v${normalizedVersion}; available=[${getBundledChangelogVersionsPreview()}]`,
          'update'
        )

        if (!latestEntry?.section) {
          await setItem(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION, normalizedVersion)
          return
        }

        applyChangelog(latestEntry.normalizedVersion, latestEntry.section)
        logBasic(
          `Falling back to latest bundled changelog v${latestEntry.normalizedVersion} for current v${normalizedVersion}`,
          'update'
        )
        await setItem(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION, normalizedVersion)
        return
      }

      applyChangelog(normalizedVersion, releaseNotes)

      await setItem(STORAGE_KEYS.LAST_SHOWN_CHANGELOG_VERSION, normalizedVersion)
      logBasic(`Showing post-update changelog for v${normalizedVersion}`, 'update')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      logError(`Failed to check post-update changelog: ${message}`, 'update')
    }
  }

  async function viewCurrentVersionChangelog() {
    try {
      const currentVersion = await invoke<string>('get_app_version')
      const normalizedVersion = normalizeVersionTag(currentVersion)
      const releaseNotes = readBundledChangelogSection(normalizedVersion)

      if (!releaseNotes) {
        const latestEntry = getLatestBundledChangelogEntry()
        logDebug(
          `Current-version changelog miss rawVersion='${currentVersion}' normalized='${normalizedVersion}' available=[${getBundledChangelogVersionsPreview()}]`,
          'update'
        )

        if (!latestEntry?.section) {
          modal.showError(t('update.changelogNotFound', { version: normalizedVersion }))
          return
        }

        applyChangelog(latestEntry.normalizedVersion, latestEntry.section)
        logBasic(
          `Falling back to latest bundled changelog v${latestEntry.normalizedVersion} for current v${normalizedVersion}`,
          'update'
        )
      } else {
        applyChangelog(normalizedVersion, releaseNotes)
        logDebug(`Showing current version changelog for v${normalizedVersion}`, 'update')
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      logError(`Failed to show current version changelog: ${message}`, 'update')
      modal.showError(t('common.error'))
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
    viewCurrentVersionChangelog,
    dismissUpdate,
    dismissPostUpdateChangelog,
    openReleaseUrl,
    toggleAutoCheck,
    toggleIncludePreRelease,
  }
})
