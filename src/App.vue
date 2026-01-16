<!-- Main App Component -->
<template>
  <div class="app-container transition-colors duration-300 bg-slate-50 dark:bg-gradient-to-br dark:from-gray-900 dark:via-slate-900 dark:to-gray-900 text-gray-900 dark:text-gray-100 font-sans selection:bg-blue-500/30">
    
    <!-- Navbar -->
    <nav class="fixed top-0 left-0 w-full z-50 transition-all duration-300">
      <div class="absolute inset-0 bg-white/70 dark:bg-gray-900/70 backdrop-blur-xl border-b border-gray-200/50 dark:border-white/5 shadow-sm dark:shadow-2xl transition-colors duration-300"></div>
      
      <div class="relative container mx-auto px-6 h-16 flex justify-between items-center">
        <!-- Logo -->
        <div class="flex items-center space-x-3 group cursor-default">
          <div class="relative w-8 h-8 flex items-center justify-center">
            <div class="absolute inset-0 bg-blue-500 rounded-lg blur opacity-40 group-hover:opacity-60 transition-opacity"></div>
            <img
              src="/icon.png"
              alt="XFastInstall"
              class="relative w-full h-full rounded-lg shadow-lg transform group-hover:scale-110 transition-transform duration-500"
            />
          </div>
          <h1 class="text-lg font-bold tracking-wide">
            <span class="text-gray-900 dark:text-white transition-colors">XFast</span><span class="text-blue-600 dark:text-blue-400 transition-colors">Install</span>
          </h1>
        </div>

        <!-- Navigation -->
        <div class="flex items-center space-x-1">
          <router-link
            to="/"
            class="relative px-4 py-2 rounded-lg group overflow-hidden transition-all duration-300"
            :class="$route.path === '/' ? 'text-blue-600 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'"
          >
            <div 
              class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
              :class="$route.path === '/' ? 'scale-x-100 opacity-100' : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'"
            ></div>
            <span class="relative flex items-center space-x-2 font-medium z-10">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"></path>
              </svg>
              <AnimatedText>{{ $t('common.home') }}</AnimatedText>
            </span>
          </router-link>
          
          <router-link
            to="/settings"
            class="relative px-4 py-2 rounded-lg group overflow-hidden transition-all duration-300"
            :class="$route.path === '/settings' ? 'text-blue-600 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'"
          >
            <div 
              class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
              :class="$route.path === '/settings' ? 'scale-x-100 opacity-100' : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'"
            ></div>
            <span class="relative flex items-center space-x-2 font-medium z-10">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
              </svg>
              <AnimatedText>{{ $t('common.settings') }}</AnimatedText>
            </span>
          </router-link>

          <div class="h-6 w-px bg-gray-200 dark:bg-white/10 mx-2 transition-colors"></div>
          
          <div class="flex items-center space-x-1">
             <ThemeSwitcher />
             <div class="px-2">
                <LanguageSwitcher />
             </div>
          </div>
        </div>
      </div>
    </nav>

    <!-- Main Content -->
    <main :class="['main-content', 'pt-16', 'flex-1', 'min-h-0', 'overflow-hidden', { 'hide-scrollbar': $route.path === '/' }]">
      <div class="h-full overflow-y-auto">
        <router-view v-slot="{ Component }">
          <transition name="page" mode="out-in">
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
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { listen } from '@tauri-apps/api/event'
import { syncLocaleToBackend } from '@/i18n'
import { logBasic, logDebug } from '@/services/logger'
import ToastNotification from '@/components/ToastNotification.vue'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'
import ThemeSwitcher from '@/components/ThemeSwitcher.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import ErrorModal from '@/components/ErrorModal.vue'
import ConfirmModal from '@/components/ConfirmModal.vue'

const { t } = useI18n()
const store = useAppStore()
const router = useRouter()

onMounted(async () => {
  // Log app startup (basic level - always logged)
  logBasic(t('log.appStarted'), 'app')
  logDebug('Loading app store and initializing', 'app')

  store.loadXplanePath()

  logDebug(`X-Plane path loaded: ${store.xplanePath || '(not set)'}`, 'app')
  logDebug(`Log level: ${store.logLevel}`, 'app')

  // Non-blocking sync locale to backend (moved from i18n module top-level)
  syncLocaleToBackend()

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
      console.log('CLI args event received:', event.payload)
      logBasic(t('log.launchedWithArgs'), 'app')
      logDebug(`CLI args: ${event.payload.join(', ')}`, 'app')
      if (event.payload && event.payload.length > 0) {
        // Use batch processing to handle multiple file selections
        // (Windows launches separate instances for each file)
        store.addCliArgsToBatch(event.payload)
        await router.push('/')
      }
    })
  } catch (error) {
    console.error('Failed to setup CLI args listener:', error)
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

/* Page transitions */
.page-enter-active,
.page-leave-active {
  transition: all 0.2s ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateX(15px);
}

.page-leave-to {
  opacity: 0;
  transform: translateX(-15px);
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