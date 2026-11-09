// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FlightPlanDetailsModal from './FlightPlanDetailsModal.vue'
import { parsePlan } from '@/services/flightPlanDetails'
const api = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@/components/FlightPlanRouteMap.vue', () => ({
  default: { template: '<div data-map-stub />' },
}))
vi.mock('@tauri-apps/api/core', () => api)
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
beforeEach(() => {
  vi.resetAllMocks()
  api.invoke.mockResolvedValue({ points: [], sources: [], warnings: [] })
})
describe('automatic navigation and separate inspector sections', () => {
  it('does not scan navigation for complete coordinates and separates waypoints from legs', async () => {
    const wrapper = mount(FlightPlanDetailsModal, {
      props: {
        name: 'route.fms',
        root: '/sim',
        plan: parsePlan('I\n1100 Version\nNUMENR 2\n11 A DRCT 10000 0 0\n11 B DRCT 10000 0 1'),
      },
      attachTo: document.body,
    })
    await flushPromises()
    expect(api.invoke).not.toHaveBeenCalled()
    expect(document.querySelector('dialog header')?.textContent).not.toContain('FMS')
    expect(document.querySelector('dialog')?.textContent).not.toContain('flightPlanDetails.navHint')
    expect(document.querySelector('dialog')?.textContent).not.toContain('flightPlanDetails.altHint')

    const tabs = document.querySelectorAll('dialog nav button')
    expect(tabs).toHaveLength(4)
    ;(tabs[1] as HTMLButtonElement).click()
    await flushPromises()
    expect(document.querySelectorAll('dialog table')).toHaveLength(1)
    ;(tabs[2] as HTMLButtonElement).click()
    await flushPromises()
    expect(document.querySelectorAll('dialog table')).toHaveLength(1)
    expect(wrapper.text()).not.toContain('flightPlanDetails.navigation')
    wrapper.unmount()
  })
  it('automatically queries local navigation when a supported point lacks coordinates', async () => {
    const wrapper = mount(FlightPlanDetailsModal, {
      props: {
        name: 'route.fms',
        root: '/sim',
        plan: parsePlan('I\n1100 Version\nNUMENR 1\n11 MISSING DRCT 10000 x y'),
      },
      attachTo: document.body,
    })
    await flushPromises()
    expect(api.invoke).toHaveBeenCalledWith('lookup_flight_plan_navigation', {
      xplanePath: '/sim',
      identifiers: ['MISSING'],
    })
    wrapper.unmount()
  })
})
