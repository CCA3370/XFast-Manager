<template>
  <Teleport to="body">
    <Transition :css="false" @enter="onEnter" @leave="onLeave">
      <div
        v-if="issueTracker.pendingUpdates.length > 0"
        class="fixed inset-0 z-[1200] flex items-center justify-center"
      >
        <!-- Backdrop (no click-to-close) -->
        <div
          ref="backdropRef"
          class="absolute inset-0 bg-black/50 backdrop-blur-sm"
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
          </div>

          <!-- Body (scrollable) -->
          <div class="overflow-y-auto flex-1 p-5 space-y-5">
            <div
              v-for="(update, index) in issueTracker.pendingUpdates"
              :key="update.issue.issueNumber"
            >
              <!-- Separator between issues -->
              <div v-if="index > 0" class="border-t border-gray-100 dark:border-white/5 -mt-1 mb-5" />

              <!-- Closed status card -->
              <div
                v-if="update.closed"
                class="flex items-start gap-3 rounded-xl border border-green-200 dark:border-green-500/25 bg-green-50 dark:bg-green-500/10 px-4 py-3 mb-3"
              >
                <!-- Closed icon -->
                <div class="mt-0.5 w-5 h-5 rounded-full bg-green-600 dark:bg-green-500 flex items-center justify-center shrink-0">
                  <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                  </svg>
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-xs font-semibold text-green-700 dark:text-green-400 mb-0.5">
                    {{ t('issueTracker.statusClosed') }}
                  </p>
                  <button
                    class="text-sm font-medium text-gray-800 dark:text-gray-100 hover:underline text-left truncate w-full"
                    @click="openUrl(update.issue.issueUrl)"
                  >
                    {{ update.issue.issueTitle }}
                  </button>
                </div>
                <button
                  class="shrink-0 text-xs text-green-600 dark:text-green-400 hover:text-green-800 dark:hover:text-green-300 font-medium transition-colors whitespace-nowrap"
                  @click="openUrl(update.issue.issueUrl)"
                >
                  {{ t('issueTracker.viewIssue') }} ↗
                </button>
              </div>

              <!-- Open issue title (only if not closed) -->
              <div v-else class="flex items-start justify-between gap-2 mb-3">
                <button
                  class="text-sm font-semibold text-blue-600 dark:text-blue-400 hover:underline truncate text-left"
                  @click="openUrl(update.issue.issueUrl)"
                >
                  {{ update.issue.issueTitle }}
                </button>
              </div>

              <!-- New comments -->
              <div v-if="update.newComments.length > 0" class="space-y-2">
                <p class="text-xs font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider">
                  {{ t('issueTracker.newComments') }}
                </p>
                <div
                  v-for="(comment, ci) in update.newComments"
                  :key="ci"
                  class="bg-gray-50 dark:bg-white/5 rounded-xl p-3 border border-gray-100 dark:border-white/5"
                >
                  <div class="flex items-center gap-1.5 mb-2">
                    <div class="w-5 h-5 rounded-full bg-gradient-to-br from-blue-400 to-blue-600 flex items-center justify-center shrink-0">
                      <span class="text-white text-[9px] font-bold leading-none">{{ comment.author.charAt(0).toUpperCase() }}</span>
                    </div>
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300">@{{ comment.author }}</span>
                    <span class="text-xs text-gray-400 dark:text-gray-500">· {{ formatDate(comment.created_at) }}</span>
                  </div>
                  <p class="text-sm text-gray-700 dark:text-gray-200 leading-relaxed whitespace-pre-wrap break-words">
                    {{ truncate(comment.body, 400) }}
                  </p>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="p-5 border-t border-gray-100 dark:border-white/5 flex items-center justify-end gap-3">
            <template v-if="issueTracker.pendingUpdates.length === 1 && !issueTracker.pendingUpdates[0].closed">
              <button
                class="px-3 py-2 bg-blue-600 hover:bg-blue-700 rounded-md text-sm text-white font-medium transition"
                @click="openUrl(issueTracker.pendingUpdates[0].issue.issueUrl)"
              >
                {{ t('issueTracker.viewIssue') }}
              </button>
            </template>
            <button
              class="px-3 py-2 bg-green-600 hover:bg-green-700 rounded-md text-sm text-white font-medium transition"
              @click="confirm"
            >
              {{ t('issueTracker.confirm') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useIssueTrackerStore } from '@/stores/issueTracker'

const { t } = useI18n()
const issueTracker = useIssueTrackerStore()

const backdropRef = ref<HTMLElement | null>(null)
const cardRef = ref<HTMLElement | null>(null)

async function confirm() {
  await issueTracker.confirmUpdates()
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
