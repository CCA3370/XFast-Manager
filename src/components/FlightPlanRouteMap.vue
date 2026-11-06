<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import * as L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import type { GeoJsonObject } from 'geojson'
import countries from '@/assets/geography/countries.json'
import { mapCoordinates, type Waypoint } from '@/services/flightPlanDetails'
const props = defineProps<{ points: Waypoint[] }>()
const { t } = useI18n()
const host = ref<HTMLDivElement | null>(null)
const failures = ref(0)
const offline = ref(!navigator.onLine)
let map: L.Map | undefined
let route: L.LayerGroup | undefined
let offlineLayer: L.LayerGroup | undefined
let tiles: L.TileLayer | undefined
let observer: ResizeObserver | undefined
let sized = false
let offlineRenderer: L.Canvas | undefined
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
const failedTiles = new Set<HTMLElement>()
const coordinates = computed(() =>
  mapCoordinates(props.points).map((point) => (point ? L.latLng(point) : null)),
)
function fit(animate = true) {
  const points = coordinates.value.filter((p) => p !== null)
  if (map && points.length)
    map.fitBounds(L.latLngBounds(points), {
      padding: [35, 35],
      maxZoom: 12,
      animate: animate && sized && !reducedMotion,
      duration: 0.35,
    })
}
function draw() {
  if (!map || !route || !offlineLayer) return
  route.clearLayers()
  offlineLayer.clearLayers()
  const valid = coordinates.value.filter((p) => p !== null)
  if (!valid.length) map.setView([0, 0], 2)
  const world = valid.length
    ? Math.round(valid.reduce((sum, p) => sum + p.lng, 0) / valid.length / 360) * 360
    : 0
  for (const offset of [-360, 0, 360]) {
    L.geoJSON(
      {
        type: 'FeatureCollection',
        features: countries.map((c) => ({
          type: 'Feature',
          properties: {},
          geometry: { type: c.type, coordinates: c.coordinates },
        })),
      } as GeoJsonObject,
      {
        pane: 'offline-basemap',
        interactive: false,
        coordsToLatLng: (c) => L.latLng(c[1]!, c[0]! + offset + world),
        style: {
          renderer: offlineRenderer,
          color: '#9daea8',
          weight: 0.6,
          fillColor: '#f1f1e7',
          fillOpacity: 1,
        },
      },
    ).addTo(offlineLayer)
  }
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
  // Reattach to let Leaflet round the tile zoom. redraw() uses fractional
  // map zooms verbatim in Leaflet 1.9, yielding invalid OSM tile URLs.
  if (map && tiles) {
    tiles.remove()
    tiles.addTo(map)
  }
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
    zoomSnap: 0.5,
    zoomDelta: 0.5,
    wheelPxPerZoomLevel: 100,
    wheelDebounceTime: 40,
    worldCopyJump: false,
  })
  map.createPane('offline-basemap').style.zIndex = '150'
  offlineRenderer = L.canvas({ pane: 'offline-basemap', padding: 0.3 })
  offlineLayer = L.layerGroup().addTo(map)
  // Normal browser caching, no prefetch: request only the current viewport.
  tiles = L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
    maxZoom: 19,
    keepBuffer: 1,
    updateWhenZooming: false,
    updateWhenIdle: true,
    attribution:
      '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a> · Natural Earth',
  })
  tiles.on('tileerror', (e: L.TileErrorEvent) => {
    failedTiles.add(e.tile)
    e.tile.style.visibility = 'hidden'
    failures.value = failedTiles.size
  })
  const clearFailure = (e: L.TileEvent) => {
    failedTiles.delete(e.tile)
    e.tile.style.visibility = ''
    failures.value = failedTiles.size
  }
  tiles.on('tileload', clearFailure)
  tiles.on('tileunload', clearFailure)
  tiles.addTo(map)
  route = L.layerGroup().addTo(map)
  L.control.scale({ imperial: false }).addTo(map)
  draw()
  observer = new ResizeObserver(() => {
    if (!host.value?.clientWidth || !host.value.clientHeight) return
    map?.invalidateSize({ animate: false })
    if (!sized) {
      sized = true
      fit()
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
  observer?.disconnect()
  map?.remove()
  map = undefined
  failedTiles.clear()
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
          @click="map?.zoomOut()"
          v-text="'−'"
        /><button
          class="map-button"
          :aria-label="t('flightPlanDetails.zoomIn')"
          @click="map?.zoomIn()"
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
    <p v-if="failures || offline" role="status" class="text-xs text-amber-600">
      {{ t('flightPlanDetails.mapUnavailable') }}
    </p>
    <p class="text-xs leading-5 text-gray-500">{{ t('flightPlanDetails.scope') }}</p>
  </section>
</template>
<style scoped>
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
