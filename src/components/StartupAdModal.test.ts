// @vitest-environment happy-dom
import { DOMWrapper, enableAutoUnmount, flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { i18n } from '@/i18n'
import { setStartupAdSeen, type StartupAd } from '@/services/startupAds'
import StartupAdModal from './StartupAdModal.vue'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@/services/startupAds', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/startupAds')>()),
  setStartupAdSeen: vi.fn(),
}))

const campaign: StartupAd = {
  id: 'campaign-one',
  imageUrl: 'blob:test-image',
  alt: 'An image <with> alternate text',
  url: 'https://example.test/promotion',
}

function render(ad: StartupAd | null = campaign) {
  return mount(StartupAdModal, {
    props: { ad },
    attachTo: document.body,
    global: { plugins: [i18n] },
  })
}

function getDialog() {
  return new DOMWrapper(document.body).get<HTMLDialogElement>('dialog.startup-ad-dialog')
}

enableAutoUnmount(afterEach)

beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined)
  vi.mocked(setStartupAdSeen).mockReset().mockResolvedValue(undefined)
  i18n.global.locale.value = 'en'
})

describe('startup ad dialog', () => {
  it('labels the ad and image, focuses close, and dismisses on Escape without writing a preference', async () => {
    const wrapper = render(null)
    await wrapper.setProps({ ad: campaign })
    await flushPromises()

    const dialog = getDialog()
    expect(dialog.element.open).toBe(true)
    expect(dialog.get('h2').text()).toBe('Advertisement')
    expect(dialog.get('img').attributes('alt')).toBe(campaign.alt)
    expect(document.activeElement).toBe(dialog.get('button[aria-label="Close"]').element)
    expect(dialog.get<HTMLInputElement>('input').element.checked).toBe(false)

    await dialog.trigger('cancel')
    expect(dialog.element.open).toBe(false)
    expect(wrapper.emitted('dismiss')).toHaveLength(1)
    expect(setStartupAdSeen).not.toHaveBeenCalled()
  })

  it('opens the ad only from the image control and dismisses on successful browser launch', async () => {
    const wrapper = render()
    await flushPromises()
    expect(invoke).not.toHaveBeenCalled()

    const imageButton = getDialog().get('button[aria-label^="Open advertisement"]')
    expect(imageButton.attributes('aria-label')).toContain(campaign.alt)
    await imageButton.trigger('click')
    await flushPromises()

    expect(invoke).toHaveBeenCalledExactlyOnceWith('open_url', { url: campaign.url })
    expect(wrapper.emitted('dismiss')).toHaveLength(1)
  })

  it('keeps the dialog available on a failed or invalid destination', async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error('Browser launch failed'))
    const wrapper = render()
    await flushPromises()
    await getDialog().get('button[aria-label^="Open advertisement"]').trigger('click')
    await flushPromises()

    expect(getDialog().get('[role="alert"]').text()).toContain('Couldn’t open')
    expect(wrapper.emitted('dismiss')).toBeUndefined()

    await wrapper.setProps({ ad: { ...campaign, url: 'javascript:alert(1)' } })
    await getDialog().get('button[aria-label^="Open advertisement"]').trigger('click')
    expect(invoke).toHaveBeenCalledTimes(1)
    expect(getDialog().element.open).toBe(true)
  })

  it('saves and reverses the opt-out without closing the dialog', async () => {
    render()
    await flushPromises()
    const checkbox = getDialog().get<HTMLInputElement>('input')

    await checkbox.setValue(true)
    await checkbox.setValue(false)
    expect(setStartupAdSeen).toHaveBeenNthCalledWith(1, campaign.id, true)
    expect(setStartupAdSeen).toHaveBeenNthCalledWith(2, campaign.id, false)
    expect(getDialog().element.open).toBe(true)

    vi.mocked(setStartupAdSeen).mockRejectedValueOnce(new Error('Store write failed'))
    await checkbox.setValue(true)
    await flushPromises()
    expect(checkbox.element.checked).toBe(false)
    expect(getDialog().get('[role="alert"]').text()).toContain('Couldn’t save')
  })
})
