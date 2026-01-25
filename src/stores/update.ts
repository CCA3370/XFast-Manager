import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { UpdateInfo } from '@/types'
import { useToastStore } from './toast'
import { useModalStore } from './modal'
import { useI18n } from 'vue-i18n'
import { logError, logDebug, logBasic } from '@/services/logger'
import { invokeVoidCommand, CommandError } from '@/services/api'

export const useUpdateStore = defineStore('update', () => {
  const { t } = useI18n()
  const toast = useToastStore()
  const modal = useModalStore()

  const updateInfo = ref<UpdateInfo | null>(null)
  const showUpdateBanner = ref(false)
  const lastCheckTime = ref<number | null>(null)
  const checkInProgress = ref(false)
  const autoCheckEnabled = ref(true)
  const includePreRelease = ref(false)

  // 从 localStorage 加载设置
  const savedAutoCheck = localStorage.getItem('autoCheckEnabled')
  if (savedAutoCheck !== null) {
    autoCheckEnabled.value = JSON.parse(savedAutoCheck)
  }

  const savedIncludePreRelease = localStorage.getItem('includePreRelease')
  if (savedIncludePreRelease !== null) {
    includePreRelease.value = JSON.parse(savedIncludePreRelease)
  }

  const savedLastCheckTime = localStorage.getItem('lastCheckTime')
  if (savedLastCheckTime) {
    lastCheckTime.value = parseInt(savedLastCheckTime, 10)
  }

  async function checkForUpdates(manual = false) {
    if (checkInProgress.value) return

    checkInProgress.value = true

    // 记录检查开始
    if (manual) {
      logBasic('User manually checking for updates', 'update')
    } else {
      logDebug('Auto-checking for updates', 'update')
    }

    try {
      const result = await invoke<UpdateInfo>('check_for_updates', {
        manual,
        includePreRelease: includePreRelease.value
      })

      // 如果有更新就显示横幅
      if (result.isUpdateAvailable) {
        updateInfo.value = result
        showUpdateBanner.value = true

        // 记录发现更新
        logBasic(
          `Update available: ${result.currentVersion} -> ${result.latestVersion}`,
          'update'
        )

        // 手动检查时显示通知
        if (manual) {
          toast.success(t('update.updateAvailableNotification', { version: result.latestVersion }))
        }
      } else {
        // 记录无更新
        logDebug('No update available', 'update')

        if (manual) {
          // 手动检查且已是最新版本时显示提示
          toast.success(t('update.upToDate'))
        }
      }

      lastCheckTime.value = Date.now()
      localStorage.setItem('lastCheckTime', lastCheckTime.value.toString())
    } catch (error) {
      const errorMessage =
        typeof error === 'string' ? error : (error as Error)?.message ?? String(error)

      if (errorMessage.includes('Cache not expired')) {
        logDebug('Update check skipped (cache not expired)', 'update')
        return
      }

      // 记录错误到日志
      logError(`Update check failed: ${errorMessage}`, 'update')

      if (manual) {
        // 手动检查时显示错误
        modal.showError(t('update.checkFailed'))
      }
      // 自动检查时静默失败（已记录到日志）
    } finally {
      checkInProgress.value = false
    }
  }

  function dismissUpdate() {
    // 只是暂时关闭横幅，不记录已关闭的版本
    // 下次检查时如果还有新版本会继续提示
    logDebug('User dismissed update banner', 'update')
    showUpdateBanner.value = false
  }

  async function openReleaseUrl() {
    // TODO: 替换为实际的论坛下载链接
    // 当前为占位符，等待正式发布后更新
    const forumUrl = 'https://example.com/xfast-manager-download' // 待替换为实际论坛链接

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

  function toggleAutoCheck() {
    autoCheckEnabled.value = !autoCheckEnabled.value
    localStorage.setItem('autoCheckEnabled', JSON.stringify(autoCheckEnabled.value))
    logBasic(`Auto-check updates ${autoCheckEnabled.value ? 'enabled' : 'disabled'}`, 'update')
  }

  function toggleIncludePreRelease() {
    includePreRelease.value = !includePreRelease.value
    localStorage.setItem('includePreRelease', JSON.stringify(includePreRelease.value))
    logBasic(`Include pre-release ${includePreRelease.value ? 'enabled' : 'disabled'}`, 'update')
  }

  return {
    updateInfo,
    showUpdateBanner,
    lastCheckTime,
    checkInProgress,
    autoCheckEnabled,
    includePreRelease,
    checkForUpdates,
    dismissUpdate,
    openReleaseUrl,
    toggleAutoCheck,
    toggleIncludePreRelease
  }
})
