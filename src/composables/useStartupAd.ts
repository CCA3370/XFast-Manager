import { onBeforeUnmount, shallowRef, watch } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { loadSeenStartupAdIds, loadStartupAd, type StartupAd } from '@/services/startupAds'

const STARTUP_WINDOW_MS = 5_000

interface StartupAdOptions {
  locale: () => string
  canLoad: () => boolean
  canShow: () => boolean
  blocked: () => boolean
}

export function useStartupAd({ locale, canLoad, canShow, blocked }: StartupAdOptions) {
  const ad = shallowRef<StartupAd | null>(null)
  let phase: 'idle' | 'waiting' | 'loading' | 'ready' | 'open' | 'done' = 'idle'
  let candidate: StartupAd | null = null
  let deadline = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let request: AbortController | undefined
  let unlistenDrag: (() => void) | undefined

  function stopWaiting() {
    clearTimeout(timer)
    request?.abort()
    window.removeEventListener('pointerdown', onInput, true)
    window.removeEventListener('keydown', onInput, true)
  }

  function dismiss() {
    if (phase === 'done') return
    phase = 'done'
    stopWaiting()
    window.removeEventListener('dragenter', dismiss, true)
    unlistenDrag?.()
    const image = ad.value ?? candidate
    ad.value = null
    candidate = null
    if (image) URL.revokeObjectURL(image.imageUrl)
  }

  function onInput() {
    if (phase !== 'open') dismiss()
  }

  async function prepare() {
    const controller = new AbortController()
    request = controller
    try {
      const seenIds = await loadSeenStartupAdIds()
      if (controller.signal.aborted) return
      const image = await loadStartupAd({ locale: locale(), seenIds, signal: controller.signal })
      if (controller.signal.aborted) {
        if (image) URL.revokeObjectURL(image.imageUrl)
        return
      }
      if (!image) {
        dismiss()
        return
      }
      candidate = image
      phase = 'ready'
      advance()
    } catch {
      dismiss()
    }
  }

  function advance() {
    if (phase === 'idle' || phase === 'done') return
    if (blocked()) {
      dismiss()
      return
    }
    if (phase === 'waiting' && canLoad()) {
      phase = 'loading'
      void prepare()
    }
    if (phase !== 'ready' || !candidate || !canShow()) return

    if (
      performance.now() >= deadline ||
      document.visibilityState !== 'visible' ||
      !document.hasFocus() ||
      (candidate.endsAt !== undefined && Date.now() >= candidate.endsAt)
    ) {
      dismiss()
      return
    }

    phase = 'open'
    ad.value = candidate
    candidate = null
    stopWaiting()
  }

  function start() {
    if (
      phase !== 'idle' ||
      typeof globalThis.HTMLDialogElement?.prototype.showModal !== 'function'
    ) {
      return
    }
    phase = 'waiting'
    deadline = performance.now() + STARTUP_WINDOW_MS
    timer = setTimeout(dismiss, STARTUP_WINDOW_MS)
    window.addEventListener('pointerdown', onInput, true)
    window.addEventListener('keydown', onInput, true)
    window.addEventListener('dragenter', dismiss, true)
    void Promise.resolve()
      .then(() => {
        if (phase === 'done') return
        return getCurrentWebviewWindow().onDragDropEvent(({ payload }) => {
          if (payload.type !== 'leave') dismiss()
        })
      })
      .then((unlisten) => {
        if (phase === 'done') unlisten?.()
        else unlistenDrag = unlisten
      })
      .catch(() => {
        // Regular browser and keyboard input still cancel the ad.
      })
    advance()
  }

  watch([canLoad, canShow, blocked], advance, { flush: 'sync' })
  onBeforeUnmount(dismiss)

  return { ad, start, dismiss }
}
