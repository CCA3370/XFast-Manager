import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface Toast {
  id: string
  message: string
  type: 'info' | 'success' | 'error' | 'warning'
}

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])

  function show(message: string, type: Toast['type'] = 'info', duration = 5000) {
    const id = Date.now().toString() + Math.random()
    toasts.value.push({ id, message, type })

    setTimeout(() => {
      remove(id)
    }, duration)
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
