<template>
  <Teleport to="body">
    <Transition name="modal" :css="false" @enter="onEnter" @leave="onLeave">
      <div
        v-if="modal.errorModal.visible"
        class="fixed inset-0 z-[1100] flex items-center justify-center"
      >
        <!-- Backdrop -->
        <div
          ref="backdrop"
          class="modal-backdrop absolute inset-0 bg-black/50 backdrop-blur-sm"
          @click="close"
        ></div>

        <!-- Modal card -->
        <div
          ref="card"
          role="dialog"
          aria-modal="true"
          class="modal-card relative bg-white dark:bg-gradient-to-tr dark:from-gray-800/95 dark:to-gray-900/95 text-gray-900 dark:text-gray-100 rounded-2xl max-w-md w-full p-6 mx-4 border border-gray-200 dark:border-gray-700/50"
        >
          <div class="flex items-start justify-between">
            <div class="flex items-center space-x-3">
              <!-- Improved error icon -->
              <div
                class="w-11 h-11 flex items-center justify-center rounded-full bg-gradient-to-br from-red-500 to-red-700"
              >
                <svg
                  class="w-7 h-7 text-white"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                >
                  <circle cx="12" cy="12" r="11" stroke-width="2" />
                  <path stroke-linecap="round" stroke-width="2.5" d="M12 6v7" />
                  <circle cx="12" cy="17" r="1.5" fill="currentColor" stroke="none" />
                </svg>
              </div>
              <div>
                <h3 class="text-lg font-semibold leading-tight">
                  {{ modal.errorModal.title || t('common.error') }}
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  {{ t('modal.errorInfo') }}
                </p>
              </div>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-100 transition-colors p-1 -mr-1 -mt-1"
              @click="close"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          <div
            class="mt-4 text-sm allow-select max-h-48 overflow-auto whitespace-pre-wrap leading-relaxed text-gray-700 dark:text-gray-100"
          >
            {{ modal.errorModal.message }}
          </div>

          <!-- Privacy notice for bug reporting -->
          <div
            class="mt-3 px-3 py-2.5 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800/30 rounded-lg text-xs leading-relaxed text-blue-800 dark:text-blue-200"
          >
            {{ t('modal.bugReportPrivacyNotice') }}
          </div>

          <div class="mt-6 flex justify-end items-center space-x-3">
            <!-- Upload Bug Report -->
            <button
              class="px-3 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-md text-sm text-white transition flex items-center space-x-2"
              :disabled="isSubmitting"
              :title="t('modal.submitBugReport')"
              @click="submitBugReport"
            >
              <svg v-if="isSubmitting" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                />
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                />
              </svg>
              <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                />
              </svg>
              <span class="text-sm">{{ t('modal.submitBugReport') }}</span>
            </button>
            <!-- Copy -->
            <button
              class="px-3 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-md text-sm text-gray-700 dark:text-gray-200 transition flex items-center space-x-2"
              :aria-label="t('copy.copy')"
              @click="copyAll"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <rect
                  x="9"
                  y="9"
                  width="13"
                  height="13"
                  rx="2"
                  ry="2"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                ></rect>
                <path
                  d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                ></path>
              </svg>
              <span class="text-sm">{{ $t('copy.copy') }}</span>
            </button>
            <button
              ref="okBtn"
              class="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-lg text-white font-medium transition"
              @click="close"
            >
              {{ t('common.close') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { useI18n } from 'vue-i18n'
import { logError } from '@/services/logger'
import { logger } from '@/services/logger'
import { getItem, setItem, STORAGE_KEYS, type TrackedIssue } from '@/services/storage'

const modal = useModalStore()
const okBtn = ref<HTMLElement | null>(null)
const backdrop = ref<HTMLElement | null>(null)
const card = ref<HTMLElement | null>(null)
const toast = useToastStore()
const { t } = useI18n()
const isSubmitting = ref(false)

// Animation timing constants
const CARD_SHOW_DELAY_MS = 50 // Delay before showing card after backdrop
const ENTER_ANIMATION_DURATION_MS = 450 // Total enter animation time
const LEAVE_ANIMATION_DURATION_MS = 350 // Total leave animation time

function close() {
  modal.closeError()
}

async function copyAll() {
  const text = modal.errorModal.message || ''
  try {
    await navigator.clipboard.writeText(text)
    toast.success(t('copy.copied') as string)
  } catch (e) {
    logError(`Copy failed: ${e}`, 'clipboard')
    toast.error(t('copy.copyFailed') as string)
  }
}

const BUG_REPORT_TIMEOUT_MS = 20000

async function submitBugReport() {
  if (isSubmitting.value) return
  isSubmitting.value = true

  try {
    // Collect logs
    let logs = ''
    try {
      logs = await invoke<string>('get_all_logs')
    } catch {
      logs = '(failed to retrieve logs)'
    }

    const errorTitle = modal.errorModal.title || ''
    const errorMessage = modal.errorModal.message || ''

    // Build fallback GitHub issue URL
    const fallbackTitle = `[Bug]: ${(errorTitle || errorMessage).slice(0, 80)}`
    const fallbackBody = [
      '### Bug Report (Auto-submitted)',
      '',
      '**Error Message**',
      '```',
      errorMessage,
      '```',
      '',
      '**Logs**',
      '<details>',
      '<summary>Click to expand logs</summary>',
      '',
      '```',
      logs.slice(0, 5000),
      '```',
      '</details>',
    ].join('\n')
    const fallbackUrl = `https://github.com/CCA3370/XFast-Manager/issues/new?template=bug_report.yml&labels=${encodeURIComponent('bug')}&title=${encodeURIComponent(fallbackTitle)}&body=${encodeURIComponent(fallbackBody)}`

    // Try API submission with timeout
    let submitTimeoutId: ReturnType<typeof setTimeout> | null = null
    try {
      const result = await Promise.race<{ issue_url: string; issue_number: number }>([
        invoke<{ issue_url: string; issue_number: number }>('create_bug_report_issue', {
          errorTitle,
          errorMessage,
          logs,
          category: 'Other',
        }),
        new Promise<{ issue_url: string; issue_number: number }>((_, reject) => {
          submitTimeoutId = setTimeout(() => {
            reject(new Error('BUG_REPORT_SUBMIT_TIMEOUT'))
          }, BUG_REPORT_TIMEOUT_MS)
        }),
      ])

      toast.success(t('modal.bugReportSubmitted'))
      // Open the created issue
      await invoke('open_url', { url: result.issue_url })

      // Track the issue for future update checks
      if (result.issue_number > 0) {
        try {
          const now = new Date().toISOString()
          const newEntry: TrackedIssue = {
            issueNumber: result.issue_number,
            issueTitle: `[Bug]: ${(errorTitle || errorMessage).slice(0, 80)}`,
            issueUrl: result.issue_url,
            state: 'open',
            commentCount: 0,
            reportedAt: now,
            lastCheckedAt: now,
          }
          const existing = (await getItem<TrackedIssue[]>(STORAGE_KEYS.REPORTED_ISSUES)) ?? []
          const updated = [newEntry, ...existing].slice(0, 10)
          await setItem(STORAGE_KEYS.REPORTED_ISSUES, updated)
        } catch (trackErr) {
          logError(`Failed to track reported issue: ${trackErr}`, 'bug-report')
        }
      }
    } catch {
      // Fallback: open prefilled GitHub issue page
      try {
        await invoke('open_url', { url: fallbackUrl })
        toast.success(t('modal.bugReportOpened'))
      } catch (e2) {
        logError(`Failed to open bug report URL: ${e2}`, 'bug-report')
        toast.error(t('modal.bugReportFailed'))
      }
    } finally {
      if (submitTimeoutId !== null) {
        clearTimeout(submitTimeoutId)
      }
    }
  } catch (e) {
    logError(`Bug report submission failed: ${e}`, 'bug-report')
    toast.error(t('modal.bugReportFailed'))
  } finally {
    isSubmitting.value = false
  }
}

// JavaScript-based animations
function onEnter(el: Element, done: () => void) {
  const element = el as HTMLElement
  const backdropEl = backdrop.value
  const cardEl = card.value

  if (!backdropEl || !cardEl) {
    done()
    return
  }

  // Set initial state
  element.style.opacity = '0'
  backdropEl.style.opacity = '0'
  cardEl.style.opacity = '0'
  cardEl.style.transform = 'scale(0.85) translateY(-30px)'

  // Force reflow
  void element.offsetHeight

  // Animate backdrop faster (starts immediately)
  element.style.transition = 'opacity 0.15s ease-out'
  backdropEl.style.transition = 'opacity 0.15s ease-out'

  // Animate card slower (with bounce)
  cardEl.style.transition = 'all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)'

  // Start backdrop animation immediately
  requestAnimationFrame(() => {
    element.style.opacity = '1'
    backdropEl.style.opacity = '1'

    // Delay card animation slightly so backdrop appears first
    setTimeout(() => {
      cardEl.style.opacity = '1'
      cardEl.style.transform = 'scale(1) translateY(0)'
    }, CARD_SHOW_DELAY_MS)
  })

  setTimeout(done, ENTER_ANIMATION_DURATION_MS)
}

function onLeave(el: Element, done: () => void) {
  const element = el as HTMLElement
  const backdropEl = element.querySelector('.modal-backdrop') as HTMLElement
  const cardEl = element.querySelector('.modal-card') as HTMLElement

  if (!backdropEl || !cardEl) {
    done()
    return
  }

  // Set transition properties first
  element.style.transition = 'opacity 0.3s ease-in'
  backdropEl.style.transition = 'opacity 0.3s ease-in'
  cardEl.style.transition = 'all 0.3s cubic-bezier(0.4, 0, 0.6, 1)'

  // Fallback timeout in case transitionend doesn't fire
  let fallbackTimeout: ReturnType<typeof setTimeout> | null = null

  // Listen for transition end on the card element
  const handleTransitionEnd = () => {
    cardEl.removeEventListener('transitionend', handleTransitionEnd)
    if (fallbackTimeout) {
      clearTimeout(fallbackTimeout)
      fallbackTimeout = null
    }
    done()
  }
  cardEl.addEventListener('transitionend', handleTransitionEnd)

  fallbackTimeout = setTimeout(() => {
    cardEl.removeEventListener('transitionend', handleTransitionEnd)
    done()
  }, LEAVE_ANIMATION_DURATION_MS)

  // Use double requestAnimationFrame to ensure transition is applied
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      element.style.opacity = '0'
      backdropEl.style.opacity = '0'
      cardEl.style.opacity = '0'
      cardEl.style.transform = 'scale(0.9) translateY(10px)'
    })
  })
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

// Dynamically manage keydown listener based on visibility
// This prevents memory leaks when multiple modal instances exist
watch(
  () => modal.errorModal.visible,
  (visible) => {
    if (visible) {
      window.addEventListener('keydown', onKey)
    } else {
      window.removeEventListener('keydown', onKey)
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
})

// When modal becomes visible, autofocus OK button
watch(
  () => modal.errorModal.visible,
  async (v) => {
    if (v) {
      await nextTick()
      okBtn.value?.focus()
    }
  },
)
</script>

<style scoped>
.allow-select {
  user-select: text;
}

/* Ensure long messages can scroll */
.allow-select::-webkit-scrollbar {
  height: 8px;
  width: 8px;
}

.allow-select::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.08);
  border-radius: 6px;
}
</style>
