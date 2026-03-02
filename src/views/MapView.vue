<template>
  <div class="map-page h-full relative overflow-hidden bg-slate-900 text-gray-100">
    <div v-if="!appStore.xplanePath" class="absolute inset-0 z-30 flex items-center justify-center p-6">
      <div class="max-w-lg rounded-xl border border-amber-500/30 bg-amber-500/10 p-6 text-center">
        <h2 class="text-lg font-semibold text-amber-200">{{ t('map.pathRequiredTitle') }}</h2>
        <p class="mt-2 text-sm text-amber-100/90">{{ t('map.pathRequiredDesc') }}</p>
      </div>
    </div>

    <div class="absolute top-3 left-3 z-20 w-[420px] max-w-[calc(100%-1.5rem)] space-y-2">
      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl">
        <div class="flex items-center gap-2">
          <input
            v-model="searchQuery"
            type="text"
            class="flex-1 rounded-md border border-gray-700 bg-slate-800 px-3 py-2 text-sm text-gray-100 placeholder-gray-400 focus:border-blue-400 focus:outline-none"
            :placeholder="t('map.searchPlaceholder')"
          />
          <button
            class="rounded-md border border-gray-600 bg-slate-800 px-3 py-2 text-xs hover:bg-slate-700"
            @click="refreshMapData"
          >
            {{ t('map.refresh') }}
          </button>
          <select
            :value="mapStore.mapStyleUrl"
            class="rounded-md border border-gray-600 bg-slate-800 px-2 py-2 text-xs text-gray-200"
            @change="onMapStyleChange"
          >
            <option
              v-for="style in mapStyleOptions"
              :key="style.value"
              :value="style.value"
            >
              {{ style.label }}
            </option>
          </select>
        </div>

        <div v-if="searchResults.length > 0" class="mt-2 max-h-56 overflow-y-auto rounded-md border border-gray-700">
          <button
            v-for="airport in searchResults"
            :key="`search-${airport.icao}`"
            class="flex w-full items-center justify-between border-b border-gray-700/70 px-3 py-2 text-left text-sm last:border-b-0 hover:bg-slate-700/60"
            @click="selectAirport(airport)"
          >
            <span class="font-mono text-blue-300">{{ airport.icao }}</span>
            <span class="ml-2 flex-1 truncate text-gray-200">{{ airport.name }}</span>
          </button>
        </div>
      </div>

      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl">
        <div class="mb-2 flex items-center justify-between text-xs text-gray-400">
          <span>{{ t('map.layersTitle') }}</span>
          <span>
            {{ dataStatus.loaded ? t('map.indexReady') : t('map.indexLoading') }}
          </span>
        </div>
        <div class="grid grid-cols-3 gap-2 text-xs">
          <label
            v-for="item in layerItems"
            :key="item.key"
            class="flex cursor-pointer items-center gap-1.5 rounded-md border border-gray-700 bg-slate-800 px-2 py-1.5"
          >
            <input
              type="checkbox"
              class="h-3.5 w-3.5"
              :checked="mapStore.layerVisibility[item.key]"
              @change="toggleLayer(item.key)"
            />
            <span>{{ t(item.label) }}</span>
          </label>
        </div>

        <div class="mt-3 flex items-center gap-3 text-xs text-gray-300">
          <label class="flex items-center gap-2">
            <span>{{ t('map.followPlane') }}</span>
            <input
              type="checkbox"
              class="h-3.5 w-3.5"
              :checked="mapStore.followPlane"
              @change="onToggleFollowPlane"
            />
          </label>
          <label class="flex items-center gap-2">
            <span>{{ t('map.navRadius') }}</span>
            <input
              type="number"
              min="10"
              max="200"
              :value="mapStore.navRadiusNm"
              class="w-16 rounded border border-gray-700 bg-slate-800 px-2 py-1"
              @change="onRadiusInput"
            />
          </label>
          <label class="flex items-center gap-2">
            <span>{{ t('map.vatsimInterval') }}</span>
            <input
              type="number"
              min="5"
              max="120"
              :value="mapStore.vatsimRefreshInterval"
              class="w-14 rounded border border-gray-700 bg-slate-800 px-2 py-1"
              @change="onVatsimIntervalInput"
            />
          </label>
          <button
            class="rounded-md border border-gray-600 bg-slate-800 px-2 py-1 text-[11px] hover:bg-slate-700"
            @click="reconnectPlaneStream"
          >
            {{ t('map.reconnectPlane') }}
          </button>
        </div>
      </div>

      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl">
        <div class="mb-2 flex items-center justify-between text-xs text-gray-400">
          <span>{{ t('map.filtersTitle') }}</span>
          <button
            class="rounded border border-gray-600 px-2 py-0.5 text-[11px] hover:bg-slate-700/70"
            @click="resetAirportFilters"
          >
            {{ t('map.resetFilters') }}
          </button>
        </div>

        <div class="grid grid-cols-2 gap-2 text-xs text-gray-200">
          <label class="flex items-center gap-2 rounded-md border border-gray-700 bg-slate-800 px-2 py-1.5">
            <input
              type="checkbox"
              class="h-3.5 w-3.5"
              :checked="mapStore.airportFilters.showLand"
              @change="onAirportFilterToggle('showLand', $event)"
            />
            <span>{{ t('map.filters.land') }}</span>
          </label>
          <label class="flex items-center gap-2 rounded-md border border-gray-700 bg-slate-800 px-2 py-1.5">
            <input
              type="checkbox"
              class="h-3.5 w-3.5"
              :checked="mapStore.airportFilters.showSeaplane"
              @change="onAirportFilterToggle('showSeaplane', $event)"
            />
            <span>{{ t('map.filters.seaplane') }}</span>
          </label>
          <label class="flex items-center gap-2 rounded-md border border-gray-700 bg-slate-800 px-2 py-1.5">
            <input
              type="checkbox"
              class="h-3.5 w-3.5"
              :checked="mapStore.airportFilters.showHeliport"
              @change="onAirportFilterToggle('showHeliport', $event)"
            />
            <span>{{ t('map.filters.heliport') }}</span>
          </label>
          <label class="flex items-center gap-2 rounded-md border border-gray-700 bg-slate-800 px-2 py-1.5">
            <input
              type="checkbox"
              class="h-3.5 w-3.5"
              :checked="mapStore.airportFilters.onlyCustom"
              @change="onAirportFilterToggle('onlyCustom', $event)"
            />
            <span>{{ t('map.filters.customOnly') }}</span>
          </label>
        </div>

        <div class="mt-2 flex items-center gap-2 text-xs text-gray-300">
          <span>{{ t('map.filters.minRunways') }}</span>
          <input
            type="number"
            min="0"
            max="8"
            :value="mapStore.airportFilters.minRunwayCount"
            class="w-14 rounded border border-gray-700 bg-slate-800 px-2 py-1"
            @change="onMinRunwayCountInput"
          />
        </div>
      </div>
    </div>

    <div class="absolute top-3 right-3 z-20 w-[330px] max-w-[calc(100%-1.5rem)] space-y-2">
      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl">
        <div class="text-xs text-gray-400">{{ t('map.selectedAirport') }}</div>
        <template v-if="mapStore.selectedAirport">
          <div class="mt-1 flex items-baseline justify-between">
            <div class="font-mono text-base text-blue-300">{{ mapStore.selectedAirport.icao }}</div>
            <div class="text-xs text-gray-400">{{ mapStore.selectedAirport.airportType }}</div>
          </div>
          <div class="text-sm text-gray-200 truncate">{{ mapStore.selectedAirport.name }}</div>
          <div v-if="selectedAirportDetail" class="mt-1 text-[11px] text-gray-400">
            {{ t('map.stats.runways') }}: {{ selectedAirportDetail.runways.length }}
            · {{ t('map.stats.helipads') }}: {{ selectedAirportDetail.helipads.length }}
            · {{ t('map.stats.gates') }}: {{ selectedAirportDetail.gates.length }}
            · {{ t('map.stats.taxiways') }}: {{ selectedAirportDetail.taxiways.length }}
            · {{ t('map.stats.windsocks') }}: {{ selectedAirportDetail.windsocks.length }}
            · {{ t('map.stats.signs') }}: {{ selectedAirportDetail.signs.length }}
          </div>
          <div class="mt-2 text-xs text-gray-300 leading-5">
            <div>{{ metarText || t('map.noMetar') }}</div>
            <div class="mt-1 text-gray-400">{{ tafText || t('map.noTaf') }}</div>
          </div>
        </template>
        <div v-else class="mt-1 text-sm text-gray-500">{{ t('map.noAirportSelected') }}</div>
      </div>

      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl text-xs text-gray-300">
        <div class="grid grid-cols-2 gap-y-1">
          <div>{{ t('map.stats.airports') }}: {{ airports.length }} / {{ rawAirports.length }}</div>
          <div>{{ t('map.stats.navaids') }}: {{ navSnapshot.navaids.length }}</div>
          <div>{{ t('map.stats.waypoints') }}: {{ navSnapshot.waypoints.length }}</div>
          <div>{{ t('map.stats.airways') }}: {{ navSnapshot.airways.length }}</div>
          <div>{{ t('map.stats.ils') }}: {{ navSnapshot.ils.length }}</div>
          <div>{{ t('map.stats.airspaces') }}: {{ navSnapshot.airspaces.length }}</div>
        </div>
        <div class="mt-2 border-t border-gray-700 pt-2 text-[11px] text-gray-400">
          {{ t('map.planeStatus') }}:
          <span :class="mapStore.planeConnected ? 'text-emerald-300' : 'text-rose-300'">
            {{ mapStore.planeConnected ? t('map.connected') : t('map.disconnected') }}
          </span>
          <span class="ml-2">
            {{ t('map.stream') }} {{ planeStreamStatus.running ? t('map.running') : t('map.stopped') }}
          </span>
        </div>
      </div>

      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl text-xs text-gray-300">
        <div class="mb-2 flex items-center justify-between text-gray-400">
          <span>{{ t('map.vatsimOverview') }}</span>
          <button
            class="rounded border border-gray-600 px-2 py-0.5 text-[11px] hover:bg-slate-700/70"
            @click="refreshVatsimAndEvents"
          >
            {{ t('map.refresh') }}
          </button>
        </div>
        <div class="grid grid-cols-2 gap-y-1">
          <div>{{ t('map.vatsimPilots') }}: {{ vatsimPilots.length }}</div>
          <div>{{ t('map.vatsimEvents') }}: {{ vatsimEvents.length }}</div>
        </div>
        <div v-if="busiestAirports.length > 0" class="mt-2 border-t border-gray-700 pt-2">
          <div class="mb-1 text-[11px] text-gray-400">{{ t('map.busiestAirports') }}</div>
          <div class="space-y-1">
            <button
              v-for="item in busiestAirports"
              :key="item.icao"
              class="flex w-full items-center justify-between rounded border border-gray-700/70 bg-slate-800 px-2 py-1 text-left hover:bg-slate-700/70"
              @click="focusAirportByIcao(item.icao)"
            >
              <span class="font-mono text-blue-300">{{ item.icao }}</span>
              <span class="text-[11px] text-gray-400">{{ item.departures }} / {{ item.arrivals }}</span>
            </button>
          </div>
        </div>
        <div v-if="vatsimEvents.length > 0" class="mt-2 border-t border-gray-700 pt-2">
          <div class="mb-1 text-[11px] text-gray-400">{{ t('map.upcomingEvents') }}</div>
          <div class="space-y-1">
            <div
              v-for="event in vatsimEvents.slice(0, 3)"
              :key="event.id"
              class="rounded border border-gray-700/70 bg-slate-800 px-2 py-1"
            >
              <div class="truncate text-[11px] text-gray-200">{{ event.name }}</div>
              <div class="text-[10px] text-gray-500">{{ event.startTime }}</div>
            </div>
          </div>
        </div>
      </div>

      <div class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl text-xs text-gray-300">
        <div class="mb-2 text-gray-400">SimBrief</div>
        <div class="flex items-center gap-2">
          <input
            :value="mapStore.simbriefPilotId"
            type="text"
            class="flex-1 rounded-md border border-gray-700 bg-slate-800 px-2 py-1 text-xs text-gray-100"
            :placeholder="t('map.simbriefPilotPlaceholder')"
            @change="onSimbriefPilotInput"
          />
          <button
            class="rounded-md border border-gray-600 bg-slate-800 px-2 py-1 text-[11px] hover:bg-slate-700 disabled:opacity-50"
            :disabled="isSimbriefLoading"
            @click="fetchSimbrief"
          >
            {{ isSimbriefLoading ? t('map.loading') : t('map.fetch') }}
          </button>
        </div>
        <div v-if="simbriefSummary" class="mt-2 rounded border border-gray-700/70 bg-slate-800/80 p-2 text-[11px]">
          <div class="font-mono text-blue-300">{{ simbriefSummary.callsign || '-' }}</div>
          <div class="mt-1">{{ simbriefSummary.from }} -> {{ simbriefSummary.to }}</div>
          <div class="mt-0.5 text-gray-400">{{ simbriefSummary.altitude }} / {{ simbriefSummary.distance }}</div>
        </div>
      </div>
    </div>

    <div ref="mapContainer" class="h-full w-full"></div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import maplibregl from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import { useAppStore } from '@/stores/app'
import { useMapStore } from '@/stores/map'
import { useToastStore } from '@/stores/toast'
import {
  mapGetAirportDetail,
  mapFetchMetar,
  mapFetchRainviewerManifest,
  mapFetchSimbriefLatest,
  mapFetchTaf,
  mapFetchVatsimData,
  mapFetchVatsimEvents,
  mapGetAirportsInBounds,
  mapGetDataStatus,
  mapGetNavSnapshot,
  mapGetPlaneStreamStatus,
  mapSearchAirports,
  mapStartPlaneStream,
  mapStopPlaneStream,
  mapPrepareDataIndex,
} from '@/services/map-api'
import type {
  MapAirport,
  MapAirportDetail,
  MapAirportFilters,
  MapLayerVisibility,
  MapNavSnapshot,
  MapPlaneState,
  MapPlaneStreamStatus,
  MapVatsimEvent,
  MapVatsimPilot,
  RainViewerManifest,
  MapDataStatus,
} from '@/types/map'
import { logError } from '@/services/logger'

const { t } = useI18n()
const appStore = useAppStore()
const mapStore = useMapStore()
const toast = useToastStore()

const mapContainer = ref<HTMLDivElement | null>(null)
const mapRef = ref<maplibregl.Map | null>(null)

const dataStatus = ref<MapDataStatus>({
  loaded: false,
  airportCount: 0,
  navaidCount: 0,
  waypointCount: 0,
  airwayCount: 0,
  ilsCount: 0,
  airspaceCount: 0,
})

const searchQuery = ref('')
const searchResults = ref<MapAirport[]>([])
const rawAirports = ref<MapAirport[]>([])
const airports = ref<MapAirport[]>([])
const navSnapshot = ref<MapNavSnapshot>({
  navaids: [],
  waypoints: [],
  airways: [],
  ils: [],
  airspaces: [],
})
const vatsimPilots = ref<MapVatsimPilot[]>([])
const vatsimEvents = ref<MapVatsimEvent[]>([])
const selectedAirportDetail = ref<MapAirportDetail | null>(null)
const simbriefRouteCoordinates = ref<Array<[number, number]>>([])

const metarText = ref('')
const tafText = ref('')
const planeStreamStatus = ref<MapPlaneStreamStatus>({
  running: false,
  connected: false,
  port: 8086,
})
const isSimbriefLoading = ref(false)
const simbriefSummary = ref<{
  callsign?: string
  from: string
  to: string
  altitude: string
  distance: string
} | null>(null)

const mapStyleOptions: Array<{ label: string; value: string }> = [
  { label: 'Dark Matter', value: 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json' },
  { label: 'Positron', value: 'https://basemaps.cartocdn.com/gl/positron-gl-style/style.json' },
  { label: 'Voyager', value: 'https://basemaps.cartocdn.com/gl/voyager-gl-style/style.json' },
  { label: 'OpenStreetMap', value: 'https://tiles.openfreemap.org/styles/liberty' },
]

const busiestAirports = computed(() => {
  const counts = new Map<string, { departures: number; arrivals: number }>()
  for (const pilot of vatsimPilots.value) {
    const dep = (pilot.departure || '').trim().toUpperCase()
    const arr = (pilot.arrival || '').trim().toUpperCase()

    if (dep) {
      const item = counts.get(dep) || { departures: 0, arrivals: 0 }
      item.departures += 1
      counts.set(dep, item)
    }
    if (arr) {
      const item = counts.get(arr) || { departures: 0, arrivals: 0 }
      item.arrivals += 1
      counts.set(arr, item)
    }
  }

  return Array.from(counts.entries())
    .map(([icao, item]) => ({
      icao,
      departures: item.departures,
      arrivals: item.arrivals,
      total: item.departures + item.arrivals,
    }))
    .sort((a, b) => b.total - a.total)
    .slice(0, 5)
})

const layerItems: Array<{ key: keyof MapLayerVisibility; label: string }> = [
  { key: 'airports', label: 'map.layers.airports' },
  { key: 'navaids', label: 'map.layers.navaids' },
  { key: 'waypoints', label: 'map.layers.waypoints' },
  { key: 'airways', label: 'map.layers.airways' },
  { key: 'ils', label: 'map.layers.ils' },
  { key: 'airspaces', label: 'map.layers.airspaces' },
  { key: 'plane', label: 'map.layers.plane' },
  { key: 'vatsim', label: 'map.layers.vatsim' },
  { key: 'weatherRadar', label: 'map.layers.weatherRadar' },
]

const STREAM_EVENT_STATE = 'map-plane-state'
const STREAM_EVENT_CONNECTION = 'map-plane-connection'

let refreshTimer: ReturnType<typeof setTimeout> | null = null
let weatherRadarRefreshTimer: ReturnType<typeof setInterval> | null = null
let vatsimRefreshTimer: ReturnType<typeof setInterval> | null = null
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null
let requestSeq = 0
let airportDetailSeq = 0
let searchRequestSeq = 0
let refreshInFlight = false
let refreshQueued = false
let unlistenPlaneState: UnlistenFn | null = null
let unlistenPlaneConnection: UnlistenFn | null = null
let mapInteractionsBound = false
let detailCachePath = ''
const airportDetailCache = new Map<string, MapAirportDetail>()

const EMPTY_NAV_SNAPSHOT: MapNavSnapshot = {
  navaids: [],
  waypoints: [],
  airways: [],
  ils: [],
  airspaces: [],
}

type FeatureCollection = {
  type: 'FeatureCollection'
  features: Array<Record<string, unknown>>
}

function normalizeAirportType(type: string | undefined): string {
  if (!type) return 'unknown'
  return type.toLowerCase()
}

function applyAirportFilters(items: MapAirport[]): MapAirport[] {
  const filters = mapStore.airportFilters
  return items.filter((airport) => {
    const type = normalizeAirportType(airport.airportType)
    if (!filters.showLand && type === 'land') return false
    if (!filters.showSeaplane && type === 'seaplane') return false
    if (!filters.showHeliport && type === 'heliport') return false
    if (filters.onlyCustom && !airport.isCustom) return false
    const runwayCount = Number(airport.runwayCount || 0)
    if (runwayCount < filters.minRunwayCount) return false
    return true
  })
}

function applyAirportFilterToMap() {
  airports.value = applyAirportFilters(rawAirports.value)
  updateGeoJsonSource('airports', toAirportFeatureCollection(airports.value))
}

function toAirportFeatureCollection(items: MapAirport[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.map((airport) => ({
      type: 'Feature',
      properties: {
        icao: airport.icao,
        name: airport.name,
        airportType: normalizeAirportType(airport.airportType),
      },
      geometry: {
        type: 'Point',
        coordinates: [airport.lon, airport.lat],
      },
    })),
  }
}

function toAirportRunwayFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: detail.runways
      .filter((runway) =>
        Number.isFinite(runway.end1Lat)
        && Number.isFinite(runway.end1Lon)
        && Number.isFinite(runway.end2Lat)
        && Number.isFinite(runway.end2Lon),
      )
      .map((runway) => ({
        type: 'Feature',
        properties: {
          name: runway.name,
          widthM: runway.widthM || 0,
          surfaceType: runway.surfaceType || '',
          end1Name: runway.end1Name || '',
          end2Name: runway.end2Name || '',
        },
        geometry: {
          type: 'LineString',
          coordinates: [
            [runway.end1Lon, runway.end1Lat],
            [runway.end2Lon, runway.end2Lat],
          ],
        },
      })),
  }
}

function toAirportRunwayEndFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features: Array<Record<string, unknown>> = []
  detail.runways.forEach((runway) => {
    if (Number.isFinite(runway.end1Lat) && Number.isFinite(runway.end1Lon)) {
      features.push({
        type: 'Feature',
        properties: {
          runway: runway.name,
          name: runway.end1Name || '',
        },
        geometry: {
          type: 'Point',
          coordinates: [runway.end1Lon, runway.end1Lat],
        },
      })
    }
    if (Number.isFinite(runway.end2Lat) && Number.isFinite(runway.end2Lon)) {
      features.push({
        type: 'Feature',
        properties: {
          runway: runway.name,
          name: runway.end2Name || '',
        },
        geometry: {
          type: 'Point',
          coordinates: [runway.end2Lon, runway.end2Lat],
        },
      })
    }
  })

  return {
    type: 'FeatureCollection',
    features,
  }
}

function toAirportHelipadFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: detail.helipads
      .filter((helipad) => Number.isFinite(helipad.lat) && Number.isFinite(helipad.lon))
      .map((helipad) => ({
        type: 'Feature',
        properties: {
          name: helipad.name,
          heading: helipad.heading || 0,
        },
        geometry: {
          type: 'Point',
          coordinates: [helipad.lon, helipad.lat],
        },
      })),
  }
}

function toAirportGateFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: detail.gates
      .filter((gate) => Number.isFinite(gate.lat) && Number.isFinite(gate.lon))
      .map((gate) => ({
        type: 'Feature',
        properties: {
          name: gate.name,
          locationType: gate.locationType || '',
          operationType: gate.operationType || '',
          isLegacy: gate.isLegacy,
        },
        geometry: {
          type: 'Point',
          coordinates: [gate.lon, gate.lat],
        },
      })),
  }
}

function toAirportTaxiwayFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: detail.taxiways
      .filter((edge) =>
        Number.isFinite(edge.fromLat)
        && Number.isFinite(edge.fromLon)
        && Number.isFinite(edge.toLat)
        && Number.isFinite(edge.toLon),
      )
      .map((edge) => ({
        type: 'Feature',
        properties: {
          name: edge.name || '',
        },
        geometry: {
          type: 'LineString',
          coordinates: [
            [edge.fromLon, edge.fromLat],
            [edge.toLon, edge.toLat],
          ],
        },
      })),
  }
}

function toAirportTowerFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail || !detail.tower || !Number.isFinite(detail.tower.lat) || !Number.isFinite(detail.tower.lon)) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: [
      {
        type: 'Feature',
        properties: {
          name: detail.tower.name || 'TWR',
          heightM: detail.tower.heightM || 0,
        },
        geometry: {
          type: 'Point',
          coordinates: [detail.tower.lon, detail.tower.lat],
        },
      },
    ],
  }
}

function toAirportBeaconFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail || !detail.beacon || !Number.isFinite(detail.beacon.lat) || !Number.isFinite(detail.beacon.lon)) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: [
      {
        type: 'Feature',
        properties: {
          name: detail.beacon.name || 'Beacon',
          beaconType: detail.beacon.beaconType || 0,
        },
        geometry: {
          type: 'Point',
          coordinates: [detail.beacon.lon, detail.beacon.lat],
        },
      },
    ],
  }
}

function toAirportWindsockFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: detail.windsocks
      .filter((item) => Number.isFinite(item.lat) && Number.isFinite(item.lon))
      .map((item) => ({
        type: 'Feature',
        properties: {
          name: item.name || 'Windsock',
          illuminated: item.illuminated,
        },
        geometry: {
          type: 'Point',
          coordinates: [item.lon, item.lat],
        },
      })),
  }
}

function toAirportSignFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: detail.signs
      .filter((item) => Number.isFinite(item.lat) && Number.isFinite(item.lon) && item.text)
      .map((item) => ({
        type: 'Feature',
        properties: {
          text: item.text,
          size: item.size || 0,
          heading: item.heading || 0,
        },
        geometry: {
          type: 'Point',
          coordinates: [item.lon, item.lat],
        },
      })),
  }
}

function toSimbriefRouteFeatureCollection(coordinates: Array<[number, number]>): FeatureCollection {
  if (coordinates.length < 2) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: [
      {
        type: 'Feature',
        properties: {},
        geometry: {
          type: 'LineString',
          coordinates,
        },
      },
    ],
  }
}

function toNavaidFeatureCollection(items: MapNavSnapshot['navaids']): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.map((item) => ({
      type: 'Feature',
      properties: {
        id: item.id,
        name: item.name,
        navaidType: item.navaidType,
      },
      geometry: {
        type: 'Point',
        coordinates: [item.lon, item.lat],
      },
    })),
  }
}

function toWaypointFeatureCollection(items: MapNavSnapshot['waypoints']): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.map((item) => ({
      type: 'Feature',
      properties: {
        id: item.id,
        region: item.region,
      },
      geometry: {
        type: 'Point',
        coordinates: [item.lon, item.lat],
      },
    })),
  }
}

function toAirwayFeatureCollection(items: MapNavSnapshot['airways']): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.map((item) => ({
      type: 'Feature',
      properties: {
        name: item.name,
        isHigh: item.isHigh,
      },
      geometry: {
        type: 'LineString',
        coordinates: [
          [item.fromLon, item.fromLat],
          [item.toLon, item.toLat],
        ],
      },
    })),
  }
}

function toIlsFeatureCollection(items: MapNavSnapshot['ils']): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.map((item) => {
      const start: [number, number] = [item.lon, item.lat]
      const course = Number(item.course || 0)
      const distance = 0.2
      const rad = (course * Math.PI) / 180
      const end: [number, number] = [
        item.lon + Math.sin(rad) * distance,
        item.lat + Math.cos(rad) * distance,
      ]
      return {
        type: 'Feature',
        properties: {
          id: item.id,
          airport: item.airport,
          runway: item.runway,
        },
        geometry: {
          type: 'LineString',
          coordinates: [start, end],
        },
      }
    }),
  }
}

function toAirspaceFeatureCollection(items: MapNavSnapshot['airspaces']): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items
      .filter((item) => item.coordinates.length >= 3)
      .map((item) => {
        const ring = [...item.coordinates]
        const first = ring[0]
        const last = ring[ring.length - 1]
        if (!last || first[0] !== last[0] || first[1] !== last[1]) {
          ring.push(first)
        }
        return {
          type: 'Feature',
          properties: {
            name: item.name,
            classCode: item.classCode,
            upperLimit: item.upperLimit,
            lowerLimit: item.lowerLimit,
          },
          geometry: {
            type: 'Polygon',
            coordinates: [ring],
          },
        }
      }),
  }
}

function toPlaneFeatureCollection(state: MapPlaneState | null): FeatureCollection {
  if (!state) {
    return { type: 'FeatureCollection', features: [] }
  }

  return {
    type: 'FeatureCollection',
    features: [
      {
        type: 'Feature',
        properties: {
          heading: state.heading ?? 0,
          groundspeed: state.groundspeed ?? 0,
          altitudeMSL: state.altitudeMSL ?? 0,
        },
        geometry: {
          type: 'Point',
          coordinates: [state.longitude, state.latitude],
        },
      },
    ],
  }
}

function toVatsimFeatureCollection(items: MapVatsimPilot[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.map((pilot) => ({
      type: 'Feature',
      properties: {
        callsign: pilot.callsign,
        heading: pilot.heading || 0,
      },
      geometry: {
        type: 'Point',
        coordinates: [pilot.longitude, pilot.latitude],
      },
    })),
  }
}

function onMapStyleChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const value = target.value
  if (!value || value === mapStore.mapStyleUrl) return
  void mapStore.setMapStyleUrl(value)
}

function onVatsimIntervalInput(event: Event) {
  const target = event.target as HTMLInputElement
  const value = Number(target.value)
  if (!Number.isFinite(value)) return
  void mapStore.setVatsimRefreshInterval(value)
  startVatsimTimer()
}

function onAirportFilterToggle(key: keyof MapAirportFilters, event: Event) {
  const target = event.target as HTMLInputElement
  void mapStore.setAirportFilters({ [key]: Boolean(target.checked) })
}

function onMinRunwayCountInput(event: Event) {
  const target = event.target as HTMLInputElement
  const value = Number(target.value)
  if (!Number.isFinite(value)) return
  void mapStore.setAirportFilters({ minRunwayCount: Math.max(0, Math.min(8, value)) })
}

async function resetAirportFilters() {
  await mapStore.resetAirportFilters()
}

async function focusAirportByIcao(icao: string) {
  const match = airports.value.find((item) => item.icao === icao)
  if (match) {
    await selectAirport(match)
    return
  }

  if (!appStore.xplanePath) return
  try {
    const searched = await mapSearchAirports(appStore.xplanePath, icao, 1)
    const first = searched[0]
    if (first) {
      await selectAirport(first)
    }
  } catch (error) {
    logError(`Failed to locate airport ${icao}: ${error}`, 'map')
  }
}

async function refreshVatsimEvents() {
  try {
    const response = await mapFetchVatsimEvents()
    const root = response && typeof response === 'object'
      ? response as Record<string, unknown>
      : {}
    const listRaw = Array.isArray(root.data)
      ? root.data
      : Array.isArray(root.events)
        ? root.events
        : []

    vatsimEvents.value = listRaw
      .map((item: Record<string, unknown>) => ({
        id: Number(item.id || 0),
        name: String(item.name || ''),
        startTime: String(item.start_time || item.startTime || ''),
        endTime: String(item.end_time || item.endTime || ''),
        routes: Array.isArray(item.routes)
          ? item.routes.map((r) => ({
              departure: r && typeof r === 'object'
                ? String((r as Record<string, unknown>).departure || '')
                : undefined,
              arrival: r && typeof r === 'object'
                ? String((r as Record<string, unknown>).arrival || '')
                : undefined,
            }))
          : [],
      }))
      .filter((item) => item.id > 0 && item.name)
      .slice(0, 20)
  } catch (error) {
    logError(`Failed to fetch VATSIM events: ${error}`, 'map')
  }
}

async function refreshVatsimAndEvents() {
  await Promise.allSettled([refreshVatsim(), refreshVatsimEvents()])
}

function onSimbriefPilotInput(event: Event) {
  const target = event.target as HTMLInputElement
  void mapStore.setSimbriefPilotId(target.value.trim())
}

function parseCoordinateValue(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }
  if (typeof value !== 'string') {
    return null
  }
  const raw = value.trim()
  if (!raw) {
    return null
  }
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

function findAirportCoordinate(icao: string): [number, number] | null {
  const upper = icao.trim().toUpperCase()
  if (!upper) return null

  const match = rawAirports.value.find((airport) => airport.icao === upper)
    || airports.value.find((airport) => airport.icao === upper)

  if (!match) return null
  return [match.lon, match.lat]
}

function buildSimbriefRouteCoordinates(
  payload: Record<string, unknown>,
  fromIcao: string,
  toIcao: string,
): Array<[number, number]> {
  const routeCoordinates: Array<[number, number]> = []
  const navlogRaw = payload.navlog
  const navlog = navlogRaw && typeof navlogRaw === 'object'
    ? navlogRaw as Record<string, unknown>
    : {}
  const fixesRaw = Array.isArray(navlog.fix)
    ? navlog.fix
    : Array.isArray(navlog.fixes)
      ? navlog.fixes
      : []

  for (const fixItem of fixesRaw) {
    if (!fixItem || typeof fixItem !== 'object') continue
    const fix = fixItem as Record<string, unknown>
    const lat = parseCoordinateValue(
      fix.pos_lat ?? fix.lat ?? fix.latitude ?? fix.posLat,
    )
    const lon = parseCoordinateValue(
      fix.pos_long ?? fix.lon ?? fix.longitude ?? fix.posLon,
    )
    if (lat === null || lon === null) continue
    if (lat < -90 || lat > 90 || lon < -180 || lon > 180) continue
    routeCoordinates.push([lon, lat])
  }

  const fromCoordinate = findAirportCoordinate(fromIcao)
  const toCoordinate = findAirportCoordinate(toIcao)

  const merged: Array<[number, number]> = []
  if (fromCoordinate) merged.push(fromCoordinate)
  merged.push(...routeCoordinates)
  if (toCoordinate) merged.push(toCoordinate)

  const deduped: Array<[number, number]> = []
  for (const point of merged) {
    const last = deduped.at(-1)
    if (!last || last[0] !== point[0] || last[1] !== point[1]) {
      deduped.push(point)
    }
  }

  return deduped
}

async function fetchSimbrief() {
  const pilotId = mapStore.simbriefPilotId.trim()
  if (!pilotId) {
    simbriefSummary.value = null
    simbriefRouteCoordinates.value = []
    updateSimbriefRouteFeature()
    applyLayerVisibility()
    return
  }

  isSimbriefLoading.value = true
  try {
    const payload = await mapFetchSimbriefLatest(pilotId)
    const p = payload as Record<string, unknown>
    const general = p.general && typeof p.general === 'object'
      ? p.general as Record<string, unknown>
      : {}
    const origin = p.origin && typeof p.origin === 'object'
      ? p.origin as Record<string, unknown>
      : {}
    const destination = p.destination && typeof p.destination === 'object'
      ? p.destination as Record<string, unknown>
      : {}
    const atc = p.atc && typeof p.atc === 'object'
      ? p.atc as Record<string, unknown>
      : {}

    const from = String(origin.icao_code || origin.icao || '').toUpperCase()
    const to = String(destination.icao_code || destination.icao || '').toUpperCase()

    if (!from || !to) {
      throw new Error('No valid route in SimBrief payload')
    }

    simbriefSummary.value = {
      callsign: String(atc.callsign || ''),
      from,
      to,
      altitude: String(general.initial_altitude || general.cruise_altitude || '-'),
      distance: String(general.air_distance || general.route_distance || '-'),
    }

    simbriefRouteCoordinates.value = buildSimbriefRouteCoordinates(p, from, to)
    updateSimbriefRouteFeature()
    applyLayerVisibility()
  } catch (error) {
    logError(`Failed to fetch SimBrief: ${error}`, 'map')
    simbriefRouteCoordinates.value = []
    updateSimbriefRouteFeature()
    applyLayerVisibility()
    toast.warning(t('map.simbriefFetchFailed'))
  } finally {
    isSimbriefLoading.value = false
  }
}

async function reconnectPlaneStream() {
  try {
    await mapStopPlaneStream()
  } catch {
    // ignore stop error; reconnect sequence continues
  }
  await initializePlaneStreamListeners()
}

function updateGeoJsonSource(sourceId: string, data: FeatureCollection) {
  const map = mapRef.value
  if (!map) return

  const source = map.getSource(sourceId) as maplibregl.GeoJSONSource | undefined
  if (!source) return
  source.setData(data)
}

function applyLayerVisibility() {
  const map = mapRef.value
  if (!map) return

  const list: Array<{ id: string; visible: boolean }> = [
    { id: 'airport-runways-line', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runways-centerline', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-ends-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-ends-label', visible: mapStore.layerVisibility.airports },
    { id: 'airport-helipads-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-gates-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-taxiways-line', visible: mapStore.layerVisibility.airports },
    { id: 'airport-taxiways-label', visible: mapStore.layerVisibility.airports },
    { id: 'airport-tower-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-tower-label', visible: mapStore.layerVisibility.airports },
    { id: 'airport-beacon-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-windsocks-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-signs-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-signs-label', visible: mapStore.layerVisibility.airports },
    { id: 'airports-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airports-label', visible: mapStore.layerVisibility.airports },
    { id: 'navaids-circle', visible: mapStore.layerVisibility.navaids },
    { id: 'waypoints-circle', visible: mapStore.layerVisibility.waypoints },
    { id: 'airways-line', visible: mapStore.layerVisibility.airways },
    { id: 'ils-line', visible: mapStore.layerVisibility.ils },
    { id: 'airspaces-fill', visible: mapStore.layerVisibility.airspaces },
    { id: 'airspaces-line', visible: mapStore.layerVisibility.airspaces },
    { id: 'plane-circle', visible: mapStore.layerVisibility.plane },
    { id: 'vatsim-circle', visible: mapStore.layerVisibility.vatsim },
    { id: 'simbrief-route-line', visible: simbriefRouteCoordinates.value.length > 1 },
    { id: 'rainviewer-layer', visible: mapStore.layerVisibility.weatherRadar },
  ]

  for (const item of list) {
    if (map.getLayer(item.id)) {
      map.setLayoutProperty(item.id, 'visibility', item.visible ? 'visible' : 'none')
    }
  }
}

function setupMapSourcesAndLayers(map: maplibregl.Map) {
  if (!map.getSource('airports')) {
    map.addSource('airports', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('navaids')) {
    map.addSource('navaids', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('waypoints')) {
    map.addSource('waypoints', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airways')) {
    map.addSource('airways', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('ils')) {
    map.addSource('ils', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airspaces')) {
    map.addSource('airspaces', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('plane')) {
    map.addSource('plane', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('vatsim')) {
    map.addSource('vatsim', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-runways')) {
    map.addSource('airport-runways', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-runway-ends')) {
    map.addSource('airport-runway-ends', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-helipads')) {
    map.addSource('airport-helipads', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-gates')) {
    map.addSource('airport-gates', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-taxiways')) {
    map.addSource('airport-taxiways', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-tower')) {
    map.addSource('airport-tower', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-beacon')) {
    map.addSource('airport-beacon', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-windsocks')) {
    map.addSource('airport-windsocks', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('airport-signs')) {
    map.addSource('airport-signs', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getSource('simbrief-route')) {
    map.addSource('simbrief-route', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  if (!map.getLayer('airspaces-fill')) {
    map.addLayer({
      id: 'airspaces-fill',
      type: 'fill',
      source: 'airspaces',
      paint: {
        'fill-color': '#7dd3fc',
        'fill-opacity': 0.09,
      },
    })
  }

  if (!map.getLayer('airspaces-line')) {
    map.addLayer({
      id: 'airspaces-line',
      type: 'line',
      source: 'airspaces',
      paint: {
        'line-color': '#38bdf8',
        'line-width': 1,
      },
    })
  }

  if (!map.getLayer('airways-line')) {
    map.addLayer({
      id: 'airways-line',
      type: 'line',
      source: 'airways',
      paint: {
        'line-color': '#e2e8f0',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          4,
          0.4,
          9,
          1.2,
        ],
        'line-opacity': 0.7,
      },
    })
  }

  if (!map.getLayer('ils-line')) {
    map.addLayer({
      id: 'ils-line',
      type: 'line',
      source: 'ils',
      paint: {
        'line-color': '#fb923c',
        'line-width': 1.2,
      },
    })
  }

  if (!map.getLayer('simbrief-route-line')) {
    map.addLayer({
      id: 'simbrief-route-line',
      type: 'line',
      source: 'simbrief-route',
      paint: {
        'line-color': '#facc15',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          4,
          1,
          10,
          2.5,
        ],
        'line-opacity': 0.75,
      },
    })
  }

  if (!map.getLayer('airport-runways-line')) {
    map.addLayer({
      id: 'airport-runways-line',
      type: 'line',
      source: 'airport-runways',
      minzoom: 9,
      paint: {
        'line-color': '#f8fafc',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          9,
          1,
          13,
          4,
        ],
        'line-opacity': 0.85,
      },
    })
  }

  if (!map.getLayer('airport-runways-centerline')) {
    map.addLayer({
      id: 'airport-runways-centerline',
      type: 'line',
      source: 'airport-runways',
      minzoom: 11,
      paint: {
        'line-color': '#0f172a',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          11,
          0.5,
          15,
          1.2,
        ],
        'line-opacity': 0.8,
        'line-dasharray': [1.4, 1.1],
      },
    })
  }

  if (!map.getLayer('airport-runway-ends-circle')) {
    map.addLayer({
      id: 'airport-runway-ends-circle',
      type: 'circle',
      source: 'airport-runway-ends',
      minzoom: 10,
      paint: {
        'circle-color': '#fde68a',
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          10,
          2,
          14,
          4,
        ],
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 1,
      },
    })
  }

  if (!map.getLayer('airport-runway-ends-label')) {
    map.addLayer({
      id: 'airport-runway-ends-label',
      type: 'symbol',
      source: 'airport-runway-ends',
      minzoom: 11,
      layout: {
        'text-field': ['get', 'name'],
        'text-size': 10,
        'text-font': ['Noto Sans Regular'],
        'text-offset': [0, 0.9],
      },
      paint: {
        'text-color': '#fef3c7',
        'text-halo-color': '#020617',
        'text-halo-width': 1,
      },
    })
  }

  if (!map.getLayer('airport-helipads-circle')) {
    map.addLayer({
      id: 'airport-helipads-circle',
      type: 'circle',
      source: 'airport-helipads',
      minzoom: 10,
      paint: {
        'circle-color': '#c084fc',
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          10,
          2,
          14,
          5,
        ],
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 1,
      },
    })
  }

  if (!map.getLayer('airport-gates-circle')) {
    map.addLayer({
      id: 'airport-gates-circle',
      type: 'circle',
      source: 'airport-gates',
      minzoom: 11,
      paint: {
        'circle-color': '#34d399',
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          11,
          1.5,
          15,
          3.2,
        ],
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 0.8,
      },
    })
  }

  if (!map.getLayer('airport-taxiways-line')) {
    map.addLayer({
      id: 'airport-taxiways-line',
      type: 'line',
      source: 'airport-taxiways',
      minzoom: 11,
      paint: {
        'line-color': '#fbbf24',
        'line-width': [
          'interpolate',
          ['linear'],
          ['zoom'],
          11,
          0.8,
          15,
          2,
        ],
        'line-opacity': 0.8,
        'line-dasharray': [1.2, 1.4],
      },
    })
  }

  if (!map.getLayer('airport-taxiways-label')) {
    map.addLayer({
      id: 'airport-taxiways-label',
      type: 'symbol',
      source: 'airport-taxiways',
      minzoom: 13,
      layout: {
        'symbol-placement': 'line-center',
        'text-field': ['get', 'name'],
        'text-size': 9,
        'text-font': ['Noto Sans Regular'],
      },
      paint: {
        'text-color': '#fef3c7',
        'text-halo-color': '#020617',
        'text-halo-width': 1,
      },
    })
  }

  if (!map.getLayer('airport-tower-circle')) {
    map.addLayer({
      id: 'airport-tower-circle',
      type: 'circle',
      source: 'airport-tower',
      minzoom: 10,
      paint: {
        'circle-color': '#f59e0b',
        'circle-radius': 4,
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 1.2,
      },
    })
  }

  if (!map.getLayer('airport-tower-label')) {
    map.addLayer({
      id: 'airport-tower-label',
      type: 'symbol',
      source: 'airport-tower',
      minzoom: 11,
      layout: {
        'text-field': ['get', 'name'],
        'text-size': 10,
        'text-font': ['Noto Sans Regular'],
        'text-offset': [0, 1],
      },
      paint: {
        'text-color': '#fde68a',
        'text-halo-color': '#020617',
        'text-halo-width': 1,
      },
    })
  }

  if (!map.getLayer('airport-beacon-circle')) {
    map.addLayer({
      id: 'airport-beacon-circle',
      type: 'circle',
      source: 'airport-beacon',
      minzoom: 10,
      paint: {
        'circle-color': '#fb7185',
        'circle-radius': 3.2,
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 1,
      },
    })
  }

  if (!map.getLayer('airport-windsocks-circle')) {
    map.addLayer({
      id: 'airport-windsocks-circle',
      type: 'circle',
      source: 'airport-windsocks',
      minzoom: 10,
      paint: {
        'circle-color': [
          'case',
          ['to-boolean', ['get', 'illuminated']],
          '#22d3ee',
          '#0ea5e9',
        ],
        'circle-radius': 2.6,
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 0.8,
      },
    })
  }

  if (!map.getLayer('airport-signs-circle')) {
    map.addLayer({
      id: 'airport-signs-circle',
      type: 'circle',
      source: 'airport-signs',
      minzoom: 12,
      paint: {
        'circle-color': '#f97316',
        'circle-radius': 2.2,
        'circle-stroke-color': '#020617',
        'circle-stroke-width': 0.8,
      },
    })
  }

  if (!map.getLayer('airport-signs-label')) {
    map.addLayer({
      id: 'airport-signs-label',
      type: 'symbol',
      source: 'airport-signs',
      minzoom: 13,
      layout: {
        'text-field': ['get', 'text'],
        'text-size': 9,
        'text-font': ['Noto Sans Regular'],
        'text-offset': [0, 0.9],
      },
      paint: {
        'text-color': '#fed7aa',
        'text-halo-color': '#020617',
        'text-halo-width': 1,
      },
    })
  }

  if (!map.getLayer('airports-circle')) {
    map.addLayer({
      id: 'airports-circle',
      type: 'circle',
      source: 'airports',
      paint: {
        'circle-color': '#60a5fa',
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          3,
          1.5,
          7,
          4,
          12,
          7,
        ],
        'circle-stroke-color': '#0f172a',
        'circle-stroke-width': 1,
      },
    })
  }

  if (!map.getLayer('airports-label')) {
    map.addLayer({
      id: 'airports-label',
      type: 'symbol',
      source: 'airports',
      minzoom: 6,
      layout: {
        'text-field': ['get', 'icao'],
        'text-size': 11,
        'text-font': ['Noto Sans Regular'],
        'text-offset': [0, 1.3],
      },
      paint: {
        'text-color': '#bfdbfe',
        'text-halo-color': '#020617',
        'text-halo-width': 1.2,
      },
    })
  }

  if (!map.getLayer('navaids-circle')) {
    map.addLayer({
      id: 'navaids-circle',
      type: 'circle',
      source: 'navaids',
      paint: {
        'circle-color': '#22d3ee',
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          4,
          1,
          10,
          3,
        ],
      },
    })
  }

  if (!map.getLayer('waypoints-circle')) {
    map.addLayer({
      id: 'waypoints-circle',
      type: 'circle',
      source: 'waypoints',
      paint: {
        'circle-color': '#4ade80',
        'circle-radius': 1.8,
      },
    })
  }

  if (!map.getLayer('plane-circle')) {
    map.addLayer({
      id: 'plane-circle',
      type: 'circle',
      source: 'plane',
      paint: {
        'circle-color': '#facc15',
        'circle-radius': 6,
        'circle-stroke-color': '#0f172a',
        'circle-stroke-width': 1.5,
      },
    })
  }

  if (!map.getLayer('vatsim-circle')) {
    map.addLayer({
      id: 'vatsim-circle',
      type: 'circle',
      source: 'vatsim',
      paint: {
        'circle-color': '#f43f5e',
        'circle-radius': 3,
      },
    })
  }

  if (!mapInteractionsBound) {
    map.on('click', 'airports-circle', (event) => {
      const first = event.features?.[0]
      if (!first || !first.properties) return

      const icao = String(first.properties.icao || '')
      const airport = airports.value.find((item) => item.icao === icao)
      if (airport) {
        void selectAirport(airport)
      }
    })

    map.on('mouseenter', 'airports-circle', () => {
      map.getCanvas().style.cursor = 'pointer'
    })

    map.on('mouseleave', 'airports-circle', () => {
      map.getCanvas().style.cursor = ''
    })

    mapInteractionsBound = true
  }

  applyLayerVisibility()
}

async function refreshAirportWeather(airport: MapAirport) {
  try {
    const [metar, taf] = await Promise.allSettled([
      mapFetchMetar(airport.icao),
      mapFetchTaf(airport.icao),
    ])

    metarText.value = metar.status === 'fulfilled' ? metar.value : ''
    tafText.value = taf.status === 'fulfilled' ? taf.value : ''
  } catch (error) {
    logError(`Failed to fetch weather for ${airport.icao}: ${error}`, 'map')
  }
}

async function refreshAirportDetail(airport: MapAirport) {
  const xplanePath = appStore.xplanePath
  if (!xplanePath) return

  if (detailCachePath !== xplanePath) {
    detailCachePath = xplanePath
    airportDetailCache.clear()
  }

  const cached = airportDetailCache.get(airport.icao)
  if (cached) {
    selectedAirportDetail.value = cached
    updateAirportDetailFeatures(cached)
    return
  }

  const seq = ++airportDetailSeq
  try {
    const detail = await mapGetAirportDetail(xplanePath, airport.icao)
    if (seq !== airportDetailSeq) return
    airportDetailCache.set(airport.icao, detail)
    selectedAirportDetail.value = detail
    updateAirportDetailFeatures(detail)
  } catch (error) {
    if (seq !== airportDetailSeq) return
    clearAirportDetailFeatures()
    logError(`Failed to fetch airport detail for ${airport.icao}: ${error}`, 'map')
  }
}

async function selectAirport(airport: MapAirport) {
  mapStore.setSelectedAirport(airport)
  searchResults.value = []
  searchQuery.value = airport.icao
  clearAirportDetailFeatures()

  const map = mapRef.value
  if (map) {
    map.easeTo({
      center: [airport.lon, airport.lat],
      zoom: Math.max(map.getZoom(), 8),
      duration: 500,
    })
  }

  await Promise.allSettled([refreshAirportWeather(airport), refreshAirportDetail(airport)])
}

async function updateWeatherRadar(manifest?: RainViewerManifest) {
  const map = mapRef.value
  if (!map) return

  if (!mapStore.layerVisibility.weatherRadar) {
    if (map.getLayer('rainviewer-layer')) map.removeLayer('rainviewer-layer')
    if (map.getSource('rainviewer')) map.removeSource('rainviewer')
    return
  }

  let resolved = manifest
  if (!resolved) {
    resolved = await mapFetchRainviewerManifest()
  }

  const frames = [...(resolved.radar?.past || []), ...(resolved.radar?.nowcast || [])]
  const latestFrame = frames.at(-1)
  if (!latestFrame || !resolved.host) return

  const sourceUrl = `${resolved.host}${latestFrame.path}/512/{z}/{x}/{y}/2/1_1.png`

  if (map.getLayer('rainviewer-layer')) map.removeLayer('rainviewer-layer')
  if (map.getSource('rainviewer')) map.removeSource('rainviewer')

  map.addSource('rainviewer', {
    type: 'raster',
    tiles: [sourceUrl],
    tileSize: 512,
    maxzoom: 6,
  })

  map.addLayer({
    id: 'rainviewer-layer',
    type: 'raster',
    source: 'rainviewer',
    paint: {
      'raster-opacity': 0.6,
    },
  })

  applyLayerVisibility()
}

function updatePlaneFeature(state: MapPlaneState | null) {
  updateGeoJsonSource('plane', toPlaneFeatureCollection(state))
}

function updateVatsimFeature(items: MapVatsimPilot[]) {
  updateGeoJsonSource('vatsim', toVatsimFeatureCollection(items))
}

function updateAirportDetailFeatures(detail: MapAirportDetail | null) {
  updateGeoJsonSource('airport-runways', toAirportRunwayFeatureCollection(detail))
  updateGeoJsonSource('airport-runway-ends', toAirportRunwayEndFeatureCollection(detail))
  updateGeoJsonSource('airport-helipads', toAirportHelipadFeatureCollection(detail))
  updateGeoJsonSource('airport-gates', toAirportGateFeatureCollection(detail))
  updateGeoJsonSource('airport-taxiways', toAirportTaxiwayFeatureCollection(detail))
  updateGeoJsonSource('airport-tower', toAirportTowerFeatureCollection(detail))
  updateGeoJsonSource('airport-beacon', toAirportBeaconFeatureCollection(detail))
  updateGeoJsonSource('airport-windsocks', toAirportWindsockFeatureCollection(detail))
  updateGeoJsonSource('airport-signs', toAirportSignFeatureCollection(detail))
}

function clearAirportDetailFeatures() {
  selectedAirportDetail.value = null
  updateAirportDetailFeatures(null)
}

function updateSimbriefRouteFeature() {
  updateGeoJsonSource('simbrief-route', toSimbriefRouteFeatureCollection(simbriefRouteCoordinates.value))
}

async function refreshVatsim() {
  if (!mapStore.layerVisibility.vatsim) {
    vatsimPilots.value = []
    updateVatsimFeature([])
    return
  }

  try {
    const response = await mapFetchVatsimData()
    const pilotsRaw = Array.isArray(response?.pilots) ? response.pilots : []

    const parsed: MapVatsimPilot[] = pilotsRaw
      .map((item: Record<string, unknown>) => ({
        callsign: String(item.callsign || ''),
        name: item.name ? String(item.name) : undefined,
        latitude: Number(item.latitude || 0),
        longitude: Number(item.longitude || 0),
        altitude: Number(item.altitude || 0),
        groundspeed: Number(item.groundspeed || 0),
        heading: Number(item.heading || 0),
        departure: item.flight_plan && typeof item.flight_plan === 'object'
          ? String((item.flight_plan as Record<string, unknown>).departure || '')
          : undefined,
        arrival: item.flight_plan && typeof item.flight_plan === 'object'
          ? String((item.flight_plan as Record<string, unknown>).arrival || '')
          : undefined,
      }))
      .filter((pilot) => Number.isFinite(pilot.latitude) && Number.isFinite(pilot.longitude))

    vatsimPilots.value = parsed
    updateVatsimFeature(parsed)
  } catch (error) {
    logError(`Failed to fetch VATSIM data: ${error}`, 'map')
  }
}

function scheduleDataRefresh() {
  if (refreshTimer) clearTimeout(refreshTimer)
  refreshTimer = setTimeout(() => {
    refreshMapData()
  }, 220)
}

function isDataLayer(layer: keyof MapLayerVisibility): boolean {
  return layer === 'airports'
    || layer === 'navaids'
    || layer === 'waypoints'
    || layer === 'airways'
    || layer === 'ils'
    || layer === 'airspaces'
}

async function refreshMapData() {
  const map = mapRef.value
  const xplanePath = appStore.xplanePath
  if (!map || !xplanePath) return

  if (refreshInFlight) {
    refreshQueued = true
    return
  }

  refreshInFlight = true
  const currentSeq = ++requestSeq

  try {
    const boundsObj = map.getBounds()
    const center = map.getCenter()
    const zoom = map.getZoom()

    const airportLimit = zoom < 4
      ? 700
      : zoom < 6
        ? 1200
        : zoom < 8
          ? 2000
          : 3500

    const includeNav =
      mapStore.layerVisibility.navaids
      || mapStore.layerVisibility.waypoints
      || mapStore.layerVisibility.airways
      || mapStore.layerVisibility.ils
      || mapStore.layerVisibility.airspaces

    const airportPromise = mapStore.layerVisibility.airports
      ? mapGetAirportsInBounds(
        xplanePath,
        {
          north: boundsObj.getNorth(),
          south: boundsObj.getSouth(),
          east: boundsObj.getEast(),
          west: boundsObj.getWest(),
        },
        airportLimit,
      )
      : Promise.resolve([] as MapAirport[])

    const navPromise = includeNav
      ? mapGetNavSnapshot(xplanePath, {
        lat: center.lat,
        lon: center.lng,
        radiusNm: mapStore.navRadiusNm,
        includeNavaids: mapStore.layerVisibility.navaids,
        includeWaypoints: mapStore.layerVisibility.waypoints,
        includeAirways: mapStore.layerVisibility.airways,
        includeIls: mapStore.layerVisibility.ils,
        includeAirspaces: mapStore.layerVisibility.airspaces,
      })
      : Promise.resolve({ ...EMPTY_NAV_SNAPSHOT })

    const [airportResult, navResult] = await Promise.all([airportPromise, navPromise])

    if (currentSeq !== requestSeq) return

    rawAirports.value = airportResult
    airports.value = applyAirportFilters(airportResult)
    navSnapshot.value = navResult

    updateGeoJsonSource('airports', toAirportFeatureCollection(airports.value))
    updateGeoJsonSource('navaids', toNavaidFeatureCollection(navResult.navaids))
    updateGeoJsonSource('waypoints', toWaypointFeatureCollection(navResult.waypoints))
    updateGeoJsonSource('airways', toAirwayFeatureCollection(navResult.airways))
    updateGeoJsonSource('ils', toIlsFeatureCollection(navResult.ils))
    updateGeoJsonSource('airspaces', toAirspaceFeatureCollection(navResult.airspaces))
  } catch (error) {
    logError(`Failed to refresh map data: ${error}`, 'map')
  } finally {
    refreshInFlight = false
    if (refreshQueued) {
      refreshQueued = false
      scheduleDataRefresh()
    }
  }
}

async function initializeMapData() {
  const xplanePath = appStore.xplanePath
  if (!xplanePath) return

  try {
    dataStatus.value = await mapPrepareDataIndex(xplanePath)
    dataStatus.value = await mapGetDataStatus()
  } catch (error) {
    logError(`Failed to initialize map data index: ${error}`, 'map')
    toast.warning(t('map.indexInitFailed'))
  }
}

async function initializePlaneStreamListeners() {
  if (unlistenPlaneState) {
    unlistenPlaneState()
    unlistenPlaneState = null
  }

  if (unlistenPlaneConnection) {
    unlistenPlaneConnection()
    unlistenPlaneConnection = null
  }

  await mapStartPlaneStream(8086)
  planeStreamStatus.value = await mapGetPlaneStreamStatus()

  const stateUnlisten = await listen<MapPlaneState>(STREAM_EVENT_STATE, (event) => {
    mapStore.setPlaneState(event.payload)
    updatePlaneFeature(event.payload)

    const map = mapRef.value
    if (map && mapStore.followPlane && mapStore.layerVisibility.plane) {
      map.easeTo({
        center: [event.payload.longitude, event.payload.latitude],
        bearing: Number(event.payload.heading || 0),
        duration: 350,
      })
    }
  })

  const connUnlisten = await listen<boolean>(STREAM_EVENT_CONNECTION, (event) => {
    const connected = Boolean(event.payload)
    mapStore.setPlaneConnection(connected)
    planeStreamStatus.value = {
      ...planeStreamStatus.value,
      connected,
      running: true,
    }
    if (!connected) {
      mapStore.setPlaneState(null)
      updatePlaneFeature(null)
    }
  })

  unlistenPlaneState = stateUnlisten
  unlistenPlaneConnection = connUnlisten
}

function startVatsimTimer() {
  stopVatsimTimer()

  if (!mapStore.layerVisibility.vatsim) {
    vatsimEvents.value = []
    return
  }

  void refreshVatsimAndEvents()

  vatsimRefreshTimer = setInterval(() => {
    void refreshVatsimAndEvents()
  }, mapStore.vatsimRefreshInterval * 1000)
}

function stopVatsimTimer() {
  if (vatsimRefreshTimer) {
    clearInterval(vatsimRefreshTimer)
    vatsimRefreshTimer = null
  }
}

function startWeatherRadarTimer() {
  stopWeatherRadarTimer()

  if (!mapStore.layerVisibility.weatherRadar) {
    void updateWeatherRadar()
    return
  }

  void updateWeatherRadar()

  weatherRadarRefreshTimer = setInterval(() => {
    void updateWeatherRadar()
  }, 10 * 60 * 1000)
}

function stopWeatherRadarTimer() {
  if (weatherRadarRefreshTimer) {
    clearInterval(weatherRadarRefreshTimer)
    weatherRadarRefreshTimer = null
  }
}

async function toggleLayer(layer: keyof MapLayerVisibility) {
  await mapStore.toggleLayer(layer)
  applyLayerVisibility()

  if (layer === 'vatsim') {
    startVatsimTimer()
  }

  if (layer === 'weatherRadar') {
    startWeatherRadarTimer()
  }

  if (layer === 'plane' && !mapStore.layerVisibility.plane) {
    updatePlaneFeature(null)
  }

  if (isDataLayer(layer)) {
    scheduleDataRefresh()
  }
}

function onRadiusInput(event: Event) {
  const target = event.target as HTMLInputElement
  const value = Number(target.value)
  if (!Number.isFinite(value)) return

  void mapStore.setNavRadiusNm(value)
  scheduleDataRefresh()
}

function onToggleFollowPlane(event: Event) {
  const target = event.target as HTMLInputElement
  void mapStore.setFollowPlane(Boolean(target.checked))
}

watch(
  () => searchQuery.value,
  (value) => {
    const q = value.trim()

    if (searchDebounceTimer) {
      clearTimeout(searchDebounceTimer)
      searchDebounceTimer = null
    }

    if (q.length < 2 || !appStore.xplanePath) {
      searchResults.value = []
      return
    }

    const seq = ++searchRequestSeq
    searchDebounceTimer = setTimeout(async () => {
      if (!appStore.xplanePath) return
      try {
        const rows = await mapSearchAirports(appStore.xplanePath, q, 12)
        if (seq === searchRequestSeq) {
          searchResults.value = rows
        }
      } catch (error) {
        if (seq === searchRequestSeq) {
          logError(`Airport search failed: ${error}`, 'map')
        }
      }
    }, 180)
  },
)

watch(
  () => mapStore.airportFilters,
  () => {
    applyAirportFilterToMap()
  },
  { deep: true },
)

watch(
  () => mapStore.mapStyleUrl,
  (styleUrl) => {
    const map = mapRef.value
    if (!map) return

    map.once('style.load', () => {
      setupMapSourcesAndLayers(map)
      updateGeoJsonSource('airports', toAirportFeatureCollection(airports.value))
      updateGeoJsonSource('navaids', toNavaidFeatureCollection(navSnapshot.value.navaids))
      updateGeoJsonSource('waypoints', toWaypointFeatureCollection(navSnapshot.value.waypoints))
      updateGeoJsonSource('airways', toAirwayFeatureCollection(navSnapshot.value.airways))
      updateGeoJsonSource('ils', toIlsFeatureCollection(navSnapshot.value.ils))
      updateGeoJsonSource('airspaces', toAirspaceFeatureCollection(navSnapshot.value.airspaces))
      updatePlaneFeature(mapStore.planeState)
      updateVatsimFeature(vatsimPilots.value)
      updateAirportDetailFeatures(selectedAirportDetail.value)
      updateSimbriefRouteFeature()
      applyLayerVisibility()
      if (mapStore.layerVisibility.weatherRadar) {
        void updateWeatherRadar()
      }
    })

    map.setStyle(styleUrl)
  },
)

onMounted(async () => {
  await mapStore.initStore()

  if (!mapContainer.value) return

  const map = new maplibregl.Map({
    container: mapContainer.value,
    style: mapStore.mapStyleUrl,
    center: [-95, 35],
    zoom: 3,
    attributionControl: true,
  })

  map.addControl(new maplibregl.NavigationControl({ visualizePitch: true }), 'bottom-right')
  mapRef.value = map

  map.on('load', () => {
    setupMapSourcesAndLayers(map)
    updateAirportDetailFeatures(selectedAirportDetail.value)
    updateSimbriefRouteFeature()
    void refreshMapData()
  })

  map.on('moveend', () => {
    scheduleDataRefresh()
  })

  await initializeMapData()
  await initializePlaneStreamListeners()

  startVatsimTimer()
  startWeatherRadarTimer()

  if (mapStore.simbriefPilotId) {
    void fetchSimbrief()
  }
})

onBeforeUnmount(async () => {
  if (refreshTimer) {
    clearTimeout(refreshTimer)
    refreshTimer = null
  }
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
    searchDebounceTimer = null
  }

  stopVatsimTimer()
  stopWeatherRadarTimer()

  if (unlistenPlaneState) {
    unlistenPlaneState()
    unlistenPlaneState = null
  }

  if (unlistenPlaneConnection) {
    unlistenPlaneConnection()
    unlistenPlaneConnection = null
  }

  try {
    await mapStopPlaneStream()
    planeStreamStatus.value = {
      ...planeStreamStatus.value,
      running: false,
      connected: false,
    }
  } catch (error) {
    logError(`Failed to stop plane stream: ${error}`, 'map')
  }

  if (mapRef.value) {
    mapRef.value.remove()
    mapRef.value = null
  }
  refreshInFlight = false
  refreshQueued = false
  mapInteractionsBound = false
  airportDetailCache.clear()
  detailCachePath = ''
})
</script>

<style scoped>
.map-page :deep(.maplibregl-ctrl-group) {
  background-color: rgba(15, 23, 42, 0.9);
  border: 1px solid rgba(100, 116, 139, 0.35);
}

.map-page :deep(.maplibregl-ctrl button) {
  color: #e2e8f0;
}

.map-page :deep(.maplibregl-popup-content) {
  background: #0f172a;
  color: #e2e8f0;
  border: 1px solid rgba(100, 116, 139, 0.4);
}

.map-page :deep(.maplibregl-popup-tip) {
  border-top-color: #0f172a;
  border-bottom-color: #0f172a;
}
</style>
