// Bootstrap logger must be imported FIRST - before any other imports
// This sets up global error handlers immediately
import {
  bootstrapInfo,
  bootstrapError,
  bootstrapDebug,
  formatError,
  markTauriReady,
  setupGlobalErrorHandlers,
} from './services/bootstrap-logger'

// Setup global error handlers as early as possible
setupGlobalErrorHandlers()
bootstrapInfo('Application starting...', 'init')

import { createApp, type App as VueApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import Home from './views/Home.vue'
import { i18n } from './i18n'
import './style.css'
import { initStorage, getItem, STORAGE_KEYS } from './services/storage'
import { useAppStore } from './stores/app'
import { useThemeStore } from './stores/theme'
import { useLockStore } from './stores/lock'
import { useUpdateStore } from './stores/update'
import { useSceneryStore } from './stores/scenery'

// ============================================================================
// Loading Screen Error Display
// ============================================================================

/**
 * Update loading screen to show error state
 */
function showLoadingError(message: string, details?: string): void {
  const loadingScreen = document.getElementById('loading-screen')
  if (!loadingScreen) return

  // Update loading text to show error
  const loadingText = loadingScreen.querySelector('.loading-text')
  if (loadingText) {
    loadingText.textContent = 'Initialization Failed'
    loadingText.classList.add('error-text')
  }

  // Stop the spinner animation
  const loader = loadingScreen.querySelector('.loader')
  if (loader) {
    loader.classList.add('error-state')
  }

  // Add error details container if not exists
  let errorContainer = loadingScreen.querySelector('.error-container') as HTMLElement
  if (!errorContainer) {
    errorContainer = document.createElement('div')
    errorContainer.className = 'error-container'
    loadingScreen.appendChild(errorContainer)
  }

  // Build error text for copying
  const errorText = details ? `${message}\n\n${details}` : message

  // Show error message
  errorContainer.innerHTML = `
    <div class="error-message">${escapeHtml(message)}</div>
    ${details ? `<div class="error-details">${escapeHtml(details)}</div>` : ''}
    <div class="error-hint">Check the log file for more details</div>
    <div class="error-buttons">
      <button class="copy-button" id="copy-error-btn">Copy Error</button>
      <button class="retry-button" onclick="location.reload()">Retry</button>
    </div>
  `

  // Setup copy button handler
  const copyBtn = document.getElementById('copy-error-btn')
  if (copyBtn) {
    copyBtn.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(errorText)
        copyBtn.textContent = 'Copied!'
        copyBtn.classList.add('copied')
        setTimeout(() => {
          copyBtn.textContent = 'Copy Error'
          copyBtn.classList.remove('copied')
        }, 2000)
      } catch {
        // Fallback for older browsers
        const textarea = document.createElement('textarea')
        textarea.value = errorText
        textarea.style.position = 'fixed'
        textarea.style.opacity = '0'
        document.body.appendChild(textarea)
        textarea.select()
        document.execCommand('copy')
        document.body.removeChild(textarea)
        copyBtn.textContent = 'Copied!'
        copyBtn.classList.add('copied')
        setTimeout(() => {
          copyBtn.textContent = 'Copy Error'
          copyBtn.classList.remove('copied')
        }, 2000)
      }
    })
  }

  // Add error styles if not already added
  if (!document.getElementById('error-styles')) {
    const style = document.createElement('style')
    style.id = 'error-styles'
    style.textContent = `
      .error-text { color: #ff6b6b !important; }
      .loader.error-state {
        animation: none !important;
        border-color: #ff6b6b !important;
        border-top-color: #ff6b6b !important;
      }
      .error-container {
        margin-top: 30px;
        text-align: center;
        max-width: 500px;
        padding: 0 20px;
      }
      .error-message {
        color: #fff;
        font-size: 14px;
        margin-bottom: 10px;
        word-break: break-word;
      }
      .error-details {
        color: rgba(255,255,255,0.7);
        font-size: 12px;
        font-family: monospace;
        background: rgba(0,0,0,0.2);
        padding: 10px;
        border-radius: 4px;
        margin-bottom: 15px;
        max-height: 150px;
        overflow-y: auto;
        text-align: left;
        white-space: pre-wrap;
        word-break: break-all;
      }
      .error-hint {
        color: rgba(255,255,255,0.6);
        font-size: 12px;
        margin-bottom: 20px;
      }
      .error-buttons {
        display: flex;
        gap: 12px;
        justify-content: center;
      }
      .copy-button, .retry-button {
        background: rgba(255,255,255,0.2);
        border: 1px solid rgba(255,255,255,0.3);
        color: #fff;
        padding: 10px 30px;
        border-radius: 6px;
        cursor: pointer;
        font-size: 14px;
        transition: background 0.2s, border-color 0.2s;
      }
      .copy-button:hover, .retry-button:hover {
        background: rgba(255,255,255,0.3);
      }
      .copy-button.copied {
        background: rgba(76, 175, 80, 0.3);
        border-color: rgba(76, 175, 80, 0.5);
      }
    `
    document.head.appendChild(style)
  }
}

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

// ============================================================================
// Router Setup
// ============================================================================

// Preload functions for lazy-loaded views
const preloadManagement = () => import('./views/Management.vue')
const preloadSettings = () => import('./views/Settings.vue')

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: Home },
    { path: '/onboarding', component: () => import('./views/Onboarding.vue') },
    { path: '/management', component: preloadManagement },
    { path: '/management/liveries', component: () => import('./views/Liveries.vue') },
    { path: '/management/scripts', component: () => import('./views/Scripts.vue') },
    { path: '/log-analysis', component: () => import('./views/LogAnalysis.vue') },
    { path: '/scenery', redirect: '/management?tab=scenery' },
    { path: '/settings', component: preloadSettings },
  ],
})

// Preload views after initial render for faster navigation
function preloadViews() {
  // Use requestIdleCallback if available, otherwise setTimeout
  const schedulePreload = window.requestIdleCallback || ((cb: () => void) => setTimeout(cb, 100))
  schedulePreload(() => {
    preloadManagement()
    preloadSettings()
  })
}

// ============================================================================
// Store Initialization with Timeout
// ============================================================================

interface StoreInitResult {
  name: string
  success: boolean
  error?: string
  duration: number
}

/**
 * Initialize a store with timeout protection
 */
async function initStoreWithTimeout(
  name: string,
  initFn: () => Promise<void>,
  timeoutMs = 10000,
): Promise<StoreInitResult> {
  const startTime = Date.now()

  try {
    await Promise.race([
      initFn(),
      new Promise<never>((_, reject) =>
        setTimeout(
          () => reject(new Error(`Store initialization timed out after ${timeoutMs}ms`)),
          timeoutMs,
        ),
      ),
    ])

    const duration = Date.now() - startTime
    bootstrapDebug(`Store ${name} initialized in ${duration}ms`, 'store-init')
    return { name, success: true, duration }
  } catch (error) {
    const duration = Date.now() - startTime
    const errorMsg = formatError(error)
    bootstrapError(`Store ${name} failed to initialize: ${errorMsg}`, 'store-init')
    return { name, success: false, error: errorMsg, duration }
  }
}

// ============================================================================
// Main Initialization
// ============================================================================

async function initApp(): Promise<void> {
  const initStartTime = Date.now()
  let app: VueApp | null = null

  try {
    // Step 1: Initialize Tauri Store
    bootstrapInfo('Initializing storage...', 'init')
    try {
      await initStorage()
      markTauriReady()
      bootstrapInfo('Storage initialized successfully', 'init')
    } catch (error) {
      const errorMsg = formatError(error)
      bootstrapError(`Storage initialization failed: ${errorMsg}`, 'init')
      showLoadingError('Failed to initialize storage', errorMsg)
      return
    }

    // Step 2: Setup navigation guard
    bootstrapDebug('Setting up navigation guard...', 'init')
    router.beforeEach(async (to, _from, next) => {
      try {
        const completed = await getItem<string>(STORAGE_KEYS.ONBOARDING_COMPLETED)
        if (completed !== 'true' && to.path !== '/onboarding') {
          next('/onboarding')
          return
        }
        next()
      } catch (error) {
        bootstrapError(`Navigation guard error: ${formatError(error)}`, 'router')
        next() // Continue anyway to avoid blocking
      }
    })

    // Step 3: Create Vue app and Pinia
    bootstrapInfo('Creating Vue application...', 'init')
    const pinia = createPinia()
    app = createApp(App)

    // Setup Vue error handler
    app.config.errorHandler = (err, _instance, info) => {
      const errorMsg = formatError(err)
      bootstrapError(`Vue error [${info}]: ${errorMsg}`, 'vue-error')
      console.error('Vue error:', err, '\nInfo:', info)
    }

    app.config.warnHandler = (msg, _instance, trace) => {
      bootstrapDebug(`Vue warning: ${msg}`, 'vue-warn')
      console.warn('Vue warning:', msg, '\nTrace:', trace)
    }

    app.use(pinia)
    app.use(router)
    app.use(i18n)
    bootstrapDebug('Vue plugins installed', 'init')

    // Step 4: Initialize stores
    bootstrapInfo('Loading stores...', 'init')
    const appStore = useAppStore()
    const themeStore = useThemeStore()
    const lockStore = useLockStore()
    const updateStore = useUpdateStore()
    const sceneryStore = useSceneryStore()

    // Step 5: Initialize all stores with timeout protection
    bootstrapInfo('Initializing stores...', 'init')
    const storeResults = await Promise.all([
      initStoreWithTimeout('appStore', () => appStore.initStore()),
      initStoreWithTimeout('themeStore', () => themeStore.initStore()),
      initStoreWithTimeout('lockStore', () => lockStore.initStore()),
      initStoreWithTimeout('updateStore', () => updateStore.initStore()),
      initStoreWithTimeout('sceneryStore', () => sceneryStore.initStore()),
    ])

    // Check for store initialization failures
    const failedStores = storeResults.filter((r) => !r.success)
    if (failedStores.length > 0) {
      const failedNames = failedStores.map((r) => r.name).join(', ')
      const errorDetails = failedStores.map((r) => `${r.name}: ${r.error}`).join('\n')
      bootstrapError(`Some stores failed to initialize: ${failedNames}`, 'init')

      // Show error but continue - app might still work partially
      showLoadingError(`Failed to initialize: ${failedNames}`, errorDetails)
      // Don't return - try to mount the app anyway
    }

    // Log store initialization summary
    const totalStoreTime = storeResults.reduce((sum, r) => sum + r.duration, 0)
    bootstrapInfo(`All stores processed in ${totalStoreTime}ms`, 'init')

    // Step 6: Mount the app
    bootstrapInfo('Mounting Vue application...', 'init')
    app.mount('#app')

    // Step 7: Preload views for faster navigation
    preloadViews()

    // Step 8: Hide loading screen
    const totalInitTime = Date.now() - initStartTime
    bootstrapInfo(`Application initialized successfully in ${totalInitTime}ms`, 'init')

    setTimeout(() => {
      const loadingScreen = document.getElementById('loading-screen')
      if (loadingScreen) {
        // Only hide if no error is shown
        if (!loadingScreen.querySelector('.error-container')) {
          loadingScreen.classList.add('fade-out')
          setTimeout(() => {
            loadingScreen.remove()
          }, 300)
        }
      }
    }, 100)
  } catch (error) {
    const errorMsg = formatError(error)
    bootstrapError(`Critical initialization error: ${errorMsg}`, 'init')
    showLoadingError('Application failed to start', errorMsg)

    // Try to mount a minimal error state if app was created
    if (app) {
      try {
        app.mount('#app')
      } catch {
        // Ignore mount errors at this point
      }
    }
  }
}

// Start the application
bootstrapDebug('Calling initApp()', 'init')
initApp().catch((error) => {
  const errorMsg = formatError(error)
  bootstrapError(`Unhandled error in initApp: ${errorMsg}`, 'init')
  showLoadingError('Unexpected error during startup', errorMsg)
})
