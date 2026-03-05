import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useFeedbackStore = defineStore('feedback', () => {
  const showSubmitModal = ref(false)
  const feedbackModalDirty = ref(false)

  function openSubmitModal() {
    showSubmitModal.value = true
  }

  function closeSubmitModal() {
    showSubmitModal.value = false
    feedbackModalDirty.value = false
  }

  function setModalDirty(dirty: boolean) {
    feedbackModalDirty.value = dirty
  }

  return {
    showSubmitModal,
    feedbackModalDirty,
    openSubmitModal,
    closeSubmitModal,
    setModalDirty,
  }
})
