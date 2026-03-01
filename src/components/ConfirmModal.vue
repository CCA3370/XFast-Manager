<template>
  <Teleport to="body">
    <Transition name="modal" :css="false" @enter="onEnter" @leave="onLeave">
      <div v-if="isVisible" class="fixed inset-0 z-[1300] flex items-center justify-center">
        <!-- Backdrop -->
        <div
          ref="backdrop"
          class="modal-backdrop absolute inset-0 bg-black/50 backdrop-blur-sm"
          @click="handleCancel"
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
              <!-- Icon based on type -->
              <div
                class="w-11 h-11 flex items-center justify-center rounded-full"
                :class="{
                  'bg-gradient-to-br from-amber-500 to-yellow-600': currentType === 'warning',
                  'bg-gradient-to-br from-red-500 to-red-700': currentType === 'danger',
                }"
              >
                <svg
                  v-if="currentType === 'warning'"
                  class="w-7 h-7 text-white"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                  ></path>
                </svg>
                <svg
                  v-else
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
                <h3 class="text-lg font-semibold leading-tight">{{ currentTitle }}</h3>
              </div>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-100 transition-colors p-1 -mr-1 -mt-1"
              @click="handleCancel"
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

          <div class="mt-4 space-y-3">
            <p class="text-sm text-gray-700 dark:text-gray-100 leading-relaxed whitespace-pre-line">
              {{ currentMessage }}
            </p>

            <!-- Item name highlight -->
            <div v-if="currentItemName" class="p-3 bg-gray-100 dark:bg-gray-700/50 rounded-lg">
              <p class="text-sm font-mono text-gray-900 dark:text-gray-100 break-all">
                {{ currentItemName }}
              </p>
            </div>

            <div
              v-if="currentWarning"
              class="p-3 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/30 rounded-lg"
            >
              <p class="text-sm text-amber-700 dark:text-amber-200 leading-relaxed">
                {{ currentWarning }}
              </p>
            </div>
          </div>

          <div class="mt-6 flex justify-end items-center space-x-3">
            <button
              ref="cancelBtn"
              :disabled="isLoading"
              class="px-4 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-lg text-gray-700 dark:text-white font-medium transition disabled:opacity-50"
              @click="handleCancel"
            >
              {{ currentCancelText }}
            </button>
            <button
              :disabled="isLoading"
              class="px-4 py-2 rounded-lg text-white font-medium transition disabled:opacity-50 flex items-center gap-2"
              :class="{
                'bg-amber-500 hover:bg-amber-600': currentType === 'warning',
                'bg-red-600 hover:bg-red-700': currentType === 'danger',
              }"
              @click="handleConfirm"
            >
              <svg
                v-if="isLoading"
                class="animate-spin h-4 w-4"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              {{ isLoading ? currentLoadingText : currentConfirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch, nextTick } from 'vue'
import { useModalStore } from '@/stores/modal'
import { useI18n } from 'vue-i18n'
import gsap from 'gsap'

const { t } = useI18n()

// Props for local mode (when used as a reusable component)
const props = withDefaults(
  defineProps<{
    show?: boolean
    title?: string
    message?: string
    itemName?: string
    warning?: string
    confirmText?: string
    cancelText?: string
    loadingText?: string
    isLoading?: boolean
    variant?: 'warning' | 'danger'
  }>(),
  {
    show: false,
    isLoading: false,
    variant: 'danger',
  },
)

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'confirm'): void
  (e: 'cancel'): void
}>()

const modal = useModalStore()
const cancelBtn = ref<HTMLElement | null>(null)
const backdrop = ref<HTMLElement | null>(null)
const card = ref<HTMLElement | null>(null)

// Determine if using local mode (props) or global mode (modalStore)
const isLocalMode = computed(
  () => (props.show !== undefined && props.show !== false) || props.title !== undefined,
)

// Computed visibility
const isVisible = computed(() => {
  if (isLocalMode.value) {
    return props.show
  }
  return modal.confirmModal.visible && modal.confirmModal.options !== null
})

// Computed values that work in both modes
const currentTitle = computed(() => {
  if (isLocalMode.value) return props.title || ''
  return modal.confirmModal.options?.title || ''
})

const currentMessage = computed(() => {
  if (isLocalMode.value) return props.message || ''
  return modal.confirmModal.options?.message || ''
})

const currentItemName = computed(() => {
  if (isLocalMode.value) return props.itemName
  return undefined
})

const currentWarning = computed(() => {
  if (isLocalMode.value) return props.warning
  return modal.confirmModal.options?.warning
})

const currentType = computed(() => {
  if (isLocalMode.value) return props.variant
  return modal.confirmModal.options?.type || 'danger'
})

const currentConfirmText = computed(() => {
  if (isLocalMode.value) return props.confirmText || t('common.confirm')
  return modal.confirmModal.options?.confirmText || t('common.confirm')
})

const currentCancelText = computed(() => {
  if (isLocalMode.value) return props.cancelText || t('common.cancel')
  return modal.confirmModal.options?.cancelText || t('common.cancel')
})

const currentLoadingText = computed(() => {
  if (isLocalMode.value) return props.loadingText || t('common.loading')
  return t('common.loading')
})

function handleConfirm() {
  if (isLocalMode.value) {
    emit('confirm')
  } else {
    modal.confirmAction()
  }
}

function handleCancel() {
  if (isLocalMode.value) {
    emit('update:show', false)
    emit('cancel')
  } else {
    modal.cancelAction()
  }
}

// GSAP animations
function onEnter(_el: Element, done: () => void) {
  const tl = gsap.timeline({ onComplete: done })

  tl.fromTo(backdrop.value, { opacity: 0 }, { opacity: 1, duration: 0.2, ease: 'power2.out' })

  tl.fromTo(
    card.value,
    { opacity: 0, scale: 0.9, y: 20 },
    { opacity: 1, scale: 1, y: 0, duration: 0.3, ease: 'back.out(1.7)' },
    '-=0.1',
  )
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
  }, 350)

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

// Focus management
watch(isVisible, async (visible) => {
  if (visible) {
    await nextTick()
    cancelBtn.value?.focus()
  }
})

// Keyboard shortcuts
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    handleCancel()
  } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault()
    handleConfirm()
  }
}

// Dynamically manage keydown listener based on visibility
// This prevents memory leaks when multiple modal instances exist
watch(
  isVisible,
  (visible) => {
    if (visible) {
      document.addEventListener('keydown', handleKeydown)
    } else {
      document.removeEventListener('keydown', handleKeydown)
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.allow-select {
  user-select: text;
  -webkit-user-select: text;
}
</style>
