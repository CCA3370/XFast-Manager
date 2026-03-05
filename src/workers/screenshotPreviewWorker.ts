type ScreenshotCrop = {
  x: number
  y: number
  width: number
  height: number
}

type ScreenshotEditParams = {
  crop: ScreenshotCrop | null
  rotate: number
  exposure: number
  contrast: number
  saturation: number
  temperature: number
  highlights: number
  shadows: number
  sharpness: number
  denoise: number
}

type InitMessage = { type: 'init'; blob: Blob }
type RenderMessage = {
  type: 'render'
  requestId: number
  params: ScreenshotEditParams
  maxDimension: number
  fast: boolean
}

type WorkerMessage = InitMessage | RenderMessage

type WorkerResponse =
  | { type: 'ready' }
  | { type: 'result'; requestId: number; blob: Blob }
  | { type: 'error'; requestId: number; message: string }

let sourceBitmap: ImageBitmap | null = null

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v))
}

function makeCanvas(width: number, height: number): OffscreenCanvas {
  const w = Math.max(1, Math.round(width))
  const h = Math.max(1, Math.round(height))
  return new OffscreenCanvas(w, h)
}

function normalizeCrop(crop: ScreenshotCrop | null, imgW: number, imgH: number): ScreenshotCrop {
  if (!crop) return { x: 0, y: 0, width: imgW, height: imgH }
  const x = clamp(Math.round(crop.x), 0, imgW - 1)
  const y = clamp(Math.round(crop.y), 0, imgH - 1)
  const width = clamp(Math.round(crop.width), 1, imgW - x)
  const height = clamp(Math.round(crop.height), 1, imgH - y)
  return { x, y, width, height }
}

function applyPixelAdjustments(canvas: OffscreenCanvas, p: ScreenshotEditParams) {
  if (
    p.exposure === 0 &&
    p.contrast === 0 &&
    p.saturation === 0 &&
    p.temperature === 0 &&
    p.highlights === 0 &&
    p.shadows === 0
  ) {
    return
  }
  const ctx = canvas.getContext('2d', { willReadFrequently: true })
  if (!ctx) return
  const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height)
  const d = imageData.data
  const exp = p.exposure / 100
  const con = p.contrast / 100
  const sat = p.saturation / 100
  const temp = p.temperature / 100
  const hi = p.highlights / 100
  const sh = p.shadows / 100
  const expMul = Math.pow(2, exp * 1.5)
  const conMul = 1 + con
  const satMul = 1 + sat

  for (let i = 0; i < d.length; i += 4) {
    let r = d[i] / 255
    let g = d[i + 1] / 255
    let b = d[i + 2] / 255
    r *= expMul
    g *= expMul
    b *= expMul
    r = (r - 0.5) * conMul + 0.5
    g = (g - 0.5) * conMul + 0.5
    b = (b - 0.5) * conMul + 0.5
    const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
    r = luma + (r - luma) * satMul
    g = luma + (g - luma) * satMul
    b = luma + (b - luma) * satMul
    r += temp * 0.08
    b -= temp * 0.08
    if (luma > 0.5) {
      const a = (luma - 0.5) * 2 * hi
      r = a >= 0 ? r + (1 - r) * a : r * (1 + a)
      g = a >= 0 ? g + (1 - g) * a : g * (1 + a)
      b = a >= 0 ? b + (1 - b) * a : b * (1 + a)
    } else {
      const a = (0.5 - luma) * 2 * sh
      r = a >= 0 ? r + (1 - r) * a : r * (1 + a)
      g = a >= 0 ? g + (1 - g) * a : g * (1 + a)
      b = a >= 0 ? b + (1 - b) * a : b * (1 + a)
    }
    d[i] = Math.round(clamp(r, 0, 1) * 255)
    d[i + 1] = Math.round(clamp(g, 0, 1) * 255)
    d[i + 2] = Math.round(clamp(b, 0, 1) * 255)
  }
  ctx.putImageData(imageData, 0, 0)
}

function applyDenoise(canvas: OffscreenCanvas, denoise: number) {
  if (denoise <= 0) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  const source = makeCanvas(canvas.width, canvas.height)
  const sctx = source.getContext('2d')
  if (!sctx) return
  sctx.drawImage(canvas, 0, 0)
  const strength = clamp(denoise / 100, 0, 1)
  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.drawImage(source, 0, 0)
  ctx.filter = `blur(${(strength * 1.8).toFixed(2)}px)`
  ctx.globalAlpha = Math.min(0.65, strength * 0.75)
  ctx.drawImage(source, 0, 0)
  ctx.filter = 'none'
  ctx.globalAlpha = 1
}

function applySharpen(canvas: OffscreenCanvas, sharpness: number) {
  if (sharpness <= 0) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  const { width, height } = canvas
  const srcImage = ctx.getImageData(0, 0, width, height)
  const src = srcImage.data
  const dstImage = ctx.createImageData(width, height)
  const dst = dstImage.data
  const amount = clamp(sharpness / 100, 0, 1)

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const idx = (y * width + x) * 4
      if (x === 0 || y === 0 || x === width - 1 || y === height - 1) {
        dst[idx] = src[idx]
        dst[idx + 1] = src[idx + 1]
        dst[idx + 2] = src[idx + 2]
        dst[idx + 3] = src[idx + 3]
        continue
      }
      const left = idx - 4
      const right = idx + 4
      const top = idx - width * 4
      const bottom = idx + width * 4
      for (let c = 0; c < 3; c += 1) {
        const base = src[idx + c]
        const edge = base * 5 - src[left + c] - src[right + c] - src[top + c] - src[bottom + c]
        dst[idx + c] = Math.round(clamp(base * (1 - amount) + edge * amount, 0, 255))
      }
      dst[idx + 3] = src[idx + 3]
    }
  }
  ctx.putImageData(dstImage, 0, 0)
}

function processImage(
  source: ImageBitmap,
  params: ScreenshotEditParams,
  maxDimension: number,
): OffscreenCanvas {
  const crop = normalizeCrop(params.crop, source.width, source.height)
  const base = makeCanvas(crop.width, crop.height)
  const bctx = base.getContext('2d')
  if (bctx) {
    bctx.drawImage(source, crop.x, crop.y, crop.width, crop.height, 0, 0, crop.width, crop.height)
  }

  let work: OffscreenCanvas = base
  if (params.rotate !== 0) {
    const angleRad = Math.abs((params.rotate * Math.PI) / 180)
    const W = base.width
    const H = base.height
    let scale = Math.cos(angleRad) + Math.sin(angleRad) * Math.max(W / H, H / W)
    scale *= 1.002

    const rot = makeCanvas(W, H)
    const rctx = rot.getContext('2d')
    if (rctx) {
      rctx.translate(W / 2, H / 2)
      rctx.rotate((params.rotate * Math.PI) / 180)
      rctx.scale(scale, scale)
      rctx.drawImage(base, -W / 2, -H / 2)
      work = rot
    }
  }

  applyPixelAdjustments(work, params)
  applyDenoise(work, params.denoise)
  applySharpen(work, params.sharpness)

  const side = Math.max(work.width, work.height)
  if (!maxDimension || side <= maxDimension) return work

  const ratio = maxDimension / side
  const scaled = makeCanvas(work.width * ratio, work.height * ratio)
  const sctx = scaled.getContext('2d')
  if (sctx) sctx.drawImage(work, 0, 0, scaled.width, scaled.height)
  return scaled
}

async function onInit(blob: Blob) {
  if (sourceBitmap) {
    sourceBitmap.close()
    sourceBitmap = null
  }
  sourceBitmap = await createImageBitmap(blob)
  const payload: WorkerResponse = { type: 'ready' }
  self.postMessage(payload)
}

async function onRender(message: RenderMessage) {
  if (!sourceBitmap) throw new Error('worker_not_initialized')
  const canvas = processImage(sourceBitmap, message.params, message.maxDimension)
  const blob = await canvas.convertToBlob({
    type: message.fast ? 'image/jpeg' : 'image/png',
    quality: message.fast ? 0.8 : 1,
  })
  const payload: WorkerResponse = { type: 'result', requestId: message.requestId, blob }
  self.postMessage(payload)
}

self.onmessage = async (ev: MessageEvent<WorkerMessage>) => {
  const message = ev.data
  try {
    if (message.type === 'init') {
      await onInit(message.blob)
      return
    }
    if (message.type === 'render') {
      await onRender(message)
    }
  } catch (error) {
    const payload: WorkerResponse = {
      type: 'error',
      requestId: message.type === 'render' ? message.requestId : -1,
      message: error instanceof Error ? error.message : String(error),
    }
    self.postMessage(payload)
  }
}
