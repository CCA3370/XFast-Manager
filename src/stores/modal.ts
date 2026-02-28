import { defineStore } from 'pinia'
import { ref } from 'vue'
import { logger } from '@/services/logger'

/** Options for showing a confirmation modal */
export interface ConfirmOptions {
  title: string
  message: string
  warning?: string
  confirmText: string
  cancelText: string
  type: 'warning' | 'danger'
  onConfirm: () => void
  onCancel: () => void
}

/** State for the error modal */
export interface ErrorModalState {
  visible: boolean
  title: string
  message: string
  hideReport?: boolean
}

/** State for the confirm modal */
export interface ConfirmModalState {
  visible: boolean
  options: ConfirmOptions | null
}

export const useModalStore = defineStore('modal', () => {
  const errorModal = ref<ErrorModalState>({ visible: false, title: '', message: '' })
  const confirmModal = ref<ConfirmModalState>({ visible: false, options: null })

  function showError(message: string, title = '', options?: { hideReport?: boolean }) {
    // Deduplicate error messages by splitting on newlines and removing duplicates
    const lines = message.split('\n').filter((line) => line.trim() !== '')
    const uniqueLines = Array.from(new Set(lines))
    const deduplicatedMessage = uniqueLines.join('\n')

    errorModal.value = { visible: true, title, message: deduplicatedMessage, hideReport: options?.hideReport }
    // Automatically log error modal messages
    logger.error(`[Modal] ${title ? title + ': ' : ''}${deduplicatedMessage}`, 'ui')
  }

  function closeError() {
    errorModal.value.visible = false
  }

  function showConfirm(options: ConfirmOptions) {
    confirmModal.value = { visible: true, options }
  }

  function closeConfirm() {
    confirmModal.value.visible = false
    confirmModal.value.options = null
  }

  function confirmAction() {
    if (confirmModal.value.options?.onConfirm) {
      confirmModal.value.options.onConfirm()
    }
    closeConfirm()
  }

  function cancelAction() {
    if (confirmModal.value.options?.onCancel) {
      confirmModal.value.options.onCancel()
    }
    closeConfirm()
  }

  return {
    errorModal,
    showError,
    closeError,
    confirmModal,
    showConfirm,
    closeConfirm,
    confirmAction,
    cancelAction,
  }
})
