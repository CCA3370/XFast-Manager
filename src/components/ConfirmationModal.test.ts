// @vitest-environment happy-dom

import { createPinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { expect, it } from 'vitest'
import { useAppStore } from '@/stores/app'
import { AddonType, type InstallTask } from '@/types'
import ConfirmationModal from './ConfirmationModal.vue'

it('asks before installing an enabled standalone XLua runtime', async () => {
  const pinia = createPinia()
  const store = useAppStore(pinia)
  const task: InstallTask = {
    id: 'xlua',
    displayName: 'xlua',
    sourcePath: '/downloads/xlua.zip',
    targetPath: '/xplane/Resources/plugins/xlua',
    type: AddonType.Plugin,
    isXluaRuntime: true,
  }
  store.setCurrentTasks([
    task,
    {
      ...task,
      id: 'other',
      displayName: 'Other Plugin',
      targetPath: '/xplane/Resources/plugins/other',
      isXluaRuntime: false,
    },
  ])
  const wrapper = mount(ConfirmationModal, {
    global: {
      plugins: [pinia],
      stubs: { teleport: true, TaskListSection: true, AnimatedText: { template: '<slot />' } },
      mocks: { $t: (key: string) => key },
    },
  })
  const button = (label: string) =>
    wrapper.findAll('button').find((candidate) => candidate.text() === label)!

  expect(wrapper.get('[role="alert"]').text()).toBe('modal.xluaInstallWarning')
  await button('common.cancel').trigger('click')
  expect(wrapper.emitted('close')).toHaveLength(1)
  expect(wrapper.emitted('confirm')).toBeUndefined()

  await button('modal.installAnyway').trigger('click')
  expect(wrapper.emitted('confirm')).toHaveLength(1)

  store.setTaskEnabled('xlua', false)
  await wrapper.vm.$nextTick()
  expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  expect(button('modal.startInstallation').attributes('disabled')).toBeUndefined()
  wrapper.unmount()
})
