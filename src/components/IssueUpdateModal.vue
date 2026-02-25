<template>
  <Teleport to="body">
    <Transition :css="false" @enter="onEnter" @leave="onLeave">
      <div
        v-if="issueTracker.pendingUpdates.length > 0"
        class="fixed inset-0 z-[1200] flex items-center justify-center"
      >
        <!-- Backdrop -->
        <div
          ref="backdropRef"
          class="absolute inset-0 bg-black/50 backdrop-blur-sm"
          @click="dismiss"
        />

        <!-- Card -->
        <div
          ref="cardRef"
          role="dialog"
          aria-modal="true"
          class="relative bg-white dark:bg-gradient-to-tr dark:from-gray-800/95 dark:to-gray-900/95 text-gray-900 dark:text-gray-100 rounded-2xl max-w-lg w-full mx-4 border border-gray-200 dark:border-gray-700/50 shadow-2xl flex flex-col"
          style="max-height: 80vh"
          @click.stop
        >
          <!-- Header -->
          <div class="flex items-start justify-between p-5 border-b border-gray-100 dark:border-white/5">
            <div class="flex items-center space-x-3">
              <div
                class="w-10 h-10 flex items-center justify-center rounded-full bg-gradient-to-br from-blue-500 to-blue-700 shrink-0"
              >
                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </div>
              <div>
                <h3 class="text-base font-semibold leading-tight">
                  {{ t('issueTracker.modalTitle') }}
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  {{ t('issueTracker.subtitle') }}
                </p>
              </div>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-100 transition-colors p-1 -mr-1 -mt-1 shrink-0"
              @click="dismiss"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Body (scrollable) -->
          <div class="overflow-y-auto flex-1 p-5 space-y-4">
            <div
              v-for="(update, index) in issueTracker.pendingUpdates"
              :key="update.issue.issueNumber"
            >
              <!-- Separator between issues -->
              <div v-if="index > 0" class="border-t border-gray-100 dark:border-white/5 mb-4" />

              <!-- Issue header -->
              <div class="flex items-start justify-between gap-2 mb-3">
                <a
                  class="text-sm font-semibold text-blue-600 dark:text-blue-400 hover:underline truncate cursor-pointer"
                  @click="openUrl(update.issue.issueUrl)"
                >
                  {{ update.issue.issueTitle }}
                </a>
                <span
                  v-if="update.closed"
                  class="shrink-0 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                >
                  {{ t('issueTracker.statusClosed') }}
                </span>
              </div>

              <!-- Closed notice -->
              <p v-if="update.closed && update.newComments.length === 0" class="text-sm text-gray-500 dark:text-gray-400">
                {{ t('issueTracker.statusClosed') }}
              </p>

              <!-- New comments -->
              <div v-if="update.newComments.length > 0" class="space-y-2">
                <p class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
                  {{ t('issueTracker.newComments') }}
                </p>
                <div
                  v-for="(comment, ci) in update.newComments"
                  :key="ci"
                  class="bg-gray-50 dark:bg-white/5 rounded-lg p-3 text-sm border border-gray-100 dark:border-white/5"
                >
                  <p class="text-xs text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('issueTracker.by') }}
                    <span class="font-medium text-gray-700 dark:text-gray-300">@{{ comment.author }}</span>
                    · {{ formatDate(comment.created_at) }}
                  </p>
                  <p class="text-gray-700 dark:text-gray-200 leading-relaxed whitespace-pre-wrap break-words">
                    {{ truncate(comment.body, 400) }}
                  </p>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="p-5 border-t border-gray-100 dark:border-white/5 flex items-center justify-end gap-3">
            <template v-if="issueTracker.pendingUpdates.length === 1">
              <button
                class="px-3 py-2 bg-blue-600 hover:bg-blue-700 rounded-md text-sm text-white font-medium transition"
                @click="openUrl(issueTracker.pendingUpdates[0].issue.issueUrl)"
              >
                {{ t('issueTracker.viewIssue') }}
              </button>
            </template>
            <button
              class="px-3 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-md text-sm text-gray-700 dark:text-gray-200 font-medium transition"
              @click="dismiss"
            >
              {{ t('issueTracker.dismiss') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useIssueTrackerStore } from '@/stores/issueTracker'

const { t } = useI18n()
const issueTracker = useIssueTrackerStore()

const backdropRef = ref<HTMLElement | null>(null)
const cardRef = ref<HTMLElement | null>(null)

function dismiss() {
  issueTracker.clearUpdates()
}

async function openUrl(url: string) {
  try {
    await invoke('open_url', { url })
  } catch {
    // ignore
  }
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text
  return text.slice(0, max) + '…'
}

function formatDate(iso: string): string {
  if (!iso) return ''
  try {
    return new Date(iso).toLocaleDateString()
  } catch {
    return iso
  }
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') dismiss()
}

watch(
  () => issueTracker.pendingUpdates.length > 0,
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

function onEnter(el: Element, done: () => void) {
  const container = el as HTMLElement
  const backdrop = backdropRef.value
  const card = cardRef.value
  if (!backdrop || !card) {
    done()
    return
  }

  container.style.opacity = '0'
  backdrop.style.opacity = '0'
  card.style.opacity = '0'
  card.style.transform = 'scale(0.85) translateY(-30px)'

  void container.offsetHeight

  container.style.transition = 'opacity 0.12s ease-out'
  backdrop.style.transition = 'opacity 0.12s ease-out'
  card.style.transition = 'all 0.36s cubic-bezier(0.34, 1.56, 0.64, 1)'

  requestAnimationFrame(() => {
    container.style.opacity = '1'
    backdrop.style.opacity = '1'
    setTimeout(() => {
      card.style.opacity = '1'
      card.style.transform = 'scale(1) translateY(0)'
    }, 40)
  })

  setTimeout(done, 400)
}

function onLeave(el: Element, done: () => void) {
  const container = el as HTMLElement
  const backdrop = container.querySelector('.absolute.inset-0') as HTMLElement
  const card = container.querySelector('[role="dialog"]') as HTMLElement
  if (!backdrop || !card) {
    done()
    return
  }

  container.style.transition = 'opacity 0.24s ease-in'
  backdrop.style.transition = 'opacity 0.24s ease-in'
  card.style.transition = 'all 0.24s cubic-bezier(0.4, 0, 0.6, 1)'

  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      container.style.opacity = '0'
      backdrop.style.opacity = '0'
      card.style.opacity = '0'
      card.style.transform = 'scale(0.9) translateY(10px)'
    })
  })

  setTimeout(done, 280)
}
</script>
