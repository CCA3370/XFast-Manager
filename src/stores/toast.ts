import { defineStore } from 'pinia'
import { ref } from 'vue'
import { logger } from '@/services/logger'

export interface Toast {
  id: string
  message: string
  type: 'info' | 'success' | 'error' | 'warning'
}

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])

  function show(message: string, type: Toast['type'] = 'info') {
    const id = Date.now().toString() + Math.random()
    toasts.value.push({ id, message, type })
    // Toast removal is handled by animationend event in ToastNotification.vue

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
