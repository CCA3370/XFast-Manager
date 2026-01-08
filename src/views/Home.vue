<template>
  <div class="home-view h-full flex flex-col p-4 animate-fade-in relative overflow-hidden">
    <!-- Background Decor (Dark Mode Only for deep glow) -->
    <div class="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none z-0 opacity-0 dark:opacity-100 transition-opacity duration-500">
      <div class="absolute top-1/4 left-1/4 w-64 h-64 bg-blue-500/10 rounded-full blur-3xl"></div>
      <div class="absolute bottom-1/4 right-1/4 w-96 h-96 bg-purple-500/10 rounded-full blur-3xl"></div>
    </div>

    <div class="w-full max-w-3xl mx-auto z-10 flex flex-col flex-1 min-h-0 gap-3">
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
      <div class="relative group flex-1 min-h-0 flex flex-col">
        <!-- Drop Zone -->
        <div
          class="drop-zone-card flex-1 relative overflow-hidden bg-white/60 dark:bg-gray-800/40 backdrop-blur-xl border-2 border-dashed border-gray-300 dark:border-gray-600/50 rounded-2xl p-6 text-center transition-all duration-500 group-hover:border-blue-400 dark:group-hover:border-blue-500/50 group-hover:bg-white/80 dark:group-hover:bg-gray-800/60 shadow-sm dark:shadow-none flex flex-col items-center justify-center"
          :class="{ 
            'drag-over ring-4 ring-blue-500/20 border-blue-500 scale-[1.02]': isDragging, 
            'animate-pulse border-blue-400': store.isAnalyzing, 
            'debug-drop': debugDropFlash 
          }"
        >
          <!-- Hover Gradient -->
          <div class="absolute inset-0 bg-gradient-to-br from-blue-50/50 to-purple-50/50 dark:from-blue-600/5 dark:to-purple-600/5 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"></div>

          <div class="relative z-10 flex flex-col items-center justify-center space-y-4">
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
        </div>

        <!-- Progress Overlays -->
        <transition name="fade">
          <div v-if="store.isAnalyzing || store.isInstalling" class="absolute inset-0 z-20 bg-white/90 dark:bg-gray-900/80 backdrop-blur-md rounded-2xl flex items-center justify-center p-6 transition-colors duration-300">
            <div class="w-full max-w-md space-y-4 text-center">
              
              <!-- Analyzing State -->
              <div v-if="store.isAnalyzing" class="space-y-4">
                <div class="relative w-20 h-20 mx-auto">
                  <div class="absolute inset-0 border-4 border-blue-200 dark:border-blue-500/30 rounded-full"></div>
                  <div class="absolute inset-0 border-4 border-blue-500 rounded-full border-t-transparent animate-spin"></div>
                  <div class="absolute inset-0 flex items-center justify-center">
                    <svg class="w-8 h-8 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                    </svg>
                  </div>
                </div>
                <div>
                  <h3 class="text-2xl font-bold text-gray-900 dark:text-white"><AnimatedText>{{ $t('home.analyzing') }}</AnimatedText></h3>
                  <p class="text-gray-500 dark:text-gray-400 mt-2"><AnimatedText>{{ $t('home.pleaseWait') }}</AnimatedText></p>
                </div>
              </div>

              <!-- Installing State with Progress -->
              <div v-if="store.isInstalling" class="space-y-3">
                <!-- Circular Progress -->
                <div class="relative w-20 h-20 mx-auto">
                  <svg class="w-full h-full -rotate-90">
                    <circle cx="40" cy="40" r="36" stroke-width="5" fill="none"
                      class="text-emerald-500/20 dark:text-emerald-500/30" stroke="currentColor"/>
                    <circle cx="40" cy="40" r="36" stroke-width="5" fill="none"
                      class="text-emerald-500 dark:text-emerald-400" stroke="currentColor"
                      :stroke-dasharray="226"
                      :stroke-dashoffset="226 - 226 * (parseFloat(progressStore.formatted.percentage) / 100)"
                      stroke-linecap="round"
                      style="transition: stroke-dashoffset 0.1s linear"/>
                  </svg>
                  <span class="absolute inset-0 flex items-center justify-center text-lg font-bold text-emerald-600 dark:text-emerald-400">
                    {{ progressStore.formatted.percentage }}%
                  </span>
                </div>

                <!-- Task Info -->
                <div class="text-center">
                  <h3 class="text-xl font-bold text-gray-900 dark:text-white"><AnimatedText>{{ $t('home.installing') }}</AnimatedText></h3>
                  <p class="text-sm text-gray-600 dark:text-gray-300 mt-1">{{ progressStore.formatted.taskName }}</p>
                  <p class="text-xs text-gray-400 dark:text-gray-500 truncate max-w-xs mx-auto mt-0.5">{{ progressStore.formatted.currentFile }}</p>
                </div>

                <!-- Linear Progress Bar -->
                <div class="w-full max-w-xs mx-auto">
                  <div class="h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                    <div class="h-full bg-emerald-500 dark:bg-emerald-400 transition-all duration-100 ease-linear"
                      :style="{ width: progressStore.formatted.percentage + '%' }"/>
                  </div>
                  <div class="flex justify-between text-xs text-gray-400 dark:text-gray-500 mt-1">
                    <span>{{ progressStore.formatted.processedMB }} MB</span>
                    <span>{{ progressStore.formatted.totalMB }} MB</span>
                  </div>
                </div>

                <!-- Task Progress -->
                <p class="text-xs text-center text-gray-500 dark:text-gray-400">
                  {{ $t('home.taskProgress', { current: progressStore.formatted.taskProgress }) }}
                </p>
              </div>

            </div>
          </div>
        </transition>
      </div>

      <ConfirmationModal v-if="showConfirmation" @close="showConfirmation = false" @confirm="handleInstall" />
      <PasswordModal
        v-if="showPasswordModal"
        :archive-paths="passwordRequiredPaths"
        @confirm="handlePasswordSubmit"
        @cancel="handlePasswordCancel"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useProgressStore } from '@/stores/progress'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import ConfirmationModal from '@/components/ConfirmationModal.vue'
import PasswordModal from '@/components/PasswordModal.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import type { AnalysisResult, InstallProgress } from '@/types'
import { logOperation, logError, logDebug, logBasic } from '@/services/logger'

const { t } = useI18n()

const store = useAppStore()
const toast = useToastStore()
const modal = useModalStore()
const progressStore = useProgressStore()
const isDragging = ref(false)
const showConfirmation = ref(false)
const debugDropFlash = ref(false)

// Password modal state
const showPasswordModal = ref(false)
const passwordRequiredPaths = ref<string[]>([])
const pendingAnalysisPaths = ref<string[]>([])
const collectedPasswords = ref<Record<string, string>>({})

// Tauri drag-drop event unsubscribe function
let unlistenDragDrop: UnlistenFn | null = null
let unlistenProgress: UnlistenFn | null = null
let unlistenCliArgs: UnlistenFn | null = null

// Global listeners for drag/drop visual feedback
function onWindowDragOver(e: DragEvent) {
  e.preventDefault()
  isDragging.value = true
}

function onWindowDragLeave(e: DragEvent) {
  // Only set to false if leaving the window
  if (!e.relatedTarget) {
    isDragging.value = false
  }
}

function onWindowDrop(e: DragEvent) {
  console.log('Window drop event (HTML5)', e)
  e.preventDefault()
  isDragging.value = false
  debugDropFlash.value = true
  setTimeout(() => (debugDropFlash.value = false), 800)
}

onMounted(async () => {
  window.addEventListener('dragover', onWindowDragOver)
  window.addEventListener('dragleave', onWindowDragLeave)
  window.addEventListener('drop', onWindowDrop)

  // Use Tauri 2's native drag-drop event for getting file paths
  try {
    const webview = getCurrentWebviewWindow()
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      console.log('Tauri drag-drop event:', event)

      if (event.payload.type === 'over') {
        isDragging.value = true
      } else if (event.payload.type === 'leave' || event.payload.type === 'cancel') {
        isDragging.value = false
      } else if (event.payload.type === 'drop') {
        isDragging.value = false
        debugDropFlash.value = true
        setTimeout(() => (debugDropFlash.value = false), 800)

        const paths = event.payload.paths
        console.log('Dropped paths from Tauri:', paths)

        if (paths && paths.length > 0) {
          await analyzeFiles(paths)
        }
      }
    })
    console.log('Tauri drag-drop listener registered')
  } catch (error) {
    console.error('Failed to setup Tauri drag-drop listener:', error)
  }

  // Listen for installation progress events
  try {
    unlistenProgress = await listen<InstallProgress>('install-progress', (event) => {
      progressStore.update(event.payload)
    })
    console.log('Progress listener registered')
  } catch (error) {
    console.error('Failed to setup progress listener:', error)
  }

  // Listen for CLI args events (when app receives new files while running)
  try {
    unlistenCliArgs = await listen<string[]>('cli-args', async (event) => {
      if (event.payload && event.payload.length > 0) {
        console.log('CLI args event in Home.vue:', event.payload)
        await analyzeFiles(event.payload)
      }
    })
    console.log('CLI args listener registered in Home.vue')
  } catch (error) {
    console.error('Failed to setup CLI args listener:', error)
  }

  // Check for pending CLI args (from context menu launch)
  if (store.pendingCliArgs && store.pendingCliArgs.length > 0) {
    console.log('Processing pending CLI args:', store.pendingCliArgs)
    const args = [...store.pendingCliArgs]
    store.clearPendingCliArgs()
    await analyzeFiles(args)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('dragover', onWindowDragOver)
  window.removeEventListener('dragleave', onWindowDragLeave)
  window.removeEventListener('drop', onWindowDrop)

  // Cleanup Tauri listeners
  if (unlistenDragDrop) {
    unlistenDragDrop()
  }
  if (unlistenProgress) {
    unlistenProgress()
  }
  if (unlistenCliArgs) {
    unlistenCliArgs()
  }
})

async function analyzeFiles(paths: string[], passwords?: Record<string, string>) {
  // Log incoming files
  logOperation(t('log.filesDropped'), t('log.fileCount', { count: paths.length }))
  logDebug(`Analyzing paths: ${paths.join(', ')}`, 'analysis')

  if (!store.xplanePath) {
    console.log('No X-Plane path set')
    // Log the abort reason - toast.warning will also log via the store
    logOperation(t('log.taskAborted'), t('log.xplanePathNotSet'))
    toast.warning(t('home.pathNotSet'))
    return
  }

  store.isAnalyzing = true
  logDebug(`Starting analysis with X-Plane path: ${store.xplanePath}`, 'analysis')

  try {
    console.log('Paths to analyze:', paths)

    const result = await invoke<AnalysisResult>('analyze_addons', {
      paths,
      xplanePath: store.xplanePath,
      passwords: passwords || null
    })

    console.log('Analysis result:', result)
    logDebug(`Analysis returned ${result.tasks.length} tasks, ${result.errors.length} errors`, 'analysis')

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
    console.error('Analysis failed:', error)
    // Non-blocking log call
    logError(`${t('log.analysisFailed')}: ${error}`, 'analysis')
    modal.showError(t('home.failedToAnalyze') + ': ' + String(error))
  } finally {
    store.isAnalyzing = false
  }
}

// Handle password modal submit
async function handlePasswordSubmit(passwords: Record<string, string>) {
  showPasswordModal.value = false
  logOperation(t('log.passwordEntered'), t('log.fileCount', { count: Object.keys(passwords).length }))
  // Merge new passwords with previously collected ones
  const allPasswords = { ...collectedPasswords.value, ...passwords }
  // Re-analyze with passwords
  await analyzeFiles(pendingAnalysisPaths.value, allPasswords)
}

// Handle password modal cancel
function handlePasswordCancel() {
  showPasswordModal.value = false
  logOperation(t('log.taskAborted'), t('log.passwordCanceled'))
  pendingAnalysisPaths.value = []
  passwordRequiredPaths.value = []
  collectedPasswords.value = {}
}

async function handleInstall() {
  showConfirmation.value = false
  store.isInstalling = true
  // Non-blocking log call
  logBasic(t('log.installationStarted'), 'installation')
  logOperation(t('log.installationStarted'), t('log.taskCount', { count: store.currentTasks.length }))
  logDebug(`Installing ${store.currentTasks.length} tasks: ${store.currentTasks.map(t => t.name).join(', ')}`, 'installation')

  try {
    // Use getTasksWithOverwrite() to include overwrite settings
    const tasksWithOverwrite = store.getTasksWithOverwrite()
    const overwriteCount = tasksWithOverwrite.filter(t => t.shouldOverwrite).length
    if (overwriteCount > 0) {
      logDebug(`${overwriteCount} tasks will overwrite existing files`, 'installation')
    }

    await invoke('install_addons', {
      tasks: tasksWithOverwrite
    })
    // Non-blocking log call
    logBasic(t('log.installationCompleted'), 'installation')
    logOperation(t('log.installationCompleted'))
    logDebug('All tasks installed successfully', 'installation')
    toast.success(t('home.installationCompleted'))
    store.clearTasks()
  } catch (error) {
    console.error('Installation failed:', error)
    // Non-blocking log call
    logError(`${t('log.installationFailed')}: ${error}`, 'installation')
    modal.showError(t('home.installationFailed') + ': ' + String(error))
  } finally {
    store.isInstalling = false
  }
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
</style>
