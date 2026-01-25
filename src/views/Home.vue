<template>
  <div class="home-view h-full flex flex-col p-6 animate-fade-in relative overflow-hidden select-none">
    <!-- Background Decor (Dark Mode Only for deep glow) -->
    <div class="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none z-0 opacity-0 dark:opacity-100 transition-opacity duration-500">
      <div class="absolute top-1/4 left-1/4 w-64 h-64 bg-blue-500/10 rounded-full blur-3xl"></div>
      <div class="absolute bottom-1/4 right-1/4 w-96 h-96 bg-purple-500/10 rounded-full blur-3xl"></div>
    </div>

    <div class="w-full z-10 flex flex-col flex-1 min-h-0 gap-3">
      <!-- Update Banner -->
      <UpdateBanner
        :visible="updateStore.showUpdateBanner"
        :update-info="updateStore.updateInfo"
        @view-release="updateStore.openReleaseUrl"
        @dismiss="updateStore.dismissUpdate"
      />

      <!-- Warning Alert (Compact) -->
      <transition name="slide-down">
        <div
          v-if="!store.xplanePath"
          class="flex-shrink-0 bg-yellow-50/90 dark:bg-yellow-500/10 backdrop-blur-md border border-yellow-200 dark:border-yellow-500/20 rounded-xl p-3 flex items-center space-x-3 shadow-lg shadow-yellow-500/5 transition-colors duration-300"
        >
          <div class="p-2 bg-yellow-100 dark:bg-yellow-500/20 rounded-lg flex-shrink-0">
            <svg class="w-5 h-5 text-yellow-600 dark:text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
            </svg>
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-yellow-800 dark:text-yellow-100 truncate"><AnimatedText>{{ $t('home.setPathFirst') }}</AnimatedText></p>
            <p class="text-xs text-yellow-700 dark:text-yellow-200/70 truncate"><AnimatedText>{{ $t('home.pathNotSetDesc') }}</AnimatedText></p>
          </div>
          <router-link
            to="/settings"
            class="flex-shrink-0 inline-flex items-center px-3 py-1.5 bg-yellow-200/50 dark:bg-yellow-500/20 hover:bg-yellow-200 dark:hover:bg-yellow-500/30 text-yellow-800 dark:text-yellow-200 text-xs font-medium rounded-lg transition-colors duration-200 border border-yellow-300 dark:border-yellow-500/30"
          >
            <AnimatedText>{{ $t('home.goToSettings') }}</AnimatedText>
            <svg class="w-3 h-3 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
            </svg>
          </router-link>
        </div>
      </transition>

      <!-- Main Action Area (Flexible Height) -->
      <div class="flex-1 min-h-0 bg-white/60 dark:bg-gray-800/40 backdrop-blur-xl border-2 border-dashed border-gray-300 dark:border-gray-600/50 rounded-2xl p-6 text-center transition-all duration-500 hover:border-blue-400 dark:hover:border-blue-500/50 hover:bg-white/80 dark:hover:bg-gray-800/60 shadow-sm dark:shadow-none flex flex-col items-center justify-center relative drop-zone-card"
        :class="{
          'drag-over ring-4 ring-4-blue-500/20 border-blue-500 scale-[1.02]': isDragging,
          'animate-pulse border-blue-400': store.isAnalyzing,
          'debug-drop': debugDropFlash
        }"
      >
          <!-- Hover Gradient -->
          <div class="absolute inset-0 bg-gradient-to-br from-blue-50/50 to-purple-50/50 dark:from-blue-600/5 dark:to-purple-600/5 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"></div>

          <div v-if="!store.isAnalyzing && !store.isInstalling && !store.showCompletion" class="relative z-10 flex flex-col items-center justify-center space-y-4">
            <!-- Icon -->
            <div class="w-16 h-16 rounded-full bg-blue-50 dark:bg-gray-700/50 flex items-center justify-center group-hover:scale-110 transition-transform duration-500 relative">
              <div class="absolute inset-0 bg-blue-500/20 rounded-full blur-xl opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
              <svg
                class="w-8 h-8 text-gray-400 dark:text-gray-400 group-hover:text-blue-500 dark:group-hover:text-blue-400 transition-colors duration-300"
                :class="{ 'text-blue-500 dark:text-blue-400 animate-bounce': isDragging }"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
              </svg>
            </div>

            <!-- Text -->
            <div class="space-y-1 text-center">
              <h2 class="text-2xl font-bold text-gray-900 dark:text-white tracking-tight group-hover:text-blue-600 dark:group-hover:text-blue-100 transition-colors">
                <AnimatedText>{{ $t('home.dropFilesHere') }}</AnimatedText>
              </h2>
              <p class="text-gray-500 dark:text-gray-400 max-w-md mx-auto text-sm">
                <AnimatedText>{{ $t('home.supportedFormats') }}</AnimatedText>
              </p>
            </div>

            <!-- Features Badges -->
            <div class="flex flex-wrap justify-center gap-2 pt-2">
              <span class="px-2.5 py-1 rounded-full bg-white dark:bg-gray-700/50 border border-gray-200 dark:border-white/5 text-gray-600 dark:text-gray-300 text-xs font-medium flex items-center space-x-1.5 shadow-sm dark:shadow-none">
                <svg class="w-3.5 h-3.5 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
                <span><AnimatedText>{{ $t('home.autoDetect') }}</AnimatedText></span>
              </span>
              <span class="px-2.5 py-1 rounded-full bg-white dark:bg-gray-700/50 border border-gray-200 dark:border-white/5 text-gray-600 dark:text-gray-300 text-xs font-medium flex items-center space-x-1.5 shadow-sm dark:shadow-none">
                <svg class="w-3.5 h-3.5 text-purple-500 dark:text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                </svg>
                <span><AnimatedText>{{ $t('home.fastInstall') }}</AnimatedText></span>
              </span>
              <span class="px-2.5 py-1 rounded-full bg-white dark:bg-gray-700/50 border border-gray-200 dark:border-white/5 text-gray-600 dark:text-gray-300 text-xs font-medium flex items-center space-x-1.5 shadow-sm dark:shadow-none">
                <svg class="w-3.5 h-3.5 text-emerald-500 dark:text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
                </svg>
                <span><AnimatedText>{{ $t('home.batchProcess') }}</AnimatedText></span>
              </span>
            </div>
          </div>

        <!-- Progress Overlays -->
        <transition name="fade" mode="out-in">
          <AnalyzingOverlay v-if="store.isAnalyzing" key="analyzing" />

          <InstallProgressOverlay
            v-else-if="store.isInstalling"
            key="installing"
            :percentage="progressStore.formatted.percentage"
            :task-name="progressStore.formatted.taskName"
            :processed-m-b="progressStore.formatted.processedMB"
            :total-m-b="progressStore.formatted.totalMB"
            :task-progress="progressStore.formatted.taskProgress"
            @skip="handleSkipTask"
            @cancel="handleCancelInstallation"
          />
        </transition>

        <!-- Completion View (shows behind animation) -->
        <transition name="fade-in-slow" mode="out-in">
          <div v-if="store.showCompletion" class="absolute inset-0 z-20 bg-white dark:bg-gray-900 rounded-2xl flex items-center justify-center p-6 transition-colors duration-300 pointer-events-auto">
            <div class="w-full max-w-md">
              <CompletionView
                :result="store.installResult!"
                @confirm="handleCompletionConfirm"
              />
            </div>
          </div>
        </transition>

        <!-- Completion Animation Overlay (on top of completion view) -->
        <transition name="fade">
          <div v-if="store.showCompletionAnimation" class="absolute inset-0 z-30 flex items-start justify-center pointer-events-none"
               :class="store.installResult && store.installResult.failedTasks === 0 ? 'pt-[80px]' : 'pt-[40px]'">
            <div class="w-full max-w-md">
                <!-- Success Icon with Animation -->
                <div v-if="store.installResult && store.installResult.failedTasks === 0" class="relative w-20 h-20 mx-auto">
                  <!-- Animated checkmark circle -->
                  <div class="absolute inset-0 bg-gradient-to-r from-green-500 to-emerald-600 rounded-full animate-scale-in flex items-center justify-center shadow-2xl">
                    <svg class="w-10 h-10 text-white animate-check-draw" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path class="check-path" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                  </div>
                  <!-- Ripple effect -->
                  <div class="absolute inset-0 bg-green-500 rounded-full animate-ripple opacity-0"></div>
                </div>
                <!-- Partial Success Icon -->
                <div v-else-if="store.installResult && store.installResult.successfulTasks > 0" class="relative w-20 h-20 mx-auto">
                  <div class="absolute inset-0 bg-gradient-to-r from-yellow-500 to-orange-600 rounded-full animate-scale-in flex items-center justify-center shadow-2xl">
                    <svg class="w-10 h-10 text-white animate-check-draw" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                    </svg>
                  </div>
                  <div class="absolute inset-0 bg-yellow-500 rounded-full animate-ripple opacity-0"></div>
                </div>
                <!-- Failure Icon -->
                <div v-else class="relative w-20 h-20 mx-auto">
                  <div class="absolute inset-0 bg-gradient-to-r from-red-500 to-red-600 rounded-full animate-scale-in flex items-center justify-center shadow-2xl">
                    <svg class="w-10 h-10 text-white animate-check-draw" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                  </div>
                  <div class="absolute inset-0 bg-red-500 rounded-full animate-ripple opacity-0"></div>
                </div>
            </div>
          </div>
        </transition>
      </div>

      <ConfirmationModal v-if="showConfirmation" @close="showConfirmation = false" @confirm="handleInstall" />
      <PasswordModal
        v-if="showPasswordModal"
        :archive-paths="passwordRequiredPaths"
        :error-message="passwordErrorMessage"
        @confirm="handlePasswordSubmit"
        @cancel="handlePasswordCancel"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useProgressStore } from '@/stores/progress'
import { useUpdateStore } from '@/stores/update'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import ConfirmationModal from '@/components/ConfirmationModal.vue'
import PasswordModal from '@/components/PasswordModal.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import CompletionView from '@/components/CompletionView.vue'
import UpdateBanner from '@/components/UpdateBanner.vue'
import InstallProgressOverlay from '@/components/InstallProgressOverlay.vue'
import AnalyzingOverlay from '@/components/AnalyzingOverlay.vue'
import type { AnalysisResult, InstallProgress, InstallResult } from '@/types'
import { getErrorMessage } from '@/types'
import { logOperation, logError, logDebug, logBasic } from '@/services/logger'

const { t } = useI18n()

const store = useAppStore()
const toast = useToastStore()
const modal = useModalStore()
const updateStore = useUpdateStore()
const progressStore = useProgressStore()
const isDragging = ref(false)
const showConfirmation = ref(false)
const debugDropFlash = ref(false)

// Password modal state
const showPasswordModal = ref(false)
const passwordRequiredPaths = ref<string[]>([])
const pendingAnalysisPaths = ref<string[]>([])
const collectedPasswords = ref<Record<string, string>>({})
const passwordRetryCount = ref(0)
const passwordErrorMessage = ref('')
const MAX_PASSWORD_RETRIES = 3

// Password rate limiting
const passwordAttemptTimestamps = ref<number[]>([])
const MIN_PASSWORD_ATTEMPT_DELAY_MS = 1000 // 1 second between attempts
const PASSWORD_RATE_LIMIT_WINDOW_MS = 10000 // 10 second window for rate limiting
const DEBUG_DROP_FLASH_DURATION_MS = 800 // Duration for debug drop flash visual feedback
const COMPLETION_ANIMATION_DELAY_MS = 100 // Brief delay before hiding progress to allow animation to start

// Timer tracking for cleanup on unmount to prevent memory leaks
// Using Set for O(1) add/delete operations instead of array
const activeTimeoutIds = new Set<ReturnType<typeof setTimeout>>()

// Tauri drag-drop event unsubscribe function
let unlistenDragDrop: UnlistenFn | null = null
let unlistenProgress: UnlistenFn | null = null
let unlistenDeletionSkipped: UnlistenFn | null = null

// Helper to create tracked timeouts that will be cleaned up on unmount
function setTrackedTimeout(callback: () => void, delay: number): ReturnType<typeof setTimeout> {
  const id = setTimeout(() => {
    callback()
    // Remove the timeout ID from the tracking set after it fires
    activeTimeoutIds.delete(id)
  }, delay)
  activeTimeoutIds.add(id)
  return id
}

// Watch for pending CLI args changes
watch(() => store.pendingCliArgs, async (args) => {
  if (args && args.length > 0) {
    // Use store mutex flag for TOCTOU-safe concurrency control
    // This prevents race conditions when multiple watch events fire quickly
    if (store.isAnalyzeInProgress || store.isAnalyzing || store.isInstalling) {
      logDebug('Analysis in progress, re-queueing args for later', 'app')
      store.addCliArgsToBatch(args)
      store.clearPendingCliArgs()
      return
    }

    // Set mutex immediately before any async operation
    store.isAnalyzeInProgress = true

    logDebug(`Processing pending CLI args from watcher: ${args.join(', ')}`, 'app')
    const argsCopy = [...args]
    store.clearPendingCliArgs()
    try {
      await analyzeFiles(argsCopy)
    } catch (error) {
      logError(`Failed to process CLI args: ${error}`, 'app')
      modal.showError(getErrorMessage(error))
    } finally {
      // Always release mutex
      store.isAnalyzeInProgress = false
    }
  }
})

// Global listeners for drag/drop visual feedback
function onWindowDragOver(e: DragEvent) {
  e.preventDefault()
  // Ignore drag events when installing
  if (store.isInstalling) {
    return
  }
  isDragging.value = true
}

function onWindowDragLeave(e: DragEvent) {
  // Ignore drag events when installing
  if (store.isInstalling) {
    return
  }
  // Only set to false if leaving the window
  if (!e.relatedTarget) {
    isDragging.value = false
  }
}

function onWindowDrop(e: DragEvent) {
  logDebug('Window drop event (HTML5)', 'drag-drop')
  e.preventDefault()
  // Ignore drop events when installing
  if (store.isInstalling) {
    return
  }
  isDragging.value = false
  debugDropFlash.value = true
  setTrackedTimeout(() => (debugDropFlash.value = false), DEBUG_DROP_FLASH_DURATION_MS)
}

onMounted(async () => {
  window.addEventListener('dragover', onWindowDragOver)
  window.addEventListener('dragleave', onWindowDragLeave)
  window.addEventListener('drop', onWindowDrop)

  // Use Tauri 2's native drag-drop event for getting file paths
  try {
    const webview = getCurrentWebviewWindow()
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      logDebug(`Tauri drag-drop event: ${event.payload.type}`, 'drag-drop')

      // Ignore all drag-drop events when installing
      if (store.isInstalling) {
        logDebug('Ignoring drag-drop event (installing)', 'drag-drop')
        return
      }

      if (event.payload.type === 'over') {
        isDragging.value = true
      } else if (event.payload.type === 'leave') {
        isDragging.value = false
      } else if (event.payload.type === 'drop') {
        isDragging.value = false
        debugDropFlash.value = true
        setTrackedTimeout(() => (debugDropFlash.value = false), DEBUG_DROP_FLASH_DURATION_MS)

        // If showing completion, close it and start new analysis
        if (store.showCompletion) {
          store.clearInstallResult()
        }

        const paths = event.payload.paths
        logDebug(`Dropped paths from Tauri: ${paths.join(', ')}`, 'drag-drop')

        if (paths && paths.length > 0) {
          try {
            await analyzeFiles(paths)
          } catch (error) {
            logError(`Failed to analyze dropped files: ${error}`, 'drag-drop')
            modal.showError(getErrorMessage(error))
          }
        }
      }
    })
    logDebug('Tauri drag-drop listener registered', 'drag-drop')
  } catch (error) {
    logError(`Failed to setup Tauri drag-drop listener: ${error}`, 'drag-drop')
  }

  // Listen for installation progress events
  try {
    unlistenProgress = await listen<InstallProgress>('install-progress', (event) => {
      progressStore.update(event.payload)
    })
    logDebug('Progress listener registered', 'install')
  } catch (error) {
    logError(`Failed to setup progress listener: ${error}`, 'install')
  }

  // Listen for source deletion skipped events
  try {
    unlistenDeletionSkipped = await listen<string>('source-deletion-skipped', (event) => {
      const path = event.payload
      toast.info(t('home.sourceDeletionSkipped', { path }))
    })
    logDebug('Source deletion skipped listener registered', 'install')
  } catch (error) {
    logError(`Failed to setup source deletion skipped listener: ${error}`, 'install')
  }

  // Note: Pending CLI args are now handled by the watcher above
  // No need to manually check here - the watcher will trigger automatically
})

onBeforeUnmount(() => {
  window.removeEventListener('dragover', onWindowDragOver)
  window.removeEventListener('dragleave', onWindowDragLeave)
  window.removeEventListener('drop', onWindowDrop)

  // Cleanup all tracked timeouts to prevent memory leaks
  activeTimeoutIds.forEach(id => clearTimeout(id))
  activeTimeoutIds.clear()

  // Cleanup Tauri listeners
  if (unlistenDragDrop) {
    unlistenDragDrop()
  }
  if (unlistenProgress) {
    unlistenProgress()
  }
  if (unlistenDeletionSkipped) {
    unlistenDeletionSkipped()
  }
})

async function analyzeFiles(paths: string[], passwords?: Record<string, string>) {
  // Log incoming files
  logOperation(t('log.filesDropped'), t('log.fileCount', { count: paths.length }))
  logDebug(`Analyzing paths: ${paths.join(', ')}`, 'analysis')

  // Reset password retry counter for new analysis (not a retry with passwords)
  if (!passwords || Object.keys(passwords).length === 0) {
    passwordRetryCount.value = 0
  }

  if (!store.xplanePath) {
    logDebug('No X-Plane path set', 'analysis')
    // Log the abort reason - toast.warning will also log via the store
    logOperation(t('log.taskAborted'), t('log.xplanePathNotSet'))
    toast.warning(t('home.pathNotSet'))
    return
  }

  store.isAnalyzing = true
  logDebug(`Starting analysis with X-Plane path: ${store.xplanePath}`, 'analysis')

  try {
    logDebug(`Paths to analyze: ${paths.join(', ')}`, 'analysis')

    const result = await invoke<AnalysisResult>('analyze_addons', {
      paths,
      xplanePath: store.xplanePath,
      passwords: passwords || null,
      verificationPreferences: store.verificationPreferences
    })

    logDebug(`Analysis result: ${result.tasks.length} tasks, ${result.errors.length} errors`, 'analysis')

    // Check if any archives require passwords
    if (result.passwordRequired && result.passwordRequired.length > 0) {
      // Log password requirement
      logOperation(t('log.passwordRequired'), t('log.fileCount', { count: result.passwordRequired.length }))
      logDebug(`Password required for: ${result.passwordRequired.join(', ')}`, 'analysis')
      // Store the original paths for re-analysis after password input
      pendingAnalysisPaths.value = paths
      passwordRequiredPaths.value = result.passwordRequired
      // Preserve already collected passwords
      if (passwords) {
        collectedPasswords.value = { ...passwords }
      }
      showPasswordModal.value = true
      store.isAnalyzing = false
      return
    }

    if (result.errors.length > 0) {
      logDebug(`Errors during analysis: ${result.errors.join('; ')}`, 'analysis')
      // Check if errors indicate wrong password
      const passwordErrors = result.errors.filter(err =>
        err.includes('Wrong password') || err.includes('password') || err.includes('Password')
      )

      if (passwordErrors.length > 0 && passwords) {
        // Increment retry counter BEFORE checking limit to prevent race condition
        passwordRetryCount.value++

        // Check if we've exceeded retry limit (use >= to prevent off-by-one error)
        if (passwordRetryCount.value >= MAX_PASSWORD_RETRIES) {
          logOperation(t('log.taskAborted'), t('log.passwordMaxRetries'))
          modal.showError(t('password.maxRetries') + '\n\n' + result.errors.join('\n'))
          resetPasswordState()
          store.isAnalyzing = false
          return
        }

        // 从错误信息中提取密码错误的文件路径
        const wrongPasswordPaths = extractWrongPasswordPaths(passwordErrors)
        if (wrongPasswordPaths.length > 0) {
          passwordRequiredPaths.value = wrongPasswordPaths
        }

        // 设置错误提示并重新显示密码模态框
        passwordErrorMessage.value = t('password.wrongPassword')
        showPasswordModal.value = true
        store.isAnalyzing = false
        return
      }

      // Show errors as a modal popup; keep other notifications as toasts
      modal.showError(result.errors.join('\n'))
    }

    if (result.tasks.length > 0) {
      // Filter tasks based on preferences
      const allowedTasks = result.tasks.filter(task => store.installPreferences[task.type])
      const ignoredCount = result.tasks.length - allowedTasks.length

      logDebug(`Filtered tasks: ${allowedTasks.length} allowed, ${ignoredCount} ignored`, 'analysis')

      if (ignoredCount > 0) {
        toast.info(t('home.ignoredTasks', { count: ignoredCount }))
      }

      if (allowedTasks.length > 0) {
        store.setCurrentTasks(allowedTasks)
        showConfirmation.value = true
        // Reset password state on successful analysis
        resetPasswordState()
        // Non-blocking log call
        logBasic(t('log.analysisCompleted'), 'analysis')
        logOperation(t('log.analysisCompleted'), t('log.taskCount', { count: allowedTasks.length }))
        logDebug(`Task types: ${allowedTasks.map(t => t.type).join(', ')}`, 'analysis')
      } else if (ignoredCount > 0) {
        toast.warning(t('home.allIgnored'))
      } else {
        toast.warning(t('home.noValidAddons'))
      }
    } else {
      logDebug('No valid addons detected in analysis', 'analysis')
      toast.warning(t('home.noValidAddons'))
    }
  } catch (error) {
    // Non-blocking log call (also prints to console.error internally)
    logError(`${t('log.analysisFailed')}: ${error}`, 'analysis')
    modal.showError(t('home.failedToAnalyze') + ': ' + getErrorMessage(error))
  } finally {
    store.isAnalyzing = false
  }
}

// Handle password modal submit
async function handlePasswordSubmit(passwords: Record<string, string>) {
  // Rate limiting: check if attempts are too frequent
  const now = Date.now()
  const recentAttempts = passwordAttemptTimestamps.value.filter(
    t => now - t < PASSWORD_RATE_LIMIT_WINDOW_MS // Within rate limit window
  )

  if (recentAttempts.length > 0) {
    const lastAttempt = Math.max(...recentAttempts)
    const timeSinceLastAttempt = now - lastAttempt

    if (timeSinceLastAttempt < MIN_PASSWORD_ATTEMPT_DELAY_MS) {
      toast.warning(t('password.tooFast') || 'Please wait before trying again')
      return
    }
  }

  // Record this attempt
  passwordAttemptTimestamps.value.push(now)
  // Keep only recent attempts (last 10 seconds)
  passwordAttemptTimestamps.value = passwordAttemptTimestamps.value.filter(
    t => now - t < PASSWORD_RATE_LIMIT_WINDOW_MS
  )

  showPasswordModal.value = false
  passwordErrorMessage.value = ''  // Clear error message
  logOperation(t('log.passwordEntered'), t('log.fileCount', { count: Object.keys(passwords).length }))
  // Merge new passwords with previously collected ones
  const allPasswords = { ...collectedPasswords.value, ...passwords }

  // Note: retry counter is incremented only when password is wrong (in analyzeFiles)
  // Re-analyze with passwords
  await analyzeFiles(pendingAnalysisPaths.value, allPasswords)
}

// Handle password modal cancel
async function handlePasswordCancel() {
  showPasswordModal.value = false
  logOperation(t('log.taskAborted'), t('log.passwordCanceled'))

  // After cancel, continue analyzing files that don't require password
  const nonPasswordPaths = pendingAnalysisPaths.value.filter(
    p => !passwordRequiredPaths.value.includes(p)
  )

  resetPasswordState()

  // If there are files that don't need password, continue analyzing them
  if (nonPasswordPaths.length > 0) {
    await analyzeFiles(nonPasswordPaths)
  }
}

// Extract wrong password file paths from error messages
function extractWrongPasswordPaths(errors: string[]): string[] {
  const paths: string[] = []
  for (const err of errors) {
    // Match "Wrong password for archive: {path}" format
    const match = err.match(/Wrong password for archive:\s*(.+)$/i)
    if (match && match[1]) {
      paths.push(match[1].trim())
    }
  }
  // If unable to extract, return current passwordRequiredPaths
  return paths.length > 0 ? paths : passwordRequiredPaths.value
}

// Reset password state
function resetPasswordState() {
  pendingAnalysisPaths.value = []
  passwordRequiredPaths.value = []
  collectedPasswords.value = {}
  passwordRetryCount.value = 0
  passwordErrorMessage.value = ''
}

async function handleInstall() {
  showConfirmation.value = false

  // Filter only enabled tasks
  const enabledTasks = store.currentTasks.filter(task => store.getTaskEnabled(task.id))

  if (enabledTasks.length === 0) {
    toast.warning(t('home.noTasksEnabled'))
    return
  }

  store.isInstalling = true
  // Non-blocking log call
  logBasic(t('log.installationStarted'), 'installation')
  logOperation(t('log.installationStarted'), t('log.taskCount', { count: enabledTasks.length }))
  logDebug(`Installing ${enabledTasks.length} tasks: ${enabledTasks.map(t => t.displayName).join(', ')}`, 'installation')

  try {
    // Prepare enabled tasks with overwrite settings
    const tasksWithOverwrite = enabledTasks.map(task => ({
      ...task,
      shouldOverwrite: store.getTaskOverwrite(task.id) ?? false,
      sizeConfirmed: store.getTaskSizeConfirmed(task.id) ?? false,
    }))

    const overwriteCount = tasksWithOverwrite.filter(t => t.shouldOverwrite).length
    if (overwriteCount > 0) {
      logDebug(`${overwriteCount} tasks will overwrite existing files`, 'installation')
    }

    const result = await invoke<InstallResult>('install_addons', {
      tasks: tasksWithOverwrite,
      atomicInstallEnabled: store.atomicInstallEnabled,
      xplanePath: store.xplanePath,
      deleteSourceAfterInstall: store.deleteSourceAfterInstall,
      autoSortScenery: store.autoSortScenery
    })

    // Log results
    logBasic(t('log.installationCompleted'), 'installation')
    logOperation(`${result.successfulTasks}/${result.totalTasks} tasks completed successfully`)

    // Log failed tasks if any
    if (result.failedTasks > 0) {
      result.taskResults
        .filter(r => !r.success)
        .forEach(r => {
          logError(`${r.taskName}: ${r.errorMessage}`, 'installation')
        })
    }

    // Ensure progress bar shows 100% before transitioning
    progressStore.setPercentage(100)

    // Save installation result immediately (this will trigger the animation)
    store.setInstallResult(result)

    // Keep isInstalling true for a brief moment to show the animation
    // Then set it to false after animation starts
    setTrackedTimeout(() => {
      store.isInstalling = false
      progressStore.reset()
    }, COMPLETION_ANIMATION_DELAY_MS)

  } catch (error) {
    // Non-blocking log call (also prints to console.error internally)
    logError(`${t('log.installationFailed')}: ${error}`, 'installation')
    modal.showError(t('home.installationFailed') + ': ' + getErrorMessage(error))
    store.isInstalling = false
    progressStore.reset()
  }
}

// Handle skip current task
async function handleSkipTask() {
  const confirmed = await showConfirmDialog({
    title: t('taskControl.skipConfirmTitle'),
    message: t('taskControl.skipConfirmMessage'),
    warning: t('taskControl.skipWarningClean'),
    confirmText: t('taskControl.confirmSkip'),
    cancelText: t('common.cancel'),
    type: 'warning'
  })

  if (confirmed) {
    try {
      await invoke('skip_current_task')
      toast.info(t('taskControl.taskSkipped'))
    } catch (error) {
      logError(`Failed to skip task: ${error}`, 'installation')
      modal.showError(getErrorMessage(error))
    }
  }
}

// Handle cancel all tasks
async function handleCancelInstallation() {
  const confirmed = await showConfirmDialog({
    title: t('taskControl.cancelConfirmTitle'),
    message: t('taskControl.cancelConfirmMessage'),
    warning: t('taskControl.cancelWarningClean'),
    confirmText: t('taskControl.confirmCancel'),
    cancelText: t('common.cancel'),
    type: 'danger'
  })

  if (confirmed) {
    try {
      await invoke('cancel_installation')
      toast.info(t('taskControl.tasksCancelled'))
    } catch (error) {
      logError(`Failed to cancel installation: ${error}`, 'installation')
      modal.showError(getErrorMessage(error))
    }
  }
}

// Show confirm dialog helper
function showConfirmDialog(options: {
  title: string
  message: string
  warning?: string
  confirmText: string
  cancelText: string
  type: 'warning' | 'danger'
}): Promise<boolean> {
  return new Promise((resolve) => {
    modal.showConfirm({
      title: options.title,
      message: options.message,
      warning: options.warning,
      confirmText: options.confirmText,
      cancelText: options.cancelText,
      type: options.type,
      onConfirm: () => resolve(true),
      onCancel: () => resolve(false)
    })
  })
}

function handleCompletionConfirm() {
  store.clearInstallResult()
  store.clearTasks()
}
</script>

<style scoped>
/* debug drop visual */
.debug-drop {
  border-color: #10b981 !important; /* emerald */
  box-shadow: 0 0 30px rgba(16,185,129,0.15) !important;
}

/* Animations */
@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes bounce-in {
  0% {
    opacity: 0;
    transform: scale(0.3);
  }
  50% {
    opacity: 1;
    transform: scale(1.05);
  }
  70% {
    transform: scale(0.9);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

.animate-fade-in {
  animation: fade-in 0.6s ease-out;
}

.animate-bounce-in {
  animation: bounce-in 0.8s ease-out;
}

/* Drop zone styles */
.drop-zone-card {
  position: relative;
  overflow: hidden;
}

.drop-zone-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(59, 130, 246, 0.1), transparent);
  transition: left 0.6s;
}

.drop-zone-card:hover::before {
  left: 100%;
}

.drop-zone-card.drag-over {
  border-color: #3b82f6;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(147, 51, 234, 0.1));
  transform: scale(1.02);
  box-shadow: 0 0 40px rgba(59, 130, 246, 0.3);
}

.icon-container {
  filter: drop-shadow(0 0 10px rgba(59, 130, 246, 0.3));
}

/* Progress cards */
.progress-card {
  backdrop-filter: blur(10px);
}

/* Smooth progress animations */
.progress-container {
  /* Add padding to prevent clipping of drop-shadow */
  padding: 10px;
  margin: -10px;
  overflow: visible;
}

.progress-circle {
  transition: stroke-dashoffset 50ms cubic-bezier(0.4, 0, 0.2, 1);
  will-change: stroke-dashoffset;
}

.progress-bar {
  transition: width 50ms cubic-bezier(0.4, 0, 0.2, 1);
  will-change: width;
}

.progress-bar-glow {
  transition: width 50ms cubic-bezier(0.4, 0, 0.2, 1);
  will-change: width;
  background: theme('colors.emerald.500');
  filter: blur(6px);
  opacity: 0.7;
}

.dark .progress-bar-glow {
  background: theme('colors.emerald.400');
  opacity: 0.8;
}

.progress-text {
  transition: opacity 100ms ease-out;
  will-change: opacity;
}

/* Optimized pulse animation - uses opacity instead of drop-shadow to avoid overflow clipping */
@keyframes progress-pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.6;
  }
}

.progress-circle {
  animation: progress-pulse 2s ease-in-out infinite;
}

/* Completion Animation Styles */
@keyframes scale-in {
  0% {
    opacity: 0;
    transform: scale(0);
  }
  50% {
    transform: scale(1.1);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes scale-in-shrink {
  0% {
    opacity: 0;
    transform: scale(0);
  }
  30% {
    transform: scale(1.1);
  }
  60% {
    opacity: 1;
    transform: scale(1);
  }
  100% {
    opacity: 1;
    transform: scale(0.67);
  }
}

@keyframes ripple {
  0% {
    transform: scale(1);
    opacity: 0.5;
  }
  100% {
    transform: scale(1.5);
    opacity: 0;
  }
}

@keyframes check-draw {
  0% {
    stroke-dashoffset: 100;
  }
  100% {
    stroke-dashoffset: 0;
  }
}

@keyframes icon-shrink {
  0% {
    transform: scale(1);
  }
  60% {
    transform: scale(1);
  }
  100% {
    transform: scale(1);
  }
}

.animate-scale-in {
  animation: scale-in 0.5s cubic-bezier(0.68, -0.55, 0.265, 1.55);
}

.animate-scale-in-shrink {
  animation: scale-in-shrink 1.8s cubic-bezier(0.68, -0.55, 0.265, 1.55) forwards;
}

.animate-check-draw .check-path {
  stroke-dasharray: 100;
  stroke-dashoffset: 100;
  animation: check-draw 0.6s ease-in-out 0.3s forwards;
}

.animate-icon-shrink {
  animation: icon-shrink 1.8s cubic-bezier(0.68, -0.55, 0.265, 1.55) forwards;
}

.animate-ripple {
  animation: ripple 0.8s ease-out 0.2s;
}

/* Fade out transition for completion animation overlay */
.fade-out-enter-active,
.fade-out-leave-active {
  transition: opacity 0.2s ease-out;
}

.fade-out-enter-from {
  opacity: 0;
}

.fade-out-leave-to {
  opacity: 0;
}

/* Slow fade-in transition for completion view */
.fade-in-slow-enter-active {
  transition: opacity 0.8s ease-out;
}

.fade-in-slow-leave-active {
  transition: opacity 0.5s ease-out;
}

.fade-in-slow-enter-from,
.fade-in-slow-leave-to {
  opacity: 0;
}
</style>
