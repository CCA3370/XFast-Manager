<template>
  <Teleport to="body">
    <Transition name="modal" :css="false" @enter="onEnter" @leave="onLeave">
      <div v-if="visible" class="fixed inset-0 z-[1100] flex items-center justify-center">
        <!-- Backdrop -->
        <div
          ref="backdrop"
          class="modal-backdrop absolute inset-0 bg-black/50 backdrop-blur-sm"
          @click="$emit('close')"
        ></div>

        <!-- Modal Card -->
        <div
          ref="card"
          role="dialog"
          aria-modal="true"
          class="modal-card relative bg-white dark:bg-gradient-to-tr dark:from-gray-800/95 dark:to-gray-900/95 text-gray-900 dark:text-gray-100 rounded-2xl max-w-2xl w-full mx-4 max-h-[80vh] flex flex-col border border-gray-200 dark:border-gray-700/50"
        >
          <!-- Header -->
          <div class="flex-shrink-0 p-4 pb-3 border-b border-gray-200 dark:border-gray-700/50">
            <div class="flex items-start justify-between">
              <div class="flex items-center space-x-2">
                <div
                  class="w-8 h-8 flex items-center justify-center rounded-full bg-gradient-to-br from-red-500 to-red-700"
                >
                  <svg
                    class="w-5 h-5 text-white"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M6 18L18 6M6 6l12 12"
                    ></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-base font-semibold leading-tight">
                    {{ $t('completion.failedTasks') }}
                  </h3>
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                    {{ failedTasks.length }} {{ $t('completion.failed') }}
                  </p>
                </div>
              </div>
              <button
                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-100 transition-colors p-1 -mr-1 -mt-1"
                @click="$emit('close')"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

          <!-- Scrollable Task List -->
          <div class="flex-1 overflow-y-auto p-4 space-y-2 custom-scrollbar">
            <div
              v-for="task in failedTasks"
              :key="task.taskId"
              class="bg-gray-100 dark:bg-gray-800/50 border border-red-200 dark:border-red-500/20 rounded-lg hover:border-red-300 dark:hover:border-red-500/40 transition-colors overflow-hidden"
            >
              <button
                type="button"
                class="w-full p-3 text-left"
                @click="$emit('viewTask', task)"
              >
                <div class="flex items-start space-x-2">
                  <svg
                    class="w-4 h-4 text-red-500 flex-shrink-0 mt-0.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M6 18L18 6M6 6l12 12"
                    ></path>
                  </svg>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                      <span
                        class="inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium bg-red-100 dark:bg-red-500/20 text-red-700 dark:text-red-300 border border-red-200 dark:border-red-500/30 flex-shrink-0"
                      >
                        {{ getSimpleErrorReason(task.errorMessage) }}
                      </span>
                      <span class="font-medium text-gray-900 dark:text-white text-sm truncate">{{
                        task.taskName
                      }}</span>
                    </div>
                  </div>
                  <svg
                    class="w-4 h-4 text-gray-400 dark:text-gray-500 flex-shrink-0"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M9 5l7 7-7 7"
                    ></path>
                  </svg>
                </div>
              </button>
            </div>
          </div>

          <!-- Footer -->
          <div
            class="flex-shrink-0 p-4 pt-3 border-t border-gray-200 dark:border-gray-700/50 flex justify-end space-x-2"
          >
            <button
              class="px-3 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-lg text-xs text-gray-700 dark:text-gray-200 transition flex items-center space-x-1.5"
              @click="copyAllErrors"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
              <span>{{ $t('copy.copy') }}</span>
            </button>
            <button
              ref="closeBtn"
              class="px-3 py-2 bg-red-600 hover:bg-red-700 rounded-lg text-white text-xs font-medium transition"
              @click="$emit('close')"
            >
              {{ $t('common.close') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { TaskResult } from '@/types'
import { useI18n } from 'vue-i18n'
import { useToastStore } from '@/stores/toast'
import { logError } from '@/services/logger'

const { t } = useI18n()
const toast = useToastStore()

const props = defineProps<{
  visible: boolean
  failedTasks: TaskResult[]
}>()

defineEmits<{
  close: []
  viewTask: [task: TaskResult]
}>()

const backdrop = ref<HTMLElement | null>(null)
const card = ref<HTMLElement | null>(null)
const closeBtn = ref<HTMLElement | null>(null)

// Animation timing constants
const CARD_SHOW_DELAY_MS = 50 // Delay before showing card after backdrop
const ENTER_ANIMATION_DURATION_MS = 450 // Total enter animation time
const LEAVE_ANIMATION_DURATION_MS = 350 // Total leave animation time

// Error categorization function (expanded version)
function getSimpleErrorReason(errorMessage?: string): string {
  if (!errorMessage) return t('completion.unknownError')

  const msg = errorMessage.toLowerCase()

  // Password issues (highest priority)
  if (msg.includes('password required') || msg.includes('encrypted file')) {
    return t('completion.passwordRequired')
  }
  if (msg.includes('wrong password') || msg.includes('decryption failed')) {
    return t('completion.wrongPassword')
  }
  if (msg.includes('password') || msg.includes('密码')) {
    return t('completion.passwordError')
  }

  // Path traversal security
  if (msg.includes('path traversal') || msg.includes('unsafe path')) {
    return t('completion.pathTraversal')
  }

  // Archive format issues
  if (msg.includes('unsupported archive format')) {
    return t('completion.unsupportedFormat')
  }
  if (msg.includes('corrupt') || msg.includes('failed to read archive')) {
    return t('completion.corruptArchive')
  }

  // Extraction safety limits
  if (msg.includes('compression ratio exceeded')) {
    return t('completion.compressionRatio')
  }
  if (msg.includes('size limit exceeded') || msg.includes('extraction size')) {
    return t('completion.sizeLimitExceeded')
  }

  // Verification failures
  if (
    msg.includes('verification failed') ||
    msg.includes('hash mismatch') ||
    msg.includes('校验失败')
  ) {
    return t('completion.verificationFailed')
  }

  // Disk space issues
  if (
    msg.includes('no space') ||
    msg.includes('disk space') ||
    msg.includes('insufficient space') ||
    msg.includes('磁盘空间')
  ) {
    return t('completion.diskSpaceFull')
  }

  // Permission issues
  if (msg.includes('permission denied') || msg.includes('access denied') || msg.includes('权限')) {
    return t('completion.permissionDenied')
  }

  // File system operations
  if (msg.includes('failed to create directory')) {
    return t('completion.createDirFailed')
  }
  if (msg.includes('failed to copy file')) {
    return t('completion.copyFileFailed')
  }
  if (msg.includes('failed to remove') || msg.includes('failed to delete')) {
    return t('completion.removeFailed')
  }
  if (msg.includes('file not found') || msg.includes('not found') || msg.includes('找不到')) {
    return t('completion.fileNotFound')
  }

  // Extraction errors
  if (
    msg.includes('failed to extract') ||
    msg.includes('extraction failed') ||
    msg.includes('解压')
  ) {
    return t('completion.extractionFailed')
  }

  // User actions
  if (msg.includes('cancelled by user')) {
    return t('completion.cancelledByUser')
  }
  if (msg.includes('skipped by user')) {
    return t('completion.skippedByUser')
  }

  // Network (unlikely but included)
  if (msg.includes('network') || msg.includes('connection') || msg.includes('网络')) {
    return t('completion.networkError')
  }

  return t('completion.unknownError')
}

async function copyAllErrors() {
  const errorText = props.failedTasks
    .map((task) => `${task.taskName}:\n${task.errorMessage || 'Unknown error'}`)
    .join('\n\n')

  try {
    await navigator.clipboard.writeText(errorText)
    toast.success(t('copy.copied') as string)
  } catch (e) {
    logError(`Copy failed: ${e}`, 'clipboard')
    toast.error(t('copy.copyFailed') as string)
  }
}

// Animation functions (same as ErrorModal.vue)
function onEnter(el: Element, done: () => void) {
  const element = el as HTMLElement
  const backdropEl = backdrop.value
  const cardEl = card.value

  if (!backdropEl || !cardEl) {
    done()
    return
  }

  element.style.opacity = '0'
  backdropEl.style.opacity = '0'
  cardEl.style.opacity = '0'
  cardEl.style.transform = 'scale(0.85) translateY(-30px)'

  void element.offsetHeight

  element.style.transition = 'opacity 0.15s ease-out'
  backdropEl.style.transition = 'opacity 0.15s ease-out'
  cardEl.style.transition = 'all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)'

  requestAnimationFrame(() => {
    element.style.opacity = '1'
    backdropEl.style.opacity = '1'

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

  element.style.transition = 'opacity 0.3s ease-in'
  backdropEl.style.transition = 'opacity 0.3s ease-in'
  cardEl.style.transition = 'all 0.3s cubic-bezier(0.4, 0, 0.6, 1)'

  let fallbackTimeout: ReturnType<typeof setTimeout> | null = null

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

  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      element.style.opacity = '0'
      backdropEl.style.opacity = '0'
      cardEl.style.opacity = '0'
      cardEl.style.transform = 'scale(0.9) translateY(10px)'
    })
  })
}

// Auto-focus close button when modal opens
watch(
  () => props.visible,
  async (v) => {
    if (v) {
      await nextTick()
      closeBtn.value?.focus()
    }
  },
)
</script>

<style scoped>
.custom-scrollbar {
  scrollbar-width: thin;
  scrollbar-color: rgba(156, 163, 175, 0.5) transparent;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 8px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.5);
  border-radius: 4px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.7);
}
</style>
