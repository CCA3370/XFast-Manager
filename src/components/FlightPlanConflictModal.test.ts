// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import FlightPlanConflictModal from './FlightPlanConflictModal.vue'

const conflict = {
  status: 'conflict' as const,
  name: 'route.fms',
  existingSize: 30,
  incomingSize: 31,
}
function render() {
  i18n.global.locale.value = 'en'
  return mount(FlightPlanConflictModal, {
    props: { conflict },
    attachTo: document.body,
    global: { plugins: [i18n], stubs: { teleport: true } },
  })
}

describe('flight plan conflict dialog', () => {
  it('offers replace, rename and cancel, with remembering initially unchecked', async () => {
    const wrapper = render()
    await flushPromises()
    expect(wrapper.get('dialog').element.open).toBe(true)
    expect(wrapper.get<HTMLInputElement>('input[type=checkbox]').element.checked).toBe(false)
    expect(wrapper.get('[data-testid=replace-flight-plan]').text()).toBe('Replace')
    expect(wrapper.get('[data-testid=cancel-flight-plan]').text()).toBe('Cancel')
    expect(document.activeElement).toBe(wrapper.get('[data-testid=rename-flight-plan]').element)
    await wrapper.get('input').setValue(true)
    await wrapper.get('[data-testid=rename-flight-plan]').trigger('click')
    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'rename', remember: true }])
    wrapper.unmount()
  })

  it('cancels without remembering and resets the checkbox for the next file', async () => {
    const wrapper = render()
    await flushPromises()
    await wrapper.get('input').setValue(true)
    await wrapper.get('[data-testid=cancel-flight-plan]').trigger('click')
    expect(wrapper.emitted('resolve')?.[0]).toEqual([{ action: 'cancel', remember: false }])
    await wrapper.setProps({ conflict: { ...conflict, name: 'next.fms' } })
    await flushPromises()
    expect(wrapper.get<HTMLInputElement>('input').element.checked).toBe(false)
    await wrapper.get('dialog').trigger('cancel')
    expect(wrapper.emitted('resolve')?.[1]).toEqual([{ action: 'cancel', remember: false }])
    wrapper.unmount()
  })
})
