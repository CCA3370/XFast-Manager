// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import FlightPlanCharts from './FlightPlanCharts.vue'
import { parsePlan } from '@/services/flightPlanDetails'
vi.mock('@/components/FlightPlanRouteMap.vue', () => ({
  default: { template: '<div data-map-stub />' },
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
describe('flight plan diagrams', () => {
  it('shows the dateline route without drawing through unknown altitude', () => {
    const plan = parsePlan(
      'I\n1100 Version\nNUMENR 3\n11 A DRCT 10000 0 179\n11 B DRCT 0 0 -179\n11 C DRCT 12000 0 -178\n',
    )
    const wrapper = mount(FlightPlanCharts, { props: { points: plan.points } })
    const charts = wrapper.findAll('svg')
    expect(charts).toHaveLength(1)
    expect(charts[0]!.findAll('line')).toHaveLength(0)
    wrapper.unmount()
  })
  it('labels each constant-altitude run once, centered over its full distance', () => {
    const plan = parsePlan(
      'I\n1100 Version\nNUMENR 5\n11 A DRCT 10000 0 0\n11 B DRCT 10000 0 1\n11 C DRCT 10000 0 3\n11 D DRCT 20000 0 4\n11 E DRCT 20000 0 6\n',
    )
    const wrapper = mount(FlightPlanCharts, { props: { points: plan.points } })
    const chart = wrapper.findAll('svg')[0]!
    const labels = chart.findAll('[data-altitude-label]')
    const dots = chart.findAll('circle')
    expect(labels).toHaveLength(2)
    expect(Number(labels[0]!.attributes('x'))).toBeCloseTo(
      (Number(dots[0]!.attributes('cx')) + Number(dots[2]!.attributes('cx'))) / 2,
    )
    expect(Number(labels[1]!.attributes('x'))).toBeCloseTo(
      (Number(dots[3]!.attributes('cx')) + Number(dots[4]!.attributes('cx'))) / 2,
    )
    wrapper.unmount()
  })
})
