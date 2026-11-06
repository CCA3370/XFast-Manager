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
vi.mock('@/components/FlightPlanRouteMap.vue', () => ({
  default: { template: '<div data-map-stub />' },
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
    expect(
      mocks.invoke.mock.calls.every(([command]) =>
        ['list_flight_plans', 'read_flight_plan'].includes(command),
      ),
    ).toBe(true)
    wrapper.unmount()
  })

  it('lazily expands basic information and opens the route details dialog', async () => {
    mocks.invoke.mockImplementation(async (command: string) =>
      command === 'list_flight_plans'
        ? [{ name: 'route.fms', size: 300, modified: 1 }]
        : 'I\n1100 Version\nCYCLE 2611\nADEP KAAA\nADES KBBB\nNUMENR 2\n1 KAAA ADEP 100 30 120\n1 KBBB ADES 100 31 121\n',
    )
    const wrapper = mount(FlightPlans, { attachTo: document.body })
    await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledTimes(2)
    await wrapper.get('li button').trigger('click')
    await flushPromises()
    expect(wrapper.get('li').text()).toContain('KAAA')
    expect(wrapper.get('li button').attributes('aria-expanded')).toBe('true')
    await wrapper.findAll('li button')[1]!.trigger('click')
    await flushPromises()
    expect(document.querySelector('dialog')?.open).toBe(true)
    expect(document.querySelector('dialog')?.textContent).toContain('KBBB')
    wrapper.unmount()
  })

  it('combines airport, cycle and text filters using parsed metadata, with no future-cycle option', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-11-06T12:00:00Z'))
    const fixtures = [
      ['a.fms', 'ZBAA', 'ZSPD', '2611'],
      ['b.fms', 'ZBAA', 'ZGGG', '2610'],
      ['c.fms', 'ZSPD', 'ZBAA', '2611'],
      ['unknown.fms', 'ZBAA', 'ZSPD', ''],
      ['future.fms', 'ZBAA', 'ZSPD', '2612'],
    ]
    mocks.invoke.mockImplementation(async (command: string, args: { name: string }) =>
      command === 'list_flight_plans'
        ? fixtures.map(([name]) => ({ name, size: 100, modified: 1 }))
        : (() => {
            const [, dep, arr, cycle] = fixtures.find((p) => p[0] === args.name)!
            return `I\n1100 Version\nCYCLE ${cycle}\nADEP ${dep}\nADES ${arr}\nNUMENR 0\n`
          })(),
    )
    const wrapper = mount(FlightPlans)
    await flushPromises()
    const selects = wrapper.findAll('select')
    expect(selects[2]!.find('option[value="future"]').exists()).toBe(false)
    await selects[0]!.setValue('ZBAA')
    expect(wrapper.findAll('li')).toHaveLength(4)
    await selects[1]!.setValue('ZSPD')
    expect(wrapper.findAll('li')).toHaveLength(3)
    await selects[2]!.setValue('current')
    expect(wrapper.findAll('li')).toHaveLength(1)
    expect(wrapper.get('li').text()).toContain('a.fms')
    await selects[2]!.setValue('outdated')
    expect(wrapper.findAll('li')).toHaveLength(0)
    await selects[1]!.setValue('')
    expect(wrapper.get('li').text()).toContain('b.fms')
    await selects[1]!.setValue('ZSPD')
    await selects[2]!.setValue('unknown')
    expect(wrapper.findAll('li')).toHaveLength(2)
    await wrapper.get('input').setValue('unknown')
    expect(wrapper.findAll('li')).toHaveLength(1)
    wrapper.unmount()
    vi.useRealTimers()
  })

  it('requires confirmation before deleting a listed plan', async () => {
    mocks.invoke.mockResolvedValue([{ name: 'route.fms', size: 300, modified: 1 }])
    const wrapper = mount(FlightPlans)
    await flushPromises()
    await wrapper.find('[data-testid="delete-plan"]').trigger('click')
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
