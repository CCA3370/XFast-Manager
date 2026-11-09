<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import * as L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import { loadMapTile, type TileResult } from '@/services/mapTiles'
import { smoothMapZoom } from '@/utils/smoothMapZoom'
import { mapCoordinates, type Waypoint } from '@/services/flightPlanDetails'
// Leaflet 1.9's gesture lifecycle keeps existing tile levels during a continuous
// transform; public setView() would invalidate all tiles on every animation frame.
type AnimatedMap = L.Map & {
  _moveStart(changed: boolean): void
  _moveEnd(changed: boolean): void
  _move(center: L.LatLng, zoom: number, data: { pinch: boolean; round: boolean }): void
}
const props = defineProps<{ points: Waypoint[] }>()
const { t } = useI18n()
const host = ref<HTMLDivElement | null>(null)
const failures = ref(0)
const sources = new Map<HTMLElement, string>()
let disposed = false
class CachedMapLayer extends L.GridLayer {
  private pending = new Map<HTMLElement, { controller: AbortController; revoke?: () => void }>()
  constructor() {
    super({
      maxZoom: 19,
      keepBuffer: 2,
      updateWhenZooming: true,
      updateWhenIdle: true,
      attribution:
        '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>',
    })
    this.on('tileunload', (e) => {
      const job = this.pending.get(e.tile)
      job?.controller.abort()
      job?.revoke?.()
      this.pending.delete(e.tile)
      sources.delete(e.tile)
      failures.value = [...sources.values()].filter((s) => s === 'missing').length
    })
  }
  createTile(coords: L.Coords, done: L.DoneCallback) {
    const image = document.createElement('img')
    image.alt = ''
    image.dataset.tileKey = `${coords.z}/${coords.x}/${coords.y}`
    image.width = 256
    image.height = 256
    image.draggable = false
    const job: { controller: AbortController; revoke?: () => void } = {
      controller: new AbortController(),
    }
    this.pending.set(image, job)
    void loadMapTile(coords.z, coords.x, coords.y, job.controller.signal).then(
      (result: TileResult) => {
        if (job.controller.signal.aborted || disposed) {
          result.revoke?.()
          return
        }
        job.revoke = result.revoke
        sources.set(image, result.source)
        image.dataset.mapSource = result.source
        failures.value = [...sources.values()].filter((s) => s === 'missing').length
        image.onload = () => done(undefined, image)
        image.onerror = () => done(new Error('Map image unavailable'), image)
        image.src = result.url
      },
    )
    return image
  }
}
function refreshTiles() {
  if (map && tiles) {
    tiles.remove()
    tiles.addTo(map)
  }
}
const offline = ref(!navigator.onLine)
let map: L.Map | undefined
let route: L.LayerGroup | undefined
let tiles: L.GridLayer | undefined
let observer: ResizeObserver | undefined
let sized = false
let smooth: ReturnType<typeof smoothMapZoom> | undefined
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
const coordinates = computed(() =>
  mapCoordinates(props.points).map((point) => (point ? L.latLng(point) : null)),
)
function fit(animate = true) {
  smooth?.cancel()
  const points = coordinates.value.filter((p) => p !== null)
  if (map && points.length)
    map.fitBounds(L.latLngBounds(points), {
      padding: [35, 35],
      maxZoom: 12,
      animate: animate && sized && !reducedMotion,
      duration: 0.18,
    })
}
function draw() {
  if (!map || !route) return
  route.clearLayers()
  const valid = coordinates.value.filter((p) => p !== null)
  if (!valid.length) map.setView([0, 0], 2)
  let segment: L.LatLng[] = []
  const finish = () => {
    if (segment.length > 1) {
      L.polyline(segment, { color: '#ffffff', weight: 6, opacity: 0.9, interactive: false }).addTo(
        route!,
      )
      L.polyline(segment, { color: '#2563eb', weight: 3, opacity: 0.95, interactive: false }).addTo(
        route!,
      )
    }
    segment = []
  }
  coordinates.value.forEach((point, index) => {
    if (!point) {
      finish()
      return
    }
    segment.push(point)
    const title = document.createElement('span')
    title.textContent = `${index + 1} · ${props.points[index]!.id}`
    L.circleMarker(point, {
      radius: index === 0 || index === props.points.length - 1 ? 5.5 : 4,
      color: '#fff',
      weight: 2,
      fillColor:
        index === 0 ? '#059669' : index === props.points.length - 1 ? '#d97706' : '#2563eb',
      fillOpacity: 1,
    })
      .bindTooltip(title, {
        permanent: props.points.length < 35,
        direction: 'top',
        offset: [0, -5],
        className: 'flight-route-label',
      })
      .addTo(route!)
  })
  finish()
  route.eachLayer((layer) => {
    if (layer instanceof L.CircleMarker) layer.bringToFront()
  })
  fit(false)
}
function networkOnline() {
  offline.value = false
  refreshTiles()
}
function networkOffline() {
  offline.value = true
}
onMounted(() => {
  if (!host.value) return
  map = L.map(host.value, {
    zoomControl: false,
    minZoom: 1,
    maxZoom: 19,
    zoomAnimation: !reducedMotion,
    fadeAnimation: !reducedMotion,
    markerZoomAnimation: !reducedMotion,
    zoomSnap: 0,
    scrollWheelZoom: false,
    zoomDelta: 1,
    worldCopyJump: false,
  })
  tiles = new CachedMapLayer()
  tiles.addTo(map)
  route = L.layerGroup().addTo(map)
  L.control.scale({ imperial: false }).addTo(map)
  draw()
  smooth = smoothMapZoom(
    {
      begin: () => {
        host.value?.classList.add('is-zooming')
        ;(map as AnimatedMap)._moveStart(true)
      },
      end: () => {
        ;(map as AnimatedMap)._moveEnd(true)
        host.value?.classList.remove('is-zooming')
      },
      getZoom: () => map!.getZoom(),
      getMinZoom: () => map!.getMinZoom(),
      getMaxZoom: () => map!.getMaxZoom(),
      setZoomAround: (point, zoom) => {
        const half = map!.getSize().divideBy(2)
        const offset = L.point(point)
          .subtract(half)
          .multiplyBy(1 - 1 / map!.getZoomScale(zoom))
        const center = map!.containerPointToLatLng(half.add(offset))
        ;(map as AnimatedMap)._move(center, zoom, { pinch: true, round: false })
      },
    },
    host.value,
  )
  observer = new ResizeObserver(() => {
    if (!host.value?.clientWidth || !host.value.clientHeight) return
    map?.invalidateSize({ animate: false })
    if (!sized) {
      sized = true
      fit(false)
    }
  })
  observer.observe(host.value)
  window.addEventListener('online', networkOnline)
  window.addEventListener('offline', networkOffline)
})
watch(() => props.points, draw)
onBeforeUnmount(() => {
  window.removeEventListener('online', networkOnline)
  window.removeEventListener('offline', networkOffline)
  smooth?.dispose()
  observer?.disconnect()
  map?.remove()
  map = undefined
  disposed = true
  sources.clear()
})
function attribution(event: MouseEvent) {
  const target = (event.target as HTMLElement).closest('a')
  if (!target) return
  event.preventDefault()
  const href = target.href
  if (href.startsWith('https://leafletjs.com') || href.startsWith('https://www.openstreetmap.org/'))
    void invoke('open_url', { url: href }).catch(() => {})
}
</script>
<template>
  <section class="space-y-2">
    <div class="flex items-center justify-between gap-2">
      <h3 class="font-semibold">{{ t('flightPlanDetails.route') }}</h3>
      <div class="flex gap-1 text-sm">
        <button
          class="map-button"
          :aria-label="t('flightPlanDetails.zoomOut')"
          @click="smooth?.step(-1)"
          v-text="'−'"
        /><button
          class="map-button"
          :aria-label="t('flightPlanDetails.zoomIn')"
          @click="smooth?.step(1)"
          v-text="'+'"
        /><button class="map-button" @click="fit()">{{ t('flightPlanDetails.fit') }}</button>
      </div>
    </div>
    <div
      ref="host"
      class="route-map rounded-lg border border-gray-400/20"
      role="region"
      :aria-label="t('flightPlanDetails.route')"
      @click="attribution"
    />
    <p v-if="failures" role="status" class="text-xs text-amber-600">
      {{ t('flightPlanDetails.missingMap') }}
    </p>
    <p v-else-if="offline" role="status" class="text-xs text-amber-600">
      {{ t('flightPlanDetails.mapUnavailable') }}
    </p>
  </section>
</template>
<style scoped>
.route-map.is-zooming :deep(.leaflet-tile-container) {
  will-change: transform;
}
.route-map :deep(.leaflet-zoom-anim .leaflet-zoom-animated) {
  transition-duration: 0.14s;
}
.route-map {
  height: clamp(280px, 42vh, 420px);
  z-index: 0;
  background: #e5edf3;
}
.route-map,
.route-map :deep(*) {
  user-select: none;
  -webkit-user-select: none;
}
.route-map :deep(.flight-route-label) {
  background: rgb(255 255 255 / 90%);
  border: 1px solid rgb(148 163 184 / 25%);
  border-radius: 5px;
  box-shadow: 0 1px 4px rgb(15 23 42 / 12%);
  color: #153f85;
  font:
    11px ui-monospace,
    monospace;
  padding: 2px 4px;
}
.map-button {
  border: 1px solid #94a3b84d;
  border-radius: 0.375rem;
  padding: 0.2rem 0.65rem;
}
</style>
