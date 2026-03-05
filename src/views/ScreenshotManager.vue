<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import type {
  AircraftInfo,
  ManagementData,
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
type EditorTool = 'crop' | SliderKey

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
const videoCoverUrls = reactive(new Map<string, string>())
const pendingVideoCovers = reactive(new Set<string>())
const busyFile = ref<string | null>(null)
const enhancedPreviews = reactive(new Map<string, EnhancedPreviewState>())

const editorOpen = ref(false)
const editorItem = ref<ScreenshotMediaItem | null>(null)
const editorImage = ref<HTMLImageElement | null>(null)
const editorSourceBlob = ref<Blob | null>(null)
const editorInputUrl = ref('')
const editorPreviewUrl = ref('')
const editorError = ref('')
const editorImageLoading = ref(false)
const editorPlaceholderAspect = ref<number | null>(null)
const previewBusy = ref(false)
const saveBusy = ref(false)
const bounds = ref({ width: 0, height: 0 })
const edit = ref<ScreenshotEditParams>(defaultEditParams())
const activeTool = ref<EditorTool | null>(null)
const isInteractiveAdjusting = ref(false)
let previewTimer: number | null = null
let previewRenderSeq = 0
let displayedPreviewSeq = 0
let pendingHighQualityPreview = false

const FAST_PREVIEW_MAX_SIDE = 1100
const DRAG_PREVIEW_MAX_SIDE = 760
const FINAL_PREVIEW_MAX_SIDE = 1600

let previewWorker: Worker | null = null
let previewWorkerReady = false
let previewWorkerReqId = 0
const previewWorkerPending = new Map<
  number,
  {
    resolve: (blob: Blob) => void
    reject: (reason?: unknown) => void
  }
>()

/* ---- crop overlay drag state ---- */
const cropContainerRef = ref<HTMLElement | null>(null)
const cropImgRef = ref<HTMLImageElement | null>(null)
const cropGeometryTick = ref(0)
const UNCLASSIFIED_AIRCRAFT_KEY = '__unclassified__'
const activeAircraftGroup = ref<string | null>(null)
const aircraftFolderByAcfStem = ref<Record<string, string>>({})
type CropDragMode = 'move' | 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'e' | 'w' | null
const cropDragMode = ref<CropDragMode>(null)
const cropDragStart = ref({ mx: 0, my: 0, cx: 0, cy: 0, cw: 0, ch: 0 })

watch(
  () => edit.value.rotate,
  () => {
    // Read DOM geometry after the transformed frame has been painted.
    requestAnimationFrame(() => {
      cropGeometryTick.value += 1
    })
  },
  { flush: 'post' },
)

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

function resolveAircraftGroupKey(fileName: string): string {
  const baseName = fileName.replace(/\.[^.]+$/, '')
  const match = baseName.match(/^(.*?)\s-\s\d{4}-\d{2}-\d{2}\s\d{2}\.\d{2}\.\d{2}$/)
  const prefix = match?.[1]?.trim()
  return prefix ? prefix : UNCLASSIFIED_AIRCRAFT_KEY
}

type AircraftGroup = {
  key: string
  label: string
  items: ScreenshotMediaItem[]
  coverItem: ScreenshotMediaItem
  totalSize: number
  latestModified: number
}

function hashString(input: string): number {
  let hash = 2166136261
  for (let i = 0; i < input.length; i += 1) {
    hash ^= input.charCodeAt(i)
    hash = Math.imul(hash, 16777619)
  }
  return hash >>> 0
}

function randomIndexBySeed(seed: string, length: number): number {
  if (length <= 1) return 0
  return hashString(seed) % length
}

function getGroupCoverSrc(item: ScreenshotMediaItem): string {
  if (item.mediaType === 'image') return toSrc(item.path)
  return videoCoverUrls.get(item.fileName) || ''
}

function drawVideoCoverToBlob(video: HTMLVideoElement): Promise<Blob | null> {
  const canvas = document.createElement('canvas')
  canvas.width = video.videoWidth
  canvas.height = video.videoHeight
  const ctx = canvas.getContext('2d')
  if (!ctx) return Promise.resolve(null)
  ctx.drawImage(video, 0, 0, canvas.width, canvas.height)
  return new Promise((resolve) => {
    canvas.toBlob(
      (blob) => resolve(blob),
      'image/jpeg',
      0.85,
    )
  })
}

async function captureVideoCoverFromItem(item: ScreenshotMediaItem): Promise<void> {
  if (item.mediaType !== 'video' || !item.previewable) return
  if (videoCoverUrls.has(item.fileName) || pendingVideoCovers.has(item.fileName)) return
  pendingVideoCovers.add(item.fileName)
  const video = document.createElement('video')
  video.muted = true
  video.playsInline = true
  video.preload = 'metadata'
  try {
    const ready = await new Promise<boolean>((resolve) => {
      let settled = false
      const done = (ok: boolean) => {
        if (settled) return
        settled = true
        video.onloadeddata = null
        video.onerror = null
        resolve(ok)
      }
      const timer = window.setTimeout(() => {
        done(false)
      }, 4000)
      video.onloadeddata = () => {
        window.clearTimeout(timer)
        done(true)
      }
      video.onerror = () => {
        window.clearTimeout(timer)
        done(false)
      }
      video.src = toSrc(item.path)
    })
    if (!ready || video.videoWidth <= 0 || video.videoHeight <= 0) return
    const blob = await drawVideoCoverToBlob(video)
    if (!blob) return
    const nextUrl = URL.createObjectURL(blob)
    const prev = videoCoverUrls.get(item.fileName)
    if (prev) URL.revokeObjectURL(prev)
    videoCoverUrls.set(item.fileName, nextUrl)
  } finally {
    pendingVideoCovers.delete(item.fileName)
    video.pause()
    video.removeAttribute('src')
    video.load()
  }
}

function ensureGroupVideoCovers(): void {
  for (const group of aircraftGroups.value) {
    if (group.coverItem.mediaType !== 'video') continue
    void captureVideoCoverFromItem(group.coverItem)
  }
}

function pruneVideoCovers() {
  const valid = new Set(items.value.map((it) => it.fileName))
  for (const [fileName, url] of videoCoverUrls.entries()) {
    if (valid.has(fileName)) continue
    URL.revokeObjectURL(url)
    videoCoverUrls.delete(fileName)
  }
}

function captureVideoCover(event: Event, item: ScreenshotMediaItem) {
  if (item.mediaType !== 'video' || videoCoverUrls.has(item.fileName)) return
  const video = event.target as HTMLVideoElement | null
  if (!video || video.videoWidth <= 0 || video.videoHeight <= 0) return
  void drawVideoCoverToBlob(video).then((blob) => {
    if (!blob) return
    const nextUrl = URL.createObjectURL(blob)
    const prev = videoCoverUrls.get(item.fileName)
    if (prev) URL.revokeObjectURL(prev)
    videoCoverUrls.set(item.fileName, nextUrl)
  })
}

const aircraftGroups = computed<AircraftGroup[]>(() => {
  const grouped = new Map<string, Omit<AircraftGroup, 'coverItem'>>()
  for (const item of filtered.value) {
    const key = resolveAircraftGroupKey(item.fileName)
    const label =
      key === UNCLASSIFIED_AIRCRAFT_KEY
        ? (t('screenshot.unclassified') as string)
        : aircraftFolderByAcfStem.value[key] || key
    const existing = grouped.get(key)
    if (existing) {
      existing.items.push(item)
      existing.totalSize += item.size
      existing.latestModified = Math.max(existing.latestModified, item.modifiedAt)
      continue
    }
    grouped.set(key, {
      key,
      label,
      items: [item],
      totalSize: item.size,
      latestModified: item.modifiedAt,
    })
  }

  const groups = Array.from(grouped.values()).map((group) => {
    const coverIdx = randomIndexBySeed(`${group.key}:${refreshKey.value}`, group.items.length)
    return {
      ...group,
      coverItem: group.items[coverIdx],
    }
  })
  groups.sort((a, b) => {
    if (a.key === UNCLASSIFIED_AIRCRAFT_KEY) return 1
    if (b.key === UNCLASSIFIED_AIRCRAFT_KEY) return -1
    if (sort.value === 'name') return a.label.localeCompare(b.label)
    if (sort.value === 'size') return b.totalSize - a.totalSize
    return b.latestModified - a.latestModified
  })
  return groups
})

const activeAircraftGroupData = computed(() => {
  if (!activeAircraftGroup.value) return null
  return aircraftGroups.value.find((group) => group.key === activeAircraftGroup.value) ?? null
})

const groupedItems = computed(() => activeAircraftGroupData.value?.items ?? [])

watch(aircraftGroups, (groups) => {
  if (!activeAircraftGroup.value) return
  if (!groups.some((group) => group.key === activeAircraftGroup.value)) {
    activeAircraftGroup.value = null
  }
})

function openAircraftGroup(key: string) {
  activeAircraftGroup.value = key
}

function backToAircraftGroups() {
  activeAircraftGroup.value = null
}

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

const editorPlaceholderStyle = computed(() => ({
  aspectRatio: String(editorPlaceholderAspect.value ?? 16 / 9),
}))

const activeSlider = computed(() => {
  if (!activeTool.value || activeTool.value === 'crop') return null
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

/* ---- moving ruler slider ---- */
const rulerDragStart = ref<{ x: number; val: number } | null>(null)

function onRulerPointerDown(e: PointerEvent, isRotate = false) {
  e.preventDefault()
  isInteractiveAdjusting.value = true
  if (isRotate) {
    rulerDragStart.value = { x: e.clientX, val: edit.value.rotate }
  } else if (activeSlider.value) {
    rulerDragStart.value = { x: e.clientX, val: edit.value[activeSlider.value.key] as number }
  } else {
    return
  }

  const onMove = (me: PointerEvent) => {
    if (!rulerDragStart.value) return
    const dx = me.clientX - rulerDragStart.value.x
    // Determine scaling: 400px drag covers the full range (less sensitive, smoother to control)
    const sensitivity = 400

    if (isRotate) {
      const deltaVal = -(dx / sensitivity) * 180 // 400px drag covers full -90..90 range
      let newVal = rulerDragStart.value.val + deltaVal
      newVal = Number(newVal.toFixed(1))
      newVal = clamp(newVal, -90, 90)
      if (edit.value.rotate !== newVal) {
        edit.value.rotate = newVal
        schedulePreview(true, false)
      }
    } else if (activeSlider.value) {
      const slider = activeSlider.value
      const range = slider.max - slider.min || 1
      let deltaVal = -(dx / sensitivity) * range
      let newVal = rulerDragStart.value.val + deltaVal
      newVal = clamp(Math.round(newVal), slider.min, slider.max)
      if (edit.value[slider.key] !== newVal) {
        edit.value[slider.key] = newVal
        schedulePreview(true, false)
      }
    }
  }
  const onUp = () => {
    rulerDragStart.value = null
    isInteractiveAdjusting.value = false
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onUp)
    window.removeEventListener('blur', onUp)
    schedulePreview(false, true)
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onUp)
  window.addEventListener('blur', onUp)
}

const rotateRulerTransform = computed(() => {
  const pct = (edit.value.rotate / 90) * 50
  // Keep 0deg centered and map -90..90 to the full ruler span.
  return `translateX(calc(-50% - ${pct}%))`
})

const cropPreviewImageStyle = computed(() => {
  const angle = edit.value.rotate
  if (angle === 0) return undefined
  const W = bounds.value.width || 1
  const H = bounds.value.height || 1
  const angleRad = Math.abs((angle * Math.PI) / 180)
  let scale = Math.cos(angleRad) + Math.sin(angleRad) * Math.max(W / H, H / W)
  scale *= 1.002
  return {
    transform: `rotate(${angle}deg) scale(${scale})`,
    transformOrigin: 'center center',
  }
})

const sliderRulerTransform = computed(() => {
  if (!activeSlider.value) return 'translateX(0%)'
  const { min, max } = activeSlider.value
  const val = activeSliderValue.value
  const range = max - min || 1
  const pct = ((val - min) / range) * 100
  return `translateX(-${pct}%)`
})

/* ---- crop overlay computed styles ---- */
function imgRect(): { left: number; top: number; w: number; h: number } {
  void cropGeometryTick.value
  const img = cropImgRef.value
  const container = cropContainerRef.value
  if (!img || !container) return { left: 0, top: 0, w: 0, h: 0 }
  const cr = container.getBoundingClientRect()
  const ir = img.getBoundingClientRect()
  // Use the visible intersection area (clipped by editor frame) as crop geometry base.
  const left = Math.max(ir.left, cr.left)
  const top = Math.max(ir.top, cr.top)
  const right = Math.min(ir.right, cr.right)
  const bottom = Math.min(ir.bottom, cr.bottom)
  const w = Math.max(0, right - left)
  const h = Math.max(0, bottom - top)
  return { left: left - cr.left, top: top - cr.top, w, h }
}

const cropBoxRect = computed(() => {
  const img = cropImgRef.value
  if (!img || !bounds.value.width || !edit.value.crop) return null
  const r = imgRect()
  const sx = r.w / bounds.value.width
  const sy = r.h / bounds.value.height
  const c = edit.value.crop
  return {
    left: r.left + c.x * sx,
    top: r.top + c.y * sy,
    width: c.width * sx,
    height: c.height * sy,
  }
})

const cropBoxStyle = computed(() => {
  const box = cropBoxRect.value
  if (!box) return {}
  return {
    left: `${box.left}px`,
    top: `${box.top}px`,
    width: `${box.width}px`,
    height: `${box.height}px`,
  }
})

const cropOverlayTop = computed(() => {
  const r = imgRect()
  const c = edit.value.crop
  if (!c || !r.w) return {}
  const sy = r.h / bounds.value.height
  return { left: `${r.left}px`, top: `${r.top}px`, width: `${r.w}px`, height: `${c.y * sy}px` }
})
const cropOverlayBottom = computed(() => {
  const r = imgRect()
  const c = edit.value.crop
  if (!c || !r.w) return {}
  const sy = r.h / bounds.value.height
  const bottomY = (c.y + c.height) * sy
  return {
    left: `${r.left}px`,
    top: `${r.top + bottomY}px`,
    width: `${r.w}px`,
    height: `${r.h - bottomY}px`,
  }
})
const cropOverlayLeft = computed(() => {
  const r = imgRect()
  const c = edit.value.crop
  if (!c || !r.w) return {}
  const sx = r.w / bounds.value.width
  const sy = r.h / bounds.value.height
  return {
    left: `${r.left}px`,
    top: `${r.top + c.y * sy}px`,
    width: `${c.x * sx}px`,
    height: `${c.height * sy}px`,
  }
})
const cropOverlayRight = computed(() => {
  const r = imgRect()
  const c = edit.value.crop
  if (!c || !r.w) return {}
  const sx = r.w / bounds.value.width
  const sy = r.h / bounds.value.height
  const rightX = (c.x + c.width) * sx
  return {
    left: `${r.left + rightX}px`,
    top: `${r.top + c.y * sy}px`,
    width: `${r.w - rightX}px`,
    height: `${c.height * sy}px`,
  }
})

function cropHandleStyle(pos: string): Record<string, string> {
  const box = cropBoxRect.value
  if (!box) return {}
  const bx = box.left
  const by = box.top
  const bw = box.width
  const bh = box.height
  const S = 10
  const H = S / 2
  const cursors: Record<string, string> = {
    nw: 'nwse-resize',
    ne: 'nesw-resize',
    sw: 'nesw-resize',
    se: 'nwse-resize',
    n: 'ns-resize',
    s: 'ns-resize',
    w: 'ew-resize',
    e: 'ew-resize',
  }
  const positions: Record<string, { left: number; top: number }> = {
    nw: { left: bx - H, top: by - H },
    ne: { left: bx + bw - H, top: by - H },
    sw: { left: bx - H, top: by + bh - H },
    se: { left: bx + bw - H, top: by + bh - H },
    n: { left: bx + bw / 2 - H, top: by - H },
    s: { left: bx + bw / 2 - H, top: by + bh - H },
    w: { left: bx - H, top: by + bh / 2 - H },
    e: { left: bx + bw - H, top: by + bh / 2 - H },
  }
  const p = positions[pos] || { left: 0, top: 0 }
  return {
    position: 'absolute',
    left: `${p.left}px`,
    top: `${p.top}px`,
    width: `${S}px`,
    height: `${S}px`,
    cursor: cursors[pos] || 'pointer',
  }
}

function defaultEditParams(width = 0, height = 0): ScreenshotEditParams {
  return {
    crop:
      width > 0 && height > 0 ? { x: 0, y: 0, width, height } : { x: 0, y: 0, width: 1, height: 1 },
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
    activeAircraftGroup.value = null
    aircraftFolderByAcfStem.value = {}
    clearAllEnhancedPreviews()
    return
  }
  loading.value = true
  error.value = ''
  try {
    const [media, aircraftData] = await Promise.all([
      invoke<ScreenshotMediaItem[]>('list_screenshot_media', {
        xplanePath: appStore.xplanePath,
      }),
      invoke<ManagementData<AircraftInfo>>('scan_aircraft', {
        xplanePath: appStore.xplanePath,
      }).catch(() => null),
    ])
    items.value = media
    const map: Record<string, string> = {}
    for (const entry of aircraftData?.entries ?? []) {
      const stem = entry.acfFile.replace(/\.[^.]+$/, '').trim()
      if (!stem) continue
      map[stem] = entry.folderName
    }
    aircraftFolderByAcfStem.value = map
    activeAircraftGroup.value = null
    refreshKey.value = Date.now()
    pruneEnhancedPreviews()
    pruneVideoCovers()
    ensureGroupVideoCovers()
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function blobToPng(blob: Blob): Promise<Blob> {
  if (blob.type === 'image/png') return blob
  const img = new Image()
  const url = URL.createObjectURL(blob)
  try {
    await new Promise<void>((res, rej) => {
      img.onload = () => res()
      img.onerror = rej
      img.src = url
    })
    const c = document.createElement('canvas')
    c.width = img.naturalWidth
    c.height = img.naturalHeight
    c.getContext('2d')!.drawImage(img, 0, 0)
    return await new Promise<Blob>((res, rej) =>
      c.toBlob((b) => (b ? res(b) : rej(new Error('toBlob failed'))), 'image/png'),
    )
  } finally {
    URL.revokeObjectURL(url)
  }
}

async function copyImageBlob(blob: Blob, fallbackText: string) {
  const ctor = (window as unknown as Record<string, typeof ClipboardItem>).ClipboardItem
  if (ctor && navigator.clipboard?.write) {
    const pngBlob = await blobToPng(blob)
    await navigator.clipboard.write([new ctor({ 'image/png': pngBlob })])
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
    // Copy image to clipboard first so user can paste in Reddit
    if (item.mediaType === 'image') {
      const enhanced = enhancedPreviews.get(item.fileName)
      const blob = enhanced
        ? bytesToBlob(enhanced.fullBytes, enhanced.mime || 'image/png')
        : await getRawImageBlob(item)
      await copyImageBlob(blob, item.path)
      toastStore.success(t('screenshot.redditClipboardHint') as string)
    }
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
    // Auto-crop scale factor for linear rotation
    const angleRad = Math.abs((params.rotate * Math.PI) / 180)
    const W = base.width
    const H = base.height
    let scale = Math.cos(angleRad) + Math.sin(angleRad) * Math.max(W / H, H / W)
    // Small buffer to avoid 1px rounding edge gaps
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

function ensurePreviewWorker(): Worker | null {
  if (previewWorker) return previewWorker
  if (typeof Worker === 'undefined') return null
  try {
    const worker = new Worker(new URL('../workers/screenshotPreviewWorker.ts', import.meta.url), {
      type: 'module',
    })
    worker.onmessage = (
      event: MessageEvent<{ type: string; requestId?: number; blob?: Blob; message?: string }>,
    ) => {
      const data = event.data
      if (data.type === 'ready') {
        previewWorkerReady = true
        return
      }
      if (data.type === 'result' && typeof data.requestId === 'number' && data.blob) {
        const pending = previewWorkerPending.get(data.requestId)
        if (!pending) return
        previewWorkerPending.delete(data.requestId)
        pending.resolve(data.blob)
        return
      }
      if (data.type === 'error' && typeof data.requestId === 'number') {
        const pending = previewWorkerPending.get(data.requestId)
        if (!pending) return
        previewWorkerPending.delete(data.requestId)
        pending.reject(new Error(data.message || 'preview_worker_error'))
      }
    }
    worker.onerror = () => {
      previewWorkerReady = false
    }
    previewWorker = worker
    return worker
  } catch {
    previewWorker = null
    previewWorkerReady = false
    return null
  }
}

async function initPreviewWorker(blob: Blob) {
  const worker = ensurePreviewWorker()
  if (!worker) return
  previewWorkerReady = false
  worker.postMessage({ type: 'init', blob })
}

function cloneEditParams(params: ScreenshotEditParams): ScreenshotEditParams {
  return {
    ...params,
    crop: params.crop ? { ...params.crop } : null,
  }
}

async function renderPreviewInWorker(
  params: ScreenshotEditParams,
  maxSide: number,
  isFast: boolean,
): Promise<Blob | null> {
  const worker = ensurePreviewWorker()
  if (!worker || !previewWorkerReady) return null
  const requestId = ++previewWorkerReqId
  return await new Promise<Blob>((resolve, reject) => {
    previewWorkerPending.set(requestId, { resolve, reject })
    worker.postMessage({
      type: 'render',
      requestId,
      params: cloneEditParams(params),
      maxDimension: maxSide,
      fast: isFast,
    })
  })
}

async function openEditor(item: ScreenshotMediaItem) {
  if (!appStore.xplanePath) return
  shareMenuOpen.value = false
  editorOpen.value = true
  editorItem.value = item
  editorError.value = ''
  editorImageLoading.value = true
  editorPlaceholderAspect.value =
    item.width && item.height && item.width > 0 && item.height > 0 ? item.width / item.height : null
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
    editorSourceBlob.value = blob
    void initPreviewWorker(blob)
    editorInputUrl.value = URL.createObjectURL(blob)

    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const el = new Image()
      el.onload = () => resolve(el)
      el.onerror = () => reject(new Error('load_failed'))
      el.src = editorInputUrl.value
    })
    editorImage.value = img
    bounds.value = { width: img.naturalWidth, height: img.naturalHeight }
    editorPlaceholderAspect.value =
      img.naturalWidth > 0 && img.naturalHeight > 0
        ? img.naturalWidth / img.naturalHeight
        : editorPlaceholderAspect.value
    edit.value = defaultEditParams(img.naturalWidth, img.naturalHeight)
    await renderPreview()
  } catch {
    editorError.value = t('screenshot.editNotSupported') as string
  } finally {
    editorImageLoading.value = false
  }
}

async function renderPreview(maxSide = FINAL_PREVIEW_MAX_SIDE) {
  if (!editorImage.value) return
  const seq = ++previewRenderSeq
  if (maxSide === FINAL_PREVIEW_MAX_SIDE) previewBusy.value = true
  try {
    const isFast = maxSide !== FINAL_PREVIEW_MAX_SIDE
    let workerBlob: Blob | null = null
    try {
      workerBlob = await renderPreviewInWorker(edit.value, maxSide, isFast)
    } catch {
      previewWorkerReady = false
    }
    let blob: Blob
    if (workerBlob) {
      blob = workerBlob
    } else {
      const canvas = processImage(editorImage.value, edit.value, maxSide)
      // Use JPEG for fast previews to avoid massive PNG compression lag during dragging
      blob = await canvasToBlob(
        canvas,
        isFast ? 'image/jpeg' : 'image/png',
        isFast ? 0.8 : undefined,
      )
    }
    const nextUrl = URL.createObjectURL(blob)
    if (seq <= displayedPreviewSeq || !editorOpen.value) {
      URL.revokeObjectURL(nextUrl)
      return
    }
    displayedPreviewSeq = seq
    if (editorPreviewUrl.value) URL.revokeObjectURL(editorPreviewUrl.value)
    editorPreviewUrl.value = nextUrl
  } finally {
    if (seq === previewRenderSeq) {
      previewBusy.value = false
    }
  }
}

let previewRaf: number | null = null

function schedulePreview(immediate = false, highQuality = false) {
  if (!editorOpen.value || !editorImage.value || editorError.value) return
  if (highQuality) pendingHighQualityPreview = true

  if (immediate) {
    if (previewTimer !== null) {
      window.clearTimeout(previewTimer)
      previewTimer = null
    }
    if (previewRaf !== null) {
      cancelAnimationFrame(previewRaf)
    }
    const runPreview = () => {
      previewTimer = null
      previewRaf = null
      const useFinal = pendingHighQualityPreview
      pendingHighQualityPreview = false
      const fastSide = isInteractiveAdjusting.value ? DRAG_PREVIEW_MAX_SIDE : FAST_PREVIEW_MAX_SIDE
      void renderPreview(useFinal ? FINAL_PREVIEW_MAX_SIDE : fastSide)
    }
    previewRaf = requestAnimationFrame(runPreview)
    return
  }

  // If a fast RAF render is already queued (e.g. from mouse drag), don't downgrade it to a timeout.
  if (previewRaf !== null) {
    return
  }

  if (previewTimer !== null) {
    window.clearTimeout(previewTimer)
    previewTimer = null
  }

  const runPreview = () => {
    previewTimer = null
    previewRaf = null
    const useFinal = pendingHighQualityPreview
    pendingHighQualityPreview = false
    const fastSide = isInteractiveAdjusting.value ? DRAG_PREVIEW_MAX_SIDE : FAST_PREVIEW_MAX_SIDE
    void renderPreview(useFinal ? FINAL_PREVIEW_MAX_SIDE : fastSide)
  }

  // reduced from 90 to 10 for much snappier real-time feeling while watching edits
  previewTimer = window.setTimeout(runPreview, highQuality ? 24 : 10)
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
  schedulePreview(true, true)
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
  editorSourceBlob.value = null
  editorError.value = ''
  editorImageLoading.value = false
  editorPlaceholderAspect.value = null
  activeTool.value = null
  cropDragMode.value = null
  isInteractiveAdjusting.value = false
  pendingHighQualityPreview = false
  window.removeEventListener('pointermove', onCropPointerMove)
  window.removeEventListener('pointerup', onCropPointerUp)
  if (editorInputUrl.value) URL.revokeObjectURL(editorInputUrl.value)
  editorInputUrl.value = ''
  if (editorPreviewUrl.value) URL.revokeObjectURL(editorPreviewUrl.value)
  editorPreviewUrl.value = ''
}

function toggleTool(tool: EditorTool) {
  activeTool.value = activeTool.value === tool ? null : tool
}

/* ---- graphical crop overlay drag ---- */
function cropImgScale(): { sx: number; sy: number } {
  if (!bounds.value.width || !bounds.value.height) return { sx: 1, sy: 1 }
  const r = imgRect()
  if (!r.w || !r.h) return { sx: 1, sy: 1 }
  return { sx: r.w / bounds.value.width, sy: r.h / bounds.value.height }
}

function onCropPointerDown(e: PointerEvent, mode: CropDragMode) {
  e.preventDefault()
  isInteractiveAdjusting.value = true
  cropDragMode.value = mode
  const c = edit.value.crop!
  cropDragStart.value = {
    mx: e.clientX,
    my: e.clientY,
    cx: c.x,
    cy: c.y,
    cw: c.width,
    ch: c.height,
  }
  window.addEventListener('pointermove', onCropPointerMove)
  window.addEventListener('pointerup', onCropPointerUp)
  window.addEventListener('pointercancel', onCropPointerUp)
  window.addEventListener('blur', onCropPointerUp)
}

function onCropPointerMove(e: PointerEvent) {
  if (!cropDragMode.value || !edit.value.crop) return
  const { sx, sy } = cropImgScale()
  const dx = (e.clientX - cropDragStart.value.mx) / sx
  const dy = (e.clientY - cropDragStart.value.my) / sy
  const W = bounds.value.width
  const H = bounds.value.height
  const s = cropDragStart.value
  const c = edit.value.crop
  const MIN = 20

  if (cropDragMode.value === 'move') {
    c.x = clamp(Math.round(s.cx + dx), 0, W - s.cw)
    c.y = clamp(Math.round(s.cy + dy), 0, H - s.ch)
  } else {
    let nx = s.cx,
      ny = s.cy,
      nw = s.cw,
      nh = s.ch
    if (cropDragMode.value.includes('w')) {
      nx = clamp(Math.round(s.cx + dx), 0, s.cx + s.cw - MIN)
      nw = s.cx + s.cw - nx
    }
    if (cropDragMode.value.includes('e')) {
      nw = clamp(Math.round(s.cw + dx), MIN, W - s.cx)
    }
    if (cropDragMode.value.includes('n')) {
      ny = clamp(Math.round(s.cy + dy), 0, s.cy + s.ch - MIN)
      nh = s.cy + s.ch - ny
    }
    if (cropDragMode.value.includes('s')) {
      nh = clamp(Math.round(s.ch + dy), MIN, H - s.cy)
    }
    c.x = nx
    c.y = ny
    c.width = nw
    c.height = nh
  }
}

function onCropPointerUp() {
  cropDragMode.value = null
  isInteractiveAdjusting.value = false
  window.removeEventListener('pointermove', onCropPointerMove)
  window.removeEventListener('pointerup', onCropPointerUp)
  window.removeEventListener('pointercancel', onCropPointerUp)
  window.removeEventListener('blur', onCropPointerUp)
  schedulePreview(false, true)
}

function resetSlider() {
  if (!activeSlider.value) return
  edit.value[activeSlider.value.key] = 0
}

function toolGlyph(tool: EditorTool): string {
  if (tool === 'crop') return 'CP'
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

watch(aircraftGroups, () => {
  ensureGroupVideoCovers()
})

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
  pendingHighQualityPreview = false
  for (const pending of previewWorkerPending.values()) {
    pending.reject(new Error('preview_worker_terminated'))
  }
  previewWorkerPending.clear()
  if (previewWorker) {
    previewWorker.terminate()
    previewWorker = null
    previewWorkerReady = false
  }
  if (editorInputUrl.value) URL.revokeObjectURL(editorInputUrl.value)
  if (editorPreviewUrl.value) URL.revokeObjectURL(editorPreviewUrl.value)
  for (const url of videoCoverUrls.values()) {
    URL.revokeObjectURL(url)
  }
  videoCoverUrls.clear()
  clearAllEnhancedPreviews()
})
</script>

<template>
  <div class="h-full">
    <div class="h-full flex flex-col p-5 overflow-hidden screenshot-view">
      <div class="mb-4 flex items-center justify-between gap-3">
        <div>
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">
            {{ $t('screenshot.title') }}
          </h2>
          <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
            {{ $t('screenshot.subtitle') }}
          </p>
        </div>
        <button
          class="px-3 py-1.5 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white/70 dark:bg-gray-800/60 hover:bg-gray-50 dark:hover:bg-gray-700/60 transition-colors"
          :disabled="loading"
          @click="load"
        >
          {{ loading ? $t('screenshot.loading') : $t('settings.refreshLogs') }}
        </button>
      </div>

      <div
        v-if="!appStore.xplanePath"
        class="flex-1 flex flex-col items-center justify-center text-center"
      >
        <p class="text-gray-500 dark:text-gray-400">{{ $t('screenshot.noPath') }}</p>
        <router-link
          to="/settings"
          class="mt-3 text-sm text-blue-600 dark:text-blue-400 hover:underline"
        >
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
            <button
              class="px-2.5 py-1.5 text-xs sm:text-sm"
              :class="filterType === 'all' ? 'bg-blue-600 text-white' : 'bg-white dark:bg-gray-800'"
              @click="filterType = 'all'"
            >
              {{ $t('screenshot.typeAll') }}
            </button>
            <button
              class="px-2.5 py-1.5 text-xs sm:text-sm border-l border-gray-200 dark:border-gray-700"
              :class="
                filterType === 'image' ? 'bg-blue-600 text-white' : 'bg-white dark:bg-gray-800'
              "
              @click="filterType = 'image'"
            >
              {{ $t('screenshot.typeImage') }}
            </button>
            <button
              class="px-2.5 py-1.5 text-xs sm:text-sm border-l border-gray-200 dark:border-gray-700"
              :class="
                filterType === 'video' ? 'bg-blue-600 text-white' : 'bg-white dark:bg-gray-800'
              "
              @click="filterType = 'video'"
            >
              {{ $t('screenshot.typeVideo') }}
            </button>
          </div>
          <select
            v-model="sort"
            class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm"
          >
            <option value="time">{{ $t('screenshot.sortTime') }}</option>
            <option value="name">{{ $t('screenshot.sortName') }}</option>
            <option value="size">{{ $t('screenshot.sortSize') }}</option>
          </select>
        </div>

        <div class="flex-1 overflow-y-auto">
          <div v-if="loading" class="h-full flex items-center justify-center">
            <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-500"></div>
          </div>
          <div
            v-else-if="error"
            class="rounded-xl border border-red-200 dark:border-red-900/40 bg-red-50 dark:bg-red-900/10 p-4 text-sm text-red-700 dark:text-red-300"
          >
            {{ $t('screenshot.loadFailed') }}: {{ error }}
          </div>
          <div
            v-else-if="!activeAircraftGroup && aircraftGroups.length === 0"
            class="h-full flex items-center justify-center text-gray-500 dark:text-gray-400"
          >
            {{ $t('screenshot.empty') }}
          </div>
          <div v-else-if="!activeAircraftGroup" class="space-y-2 pb-3">
            <button
              v-for="group in aircraftGroups"
              :key="group.key"
              class="w-full text-left rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-4 py-3 hover:shadow-md transition-shadow"
              @click="openAircraftGroup(group.key)"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-3 min-w-0">
                  <div
                    class="w-20 h-12 rounded-md overflow-hidden bg-gray-100 dark:bg-gray-900 shrink-0"
                  >
                    <img
                      v-if="
                        group.coverItem.mediaType === 'image' &&
                        !isThumbFailed(group.coverItem.fileName)
                      "
                      :src="getGroupCoverSrc(group.coverItem)"
                      :alt="group.coverItem.name"
                      class="w-full h-full object-cover"
                      loading="lazy"
                      @error="markThumbFailed(group.coverItem.fileName)"
                    />
                    <img
                      v-else-if="
                        group.coverItem.mediaType === 'video' && getGroupCoverSrc(group.coverItem)
                      "
                      :src="getGroupCoverSrc(group.coverItem)"
                      :alt="group.coverItem.name"
                      class="w-full h-full object-cover"
                      loading="lazy"
                    />
                    <video
                      v-else-if="
                        group.coverItem.mediaType === 'video' && group.coverItem.previewable
                      "
                      :src="toSrc(group.coverItem.path)"
                      class="w-full h-full object-cover"
                      muted
                      playsinline
                      preload="metadata"
                      @loadeddata="captureVideoCover($event, group.coverItem)"
                    ></video>
                    <div
                      v-else
                      class="w-full h-full flex items-center justify-center text-gray-400 dark:text-gray-500 text-[10px]"
                    >
                      {{ $t('screenshot.noPreview') }}
                    </div>
                  </div>
                  <div class="min-w-0">
                    <p
                      class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate"
                      :title="group.label"
                    >
                      {{ group.label }}
                    </p>
                    <p class="text-[11px] text-gray-500 dark:text-gray-400 mt-0.5">
                      {{ $t('screenshot.aircraftCount', { count: group.items.length }) }} ·
                      {{ fmtSize(group.totalSize) }}
                    </p>
                  </div>
                </div>
                <p class="text-[11px] text-gray-400 dark:text-gray-500 whitespace-nowrap">
                  {{ fmtTime(group.latestModified) }}
                </p>
              </div>
            </button>
          </div>
          <template v-else>
            <div
              class="flex items-center justify-between mb-3 rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2"
            >
              <button
                class="text-sm px-2 py-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300"
                @click="backToAircraftGroups"
              >
                {{ $t('common.back') }}
              </button>
              <p
                class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate"
                :title="activeAircraftGroupData?.label || ''"
              >
                {{ activeAircraftGroupData?.label || '' }}
              </p>
              <p class="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                {{ $t('screenshot.aircraftCount', { count: groupedItems.length }) }}
              </p>
            </div>

            <div
              v-if="groupedItems.length === 0"
              class="h-full flex items-center justify-center text-gray-500 dark:text-gray-400"
            >
              {{ $t('screenshot.empty') }}
            </div>
            <div v-else class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3 pb-3">
              <button
                v-for="item in groupedItems"
                :key="item.id"
                class="text-left rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden hover:shadow-md transition-shadow"
                @click="selected = item"
              >
                <div class="relative aspect-[16/10] bg-gray-100 dark:bg-gray-900">
                  <img
                    v-if="item.mediaType === 'image' && !isThumbFailed(item.fileName)"
                    :src="toSrc(item.path)"
                    :alt="item.name"
                    class="w-full h-full object-cover"
                    loading="lazy"
                    @error="markThumbFailed(item.fileName)"
                  />
                  <video
                    v-else-if="item.mediaType === 'video' && item.previewable"
                    :src="toSrc(item.path)"
                    class="w-full h-full object-cover"
                    muted
                    preload="metadata"
                  ></video>
                  <div
                    v-else
                    class="w-full h-full flex items-center justify-center text-gray-400 dark:text-gray-500"
                  >
                    {{ $t('screenshot.noPreview') }}
                  </div>
                  <span
                    v-if="item.mediaType === 'video'"
                    class="absolute right-2 top-2 px-1.5 py-0.5 rounded bg-black/60 text-white text-[10px] tracking-wide"
                    >{{ $t('screenshot.videoTag') }}</span
                  >
                  <span
                    v-if="item.mediaType === 'image' && hasEnhancedPreview(item.fileName)"
                    class="absolute left-2 top-2 px-1.5 py-0.5 rounded bg-emerald-500/85 text-white text-[10px] tracking-wide"
                    >AI</span
                  >
                </div>
                <div class="px-2.5 py-2 space-y-1">
                  <p
                    class="text-xs font-medium text-gray-900 dark:text-gray-100 truncate"
                    :title="item.name"
                  >
                    {{ item.name }}
                  </p>
                  <p class="text-[11px] text-gray-500 dark:text-gray-400">
                    {{ fmtSize(item.size) }} · {{ fmtTime(item.modifiedAt) }}
                  </p>
                </div>
              </button>
            </div>
          </template>
        </div>
      </template>
    </div>

    <Teleport to="body">
      <div
        v-if="selected"
        class="fixed inset-0 z-[110] bg-black/80 backdrop-blur-sm p-3 sm:p-6"
        @click.self="selected = null"
      >
        <div class="h-full w-full max-w-[1240px] mx-auto flex flex-col">
          <div class="flex justify-end mb-2 gap-2">
            <div class="relative">
              <button
                class="round-icon-btn"
                :title="$t('screenshot.shareReddit')"
                @click.stop="toggleShareMenu"
              >
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
                  />
                </svg>
              </button>
              <div
                v-if="shareMenuOpen"
                class="absolute right-0 mt-2 w-40 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shadow-xl z-10 py-1"
              >
                <button class="share-menu-item" @click.stop="shareToRedditFromMenu">
                  <span
                    class="inline-flex w-5 h-5 items-center justify-center rounded-full bg-[#ff4500]"
                  >
                    <svg class="w-3.5 h-3.5 text-white" viewBox="0 0 24 24" fill="currentColor">
                      <path
                        d="M12 0C5.373 0 0 5.373 0 12c0 3.314 1.343 6.314 3.515 8.485l-2.286 2.286C.775 23.225 1.097 24 1.738 24H12c6.627 0 12-5.373 12-12S18.627 0 12 0Zm4.388 3.199c1.104 0 1.999.895 1.999 1.999 0 1.105-.895 2-1.999 2-.946 0-1.739-.657-1.947-1.539v.002c-1.147.162-2.032 1.15-2.032 2.341v.007c1.776.067 3.4.567 4.686 1.363.473-.363 1.064-.58 1.707-.58 1.547 0 2.802 1.254 2.802 2.802 0 1.117-.655 2.081-1.601 2.531-.088 3.256-3.637 5.876-7.997 5.876-4.361 0-7.905-2.617-7.998-5.87-.954-.447-1.614-1.415-1.614-2.538 0-1.548 1.255-2.802 2.803-2.802.645 0 1.239.218 1.712.585 1.275-.79 2.881-1.291 4.64-1.365v-.01c0-1.663 1.263-3.034 2.88-3.207.188-.911.993-1.595 1.959-1.595Zm-8.085 8.376c-.784 0-1.459.78-1.506 1.797-.047 1.016.64 1.429 1.426 1.429.786 0 1.371-.369 1.418-1.385.047-1.017-.553-1.841-1.338-1.841Zm7.406 0c-.786 0-1.385.824-1.338 1.841.047 1.017.634 1.385 1.418 1.385.785 0 1.473-.413 1.426-1.429-.046-1.017-.721-1.797-1.506-1.797Zm-3.703 4.013c-.974 0-1.907.048-2.77.135-.147.015-.241.168-.183.305.483 1.154 1.622 1.964 2.953 1.964 1.33 0 2.47-.81 2.953-1.964.057-.137-.037-.29-.184-.305-.863-.087-1.795-.135-2.769-.135Z"
                      />
                    </svg>
                  </span>
                  <span>Reddit</span>
                </button>
              </div>
            </div>
            <button class="round-icon-btn" :title="$t('common.close')" @click="selected = null">
              <svg
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <div
            class="flex-1 min-h-0 rounded-xl bg-black/40 border border-white/10 overflow-hidden flex items-center justify-center"
          >
            <img
              v-if="selected.mediaType === 'image'"
              :src="selectedSrc"
              :alt="selected.name"
              class="max-w-full max-h-full object-contain"
            />
            <video
              v-else-if="selected.previewable"
              :src="selectedSrc"
              class="max-w-full max-h-full object-contain"
              controls
              autoplay
            ></video>
            <p v-else class="text-sm text-white/70">{{ $t('screenshot.unsupportedPreview') }}</p>
          </div>
          <div
            class="mt-3 rounded-xl bg-gray-900/70 border border-white/10 p-2.5 flex flex-wrap items-center gap-2"
          >
            <button
              v-if="selected.mediaType === 'image' && selected.editable"
              class="action-icon action-blue"
              @click="openEditor(selected)"
            >
              <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5M16.5 3.5a2.1 2.1 0 113 3L12 14l-4 1 1-4 7.5-7.5z"
                />
              </svg>
              <span class="action-label">{{ $t('screenshot.openEditor') }}</span>
            </button>
            <button class="action-icon action-emerald" @click="copyMedia(selected)">
              <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <rect x="9" y="9" width="10" height="10" rx="2" ry="2" stroke-width="2" />
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M5 15V7a2 2 0 012-2h8"
                />
              </svg>
              <span class="action-label">{{
                selected.mediaType === 'image'
                  ? $t('screenshot.copyImage')
                  : $t('screenshot.copyPath')
              }}</span>
            </button>
            <button
              class="action-icon action-indigo"
              :disabled="busyFile === selected.fileName"
              @click="saveAs(selected)"
            >
              <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 16V4m0 12l-4-4m4 4l4-4M4 18v1a1 1 0 001 1h14a1 1 0 001-1v-1"
                />
              </svg>
              <span class="action-label">{{ $t('screenshot.saveAs') }}</span>
            </button>
            <button
              class="action-icon action-rose"
              :disabled="busyFile === selected.fileName"
              @click="askDelete(selected)"
            >
              <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M19 7l-1 12a2 2 0 01-2 2H8a2 2 0 01-2-2L5 7m3 0V5a1 1 0 011-1h6a1 1 0 011 1v2M4 7h16"
                />
              </svg>
              <span class="action-label">{{ $t('common.delete') }}</span>
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div v-if="editorOpen" class="fixed inset-0 z-[120] bg-black/75 backdrop-blur-sm p-2 sm:p-4">
        <div
          class="h-full max-w-[1320px] mx-auto rounded-2xl bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 overflow-hidden flex flex-col"
        >
          <div
            class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between"
          >
            <h3 class="font-semibold text-gray-900 dark:text-gray-100">
              {{ $t('screenshot.editTitle') }}
            </h3>
            <div class="flex items-center gap-1.5">
              <button
                class="text-sm px-2 py-1 flex items-center gap-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-600 dark:text-gray-400"
                :disabled="previewBusy || saveBusy"
                @click="resetEditor"
                :title="$t('screenshot.reset')"
              >
                <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 12a9 9 0 109-9 9.75 9.75 0 00-6.74 2.74L3 8m0 0V3m0 5h5"
                  />
                </svg>
                <span class="hidden sm:inline">{{ $t('screenshot.reset') }}</span>
              </button>
              <button
                class="text-sm px-2 py-1 flex items-center gap-1.5 rounded hover:bg-green-50 dark:hover:bg-green-900/20 text-green-600 dark:text-green-500"
                :disabled="saveBusy"
                @click="saveEdited(true)"
                :title="$t('screenshot.saveOverwrite')"
              >
                <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 4h12l2 2v14H5V4zm3 0v5h8V4M9 20v-6h6v6"
                  />
                </svg>
                <span class="hidden sm:inline">{{ $t('screenshot.saveOverwrite') }}</span>
              </button>
              <button
                class="text-sm px-2 py-1 flex items-center gap-1.5 rounded hover:bg-blue-50 dark:hover:bg-blue-900/20 text-blue-600 dark:text-blue-500"
                :disabled="saveBusy"
                @click="saveEdited(false)"
                :title="$t('screenshot.saveAs')"
              >
                <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 16V4m0 12l-4-4m4 4l4-4M4 18v1a1 1 0 001 1h14a1 1 0 001-1v-1"
                  />
                </svg>
                <span class="hidden sm:inline">{{ $t('screenshot.saveAs') }}</span>
              </button>
              <button
                class="text-sm px-2 py-1 flex items-center gap-1.5 rounded hover:bg-emerald-50 dark:hover:bg-emerald-900/20 text-emerald-600 dark:text-emerald-500"
                :disabled="saveBusy"
                @click="copyEdited"
                :title="$t('screenshot.copyImage')"
              >
                <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <rect x="9" y="9" width="10" height="10" rx="2" ry="2" stroke-width="2" />
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 15V7a2 2 0 012-2h8"
                  />
                </svg>
                <span class="hidden sm:inline">{{ $t('screenshot.copyImage') }}</span>
              </button>
              <div class="w-px h-5 bg-gray-200 dark:bg-gray-700 mx-1"></div>
              <button
                class="text-sm px-3 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium transition-colors"
                @click="closeEditor"
              >
                {{ $t('common.close') }}
              </button>
            </div>
          </div>

          <div class="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-[1fr_300px]">
            <div
              class="p-3 bg-gray-50 dark:bg-gray-950 border-b lg:border-b-0 lg:border-r border-gray-200 dark:border-gray-700 flex items-center justify-center min-h-0"
            >
              <div
                ref="cropContainerRef"
                class="w-full h-full min-h-0 bg-black/85 overflow-hidden flex items-center justify-center relative select-none"
              >
                <template v-if="editorImageLoading">
                  <div class="w-full h-full flex items-center justify-center px-6">
                    <div
                      class="w-full max-w-[960px] max-h-full rounded-md border border-white/15 bg-white/5 relative overflow-hidden"
                      :style="editorPlaceholderStyle"
                    >
                      <div
                        class="absolute inset-0 animate-pulse bg-gradient-to-br from-white/10 to-white/5"
                      ></div>
                    </div>
                    <div
                      class="absolute inset-0 flex items-center justify-center pointer-events-none"
                    >
                      <div
                        class="animate-spin rounded-full h-9 w-9 border-2 border-white/35 border-t-white"
                      ></div>
                    </div>
                  </div>
                </template>
                <!-- Crop overlay mode -->
                <template v-else-if="activeTool === 'crop' && editorInputUrl && !editorError">
                  <img
                    ref="cropImgRef"
                    :src="editorInputUrl"
                    :alt="$t('screenshot.editPreview')"
                    class="max-w-full max-h-full object-contain"
                    :style="cropPreviewImageStyle"
                    draggable="false"
                  />
                  <!-- dark overlay outside crop area, rendered via 4 divs -->
                  <template v-if="cropImgRef">
                    <div
                      class="absolute bg-black/55 pointer-events-none"
                      :style="cropOverlayTop"
                    ></div>
                    <div
                      class="absolute bg-black/55 pointer-events-none"
                      :style="cropOverlayBottom"
                    ></div>
                    <div
                      class="absolute bg-black/55 pointer-events-none"
                      :style="cropOverlayLeft"
                    ></div>
                    <div
                      class="absolute bg-black/55 pointer-events-none"
                      :style="cropOverlayRight"
                    ></div>
                    <!-- crop box border -->
                    <div
                      class="absolute border-2 border-white/90 pointer-events-none"
                      :style="cropBoxStyle"
                    >
                      <!-- rule-of-thirds grid lines -->
                      <div class="absolute inset-0 pointer-events-none">
                        <div class="absolute left-1/3 top-0 bottom-0 w-px bg-white/30"></div>
                        <div class="absolute left-2/3 top-0 bottom-0 w-px bg-white/30"></div>
                        <div class="absolute top-1/3 left-0 right-0 h-px bg-white/30"></div>
                        <div class="absolute top-2/3 left-0 right-0 h-px bg-white/30"></div>
                      </div>
                    </div>
                    <!-- drag move area -->
                    <div
                      class="absolute cursor-move"
                      :style="cropBoxStyle"
                      @pointerdown="onCropPointerDown($event, 'move')"
                    ></div>
                    <!-- resize handles -->
                    <div
                      class="crop-handle crop-handle-nw"
                      :style="cropHandleStyle('nw')"
                      @pointerdown="onCropPointerDown($event, 'nw')"
                    ></div>
                    <div
                      class="crop-handle crop-handle-ne"
                      :style="cropHandleStyle('ne')"
                      @pointerdown="onCropPointerDown($event, 'ne')"
                    ></div>
                    <div
                      class="crop-handle crop-handle-sw"
                      :style="cropHandleStyle('sw')"
                      @pointerdown="onCropPointerDown($event, 'sw')"
                    ></div>
                    <div
                      class="crop-handle crop-handle-se"
                      :style="cropHandleStyle('se')"
                      @pointerdown="onCropPointerDown($event, 'se')"
                    ></div>
                    <!-- edge handles -->
                    <div
                      class="crop-handle crop-handle-n"
                      :style="cropHandleStyle('n')"
                      @pointerdown="onCropPointerDown($event, 'n')"
                    ></div>
                    <div
                      class="crop-handle crop-handle-s"
                      :style="cropHandleStyle('s')"
                      @pointerdown="onCropPointerDown($event, 's')"
                    ></div>
                    <div
                      class="crop-handle crop-handle-w"
                      :style="cropHandleStyle('w')"
                      @pointerdown="onCropPointerDown($event, 'w')"
                    ></div>
                    <div
                      class="crop-handle crop-handle-e"
                      :style="cropHandleStyle('e')"
                      @pointerdown="onCropPointerDown($event, 'e')"
                    ></div>
                  </template>
                </template>
                <!-- Normal preview mode -->
                <template v-else>
                  <img
                    v-if="editorSrc && !editorError"
                    :src="editorSrc"
                    :alt="$t('screenshot.editPreview')"
                    class="max-w-full max-h-full object-contain"
                  />
                  <p v-else class="text-sm text-white/70">
                    {{ editorError || $t('screenshot.editNotSupported') }}
                  </p>
                </template>
              </div>
            </div>

            <div
              class="min-h-0 overflow-y-auto p-3 space-y-4 text-xs text-gray-600 dark:text-gray-300 flex flex-col"
            >
              <!-- Tool Selection -->
              <div
                class="rounded-xl border border-gray-200 dark:border-gray-700 p-2 flex flex-wrap gap-2"
              >
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

              <!-- Tool Adjustment Area -->
              <div class="rounded-xl border border-gray-200 dark:border-gray-700 p-2">
                <div v-if="activeTool === 'crop'" class="space-y-1">
                  <div class="flex items-center justify-between">
                    <p
                      class="text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide"
                    >
                      {{ $t('screenshot.rotate') }}
                    </p>
                    <button
                      class="text-[10px] text-blue-500 hover:text-blue-600 dark:text-blue-400"
                      @click="edit.rotate = 0"
                    >
                      {{ $t('screenshot.reset') }}
                    </button>
                  </div>
                  <div class="ruler-container" @pointerdown="onRulerPointerDown($event, true)">
                    <div class="ruler-scale" :style="{ transform: rotateRulerTransform }">
                      <div
                        v-for="n in 21"
                        :key="n"
                        class="ruler-tick"
                        :class="{ 'ruler-tick-major': (n - 1) % 5 === 0 }"
                      ></div>
                    </div>
                    <div class="ruler-pointer"></div>
                    <div class="ruler-value">{{ edit.rotate }}°</div>
                  </div>
                </div>

                <!-- Slider controls -->
                <div v-else-if="activeSlider" class="space-y-2">
                  <div class="flex items-center justify-between">
                    <p
                      class="text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide"
                    >
                      {{ $t(activeSlider.label) }}
                    </p>
                    <button
                      class="text-[10px] text-blue-500 hover:text-blue-600 dark:text-blue-400"
                      @click="resetSlider"
                    >
                      {{ $t('screenshot.reset') }}
                    </button>
                  </div>
                  <div class="ruler-container" @pointerdown="onRulerPointerDown($event, false)">
                    <div class="ruler-scale" :style="{ transform: sliderRulerTransform }">
                      <div
                        v-for="n in 21"
                        :key="n"
                        class="ruler-tick"
                        :class="{ 'ruler-tick-major': (n - 1) % 5 === 0 }"
                      ></div>
                    </div>
                    <div class="ruler-pointer"></div>
                    <div class="ruler-value">{{ activeSliderValue }}</div>
                  </div>
                </div>

                <div v-else class="h-full flex items-center justify-center min-h-[40px]">
                  <p class="text-sm text-gray-400 dark:text-gray-500">
                    {{ $t('screenshot.toolHint') }}
                  </p>
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
.action-icon:focus-visible .action-label,
.action-icon.action-active .action-label {
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

.action-icon:disabled {
  opacity: 0.55;
  cursor: not-allowed;
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

/* ---- Crop handles ---- */
.crop-handle {
  position: absolute;
  width: 10px;
  height: 10px;
  background: white;
  border: 1.5px solid rgba(59, 130, 246, 0.9);
  border-radius: 2px;
  z-index: 2;
}
.crop-handle-nw {
  cursor: nwse-resize;
}
.crop-handle-ne {
  cursor: nesw-resize;
}
.crop-handle-sw {
  cursor: nesw-resize;
}
.crop-handle-se {
  cursor: nwse-resize;
}
.crop-handle-n {
  cursor: ns-resize;
}
.crop-handle-s {
  cursor: ns-resize;
}
.crop-handle-w {
  cursor: ew-resize;
}
.crop-handle-e {
  cursor: ew-resize;
}

/* ---- Moving Ruler Slider ---- */
.ruler-container {
  position: relative;
  height: 28px;
  background: transparent;
  border-radius: 8px;
  overflow: hidden;
  cursor: ew-resize;
  user-select: none;
  touch-action: none;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}
.dark .ruler-container {
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.ruler-scale {
  position: absolute;
  bottom: 0;
  left: 50%;
  width: 400px;
  height: 20px;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  will-change: transform;
  pointer-events: none;
}

.ruler-tick {
  width: 1px;
  height: 6px;
  background: rgb(203, 213, 225);
}
.dark .ruler-tick {
  background: rgb(75, 85, 99);
}

.ruler-tick-major {
  height: 10px;
  width: 1.5px;
  background: rgb(148, 163, 184);
}
.dark .ruler-tick-major {
  background: rgb(100, 116, 139);
}

.ruler-pointer {
  position: absolute;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 2px;
  height: 16px;
  background: rgb(59, 130, 246);
  pointer-events: none;
  z-index: 10;
}

.ruler-value {
  position: absolute;
  top: 2px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  font-weight: 600;
  color: rgb(71, 85, 105);
  pointer-events: none;
}
.dark .ruler-value {
  color: rgb(203, 213, 225);
}
</style>
