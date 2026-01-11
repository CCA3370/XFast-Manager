import { defineStore } from 'pinia'
import { ref } from 'vue'
import { logger } from '@/services/logger'

export interface Toast {
  id: string
  message: string
  type: 'info' | 'success' | 'error' | 'warning'
}

// Maximum number of toasts to display at once
const MAX_TOASTS = 3
// Toast display duration (matches CSS animation duration) + buffer
const TOAST_DURATION = 3500

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])

  function show(message: string, type: Toast['type'] = 'info') {
    // Remove oldest toasts if we're at the limit
    while (toasts.value.length >= MAX_TOASTS) {
      toasts.value.shift()
    }

    const id = Date.now().toString() + Math.random()
    toasts.value.push({ id, message, type })

    // Fallback timer to ensure toast is removed even if animationend doesn't fire
    // (can happen if animation is interrupted by theme change, DOM updates, etc.)
    setTimeout(() => {
      remove(id)
    }, TOAST_DURATION)

    // Automatically log toast messages
    if (type === 'error') {
      logger.error(`[Toast] ${message}`, 'ui')
    } else {
      logger.info(`[Toast:${type}] ${message}`, 'ui')
    }
  }

  function remove(id: string) {
    const index = toasts.value.findIndex(t => t.id === id)
    if (index !== -1) {
      toasts.value.splice(index, 1)
    }
  }

  function error(message: string) {
    show(message, 'error')
  }

  function success(message: string) {
    show(message, 'success')
  }

  function warning(message: string) {
    show(message, 'warning')
  }

  function info(message: string) {
    show(message, 'info')
  }

  return {
    toasts,
    show,
    remove,
    error,
    success,
    warning,
    info,
  }
})
