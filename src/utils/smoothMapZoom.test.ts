// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { smoothMapZoom } from './smoothMapZoom'
afterEach(() => vi.unstubAllGlobals())
describe('continuous map zoom', () => {
  it('accepts more input during an unfinished zoom and moves through intermediate frames', () => {
    let zoom = 5,
      id = 0
    const frames = new Map<number, FrameRequestCallback>()
    const rendered: number[] = []
    vi.stubGlobal('requestAnimationFrame', (fn: FrameRequestCallback) => {
      frames.set(++id, fn)
      return id
    })
    vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id))
    const host = document.createElement('div')
    const control = smoothMapZoom(
      {
        getZoom: () => zoom,
        getMinZoom: () => 1,
        getMaxZoom: () => 19,
        setZoomAround: (_point, z) => {
          zoom = z
          rendered.push(z)
        },
      },
      host,
    )
    const advance = (t: number) => {
      const next = [...frames.entries()][0]!
      frames.delete(next[0])
      next[1](t)
    }
    control.step(1)
    advance(16)
    control.step(1)
    for (let t = 32; frames.size && t < 1000; t += 16) advance(t)
    expect(rendered.length).toBeGreaterThan(8)
    expect(zoom).toBe(7)
    expect(rendered.every((z, i) => i === 0 || z >= rendered[i - 1]!)).toBe(true)
    control.step(100)
    for (let t = 1000; frames.size && t < 2000; t += 16) advance(t)
    expect(zoom).toBe(19)
    control.step(-1)
    control.dispose()
    expect(frames.size).toBe(0)
  })
})
