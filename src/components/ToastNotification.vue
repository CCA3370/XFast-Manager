<template>
  <Teleport to="body">
    <div class="toast-container" :class="{ 'toast-container-scenery': isSceneryPage }">
      <transition-group name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="getToastClass(toast.type)"
        >
          <!-- Icon -->
          <div class="toast-icon">
            <!-- Info Icon -->
            <svg v-if="toast.type === 'info'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <!-- Success Icon -->
            <svg v-else-if="toast.type === 'success'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <!-- Warning Icon -->
            <svg v-else-if="toast.type === 'warning'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
            </svg>
            <!-- Error Icon -->
            <svg v-else class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
          </div>

          <!-- Content -->
          <div class="toast-content">
            <span class="toast-title">{{ getToastTitle(toast.type) }}</span>
            <span class="toast-message allow-select">{{ toast.message }}</span>
          </div>

          <!-- Close Button -->
          <button @click="removeToast(toast.id)" class="toast-close" aria-label="Close">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>

          <!-- Progress Bar - removes toast when animation ends -->
          <div class="toast-progress" @animationend="removeToast(toast.id)"></div>
        </div>
      </transition-group>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useToastStore } from '@/stores/toast'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const route = useRoute()
const toastStore = useToastStore()
const toasts = computed(() => toastStore.toasts)

const isSceneryPage = computed(() => route.path === '/scenery')

function getToastClass(type: string) {
  switch (type) {
    case 'error':
      return 'toast-error'
    case 'success':
      return 'toast-success'
    case 'warning':
      return 'toast-warning'
    default:
      return 'toast-info'
  }
}

function getToastTitle(type: string) {
  switch (type) {
    case 'error':
      return t('common.error')
    case 'success':
      return t('common.success')
    case 'warning':
      return t('common.warning')
    default:
      return t('common.info')
  }
}

function removeToast(id: string) {
  toastStore.remove(id)
}
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 4.5rem;
  right: 1.25rem;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 360px;
  pointer-events: none;
}

/* Lower position for scenery page to avoid blocking action buttons */
.toast-container-scenery {
  top: 8rem;
}

.toast {
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.875rem 1rem;
  border-radius: 0.75rem;
  min-width: 280px;
  pointer-events: auto;
  overflow: hidden;
  box-shadow:
    0 4px 6px -1px rgba(0, 0, 0, 0.1),
    0 2px 4px -2px rgba(0, 0, 0, 0.1),
    0 0 0 1px rgba(0, 0, 0, 0.05);
}

:global(.dark) .toast {
  box-shadow:
    0 4px 6px -1px rgba(0, 0, 0, 0.3),
    0 2px 4px -2px rgba(0, 0, 0, 0.2),
    0 0 0 1px rgba(255, 255, 255, 0.05);
}

.toast-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: 0.5rem;
}

.toast-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.toast-title {
  font-weight: 600;
  font-size: 0.8125rem;
  line-height: 1.25;
}

.toast-message {
  font-size: 0.8125rem;
  line-height: 1.4;
  opacity: 0.9;
  word-break: break-word;
}

.toast-close {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 0.375rem;
  border: none;
  cursor: pointer;
  opacity: 0.6;
  transition: all 0.15s ease;
  margin-top: 0.125rem;
}

.toast-close:hover {
  opacity: 1;
}

.toast-progress {
  position: absolute;
  bottom: 0;
  left: 0;
  height: 3px;
  width: 100%;
  transform-origin: left;
  animation: progress-shrink 3s linear forwards;
}

@keyframes progress-shrink {
  from {
    transform: scaleX(1);
  }
  to {
    transform: scaleX(0);
  }
}

/* Info Toast */
.toast-info {
  background: rgba(239, 246, 255, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(191, 219, 254, 0.6);
}

:global(.dark) .toast-info {
  background: rgba(30, 58, 138, 0.9);
  border: 1px solid rgba(59, 130, 246, 0.3);
}

.toast-info .toast-icon {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

:global(.dark) .toast-info .toast-icon {
  background: rgba(59, 130, 246, 0.2);
  color: #93c5fd;
}

.toast-info .toast-title {
  color: #1e40af;
}

:global(.dark) .toast-info .toast-title {
  color: #bfdbfe;
}

.toast-info .toast-message {
  color: #1e3a8a;
}

:global(.dark) .toast-info .toast-message {
  color: #93c5fd;
}

.toast-info .toast-close {
  background: transparent;
  color: #3b82f6;
}

:global(.dark) .toast-info .toast-close {
  color: #93c5fd;
}

.toast-info .toast-close:hover {
  background: rgba(59, 130, 246, 0.15);
}

.toast-info .toast-progress {
  background: linear-gradient(90deg, #3b82f6, #60a5fa);
}

/* Success Toast */
.toast-success {
  background: rgba(240, 253, 244, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(187, 247, 208, 0.6);
}

:global(.dark) .toast-success {
  background: rgba(20, 83, 45, 0.9);
  border: 1px solid rgba(34, 197, 94, 0.3);
}

.toast-success .toast-icon {
  background: rgba(34, 197, 94, 0.15);
  color: #22c55e;
}

:global(.dark) .toast-success .toast-icon {
  background: rgba(34, 197, 94, 0.2);
  color: #86efac;
}

.toast-success .toast-title {
  color: #166534;
}

:global(.dark) .toast-success .toast-title {
  color: #bbf7d0;
}

.toast-success .toast-message {
  color: #14532d;
}

:global(.dark) .toast-success .toast-message {
  color: #86efac;
}

.toast-success .toast-close {
  background: transparent;
  color: #22c55e;
}

:global(.dark) .toast-success .toast-close {
  color: #86efac;
}

.toast-success .toast-close:hover {
  background: rgba(34, 197, 94, 0.15);
}

.toast-success .toast-progress {
  background: linear-gradient(90deg, #22c55e, #4ade80);
}

/* Warning Toast */
.toast-warning {
  background: rgba(255, 251, 235, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(253, 224, 71, 0.6);
}

:global(.dark) .toast-warning {
  background: rgba(113, 63, 18, 0.9);
  border: 1px solid rgba(251, 191, 36, 0.3);
}

.toast-warning .toast-icon {
  background: rgba(251, 191, 36, 0.2);
  color: #f59e0b;
}

:global(.dark) .toast-warning .toast-icon {
  background: rgba(251, 191, 36, 0.2);
  color: #fcd34d;
}

.toast-warning .toast-title {
  color: #92400e;
}

:global(.dark) .toast-warning .toast-title {
  color: #fef3c7;
}

.toast-warning .toast-message {
  color: #78350f;
}

:global(.dark) .toast-warning .toast-message {
  color: #fcd34d;
}

.toast-warning .toast-close {
  background: transparent;
  color: #f59e0b;
}

:global(.dark) .toast-warning .toast-close {
  color: #fcd34d;
}

.toast-warning .toast-close:hover {
  background: rgba(251, 191, 36, 0.2);
}

.toast-warning .toast-progress {
  background: linear-gradient(90deg, #f59e0b, #fbbf24);
}

/* Error Toast */
.toast-error {
  background: rgba(254, 242, 242, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(254, 202, 202, 0.6);
}

:global(.dark) .toast-error {
  background: rgba(127, 29, 29, 0.9);
  border: 1px solid rgba(239, 68, 68, 0.3);
}

.toast-error .toast-icon {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}

:global(.dark) .toast-error .toast-icon {
  background: rgba(239, 68, 68, 0.2);
  color: #fca5a5;
}

.toast-error .toast-title {
  color: #991b1b;
}

:global(.dark) .toast-error .toast-title {
  color: #fecaca;
}

.toast-error .toast-message {
  color: #7f1d1d;
}

:global(.dark) .toast-error .toast-message {
  color: #fca5a5;
}

.toast-error .toast-close {
  background: transparent;
  color: #ef4444;
}

:global(.dark) .toast-error .toast-close {
  color: #fca5a5;
}

.toast-error .toast-close:hover {
  background: rgba(239, 68, 68, 0.15);
}

.toast-error .toast-progress {
  background: linear-gradient(90deg, #ef4444, #f87171);
}

/* Animations */
.toast-enter-active {
  animation: toast-in 0.35s cubic-bezier(0.21, 1.02, 0.73, 1);
}

.toast-leave-active {
  animation: toast-out 0.25s cubic-bezier(0.06, 0.71, 0.55, 1) forwards;
}

@keyframes toast-in {
  0% {
    opacity: 0;
    transform: translateX(100%) scale(0.9);
  }
  100% {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
}

@keyframes toast-out {
  0% {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
  100% {
    opacity: 0;
    transform: translateX(100%) scale(0.9);
  }
}
</style>
