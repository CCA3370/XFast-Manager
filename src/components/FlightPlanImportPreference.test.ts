// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import FlightPlanImportPreference from './FlightPlanImportPreference.vue'

const preferences = vi.hoisted(() => ({
  loadFlightPlanConflictAction: vi.fn(),
  saveFlightPlanConflictAction: vi.fn(),
}))
vi.mock('@/services/flightPlanImport', () => preferences)
beforeEach(() => {
  vi.clearAllMocks()
  preferences.loadFlightPlanConflictAction.mockResolvedValue('replace')
  preferences.saveFlightPlanConflictAction.mockResolvedValue(undefined)
})

describe('flight plan preference settings', () => {
  it('allows a remembered action to be reset to asking every time', async () => {
    const wrapper = mount(FlightPlanImportPreference, { global: { plugins: [i18n] } })
    await flushPromises()
    expect(wrapper.get('select').element.value).toBe('replace')
    await wrapper.get('select').setValue('ask')
    await flushPromises()
    expect(preferences.saveFlightPlanConflictAction).toHaveBeenCalledWith(null)
    wrapper.unmount()
  })

  it('keeps the last saved action and reports a failed save', async () => {
    preferences.saveFlightPlanConflictAction.mockRejectedValue(new Error('denied'))
    const wrapper = mount(FlightPlanImportPreference, { global: { plugins: [i18n] } })
    await flushPromises()
    await wrapper.get('select').setValue('rename')
    await flushPromises()
    expect(wrapper.get('select').element.value).toBe('replace')
    expect(wrapper.find('[role=alert]').exists()).toBe(true)
    wrapper.unmount()
  })
})
