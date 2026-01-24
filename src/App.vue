<!-- Main App Component -->
<template>
  <div class="app-container transition-colors duration-300 text-gray-900 dark:text-gray-100 font-sans selection:bg-blue-500/30">
    
    <!-- Navbar -->
    <nav class="fixed top-0 left-0 w-full z-50 transition-all duration-300">
      <div class="absolute inset-0 bg-white/70 dark:bg-gray-900/70 backdrop-blur-xl border-b border-gray-200/50 dark:border-white/5 shadow-sm dark:shadow-2xl transition-colors duration-300"></div>
      
      <div class="relative container mx-auto px-6 h-12 flex justify-between items-center">
        <!-- Logo -->
        <div class="flex items-center space-x-3 group cursor-default">
          <h1 class="text-lg font-bold tracking-wide">
            <span class="text-gray-900 dark:text-white transition-colors">XFast</span><span class="text-blue-600 dark:text-blue-400 transition-colors">Manager</span>
          </h1>
        </div>

        <!-- Navigation -->
        <div class="flex items-center space-x-2">
          <div v-if="!isOnboardingRoute" class="flex items-center space-x-1">
          <router-link
            to="/"
            class="relative px-3 py-2 rounded-lg group overflow-hidden transition-all duration-300"
            :class="$route.path === '/' ? 'text-blue-600 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'"
          >
            <div
              class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
              :class="$route.path === '/' ? 'scale-x-100 opacity-100' : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'"
            ></div>
            <span class="relative flex items-center space-x-1.5 text-sm font-medium z-10">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
              </svg>
              <AnimatedText>{{ $t('common.home') }}</AnimatedText>
            </span>
          </router-link>

          <div class="relative flex items-center">
            <router-link
              to="/management"
              class="relative px-3 py-2 rounded-lg group overflow-hidden transition-all duration-300"
              :class="$route.path === '/management' ? 'text-blue-600 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'"
            >
              <div
                class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                :class="$route.path === '/management' ? 'scale-x-100 opacity-100' : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'"
              ></div>
              <span class="relative flex items-center space-x-1.5 text-sm font-medium z-10">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
                </svg>
                <AnimatedText>{{ $t('management.navTitle') }}</AnimatedText>
              </span>
            </router-link>
            <div
              v-if="store.sceneryManagerHintVisible && store.sceneryManagerHintMessageKey"
              class="absolute left-1/2 top-full -translate-x-1/2 mt-2 z-50"
            >
              <div class="relative min-w-[240px] max-w-[340px] w-max bg-cyan-50 dark:bg-cyan-900/60 border border-cyan-200 dark:border-cyan-700 text-cyan-900 dark:text-cyan-100 text-xs px-3 py-2 rounded-lg shadow-lg flex items-start gap-2">
                <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-cyan-50 dark:bg-cyan-900/60 border-l border-t border-cyan-200 dark:border-cyan-700 rotate-45"></div>
                <span class="leading-4">{{ $t(store.sceneryManagerHintMessageKey) }}</span>
                <button
                  class="ml-1 text-cyan-700/80 dark:text-cyan-200/80 hover:text-cyan-900 dark:hover:text-white"
                  @click="store.dismissSceneryManagerHint()"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>
          </div>

          <router-link
            to="/settings"
            class="relative p-2 rounded-lg group overflow-hidden transition-all duration-300"
            :class="$route.path === '/settings' ? 'text-blue-600 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'"
            :title="$t('common.settings')"
          >
            <div
              class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
              :class="$route.path === '/settings' ? 'scale-x-100 opacity-100' : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'"
            ></div>
            <span class="relative flex items-center z-10">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
              </svg>
            </span>
          </router-link>
        </div>

        <div class="h-6 w-px bg-gray-200 dark:bg-white/10 transition-colors"></div>

        <div class="flex items-center space-x-1">
          <ThemeSwitcher />
          <LanguageSwitcher />
        </div>
      </div>
      </div>
    </nav>

    <!-- Main Content -->
    <main :class="['main-content', 'pt-12', 'flex-1', 'min-h-0', 'overflow-hidden', { 'hide-scrollbar': $route.path === '/' }]">
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
import type { SceneryIndexScanResult } from '@/types'

const { t } = useI18n()
const store = useAppStore()
const updateStore = useUpdateStore()
const sceneryStore = useSceneryStore()
const modalStore = useModalStore()
const router = useRouter()
const route = useRoute()
const isOnboardingRoute = computed(() => route.path === '/onboarding')

// Route order for determining transition direction
const routeOrder: Record<string, number> = {
  '/': 0,
  '/management': 1,
  '/settings': 2,
  '/onboarding': -1
}

// Track transition direction based on route navigation
const transitionName = ref('page-right')

// Watch route changes to determine transition direction
watch(() => route.path, (newPath, oldPath) => {
  const newOrder = routeOrder[newPath] ?? 0
  const oldOrder = routeOrder[oldPath] ?? 0
  // Going to higher index (right in nav) = slide left, going to lower index = slide right
  transitionName.value = newOrder > oldOrder ? 'page-left' : 'page-right'
})

async function runSceneryIndexStartupScan() {
  if (!store.xplanePath) return

  try {
    const result = await invoke<SceneryIndexScanResult>('quick_scan_scenery_index', {
      xplanePath: store.xplanePath
    })

    if (!result.indexExists) return

    const hasChanges = result.added + result.removed + result.updated > 0
    if (hasChanges && store.autoSortScenery) {
      store.showSceneryManagerHint('sceneryManager.hintFromScan')
    }
  } catch (error) {
    logError(`Failed to quick scan scenery index: ${error}`, 'app')
  }
}

onMounted(async () => {
  // Log app startup (basic level - always logged)
  logBasic(t('log.appStarted'), 'app')
  logDebug('Loading app store and initializing', 'app')

  store.loadXplanePath()

  logDebug(`X-Plane path loaded: ${store.xplanePath || '(not set)'}`, 'app')
  logDebug(`Log level: ${store.logLevel}`, 'app')

  // Detect platform and context menu status at startup (once)
  try {
    const platform = await invoke<string>('get_platform')
    store.isWindows = platform === 'windows'
    logDebug(`Platform detected: ${platform}`, 'app')

    // Check context menu registration status (Windows only)
    if (store.isWindows) {
      store.isContextMenuRegistered = await invoke<boolean>('is_context_menu_registered')
      logDebug(`Context menu registered: ${store.isContextMenuRegistered}`, 'app')
    }
  } catch (error) {
    logError(`Failed to detect platform: ${error}`, 'app')
  }

  runSceneryIndexStartupScan()

  // Non-blocking sync locale to backend (moved from i18n module top-level)
  syncLocaleToBackend()

  // Check for updates (non-blocking, delayed to avoid affecting startup performance)
  setTimeout(() => {
    if (updateStore.autoCheckEnabled) {
      logDebug('Auto-checking for updates...', 'app')
      updateStore.checkForUpdates(false)
    }
  }, 3000) // 3 second delay

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

  // Listen for cli-args events from Rust (emitted during setup)
  // Removed invoke('get_cli_args') to avoid duplicate calls and improve startup speed
  try {
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

  // Set up window close confirmation for unsaved scenery changes
  try {
    const appWindow = getCurrentWindow()
    await appWindow.onCloseRequested(async (event) => {
      // Check if there are unsaved scenery changes
      if (sceneryStore.hasChanges) {
        // Prevent the window from closing
        event.preventDefault()

        // Show confirmation modal
        modalStore.showConfirm({
          title: t('modal.unsavedSceneryChangesTitle'),
          message: t('modal.unsavedSceneryChangesMessage'),
          warning: t('modal.unsavedSceneryChangesWarning'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'warning',
          onConfirm: async () => {
            // User confirmed, close the window
            await appWindow.destroy()
          },
          onCancel: () => {
            // User cancelled, do nothing (window stays open)
          }
        })
      }
      // If no changes, allow the window to close normally
    })
  } catch (error) {
    logError(`Failed to setup window close listener: ${error}`, 'app')
  }
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
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
  transition: left 0.5s;
}

.nav-link:hover::before {
  left: 100%;
}

</style>
