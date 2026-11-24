// @vitest-environment happy-dom
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h, ref } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useStartupAd } from './useStartupAd'
import type { StartupAd } from '@/services/startupAds'

const mocks = vi.hoisted(() => ({
  listenForDrag: vi.fn(),
  unlistenDrag: vi.fn(),
  loadSeenStartupAdIds: vi.fn<() => Promise<readonly string[]>>(),
  loadStartupAd: vi.fn<() => Promise<StartupAd | null>>(),
}))

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({ onDragDropEvent: mocks.listenForDrag }),
}))
vi.mock('@/services/startupAds', () => ({
  loadSeenStartupAdIds: mocks.loadSeenStartupAdIds,
  loadStartupAd: mocks.loadStartupAd,
}))

const creative: StartupAd = {
  id: 'test-ad',
  alt: 'Test ad',
  url: 'https://example.com/',
  imageUrl: 'blob:startup-ad',
}

let wrapper: VueWrapper | undefined

function setup({ canLoad = true, canShow = true, blocked = false } = {}) {
  const readyToLoad = ref(canLoad)
  const readyToShow = ref(canShow)
  const busy = ref(blocked)
  let startup!: ReturnType<typeof useStartupAd>
  wrapper = mount(
    defineComponent({
      setup() {
        startup = useStartupAd({
          locale: () => 'en',
          canLoad: () => readyToLoad.value,
          canShow: () => readyToShow.value,
          blocked: () => busy.value,
        })
        return () => h('div')
      },
    }),
  )
  startup.start()
  return { ...startup, readyToLoad, readyToShow, busy }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(document, 'hasFocus').mockReturnValue(true)
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  mocks.listenForDrag.mockResolvedValue(mocks.unlistenDrag)
  mocks.loadSeenStartupAdIds.mockResolvedValue(['previous-ad'])
  mocks.loadStartupAd.mockResolvedValue(creative)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  vi.useRealTimers()
})

describe('startup ad opportunity', () => {
  it('loads after the file check, opens after the changelog check, and dismisses once', async () => {
    const startup = setup({ canLoad: false, canShow: false })
    await flushPromises()
    expect(mocks.loadStartupAd).not.toHaveBeenCalled()

    startup.readyToLoad.value = true
    await flushPromises()
    expect(mocks.loadStartupAd).toHaveBeenCalledWith({
      locale: 'en',
      seenIds: ['previous-ad'],
      signal: expect.any(AbortSignal),
    })
    expect(startup.ad.value).toBeNull()

    startup.readyToShow.value = true
    expect(startup.ad.value).toEqual(creative)
    startup.dismiss()
    startup.start()
    expect(startup.ad.value).toBeNull()
    expect(mocks.loadStartupAd).toHaveBeenCalledTimes(1)
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(creative.imageUrl)
  })

  it('skips an onboarding or file launch without a network request', async () => {
    const startup = setup({ blocked: true })
    startup.busy.value = false
    await flushPromises()

    expect(mocks.loadStartupAd).not.toHaveBeenCalled()
    expect(startup.ad.value).toBeNull()
  })

  it('skips the ad when the WebView lacks native modal support', async () => {
    vi.stubGlobal('HTMLDialogElement', undefined)
    const startup = setup()
    await flushPromises()

    expect(mocks.loadStartupAd).not.toHaveBeenCalled()
    expect(startup.ad.value).toBeNull()
  })

  it('discards an arriving image after the user starts interacting', async () => {
    let finish!: (ad: StartupAd) => void
    mocks.loadStartupAd.mockReturnValueOnce(new Promise((resolve) => (finish = resolve)))
    const startup = setup()
    await flushPromises()
    window.dispatchEvent(new Event('pointerdown'))
    finish(creative)
    await flushPromises()

    expect(startup.ad.value).toBeNull()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(creative.imageUrl)
  })

  it('discards a ready image after the startup deadline', async () => {
    vi.useFakeTimers()
    const startup = setup({ canShow: false })
    await vi.advanceTimersByTimeAsync(5_000)
    startup.readyToShow.value = true

    expect(startup.ad.value).toBeNull()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(creative.imageUrl)
  })

  it('discards a ready image when its campaign expires while waiting', async () => {
    vi.useFakeTimers()
    mocks.loadStartupAd.mockResolvedValueOnce({ ...creative, endsAt: Date.now() + 100 })
    const startup = setup({ canShow: false })
    await vi.advanceTimersByTimeAsync(100)
    startup.readyToShow.value = true

    expect(startup.ad.value).toBeNull()
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(creative.imageUrl)
  })

  it.each(['file drag', 'business dialog'])('closes an open ad for %s', async (blocker) => {
    const startup = setup()
    await flushPromises()
    expect(startup.ad.value).toEqual(creative)

    if (blocker === 'file drag') {
      const [onDrag] = mocks.listenForDrag.mock.calls[0]
      onDrag({ payload: { type: 'enter' } })
    } else {
      startup.busy.value = true
    }
    expect(startup.ad.value).toBeNull()
    expect(mocks.unlistenDrag).toHaveBeenCalledTimes(1)
  })
})
