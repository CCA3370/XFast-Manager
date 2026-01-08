import { defineStore } from 'pinia'
import { ref } from 'vue'
import { logger } from '@/services/logger'

export const useModalStore = defineStore('modal', () => {
  const errorModal = ref({ visible: false, title: '', message: '' })

  function showError(message: string, title = '') {
    errorModal.value = { visible: true, title, message }
    // Automatically log error modal messages
    logger.error(`[Modal] ${title ? title + ': ' : ''}${message}`, 'ui')
  }

  function closeError() {
    errorModal.value.visible = false
  }

  return { errorModal, showError, closeError }
})
