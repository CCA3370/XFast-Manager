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
  it('lists and searches plans without offering or registering imports', async () => {
    mocks.invoke.mockResolvedValue([
      { name: 'route.fms', size: 300, modified: 1 },
      { name: 'other.fms', size: 200, modified: 1 },
    ])
    const wrapper = mount(FlightPlans)
    await flushPromises()
    expect(wrapper.findAll('li')).toHaveLength(2)
    expect(wrapper.find('[data-testid="import-flight-plans"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('flightPlans.drop')
    expect(mocks.listen).not.toHaveBeenCalled()
    expect(mocks.open).not.toHaveBeenCalled()
    await wrapper.get('input').setValue('ROUTE')
    expect(wrapper.findAll('li')).toHaveLength(1)
    expect(wrapper.get('li').text()).toContain('route.fms')
    expect(mocks.invoke.mock.calls.every(([command]) => command === 'list_flight_plans')).toBe(true)
    wrapper.unmount()
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

  it('refreshes the list without importing files', async () => {
    const wrapper = mount(FlightPlans)
    await flushPromises()
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect(
      mocks.invoke.mock.calls.filter(([command]) => command === 'list_flight_plans'),
    ).toHaveLength(2)
    expect(mocks.listen).not.toHaveBeenCalled()
    expect(mocks.open).not.toHaveBeenCalled()
    expect(mocks.invoke).not.toHaveBeenCalledWith('import_flight_plan', expect.anything())
    wrapper.unmount()
  })
})
