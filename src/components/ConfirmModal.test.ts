// @vitest-environment happy-dom

import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { expect, it, vi } from 'vitest'
import { createI18n } from 'vue-i18n'
import { useModalStore } from '@/stores/modal'
import ConfirmModal from './ConfirmModal.vue'

it.each([false, undefined])('honors closeOnBackdrop=%s', async (closeOnBackdrop) => {
  const pinia = createPinia()
  const modal = useModalStore(pinia)
  const onCancel = vi.fn()
  const onConfirm = vi.fn()
  modal.showConfirm({
    title: 'Install as an aircraft patch?',
    message: 'Select how to use the archive.',
    confirmText: 'Install as patch',
    cancelText: 'Cancel',
    type: 'warning',
    closeOnBackdrop,
    onConfirm,
    onCancel,
  })
  const wrapper = mount(ConfirmModal, {
    global: {
      plugins: [pinia, createI18n({ legacy: false, locale: 'en' })],
      stubs: { teleport: true },
    },
  })
  await wrapper.get('.modal-backdrop').trigger('click')
  if (closeOnBackdrop === false) {
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
    expect(onCancel).not.toHaveBeenCalled()
    const cancel = wrapper.findAll('button').find((button) => button.text() === 'Cancel')!
    await cancel.trigger('click')
  }
  expect(onConfirm).not.toHaveBeenCalled()
  expect(onCancel).toHaveBeenCalledTimes(1)
  expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
  wrapper.unmount()
})
