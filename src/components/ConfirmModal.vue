<template>
  <Teleport to="body">
    <Transition
      name="modal"
      @enter="onEnter"
      @leave="onLeave"
      :css="false"
    >
      <div v-if="modal.confirmModal.visible && modal.confirmModal.options" class="fixed inset-0 z-[1100] flex items-center justify-center">
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
          class="modal-card relative bg-gradient-to-tr from-gray-800/95 to-gray-900/95 text-gray-100 rounded-2xl shadow-2xl max-w-md w-full p-6 mx-4"
        >
          <div class="flex items-start justify-between">
            <div class="flex items-center space-x-3">
              <!-- Icon based on type -->
              <div
                class="w-11 h-11 flex items-center justify-center rounded-full shadow-lg"
                :class="{
                  'bg-gradient-to-br from-yellow-500 to-orange-600 shadow-yellow-500/30': modal.confirmModal.options.type === 'warning',
                  'bg-gradient-to-br from-red-500 to-red-700 shadow-red-500/30': modal.confirmModal.options.type === 'danger'
                }"
              >
                <svg v-if="modal.confirmModal.options.type === 'warning'" class="w-7 h-7 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                </svg>
                <svg v-else class="w-7 h-7 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <circle cx="12" cy="12" r="11" stroke-width="2" />
                  <path stroke-linecap="round" stroke-width="2.5" d="M12 6v7" />
                  <circle cx="12" cy="17" r="1.5" fill="currentColor" stroke="none" />
                </svg>
              </div>
              <div>
                <h3 class="text-lg font-semibold leading-tight">{{ modal.confirmModal.options.title }}</h3>
              </div>
            </div>
            <button @click="handleCancel" class="text-gray-400 hover:text-gray-100 transition-colors p-1 -mr-1 -mt-1">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="mt-4 space-y-3">
            <p class="text-sm text-gray-100 leading-relaxed">
              {{ modal.confirmModal.options.message }}
            </p>

            <div v-if="modal.confirmModal.options.warning" class="p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
              <p class="text-sm text-yellow-200 leading-relaxed">
                {{ modal.confirmModal.options.warning }}
              </p>
            </div>
          </div>

          <div class="mt-6 flex justify-end items-center space-x-3">
            <button
              ref="cancelBtn"
              @click="handleCancel"
              class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg text-white font-medium transition"
            >
              {{ modal.confirmModal.options.cancelText }}
            </button>
            <button
              ref="confirmBtn"
              @click="handleConfirm"
              class="px-4 py-2 rounded-lg text-white font-medium transition"
              :class="{
                'bg-yellow-600 hover:bg-yellow-700': modal.confirmModal.options.type === 'warning',
                'bg-red-600 hover:bg-red-700': modal.confirmModal.options.type === 'danger'
              }"
            >
              {{ modal.confirmModal.options.confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch, nextTick } from 'vue'
import { useModalStore } from '@/stores/modal'
import gsap from 'gsap'

const modal = useModalStore()
const confirmBtn = ref<HTMLElement | null>(null)
const cancelBtn = ref<HTMLElement | null>(null)
const backdrop = ref<HTMLElement | null>(null)
const card = ref<HTMLElement | null>(null)

function handleConfirm() {
  modal.confirmAction()
}

function handleCancel() {
  modal.cancelAction()
}

// GSAP animations
function onEnter(el: Element, done: () => void) {
  const tl = gsap.timeline({ onComplete: done })

  tl.fromTo(
    backdrop.value,
    { opacity: 0 },
    { opacity: 1, duration: 0.2, ease: 'power2.out' }
  )

  tl.fromTo(
    card.value,
    { opacity: 0, scale: 0.9, y: 20 },
    { opacity: 1, scale: 1, y: 0, duration: 0.3, ease: 'back.out(1.7)' },
    '-=0.1'
  )
}

function onLeave(el: Element, done: () => void) {
  const tl = gsap.timeline({ onComplete: done })

  tl.to(card.value, {
    opacity: 0,
    scale: 0.95,
    y: 10,
    duration: 0.2,
    ease: 'power2.in'
  })

  tl.to(
    backdrop.value,
    { opacity: 0, duration: 0.15, ease: 'power2.in' },
    '-=0.1'
  )
}

// Focus management
watch(() => modal.confirmModal.visible, async (visible) => {
  if (visible) {
    await nextTick()
    cancelBtn.value?.focus()
  }
})

// Keyboard shortcuts
function handleKeydown(e: KeyboardEvent) {
  if (!modal.confirmModal.visible) return

  if (e.key === 'Escape') {
    e.preventDefault()
    handleCancel()
  } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault()
    handleConfirm()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

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
