<!-- Main App Component -->
<template>
  <div
    class="app-container transition-colors duration-300 text-gray-900 dark:text-gray-100 font-sans selection:bg-blue-500/30"
  >
    <!-- Navbar -->
    <nav class="fixed top-0 left-0 w-full z-50 transition-all duration-300">
      <div
        class="absolute inset-0 bg-white/70 dark:bg-gray-900/70 backdrop-blur-xl border-b border-gray-200/50 dark:border-white/5 shadow-sm dark:shadow-2xl transition-colors duration-300"
      ></div>

      <div class="relative container mx-auto px-6 h-12 flex justify-between items-center">
        <!-- Logo -->
        <div class="flex items-center space-x-3 group cursor-default">
          <h1 class="text-lg font-bold tracking-wide">
            <span class="text-gray-900 dark:text-white transition-colors">XFast</span
            ><span class="text-blue-600 dark:text-blue-400 transition-colors">Manager</span>
          </h1>
        </div>

        <!-- Navigation -->
        <div class="flex items-center space-x-2">
          <div v-if="!isOnboardingRoute" class="flex items-center space-x-1">
            <router-link
              to="/"
              class="relative px-3 py-2 rounded-lg group overflow-hidden transition-all duration-300"
              :class="
                $route.path === '/'
                  ? 'text-blue-600 dark:text-white'
                  : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
              "
            >
              <div
                class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                :class="
                  $route.path === '/'
                    ? 'scale-x-100 opacity-100'
                    : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'
                "
              ></div>
              <span class="relative flex items-center space-x-1.5 text-sm font-medium z-10">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                  ></path>
                </svg>
                <AnimatedText>{{ $t('common.home') }}</AnimatedText>
              </span>
            </router-link>

            <div class="relative flex items-center">
              <router-link
                to="/management"
                class="relative px-3 py-2 rounded-lg group overflow-hidden transition-all duration-300"
                :class="
                  $route.path.startsWith('/management')
                    ? 'text-blue-600 dark:text-white'
                    : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
                "
              >
                <div
                  class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                  :class="
                    $route.path.startsWith('/management')
                      ? 'scale-x-100 opacity-100'
                      : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'
                  "
                ></div>
                <span class="relative flex items-center space-x-1.5 text-sm font-medium z-10">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                    ></path>
                  </svg>
                  <AnimatedText>{{ $t('management.navTitle') }}</AnimatedText>
                </span>
              </router-link>
              <div
                v-if="store.sceneryManagerHintVisible && store.sceneryManagerHintMessageKey"
                class="absolute left-1/2 top-full -translate-x-1/2 mt-2 z-50"
              >
                <div
                  class="relative min-w-[240px] max-w-[340px] w-max bg-cyan-50 dark:bg-cyan-900/60 border border-cyan-200 dark:border-cyan-700 text-cyan-900 dark:text-cyan-100 text-xs px-3 py-2 rounded-lg shadow-lg flex items-start gap-2"
                >
                  <div
                    class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-cyan-50 dark:bg-cyan-900/60 border-l border-t border-cyan-200 dark:border-cyan-700 rotate-45"
                  ></div>
                  <span class="leading-4">{{ $t(store.sceneryManagerHintMessageKey) }}</span>
                  <button
                    class="ml-1 text-cyan-700/80 dark:text-cyan-200/80 hover:text-cyan-900 dark:hover:text-white"
                    @click="store.dismissSceneryManagerHint()"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                      />
                    </svg>
                  </button>
                </div>
              </div>
            </div>

            <router-link
              to="/settings"
              class="relative p-2 rounded-lg group overflow-hidden transition-all duration-300"
              :class="
                $route.path === '/settings'
                  ? 'text-blue-600 dark:text-white'
                  : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
              "
              :title="$t('common.settings')"
            >
              <div
                class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                :class="
                  $route.path === '/settings'
                    ? 'scale-x-100 opacity-100'
                    : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'
                "
              ></div>
              <span class="relative flex items-center z-10">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                  ></path>
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                  ></path>
                </svg>
              </span>
            </router-link>
          </div>

          <div class="h-6 w-px bg-gray-200 dark:bg-white/10 transition-colors"></div>

          <div class="flex items-center space-x-1">
            <!-- Sponsor button (Chinese locale only) -->
            <button
              v-if="locale === 'zh'"
              class="relative p-2 rounded-lg group overflow-hidden transition-all duration-300 text-gray-600 dark:text-gray-400 hover:text-pink-500 dark:hover:text-pink-400"
              :title="$t('sponsor.title')"
              @click="showSponsor = true"
            >
              <div
                class="absolute inset-0 bg-pink-50 dark:bg-pink-500/10 rounded-lg transition-all duration-300 transform origin-center scale-0 opacity-0 group-hover:scale-100 group-hover:opacity-50"
              ></div>
              <span class="relative flex items-center z-10">
                <svg class="w-4 h-4 sponsor-heartbeat" fill="currentColor" viewBox="0 0 24 24">
                  <path
                    d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"
                  />
                </svg>
              </span>
            </button>
            <!-- Always on top button -->
            <button
              class="relative p-2 rounded-lg group overflow-hidden transition-all duration-300"
              :class="
                isAlwaysOnTop
                  ? 'text-blue-600 dark:text-blue-400'
                  : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
              "
              :title="isAlwaysOnTop ? 'Unpin window' : 'Pin window on top'"
              @click="toggleAlwaysOnTop"
            >
              <div
                class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-center"
                :class="
                  isAlwaysOnTop
                    ? 'scale-100 opacity-100'
                    : 'scale-0 opacity-0 group-hover:scale-100 group-hover:opacity-50'
                "
              ></div>
              <span class="relative flex items-center z-10">
                <!-- Pin icon -->
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  stroke-width="2"
                >
                  <path
                    v-if="isAlwaysOnTop"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
                    fill="currentColor"
                  />
                  <path
                    v-else
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
                  />
                </svg>
              </span>
            </button>
            <ThemeSwitcher />
            <LanguageSwitcher />
          </div>
        </div>
      </div>
    </nav>

    <!-- Main Content -->
    <main
      :class="[
        'main-content',
        'pt-12',
        'flex-1',
        'min-h-0',
        'overflow-hidden',
        { 'hide-scrollbar': $route.path === '/' },
      ]"
    >
      <div class="h-full overflow-y-auto">
        <router-view v-slot="{ Component }">
          <transition :name="transitionName" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </div>
    </main>

    <!-- Global Components -->
    <ToastNotification />
    <ErrorModal />
    <ConfirmModal />
    <ContextMenu />
    <SponsorModal :show="showSponsor" @close="showSponsor = false" />
    <IssueUpdateModal />
  </div>
</template>

<script setup lang="ts">
import { onMounted, computed, ref, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { useUpdateStore } from '@/stores/update'
import { useSceneryStore } from '@/stores/scenery'
import { useModalStore } from '@/stores/modal'
import { useIssueTrackerStore } from '@/stores/issueTracker'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { syncLocaleToBackend } from '@/i18n'
import { logBasic, logDebug, logError } from '@/services/logger'
import ToastNotification from '@/components/ToastNotification.vue'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'
import ThemeSwitcher from '@/components/ThemeSwitcher.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import ErrorModal from '@/components/ErrorModal.vue'
import ConfirmModal from '@/components/ConfirmModal.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import SponsorModal from '@/components/SponsorModal.vue'
import IssueUpdateModal from '@/components/IssueUpdateModal.vue'

const { t, locale } = useI18n()
const store = useAppStore()
const updateStore = useUpdateStore()
const sceneryStore = useSceneryStore()
const modalStore = useModalStore()
const issueTrackerStore = useIssueTrackerStore()
const router = useRouter()
const route = useRoute()
const isOnboardingRoute = computed(() => route.path === '/onboarding')

// Always on top state
const isAlwaysOnTop = ref(false)
const showSponsor = ref(false)

async function toggleAlwaysOnTop() {
  try {
    const appWindow = getCurrentWindow()
    isAlwaysOnTop.value = !isAlwaysOnTop.value
    await appWindow.setAlwaysOnTop(isAlwaysOnTop.value)
  } catch (error) {
    logError(`Failed to toggle always on top: ${error}`, 'app')
  }
}

// Route order for determining transition direction
const routeOrder: Record<string, number> = {
  '/': 0,
  '/management': 1,
  '/management/liveries': 1,
  '/settings': 2,
  '/onboarding': -1,
}

// Track transition direction based on route navigation
const transitionName = ref('page-right')

// Watch route changes to determine transition direction
watch(
  () => route.path,
  (newPath, oldPath) => {
    const newOrder = routeOrder[newPath] ?? 0
    const oldOrder = routeOrder[oldPath] ?? 0
    // Going to higher index (right in nav) = slide left, going to lower index = slide right
    transitionName.value = newOrder > oldOrder ? 'page-left' : 'page-right'
  },
)

onMounted(async () => {
  // Log app startup (basic level - always logged)
  logBasic(t('log.appStarted'), 'app')
  logDebug('Loading app store and initializing', 'app')

  // Non-blocking: load X-Plane path
  store
    .loadXplanePath()
    .then(() => {
      logDebug(`X-Plane path loaded: ${store.xplanePath || '(not set)'}`, 'app')
    })
    .catch((e) => {
      logError(`Failed to load X-Plane path: ${e}`, 'app')
    })

  logDebug(`Log level: ${store.logLevel}`, 'app')

  // Detect platform and context menu status at startup (once)
  try {
    logDebug('Detecting platform...', 'app')
    const platform = await invoke<string>('get_platform')
    store.isWindows = platform === 'windows'
    logDebug(`Platform detected: ${platform}`, 'app')

    // Check context menu registration status (Windows only)
    if (store.isWindows) {
      store.isContextMenuRegistered = await invoke<boolean>('is_context_menu_registered')
      logDebug(`Context menu registered: ${store.isContextMenuRegistered}`, 'app')

      // Sync context menu paths if registered (handles exe relocation)
      if (store.isContextMenuRegistered) {
        try {
          const updated = await invoke<boolean>('sync_context_menu_paths')
          if (updated) {
            logDebug('Context menu paths synced to current location', 'app')
          }
        } catch (error) {
          logError(`Failed to sync context menu paths: ${error}`, 'app')
        }
      }
    }
  } catch (error) {
    logError(`Failed to detect platform: ${error}`, 'app')
  }

  // Non-blocking sync locale to backend (moved from i18n module top-level)
  syncLocaleToBackend()

  // Check for updates (non-blocking, delayed to avoid affecting startup performance)
  setTimeout(() => {
    if (updateStore.autoCheckEnabled) {
      logDebug('Auto-checking for updates...', 'app')
      updateStore.checkForUpdates(false)
    }
  }, 3000) // 3 second delay

  // Check tracked issue updates in the background
  setTimeout(() => {
    issueTrackerStore.checkAllTrackedIssues()
  }, 5000) // 5 second delay

  // Disable context menu and devtools shortcuts in production
  if (import.meta.env.MODE === 'production') {
    // Disable right-click context menu
    document.addEventListener('contextmenu', (e) => {
      e.preventDefault()
      return false
    })

    // Disable F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+U (devtools shortcuts)
    document.addEventListener('keydown', (e) => {
      // F12
      if (e.key === 'F12') {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+I (Inspector)
      if (e.ctrlKey && e.shiftKey && e.key === 'I') {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+J (Console)
      if (e.ctrlKey && e.shiftKey && e.key === 'J') {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+C (Element picker)
      if (e.ctrlKey && e.shiftKey && e.key === 'C') {
        e.preventDefault()
        return false
      }
      // Ctrl+U (View source)
      if (e.ctrlKey && e.key === 'u') {
        e.preventDefault()
        return false
      }
    })
  }

  // Listen for cli-args events from Rust (emitted by single-instance plugin for subsequent launches)
  try {
    logDebug('Setting up CLI args listener...', 'app')
    await listen<string[]>('cli-args', async (event) => {
      logDebug(`CLI args event received: ${event.payload.join(', ')}`, 'app')
      logBasic(t('log.launchedWithArgs'), 'app')
      if (event.payload && event.payload.length > 0) {
        // Use batch processing to handle multiple file selections
        // (Windows launches separate instances for each file)
        store.addCliArgsToBatch(event.payload)
        await router.push('/')
      }
    })
  } catch (error) {
    logError(`Failed to setup CLI args listener: ${error}`, 'app')
  }

  // On first launch, the cli-args event from setup() fires before this listener is ready,
  // so we also poll for CLI args to handle the cold-start case
  try {
    logDebug('Getting CLI args from first launch...', 'app')
    const args = await invoke<string[]>('get_cli_args')
    if (args && args.length > 0) {
      logDebug(`CLI args from first launch: ${args.join(', ')}`, 'app')
      logBasic(t('log.launchedWithArgs'), 'app')
      store.addCliArgsToBatch(args)
      await router.push('/')
    }
  } catch (error) {
    logError(`Failed to get CLI args on startup: ${error}`, 'app')
  }

  // Set up window close confirmation for active operations
  try {
    logDebug('Setting up window close handler...', 'app')
    const appWindow = getCurrentWindow()
    await appWindow.onCloseRequested(async (event) => {
      // Check conditions in priority order
      if (store.isInstalling) {
        // Installation in progress - highest priority warning
        event.preventDefault()
        modalStore.showConfirm({
          title: t('modal.installInProgressTitle'),
          message: t('modal.installInProgressMessage'),
          warning: t('modal.installInProgressWarning'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'danger',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      } else if (store.isLibraryLinkSubmitting) {
        // Library link submission in progress
        event.preventDefault()
        modalStore.showError(t('sceneryManager.submissionInProgressCloseBlocked'))
      } else if (store.isConfirmationOpen) {
        // Confirmation modal is open - pending installation
        event.preventDefault()
        modalStore.showConfirm({
          title: t('modal.confirmationOpenTitle'),
          message: t('modal.confirmationOpenMessage'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'warning',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      } else if (sceneryStore.hasChanges) {
        // Unsaved scenery changes
        event.preventDefault()
        modalStore.showConfirm({
          title: t('modal.unsavedSceneryChangesTitle'),
          message: t('modal.unsavedSceneryChangesMessage'),
          warning: t('modal.unsavedSceneryChangesWarning'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'warning',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      }
      // If no conditions match, allow the window to close normally
    })
  } catch (error) {
    logError(`Failed to setup window close listener: ${error}`, 'app')
  }

  logDebug('App.vue onMounted completed', 'app')
})
</script>

<style scoped>
.app-container {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: linear-gradient(135deg, var(--app-bg-from), var(--app-bg-via), var(--app-bg-to));
  background-color: var(--app-bg-from);
}

nav {
  transform: translateZ(0);
  will-change: transform;
  backface-visibility: hidden;
}

.main-content {
  flex: 1;
  min-height: 0;
  /* Do not reserve scrollbar gutter globally; we will control visual scrollbar per-route */
  scrollbar-gutter: auto;
  /* Allow inner container to manage the actual scrolling */
}

/* Ensure the immediate child scroll container always shows a vertical scrollbar area
   so the scrollbar won't appear/disappear during route transitions. */
.main-content > div {
  height: 100%;
  overflow-y: auto;
}

/* Hide visual scrollbar for Home page while keeping scroll functionality */
.hide-scrollbar > div {
  overflow-y: auto;
  /* Firefox */
  scrollbar-width: none;
}
.hide-scrollbar > div::-webkit-scrollbar {
  width: 0;
  height: 0;
  display: none;
}

/* When .no-scrollbar is applied (Settings route) completely disable scrolling
   and remove any reserved scrollbar gutter so there's no scrollbar area on all platforms. */
.no-scrollbar {
  /* Reset any reserved gutter from the global setting */
  scrollbar-gutter: auto;
}
.no-scrollbar > div {
  /* Completely disable scrolling and hide scrollbars visually */
  overflow: hidden !important;
  /* Firefox */
  scrollbar-width: none;
}
.no-scrollbar > div::-webkit-scrollbar {
  /* Chromium-based */
  width: 0;
  height: 0;
  display: none;
}

/* Page transitions - left direction (going to higher index route) */
.page-left-enter-active,
.page-left-leave-active,
.page-right-enter-active,
.page-right-leave-active {
  transition: all 0.2s ease;
  will-change: transform, opacity;
  backface-visibility: hidden;
}

/* Going left (e.g., Home -> Management -> Settings) */
.page-left-enter-from {
  opacity: 0;
  transform: translateX(15px);
}

.page-left-leave-to {
  opacity: 0;
  transform: translateX(-15px);
}

/* Going right (e.g., Settings -> Management -> Home) */
.page-right-enter-from {
  opacity: 0;
  transform: translateX(-15px);
}

.page-right-leave-to {
  opacity: 0;
  transform: translateX(15px);
}

/* Navigation animations */
.nav-link {
  position: relative;
  overflow: hidden;
}

.nav-link::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
  transition: left 0.5s;
}

.nav-link:hover::before {
  left: 100%;
}

/* Sponsor heart: intermittent red pulse every 4s */
@keyframes heartbeat {
  0%,
  100% {
    transform: scale(1);
    color: inherit;
  }
  6% {
    transform: scale(1.25);
    color: #ef4444;
  }
  12% {
    transform: scale(1);
    color: #ef4444;
  }
  16% {
    transform: scale(1.2);
    color: #ef4444;
  }
  22% {
    transform: scale(1);
    color: inherit;
  }
}

.sponsor-heartbeat {
  animation: heartbeat 10s ease-in-out infinite;
}
</style>
