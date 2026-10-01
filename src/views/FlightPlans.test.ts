// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FlightPlans from './FlightPlans.vue'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
  confirm: vi.fn(),
  stop: vi.fn(),
  listen: vi.fn(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: mocks.open }))
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({ onDragDropEvent: mocks.listen }),
}))
vi.mock('@/stores/app', () => ({ useAppStore: () => ({ xplanePath: '/sim' }) }))
vi.mock('@/stores/modal', () => ({ useModalStore: () => ({ showConfirm: mocks.confirm }) }))
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))

beforeEach(() => {
  vi.resetAllMocks()
  mocks.listen.mockResolvedValue(mocks.stop)
  mocks.invoke.mockResolvedValue([])
})

describe('flight plan management', () => {
  it('imports selected files, keeps successful results when another file fails, and refreshes', async () => {
    mocks.open.mockResolvedValue(['/downloads/good.fms', '/downloads/bad.fms'])
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'import_flight_plan' && args.source.endsWith('bad.fms')) {
        throw new Error('Invalid flight plan')
      }
      return command === 'list_flight_plans' ? [] : 'good.fms'
    })
    const wrapper = mount(FlightPlans)
    await flushPromises()
    await wrapper.find('button').trigger('click')
    await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('import_flight_plan', {
      xplanePath: '/sim',
      source: '/downloads/good.fms',
    })
    expect(wrapper.find('[role="status"]').exists()).toBe(true)
    expect(wrapper.find('[role="alert"]').text()).toContain('bad.fms')
    expect(
      mocks.invoke.mock.calls.filter(([command]) => command === 'list_flight_plans'),
    ).toHaveLength(2)
    wrapper.unmount()
    expect(mocks.stop).toHaveBeenCalledOnce()
  })

  it('requires confirmation before deleting a listed plan', async () => {
    mocks.invoke.mockResolvedValue([{ name: 'route.fms', size: 300, modified: 1 }])
    const wrapper = mount(FlightPlans)
    await flushPromises()
    await wrapper.find('li button').trigger('click')
    expect(mocks.invoke).not.toHaveBeenCalledWith('delete_flight_plan', expect.anything())
    const confirmation = mocks.confirm.mock.calls[0][0]
    await confirmation.onCancel()
    expect(mocks.invoke).not.toHaveBeenCalledWith('delete_flight_plan', expect.anything())
    await confirmation.onConfirm()
    expect(mocks.invoke).toHaveBeenCalledWith('delete_flight_plan', {
      xplanePath: '/sim',
      name: 'route.fms',
    })
    wrapper.unmount()
  })

  it('imports native file drops', async () => {
    const wrapper = mount(FlightPlans)
    await flushPromises()
    mocks.listen.mock.calls[0][0]({ payload: { type: 'drop', paths: ['/route.fms'] } })
    await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('import_flight_plan', {
      xplanePath: '/sim',
      source: '/route.fms',
    })
    wrapper.unmount()
  })
})
