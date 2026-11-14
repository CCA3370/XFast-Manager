// @vitest-environment happy-dom

import { createPinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { expect, it } from 'vitest'
import { createI18n } from 'vue-i18n'
import en from '@/i18n/en'
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

it('keeps the scenery enabled and sends only the selected option to installation', async () => {
  const pinia = createPinia()
  const store = useAppStore(pinia)
  const root = 'Airport/NO747LCF'
  store.setCurrentTasks([
    {
      id: 'airport',
      displayName: 'Airport',
      sourcePath: '/downloads/Airport.zip',
      targetPath: '/xplane/Custom Scenery/Airport',
      type: AddonType.Scenery,
      sceneryOptions: [{ root, name: 'NO747LCF', files: ['Earth nav data/apt.dat'] }],
    },
  ])
  const wrapper = mount(ConfirmationModal, {
    global: {
      plugins: [pinia, createI18n({ legacy: false, locale: 'en', messages: { en } })],
      stubs: { teleport: true, AnimatedText: { template: '<slot />' } },
    },
  })

  expect(store.getTasksWithOverwrite()[0]?.selectedSceneryOption).toBeUndefined()
  const select = wrapper.get('select')
  await select.trigger('click')
  await select.setValue(root)
  expect(store.getTaskEnabled('airport')).toBe(true)
  expect(store.getTasksWithOverwrite()[0]?.selectedSceneryOption).toBe(root)

  const defaultOption = select.get('option')
  await select.setValue(defaultOption.element.value)
  expect(store.getTasksWithOverwrite()[0]?.selectedSceneryOption).toBeUndefined()
  wrapper.unmount()
})
