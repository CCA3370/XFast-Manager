interface ZoomMap {
  getZoom(): number
  getMinZoom(): number
  getMaxZoom(): number
  setZoomAround(point: [number, number], zoom: number, options: { animate: false }): unknown
}
/** Continuous zoom with a replaceable target, rather than queued level animations. */
export function smoothMapZoom(map: ZoomMap, host: HTMLElement) {
  let frame = 0,
    target = map.getZoom(),
    last = 0
  let anchor: [number, number] = [host.clientWidth / 2, host.clientHeight / 2]
  function cancel() {
    if (frame) cancelAnimationFrame(frame)
    frame = 0
    last = 0
    target = map.getZoom()
  }
  function tick(now: number) {
    const current = map.getZoom()
    const delta = last ? Math.min(64, now - last) : 16
    last = now
    const next = current + (target - current) * (1 - Math.exp(-delta / 45))
    const done = Math.abs(target - next) < 0.004
    map.setZoomAround(anchor, done ? target : next, { animate: false })
    frame = done ? 0 : requestAnimationFrame(tick)
    if (done) last = 0
  }
  function step(amount: number, point?: [number, number]) {
    if (!frame) target = map.getZoom()
    anchor = point ?? [host.clientWidth / 2, host.clientHeight / 2]
    target = Math.max(map.getMinZoom(), Math.min(map.getMaxZoom(), target + amount))
    if (!frame) frame = requestAnimationFrame(tick)
  }
  function wheel(event: WheelEvent) {
    event.preventDefault()
    const bounds = host.getBoundingClientRect()
    const pixels =
      event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? host.clientHeight : 1)
    step(Math.max(-2, Math.min(2, -pixels / 100)), [
      event.clientX - bounds.left,
      event.clientY - bounds.top,
    ])
  }
  host.addEventListener('wheel', wheel, { passive: false })
  host.addEventListener('pointerdown', cancel)
  return {
    step,
    cancel,
    dispose() {
      cancel()
      host.removeEventListener('wheel', wheel)
      host.removeEventListener('pointerdown', cancel)
    },
  }
}
