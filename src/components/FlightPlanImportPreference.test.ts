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
    // The custom dropdown shows the currently selected label as a button.
    const trigger = wrapper.get('button[aria-label="Flight plan name conflicts"]')
    expect(trigger.text()).toContain('Replace')
    // Open the dropdown and pick the "Ask every time" option.
    await trigger.trigger('click')
    await flushPromises()
    const options = wrapper.findAll('ul button')
    const askOption = options.find((o) => o.text().includes('Ask every time'))
    expect(askOption).toBeTruthy()
    await askOption!.trigger('click')
    await flushPromises()
    expect(preferences.saveFlightPlanConflictAction).toHaveBeenCalledWith(null)
    wrapper.unmount()
  })

  it('keeps the last saved action and reports a failed save', async () => {
    preferences.saveFlightPlanConflictAction.mockRejectedValue(new Error('denied'))
    const wrapper = mount(FlightPlanImportPreference, { global: { plugins: [i18n] } })
    await flushPromises()
    // Open the dropdown and pick the "Rename" option.
    const trigger = wrapper.get('button[aria-label="Flight plan name conflicts"]')
    expect(trigger.text()).toContain('Replace')
    await trigger.trigger('click')
    await flushPromises()
    const options = wrapper.findAll('ul button')
    const renameOption = options.find((o) => o.text().includes('Rename'))
    expect(renameOption).toBeTruthy()
    await renameOption!.trigger('click')
    await flushPromises()
    // On a failed save the displayed value falls back to the last saved action.
    expect(wrapper.get('button[aria-label="Flight plan name conflicts"]').text()).toContain(
      'Replace',
    )
    expect(wrapper.find('[role=alert]').exists()).toBe(true)
    wrapper.unmount()
  })
})
