<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import type {
  ScreenshotEditParams,
  ScreenshotMediaItem,
  ScreenshotMediaType,
} from '@/types'

type SortKey = 'time' | 'name' | 'size'
type SliderKey =
  | 'exposure'
  | 'contrast'
  | 'saturation'
  | 'temperature'
  | 'highlights'
  | 'shadows'
  | 'sharpness'
  | 'denoise'
type EditorTool = 'crop' | 'rotate' | SliderKey

interface ScreenshotOperationResult {
  outputPath: string
}

interface EnhancedPreviewState {
  previewUrl: string
  fullBytes: number[]
  mime: string
}

const sliders: ReadonlyArray<{ key: SliderKey; label: string; min: number; max: number }> = [
  { key: 'exposure', label: 'screenshot.exposure', min: -100, max: 100 },
  { key: 'contrast', label: 'screenshot.contrast', min: -100, max: 100 },
  { key: 'saturation', label: 'screenshot.saturation', min: -100, max: 100 },
  { key: 'temperature', label: 'screenshot.temperature', min: -100, max: 100 },
  { key: 'highlights', label: 'screenshot.highlights', min: -100, max: 100 },
  { key: 'shadows', label: 'screenshot.shadows', min: -100, max: 100 },
  { key: 'sharpness', label: 'screenshot.sharpness', min: 0, max: 100 },
  { key: 'denoise', label: 'screenshot.denoise', min: 0, max: 100 },
]

const editorTools: ReadonlyArray<{ key: EditorTool; label: string }> = [
  { key: 'crop', label: 'screenshot.crop' },
  { key: 'rotate', label: 'screenshot.rotate' },
  { key: 'exposure', label: 'screenshot.exposure' },
  { key: 'contrast', label: 'screenshot.contrast' },
  { key: 'saturation', label: 'screenshot.saturation' },
  { key: 'temperature', label: 'screenshot.temperature' },
  { key: 'highlights', label: 'screenshot.highlights' },
  { key: 'shadows', label: 'screenshot.shadows' },
  { key: 'sharpness', label: 'screenshot.sharpness' },
  { key: 'denoise', label: 'screenshot.denoise' },
]

const sliderLookup = new Map(sliders.map((s) => [s.key, s]))

const { t } = useI18n()
const appStore = useAppStore()
const toastStore = useToastStore()
const modalStore = useModalStore()

const loading = ref(false)
const error = ref('')
const items = ref<ScreenshotMediaItem[]>([])
const q = ref('')
const filterType = ref<'all' | ScreenshotMediaType>('all')
const sort = ref<SortKey>('time')
const refreshKey = ref(Date.now())

const selected = ref<ScreenshotMediaItem | null>(null)
const shareMenuOpen = ref(false)
const thumbFailed = ref<Set<string>>(new Set())
const busyFile = ref<string | null>(null)
const autoEnhanceBusyFile = ref<string | null>(null)
const enhancedPreviews = reactive(new Map<string, EnhancedPreviewState>())

const editorOpen = ref(false)
const editorItem = ref<ScreenshotMediaItem | null>(null)
const editorImage = ref<HTMLImageElement | null>(null)
const editorInputUrl = ref('')
const editorPreviewUrl = ref('')
const editorError = ref('')
const previewBusy = ref(false)
const saveBusy = ref(false)
const bounds = ref({ width: 0, height: 0 })
const edit = ref<ScreenshotEditParams>(defaultEditParams())
const activeTool = ref<EditorTool | null>(null)
let previewTimer: number | null = null
let previewRenderSeq = 0

const cropX = computed({
  get: () => edit.value.crop?.x ?? 0,
  set: (v: number) => {
    if (edit.value.crop) edit.value.crop.x = v
  },
})
const cropY = computed({
  get: () => edit.value.crop?.y ?? 0,
  set: (v: number) => {
    if (edit.value.crop) edit.value.crop.y = v
  },
})
const cropW = computed({
  get: () => edit.value.crop?.width ?? 1,
  set: (v: number) => {
    if (edit.value.crop) edit.value.crop.width = v
  },
})
const cropH = computed({
  get: () => edit.value.crop?.height ?? 1,
  set: (v: number) => {
    if (edit.value.crop) edit.value.crop.height = v
  },
})

const filtered = computed(() => {
  const keyword = q.value.trim().toLowerCase()
  const list = items.value.filter((it) => {
    if (filterType.value !== 'all' && it.mediaType !== filterType.value) return false
    if (!keyword) return true
    return it.name.toLowerCase().includes(keyword)
  })
  list.sort((a, b) => {
    if (sort.value === 'name') return a.name.toLowerCase().localeCompare(b.name.toLowerCase())
    if (sort.value === 'size') return b.size - a.size
    return b.modifiedAt - a.modifiedAt
  })
  return list
})

const selectedSrc = computed(() => {
  if (!selected.value) return ''
  if (selected.value.mediaType === 'image') {
    const enhanced = enhancedPreviews.get(selected.value.fileName)
    if (enhanced?.previewUrl) return enhanced.previewUrl
  }
  return toSrc(selected.value.path)
})
const editorSrc = computed(() => {
  if (editorPreviewUrl.value) return editorPreviewUrl.value
  return editorInputUrl.value
})

const activeSlider = computed(() => {
  if (!activeTool.value || activeTool.value === 'crop' || activeTool.value === 'rotate') return null
  return sliderLookup.get(activeTool.value as SliderKey) ?? null
})

const activeSliderValue = computed({
  get: () => {
    if (!activeSlider.value) return 0
    return edit.value[activeSlider.value.key] as number
  },
  set: (v: number) => {
    if (!activeSlider.value) return
    edit.value[activeSlider.value.key] = v
  },
})

function defaultEditParams(width = 0, height = 0): ScreenshotEditParams {
  return {
    crop:
      width > 0 && height > 0
        ? { x: 0, y: 0, width, height }
        : { x: 0, y: 0, width: 1, height: 1 },
    rotate: 0,
    exposure: 0,
    contrast: 0,
    saturation: 0,
    temperature: 0,
    highlights: 0,
    shadows: 0,
    sharpness: 0,
    denoise: 0,
  }
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v))
}

function toSrc(path: string): string {
  return `${convertFileSrc(path)}?r=${refreshKey.value}`
}

function fmtSize(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let i = 0
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024
    i += 1
  }
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}

function fmtTime(ts: number): string {
  if (!ts) return '-'
  return new Date(ts * 1000).toLocaleString()
}

function markThumbFailed(fileName: string) {
  const next = new Set(thumbFailed.value)
  next.add(fileName)
  thumbFailed.value = next
}

function isThumbFailed(fileName: string): boolean {
  return thumbFailed.value.has(fileName)
}

function bytesToBlob(bytes: number[], mime: string): Blob {
  return new Blob([new Uint8Array(bytes)], { type: mime })
}

function loadImageFromBlob(blob: Blob): Promise<HTMLImageElement> {
  const url = URL.createObjectURL(blob)
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => {
      URL.revokeObjectURL(url)
      resolve(img)
    }
    img.onerror = () => {
      URL.revokeObjectURL(url)
      reject(new Error('load_failed'))
    }
    img.src = url
  })
}

function lumaPercentile(hist: Uint32Array, total: number, percentile: number): number {
  if (total <= 0) return 0
  const target = Math.max(0, Math.min(1, percentile)) * total
  let acc = 0
  for (let i = 0; i < hist.length; i += 1) {
    acc += hist[i]
    if (acc >= target) return i / 255
  }
  return 1
}

function analyzeForAutoEnhance(source: HTMLImageElement): {
  mean: number
  darkRatio: number
  brightRatio: number
  range: number
  satMean: number
  tempCast: number
  p95: number
} {
  const maxSide = 320
  const longSide = Math.max(source.naturalWidth, source.naturalHeight)
  const ratio = longSide > maxSide ? maxSide / longSide : 1
  const width = Math.max(2, Math.round(source.naturalWidth * ratio))
  const height = Math.max(2, Math.round(source.naturalHeight * ratio))
  const canvas = makeCanvas(width, height)
  const ctx = canvas.getContext('2d')
  if (!ctx) {
    return {
      mean: 0.45,
      darkRatio: 0.2,
      brightRatio: 0.2,
      range: 0.25,
      satMean: 0.28,
      tempCast: 0,
      p95: 0.9,
    }
  }
  ctx.drawImage(source, 0, 0, width, height)
  const data = ctx.getImageData(0, 0, width, height).data
  const total = width * height
  const hist = new Uint32Array(256)
  let lumaSum = 0
  let dark = 0
  let bright = 0
  let satSum = 0
  let rSum = 0
  let bSum = 0

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i] / 255
    const g = data[i + 1] / 255
    const b = data[i + 2] / 255
    const l = clamp(0.2126 * r + 0.7152 * g + 0.0722 * b, 0, 1)
    const maxRgb = Math.max(r, g, b)
    const minRgb = Math.min(r, g, b)
    const sat = maxRgb > 1e-6 ? (maxRgb - minRgb) / maxRgb : 0
    lumaSum += l
    satSum += sat
    rSum += r
    bSum += b
    hist[Math.round(l * 255)] += 1
    if (l < 0.2) dark += 1
    if (l > 0.8) bright += 1
  }

  const p05 = lumaPercentile(hist, total, 0.05)
  const p95 = lumaPercentile(hist, total, 0.95)
  const rMean = rSum / total
  const bMean = bSum / total
  return {
    mean: lumaSum / total,
    darkRatio: dark / total,
    brightRatio: bright / total,
    range: Math.max(0.01, p95 - p05),
    satMean: satSum / total,
    tempCast: (rMean - bMean) / Math.max(0.01, (rMean + bMean) * 0.5),
    p95,
  }
}

function recommendAutoEnhanceParams(source: HTMLImageElement): ScreenshotEditParams {
  const params = defaultEditParams(source.naturalWidth, source.naturalHeight)
  const m = analyzeForAutoEnhance(source)

  params.exposure = clamp(Math.round((0.43 - m.mean) * 34), -8, 6)
  if (m.brightRatio > 0.34) params.exposure = clamp(params.exposure - 2, -8, 6)
  if (m.darkRatio > 0.46) params.exposure = clamp(params.exposure + 2, -8, 6)

  params.contrast = clamp(Math.round(18 + (0.3 - m.range) * 44), 12, 32)
  params.saturation = clamp(Math.round(20 + (0.3 - m.satMean) * 34), 16, 30)
  params.temperature = clamp(Math.round(-m.tempCast * 44), -8, 8)
  params.highlights = clamp(
    Math.round(-48 - Math.max(0, m.brightRatio - 0.28) * 52 - Math.max(0, m.p95 - 0.9) * 120),
    -78,
    -30,
  )
  params.shadows = clamp(
    Math.round(4 + Math.max(0, 0.35 - m.darkRatio) * 18 + Math.max(0, 0.38 - m.mean) * 16),
    -6,
    14,
  )
  params.sharpness = 14
  params.denoise = 4
  return params
}

function setEnhancedPreview(fileName: string, state: EnhancedPreviewState) {
  const previous = enhancedPreviews.get(fileName)
  if (previous?.previewUrl && previous.previewUrl !== state.previewUrl) {
    URL.revokeObjectURL(previous.previewUrl)
  }
  enhancedPreviews.set(fileName, state)
}

function clearEnhancedPreview(fileName: string) {
  const previous = enhancedPreviews.get(fileName)
  if (previous?.previewUrl) URL.revokeObjectURL(previous.previewUrl)
  enhancedPreviews.delete(fileName)
}

function clearAllEnhancedPreviews() {
  for (const key of Array.from(enhancedPreviews.keys())) {
    clearEnhancedPreview(key)
  }
}

function pruneEnhancedPreviews() {
  const current = new Set(items.value.map((it) => it.fileName))
  for (const key of Array.from(enhancedPreviews.keys())) {
    if (!current.has(key)) clearEnhancedPreview(key)
  }
}

function hasEnhancedPreview(fileName: string): boolean {
  return enhancedPreviews.has(fileName)
}

async function load() {
  if (!appStore.xplanePath) {
    items.value = []
    clearAllEnhancedPreviews()
    return
  }
  loading.value = true
  error.value = ''
  try {
    items.value = await invoke<ScreenshotMediaItem[]>('list_screenshot_media', {
      xplanePath: appStore.xplanePath,
    })
    refreshKey.value = Date.now()
    pruneEnhancedPreviews()
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function copyImageBlob(blob: Blob, fallbackText: string) {
  const ctor = (window as any).ClipboardItem
  if (ctor && navigator.clipboard?.write) {
    await navigator.clipboard.write([new ctor({ [blob.type || 'image/png']: blob })])
    return
  }
  await navigator.clipboard.writeText(fallbackText)
}

async function getRawImageBlob(item: ScreenshotMediaItem): Promise<Blob> {
  const bytes = await invoke<number[]>('read_screenshot_media_bytes', {
    xplanePath: appStore.xplanePath,
    fileName: item.fileName,
  })
  return bytesToBlob(bytes, extToMime(item.ext))
}

async function copyMedia(item: ScreenshotMediaItem) {
  try {
    if (item.mediaType === 'image') {
      const enhanced = enhancedPreviews.get(item.fileName)
      const blob = enhanced
        ? bytesToBlob(enhanced.fullBytes, enhanced.mime || 'image/png')
        : await getRawImageBlob(item)
      await copyImageBlob(blob, item.path)
    } else {
      await navigator.clipboard.writeText(item.path)
    }
    toastStore.success(t('screenshot.copySuccess') as string)
  } catch {
    toastStore.error(
      (item.mediaType === 'image'
        ? t('screenshot.copyImageFailed')
        : t('screenshot.copyPathFailed')) as string,
    )
  }
}

async function saveAs(item: ScreenshotMediaItem) {
  if (!appStore.xplanePath) return
  const path = await save({ defaultPath: item.name })
  if (!path || Array.isArray(path)) return
  busyFile.value = item.fileName
  try {
    const enhanced = item.mediaType === 'image' ? enhancedPreviews.get(item.fileName) : null
    if (enhanced) {
      await invoke<ScreenshotOperationResult>('save_edited_screenshot_image', {
        xplanePath: appStore.xplanePath,
        fileName: item.fileName,
        bytes: enhanced.fullBytes,
        targetPath: path,
      })
    } else {
      await invoke<ScreenshotOperationResult>('save_screenshot_media_as', {
        xplanePath: appStore.xplanePath,
        fileName: item.fileName,
        targetPath: path,
      })
    }
    toastStore.success(t('screenshot.saveAsSuccess') as string)
  } catch (e) {
    modalStore.showError(`${t('screenshot.saveAsFailed')}: ${String(e)}`)
  } finally {
    busyFile.value = null
  }
}

async function autoEnhance(item: ScreenshotMediaItem) {
  if (!appStore.xplanePath || item.mediaType !== 'image') return
  autoEnhanceBusyFile.value = item.fileName
  try {
    const sourceBlob = await getRawImageBlob(item)
    const sourceImage = await loadImageFromBlob(sourceBlob)
    const params = recommendAutoEnhanceParams(sourceImage)

    const previewCanvas = processImage(sourceImage, params, 1600)
    const previewBlob = await canvasToBlob(previewCanvas, 'image/jpeg', 0.9)

    const fullMime = extToMime(item.ext)
    const fullCanvas = processImage(sourceImage, params)
    const fullQuality = fullMime === 'image/jpeg' || fullMime === 'image/webp' ? 0.95 : undefined
    const fullBlob = await canvasToBlob(fullCanvas, fullMime, fullQuality)
    const fullBytes = Array.from(new Uint8Array(await fullBlob.arrayBuffer()))

    const previewUrl = URL.createObjectURL(previewBlob)
    setEnhancedPreview(item.fileName, {
      previewUrl,
      fullBytes,
      mime: fullMime,
    })
    toastStore.success(t('screenshot.autoEnhanceApplied') as string)
  } catch (e) {
    modalStore.showError(`Failed to auto enhance screenshot: ${String(e)}`)
  } finally {
    autoEnhanceBusyFile.value = null
  }
}

function askDelete(item: ScreenshotMediaItem) {
  modalStore.showConfirm({
    title: t('screenshot.deleteConfirmTitle') as string,
    message: t('screenshot.deleteConfirmMessage') as string,
    warning: t('screenshot.deleteConfirmWarning') as string,
    confirmText: t('common.delete') as string,
    cancelText: t('common.cancel') as string,
    type: 'danger',
    onConfirm: () => {
      void deleteMedia(item)
    },
    onCancel: () => {},
  })
}

async function deleteMedia(item: ScreenshotMediaItem) {
  if (!appStore.xplanePath) return
  busyFile.value = item.fileName
  try {
    await invoke('delete_screenshot_media', {
      xplanePath: appStore.xplanePath,
      fileName: item.fileName,
      preferTrash: true,
    })
    clearEnhancedPreview(item.fileName)
    if (selected.value?.fileName === item.fileName) selected.value = null
    if (editorItem.value?.fileName === item.fileName) closeEditor()
    toastStore.success(t('screenshot.deleteSuccess') as string)
    await load()
  } catch (e) {
    modalStore.showError(`${t('screenshot.deleteFailed')}: ${String(e)}`)
  } finally {
    busyFile.value = null
  }
}

async function share(item: ScreenshotMediaItem) {
  if (!appStore.xplanePath) return
  try {
    const url = await invoke<string>('build_reddit_share_url', {
      xplanePath: appStore.xplanePath,
      fileName: item.fileName,
      title: t('screenshot.redditTitle'),
      mode: item.mediaType === 'video' ? 'video' : 'self',
    })
    await invoke('open_url', { url })
  } catch (e) {
    modalStore.showError(String(e))
  }
}

function normalizeCrop(
  crop: ScreenshotEditParams['crop'],
  width: number,
  height: number,
): { x: number; y: number; width: number; height: number } {
  const c = crop ?? { x: 0, y: 0, width, height }
  const x = clamp(Math.floor(c.x), 0, Math.max(0, width - 1))
  const y = clamp(Math.floor(c.y), 0, Math.max(0, height - 1))
  const w = clamp(Math.floor(c.width), 1, width - x)
  const h = clamp(Math.floor(c.height), 1, height - y)
  return { x, y, width: w, height: h }
}

function makeCanvas(width: number, height: number): HTMLCanvasElement {
  const c = document.createElement('canvas')
  c.width = Math.max(1, Math.round(width))
  c.height = Math.max(1, Math.round(height))
  return c
}

function applyPixelAdjustments(canvas: HTMLCanvasElement, p: ScreenshotEditParams) {
  const ctx = canvas.getContext('2d')
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

function applyDenoise(canvas: HTMLCanvasElement, denoise: number) {
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

function applySharpen(canvas: HTMLCanvasElement, sharpness: number) {
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
  source: HTMLImageElement,
  params: ScreenshotEditParams,
  maxDimension?: number,
): HTMLCanvasElement {
  const crop = normalizeCrop(params.crop, source.naturalWidth, source.naturalHeight)
  const base = makeCanvas(crop.width, crop.height)
  const bctx = base.getContext('2d')
  if (bctx) {
    bctx.drawImage(source, crop.x, crop.y, crop.width, crop.height, 0, 0, crop.width, crop.height)
  }
  let work = base
  if (params.rotate !== 0) {
    const swap = params.rotate === 90 || params.rotate === 270
    const rot = makeCanvas(swap ? base.height : base.width, swap ? base.width : base.height)
    const rctx = rot.getContext('2d')
    if (rctx) {
      rctx.translate(rot.width / 2, rot.height / 2)
      rctx.rotate((params.rotate * Math.PI) / 180)
      rctx.drawImage(base, -base.width / 2, -base.height / 2)
      work = rot
    }
  }

  applyPixelAdjustments(work, params)
  applyDenoise(work, params.denoise)
  applySharpen(work, params.sharpness)

  if (!maxDimension) return work
  const side = Math.max(work.width, work.height)
  if (side <= maxDimension) return work
  const ratio = maxDimension / side
  const scaled = makeCanvas(work.width * ratio, work.height * ratio)
  const sctx = scaled.getContext('2d')
  if (sctx) sctx.drawImage(work, 0, 0, scaled.width, scaled.height)
  return scaled
}

function canvasToBlob(canvas: HTMLCanvasElement, mime: string, quality?: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => {
        if (!blob) reject(new Error('blob_failed'))
        else resolve(blob)
      },
      mime,
      quality,
    )
  })
}

function extToMime(ext: string): string {
  const e = ext.toLowerCase()
  if (e === 'jpg' || e === 'jpeg') return 'image/jpeg'
  if (e === 'webp') return 'image/webp'
  if (e === 'bmp') return 'image/bmp'
  return 'image/png'
}

async function openEditor(item: ScreenshotMediaItem) {
  if (!appStore.xplanePath) return
  shareMenuOpen.value = false
  editorOpen.value = true
  editorItem.value = item
  editorError.value = ''
  activeTool.value = null
  if (editorInputUrl.value) {
    URL.revokeObjectURL(editorInputUrl.value)
    editorInputUrl.value = ''
  }
  if (editorPreviewUrl.value) {
    URL.revokeObjectURL(editorPreviewUrl.value)
    editorPreviewUrl.value = ''
  }
  try {
    const enhanced = enhancedPreviews.get(item.fileName)
    const blob = enhanced
      ? bytesToBlob(enhanced.fullBytes, enhanced.mime || 'image/png')
      : await getRawImageBlob(item)
    editorInputUrl.value = URL.createObjectURL(blob)

    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const el = new Image()
      el.onload = () => resolve(el)
      el.onerror = () => reject(new Error('load_failed'))
      el.src = editorInputUrl.value
    })
    editorImage.value = img
    bounds.value = { width: img.naturalWidth, height: img.naturalHeight }
    edit.value = defaultEditParams(img.naturalWidth, img.naturalHeight)
    await renderPreview()
  } catch {
    editorError.value = t('screenshot.editNotSupported') as string
  }
}

async function renderPreview() {
  if (!editorImage.value) return
  const seq = ++previewRenderSeq
  previewBusy.value = true
  try {
    const canvas = processImage(editorImage.value, edit.value, 1600)
    const blob = await canvasToBlob(canvas, 'image/png')
    const nextUrl = URL.createObjectURL(blob)
    if (seq !== previewRenderSeq) {
      URL.revokeObjectURL(nextUrl)
      return
    }
    if (editorPreviewUrl.value) URL.revokeObjectURL(editorPreviewUrl.value)
    editorPreviewUrl.value = nextUrl
  } finally {
    if (seq === previewRenderSeq) {
      previewBusy.value = false
    }
  }
}

function schedulePreview(immediate = false) {
  if (!editorOpen.value || !editorImage.value || editorError.value) return
  if (previewTimer !== null) {
    window.clearTimeout(previewTimer)
    previewTimer = null
  }
  if (immediate) {
    void renderPreview()
    return
  }
  previewTimer = window.setTimeout(() => {
    previewTimer = null
    void renderPreview()
  }, 60)
}

async function exportEditedBlob(): Promise<Blob> {
  if (!editorItem.value || !editorImage.value) throw new Error('editor_not_ready')
  const mime = extToMime(editorItem.value.ext)
  const canvas = processImage(editorImage.value, edit.value)
  const quality = mime === 'image/jpeg' || mime === 'image/webp' ? 0.95 : undefined
  return canvasToBlob(canvas, mime, quality)
}

async function saveEdited(overwrite: boolean) {
  if (!appStore.xplanePath || !editorItem.value) return
  saveBusy.value = true
  try {
    const blob = await exportEditedBlob()
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
    let targetPath: string | null = null
    if (!overwrite) {
      const name = `${editorItem.value.name.replace(/\.[^/.]+$/, '')}_edited.${editorItem.value.ext || 'png'}`
      const selectedPath = await save({ defaultPath: name })
      if (!selectedPath || Array.isArray(selectedPath)) return
      targetPath = selectedPath
    }

    await invoke<ScreenshotOperationResult>('save_edited_screenshot_image', {
      xplanePath: appStore.xplanePath,
      fileName: editorItem.value.fileName,
      bytes,
      targetPath: targetPath ?? null,
    })
    if (overwrite) clearEnhancedPreview(editorItem.value.fileName)
    toastStore.success(
      overwrite ? (t('settings.saved') as string) : (t('screenshot.saveAsSuccess') as string),
    )
    refreshKey.value = Date.now()
    await load()
    closeEditor()
  } catch (e) {
    modalStore.showError(`${t('screenshot.saveFailed')}: ${String(e)}`)
  } finally {
    saveBusy.value = false
  }
}

async function copyEdited() {
  try {
    const blob = await exportEditedBlob()
    await copyImageBlob(blob, editorItem.value?.path || '')
    toastStore.success(t('screenshot.copySuccess') as string)
  } catch {
    toastStore.error(t('screenshot.copyImageFailed') as string)
  }
}

function resetEditor() {
  edit.value = defaultEditParams(bounds.value.width, bounds.value.height)
  activeTool.value = null
  schedulePreview(true)
}

function closeEditor() {
  if (previewTimer !== null) {
    window.clearTimeout(previewTimer)
    previewTimer = null
  }
  previewRenderSeq += 1
  editorOpen.value = false
  editorItem.value = null
  editorImage.value = null
  editorError.value = ''
  activeTool.value = null
  if (editorInputUrl.value) URL.revokeObjectURL(editorInputUrl.value)
  editorInputUrl.value = ''
  if (editorPreviewUrl.value) URL.revokeObjectURL(editorPreviewUrl.value)
  editorPreviewUrl.value = ''
}

function toggleTool(tool: EditorTool) {
  activeTool.value = activeTool.value === tool ? null : tool
}

function toolGlyph(tool: EditorTool): string {
  if (tool === 'crop') return 'CP'
  if (tool === 'rotate') return 'RT'
  if (tool === 'exposure') return 'EX'
  if (tool === 'contrast') return 'CT'
  if (tool === 'saturation') return 'SA'
  if (tool === 'temperature') return 'TP'
  if (tool === 'highlights') return 'HI'
  if (tool === 'shadows') return 'SD'
  if (tool === 'sharpness') return 'SH'
  if (tool === 'denoise') return 'DN'
  return 'TL'
}

function toggleShareMenu() {
  shareMenuOpen.value = !shareMenuOpen.value
}

function shareToRedditFromMenu() {
  if (selected.value) {
    void share(selected.value)
  }
  shareMenuOpen.value = false
}

watch(
  () => appStore.xplanePath,
  () => {
    void load()
  },
)

watch(
  edit,
  () => {
    schedulePreview()
  },
  { deep: true },
)

watch(selected, (value) => {
  if (!value) {
    shareMenuOpen.value = false
  }
})

onMounted(() => {
  void load()
})

onBeforeUnmount(() => {
  if (previewTimer !== null) {
    window.clearTimeout(previewTimer)
    previewTimer = null
  }
  previewRenderSeq += 1
  if (editorInputUrl.value) URL.revokeObjectURL(editorInputUrl.value)
  if (editorPreviewUrl.value) URL.revokeObjectURL(editorPreviewUrl.value)
  clearAllEnhancedPreviews()
})
</script>

<template>
  <div class="h-full">
    <div class="h-full flex flex-col p-5 overflow-hidden screenshot-view">
      <div class="mb-4 flex items-center justify-between gap-3">
        <div>
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">{{ $t('screenshot.title') }}</h2>
          <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">{{ $t('screenshot.subtitle') }}</p>
        </div>
        <button
          class="px-3 py-1.5 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white/70 dark:bg-gray-800/60 hover:bg-gray-50 dark:hover:bg-gray-700/60 transition-colors"
          :disabled="loading"
          @click="load"
        >
          {{ loading ? $t('screenshot.loading') : $t('settings.refreshLogs') }}
        </button>
      </div>

      <div v-if="!appStore.xplanePath" class="flex-1 flex flex-col items-center justify-center text-center">
        <p class="text-gray-500 dark:text-gray-400">{{ $t('screenshot.noPath') }}</p>
        <router-link to="/settings" class="mt-3 text-sm text-blue-600 dark:text-blue-400 hover:underline">
          {{ $t('common.settings') }}
        </router-link>
      </div>

      <template v-else>
        <div class="mb-3 flex flex-wrap items-center gap-2">
          <input
            v-model="q"
            type="text"
            :placeholder="$t('screenshot.searchPlaceholder')"
            class="w-[240px] max-w-full px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/40"
          />
          <div class="flex rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
            <button class="px-2.5 py-1.5 text-xs sm:text-sm" :class="filterType === 'all' ? 'bg-blue-600 text-white' : 'bg-white dark:bg-gray-800'" @click="filterType = 'all'">{{ $t('screenshot.typeAll') }}</button>
            <button class="px-2.5 py-1.5 text-xs sm:text-sm border-l border-gray-200 dark:border-gray-700" :class="filterType === 'image' ? 'bg-blue-600 text-white' : 'bg-white dark:bg-gray-800'" @click="filterType = 'image'">{{ $t('screenshot.typeImage') }}</button>
            <button class="px-2.5 py-1.5 text-xs sm:text-sm border-l border-gray-200 dark:border-gray-700" :class="filterType === 'video' ? 'bg-blue-600 text-white' : 'bg-white dark:bg-gray-800'" @click="filterType = 'video'">{{ $t('screenshot.typeVideo') }}</button>
          </div>
          <select v-model="sort" class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm">
            <option value="time">{{ $t('screenshot.sortTime') }}</option>
            <option value="name">{{ $t('screenshot.sortName') }}</option>
            <option value="size">{{ $t('screenshot.sortSize') }}</option>
          </select>
        </div>

        <div class="flex-1 overflow-y-auto">
          <div v-if="loading" class="h-full flex items-center justify-center">
            <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-500"></div>
          </div>
          <div v-else-if="error" class="rounded-xl border border-red-200 dark:border-red-900/40 bg-red-50 dark:bg-red-900/10 p-4 text-sm text-red-700 dark:text-red-300">
            {{ $t('screenshot.loadFailed') }}: {{ error }}
          </div>
          <div v-else-if="filtered.length === 0" class="h-full flex items-center justify-center text-gray-500 dark:text-gray-400">
            {{ $t('screenshot.empty') }}
          </div>
          <div v-else class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3 pb-3">
            <button
              v-for="item in filtered"
              :key="item.id"
              class="text-left rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden hover:shadow-md transition-shadow"
              @click="selected = item"
            >
              <div class="relative aspect-[16/10] bg-gray-100 dark:bg-gray-900">
                <img v-if="item.mediaType === 'image' && !isThumbFailed(item.fileName)" :src="toSrc(item.path)" :alt="item.name" class="w-full h-full object-cover" loading="lazy" @error="markThumbFailed(item.fileName)" />
                <video v-else-if="item.mediaType === 'video' && item.previewable" :src="toSrc(item.path)" class="w-full h-full object-cover" muted preload="metadata"></video>
                <div v-else class="w-full h-full flex items-center justify-center text-gray-400 dark:text-gray-500">N/A</div>
                <span v-if="item.mediaType === 'video'" class="absolute right-2 top-2 px-1.5 py-0.5 rounded bg-black/60 text-white text-[10px] tracking-wide">VIDEO</span>
                <span v-if="item.mediaType === 'image' && hasEnhancedPreview(item.fileName)" class="absolute left-2 top-2 px-1.5 py-0.5 rounded bg-emerald-500/85 text-white text-[10px] tracking-wide">AI</span>
              </div>
              <div class="px-2.5 py-2 space-y-1">
                <p class="text-xs font-medium text-gray-900 dark:text-gray-100 truncate" :title="item.name">{{ item.name }}</p>
                <p class="text-[11px] text-gray-500 dark:text-gray-400">{{ fmtSize(item.size) }} · {{ fmtTime(item.modifiedAt) }}</p>
              </div>
            </button>
          </div>
        </div>
      </template>
    </div>

    <Teleport to="body">
    <div v-if="selected" class="fixed inset-0 z-[110] bg-black/80 backdrop-blur-sm p-3 sm:p-6" @click.self="selected = null">
      <div class="h-full w-full max-w-[1240px] mx-auto flex flex-col">
        <div class="flex justify-end mb-2 gap-2">
          <div class="relative">
            <button class="round-icon-btn" :title="$t('screenshot.shareReddit')" @click.stop="toggleShareMenu">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M8 12h8m-8 4h6m-2-9.5V4a1 1 0 011-1h3"
                />
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M7 20h10a2 2 0 002-2V9.5a1 1 0 00-.293-.707l-3.5-3.5A1 1 0 0014.5 5H7a2 2 0 00-2 2v11a2 2 0 002 2z"
                />
              </svg>
            </button>
            <div
              v-if="shareMenuOpen"
              class="absolute right-0 mt-2 w-40 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shadow-xl z-10 py-1"
            >
              <button class="share-menu-item" @click.stop="shareToRedditFromMenu">
                <span class="inline-flex w-5 h-5 items-center justify-center rounded-full bg-[#ff4500]">
                  <svg class="w-3.5 h-3.5 text-white" viewBox="0 0 24 24" fill="currentColor">
                    <circle cx="8.5" cy="12" r="1.5" />
                    <circle cx="15.5" cy="12" r="1.5" />
                    <path d="M7.2 15.2a6.8 6.8 0 009.6 0 .8.8 0 10-1.1-1.1 5.2 5.2 0 01-7.4 0 .8.8 0 10-1.1 1.1z" />
                    <path d="M17.4 5.2a1.8 1.8 0 100 3.6 1.8 1.8 0 000-3.6zm-1.9 3.8l-2.6-.6.4-1.7 2.6.7-.4 1.6z" />
                  </svg>
                </span>
                <span>Reddit</span>
              </button>
            </div>
          </div>
          <button class="round-icon-btn" :title="$t('common.close')" @click="selected = null">x</button>
        </div>
        <div class="flex-1 min-h-0 rounded-xl bg-black/40 border border-white/10 overflow-hidden flex items-center justify-center">
          <img v-if="selected.mediaType === 'image'" :src="selectedSrc" :alt="selected.name" class="max-w-full max-h-full object-contain" />
          <video v-else-if="selected.previewable" :src="selectedSrc" class="max-w-full max-h-full object-contain" controls autoplay></video>
          <p v-else class="text-sm text-white/70">{{ $t('screenshot.unsupportedPreview') }}</p>
        </div>
        <div class="mt-3 rounded-xl bg-gray-900/70 border border-white/10 p-2.5 flex flex-wrap items-center gap-2">
          <button
            v-if="selected.mediaType === 'image' && selected.editable"
            class="action-icon action-blue"
            @click="openEditor(selected)"
          >
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5M16.5 3.5a2.1 2.1 0 113 3L12 14l-4 1 1-4 7.5-7.5z" />
            </svg>
            <span class="action-label">{{ $t('screenshot.openEditor') }}</span>
          </button>
          <button
            v-if="selected.mediaType === 'image' && selected.editable"
            class="action-icon action-slate"
            :disabled="autoEnhanceBusyFile === selected.fileName"
            @click="autoEnhance(selected)"
          >
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v3m0 12v3m9-9h-3M6 12H3m14.36 6.36l-2.12-2.12M8.76 8.76L6.64 6.64m10.72 0l-2.12 2.12M8.76 15.24l-2.12 2.12" />
            </svg>
            <span class="action-label">{{ autoEnhanceBusyFile === selected.fileName ? $t('screenshot.autoEnhanceBusy') : $t('screenshot.autoEnhance') }}</span>
          </button>
          <button class="action-icon action-emerald" @click="copyMedia(selected)">
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <rect x="9" y="9" width="10" height="10" rx="2" ry="2" stroke-width="2" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15V7a2 2 0 012-2h8" />
            </svg>
            <span class="action-label">{{ selected.mediaType === 'image' ? $t('screenshot.copyImage') : $t('screenshot.copyPath') }}</span>
          </button>
          <button
            class="action-icon action-indigo"
            :disabled="busyFile === selected.fileName"
            @click="saveAs(selected)"
          >
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 16V4m0 12l-4-4m4 4l4-4M4 18v1a1 1 0 001 1h14a1 1 0 001-1v-1" />
            </svg>
            <span class="action-label">{{ $t('screenshot.saveAs') }}</span>
          </button>
          <button
            class="action-icon action-rose"
            :disabled="busyFile === selected.fileName"
            @click="askDelete(selected)"
          >
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-1 12a2 2 0 01-2 2H8a2 2 0 01-2-2L5 7m3 0V5a1 1 0 011-1h6a1 1 0 011 1v2M4 7h16" />
            </svg>
            <span class="action-label">{{ $t('common.delete') }}</span>
          </button>
        </div>
      </div>
    </div>
    </Teleport>

    <Teleport to="body">
    <div v-if="editorOpen" class="fixed inset-0 z-[120] bg-black/75 backdrop-blur-sm p-2 sm:p-4">
      <div class="h-full max-w-[1320px] mx-auto rounded-2xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 overflow-hidden flex flex-col">
        <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
          <h3 class="font-semibold text-gray-900 dark:text-gray-100">{{ $t('screenshot.editTitle') }}</h3>
          <button class="text-sm px-2 py-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800" @click="closeEditor">{{ $t('common.close') }}</button>
        </div>

        <div class="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-[1fr_340px]">
          <div class="p-3 bg-gray-50 dark:bg-gray-950 border-b lg:border-b-0 lg:border-r border-gray-200 dark:border-gray-700 flex items-center justify-center min-h-0">
            <div class="w-full h-full min-h-0 rounded-xl bg-black/85 overflow-hidden flex items-center justify-center">
              <img v-if="editorSrc && !editorError" :src="editorSrc" :alt="$t('screenshot.editPreview')" class="max-w-full max-h-full object-contain" />
              <p v-else class="text-sm text-white/70">{{ editorError || $t('screenshot.editNotSupported') }}</p>
            </div>
          </div>

          <div class="min-h-0 overflow-y-auto p-3 space-y-3 text-xs text-gray-600 dark:text-gray-300 flex flex-col">
            <div class="rounded-xl border border-gray-200 dark:border-gray-700 p-3 min-h-[136px]">
              <div v-if="activeTool === 'crop'" class="grid grid-cols-2 gap-2">
                <label>{{ $t('screenshot.cropX') }}<input v-model.number="cropX" type="number" min="0" :max="Math.max(0, bounds.width - 1)" class="mt-1 w-full px-2 py-1.5 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm" /></label>
                <label>{{ $t('screenshot.cropY') }}<input v-model.number="cropY" type="number" min="0" :max="Math.max(0, bounds.height - 1)" class="mt-1 w-full px-2 py-1.5 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm" /></label>
                <label>{{ $t('screenshot.cropW') }}<input v-model.number="cropW" type="number" min="1" :max="Math.max(1, bounds.width)" class="mt-1 w-full px-2 py-1.5 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm" /></label>
                <label>{{ $t('screenshot.cropH') }}<input v-model.number="cropH" type="number" min="1" :max="Math.max(1, bounds.height)" class="mt-1 w-full px-2 py-1.5 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm" /></label>
              </div>

              <label v-else-if="activeTool === 'rotate'" class="block">
                {{ $t('screenshot.rotate') }}
                <select v-model.number="edit.rotate" class="mt-1 w-full px-2 py-1.5 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm">
                  <option :value="0">0 deg</option>
                  <option :value="90">90 deg</option>
                  <option :value="180">180 deg</option>
                  <option :value="270">270 deg</option>
                </select>
              </label>

              <label v-else-if="activeSlider" class="block">
                {{ $t(activeSlider.label) }}
                <input v-model.number="activeSliderValue" type="range" :min="activeSlider.min" :max="activeSlider.max" class="w-full mt-1" />
                <div class="mt-1 text-[11px] text-gray-500 dark:text-gray-400">{{ activeSliderValue }}</div>
              </label>

              <p v-else class="text-sm text-gray-500 dark:text-gray-400">{{ $t('screenshot.toolHint') }}</p>
            </div>

            <div class="mt-auto space-y-2">
              <div class="rounded-xl border border-gray-200 dark:border-gray-700 p-2 flex flex-wrap gap-2">
                <button
                  v-for="tool in editorTools"
                  :key="tool.key"
                  class="action-icon action-slate"
                  :class="{ 'action-active': activeTool === tool.key }"
                  @click="toggleTool(tool.key)"
                >
                  <span class="tool-glyph">{{ toolGlyph(tool.key) }}</span>
                  <span class="action-label">{{ $t(tool.label) }}</span>
                </button>
              </div>

              <div class="flex flex-wrap gap-2 pt-1">
                <button class="action-icon action-gray" :disabled="previewBusy || saveBusy" @click="resetEditor">
                  <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v6h6M20 20v-6h-6M5.6 13a7 7 0 0012.8 0M18.4 11a7 7 0 00-12.8 0" />
                  </svg>
                  <span class="action-label">{{ $t('screenshot.reset') }}</span>
                </button>
                <button class="action-icon action-blue" :disabled="previewBusy || saveBusy" @click="renderPreview">
                  <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v14m7-7H5" />
                  </svg>
                  <span class="action-label">{{ previewBusy ? $t('logAnalysis.analyzing') : $t('screenshot.applyPreview') }}</span>
                </button>
                <button class="action-icon action-green" :disabled="saveBusy" @click="saveEdited(true)">
                  <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 4h12l2 2v14H5V4zm3 0v5h8V4M9 20v-6h6v6" />
                  </svg>
                  <span class="action-label">{{ $t('screenshot.saveOverwrite') }}</span>
                </button>
                <button class="action-icon action-indigo" :disabled="saveBusy" @click="saveEdited(false)">
                  <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 16V4m0 12l-4-4m4 4l4-4M4 18v1a1 1 0 001 1h14a1 1 0 001-1v-1" />
                  </svg>
                  <span class="action-label">{{ $t('screenshot.saveAs') }}</span>
                </button>
                <button class="action-icon action-emerald" :disabled="saveBusy" @click="copyEdited">
                  <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <rect x="9" y="9" width="10" height="10" rx="2" ry="2" stroke-width="2" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15V7a2 2 0 012-2h8" />
                  </svg>
                  <span class="action-label">{{ $t('screenshot.copyImage') }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    </Teleport>
  </div>
</template>

<style scoped>
.screenshot-view {
  background: linear-gradient(to bottom, rgba(248, 250, 252, 0.45), rgba(241, 245, 249, 0.45));
}

.dark .screenshot-view {
  background: linear-gradient(to bottom, rgba(17, 24, 39, 0.45), rgba(31, 41, 55, 0.45));
}

.round-icon-btn {
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.2);
  color: white;
  transition: background-color 0.15s;
}

.round-icon-btn:hover {
  background: rgba(255, 255, 255, 0.35);
}

.share-menu-item {
  width: 100%;
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.5rem 0.7rem;
  font-size: 0.82rem;
  color: rgb(31 41 55);
}

.share-menu-item:hover {
  background: rgb(243 244 246);
}

.dark .share-menu-item {
  color: rgb(229 231 235);
}

.dark .share-menu-item:hover {
  background: rgb(55 65 81);
}

.action-icon {
  display: inline-flex;
  align-items: center;
  justify-content: flex-start;
  height: 2.1rem;
  border-radius: 999px;
  padding: 0 0.62rem;
  font-size: 0.78rem;
  color: white;
  border: 0;
  transition:
    transform 0.15s ease,
    filter 0.15s ease;
}

.action-icon:hover:not(:disabled) {
  transform: translateY(-1px);
  filter: brightness(1.05);
}

.action-label {
  max-width: 0;
  opacity: 0;
  overflow: hidden;
  white-space: nowrap;
  margin-left: 0;
  transition:
    max-width 0.2s ease,
    opacity 0.2s ease,
    margin-left 0.2s ease;
}

.action-icon:hover .action-label,
.action-icon:focus-visible .action-label {
  max-width: 14rem;
  opacity: 1;
  margin-left: 0.35rem;
}

.action-gray {
  background: rgb(107 114 128);
}

.action-blue {
  background: rgb(37 99 235);
}

.action-green {
  background: rgb(22 163 74);
}

.action-indigo {
  background: rgb(79 70 229);
}

.action-emerald {
  background: rgb(5 150 105);
}

.action-rose {
  background: rgb(225 29 72);
}

.action-slate {
  background: rgb(71 85 105);
}

.action-active {
  box-shadow: inset 0 0 0 2px rgba(147, 197, 253, 0.9);
}

.tool-glyph {
  display: inline-flex;
  width: 1.1rem;
  justify-content: center;
  font-size: 0.62rem;
  line-height: 1;
  letter-spacing: 0.02em;
  font-weight: 700;
}

.action-icon:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
</style>
