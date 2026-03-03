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
          <div class="mt-2 border-t border-gray-700/70 pt-2 text-[11px] leading-5">
            <div class="mb-1 flex items-center justify-between gap-2 text-gray-400">
              <span>{{ t('map.gateway.title') }}</span>
              <button
                class="rounded border border-gray-600 px-1.5 py-0.5 text-[10px] text-gray-300 hover:bg-slate-700/70 disabled:opacity-50"
                :disabled="gatewayLoading"
                @click="refreshGatewayDataForSelected"
              >
                {{ t('map.gateway.refresh') }}
              </button>
            </div>
            <div v-if="gatewayLoading" class="text-gray-500">
              {{ t('map.gateway.loading') }}
            </div>
            <template v-else-if="gatewaySummary">
              <div class="text-gray-200">
                {{ t('map.gateway.recommended') }}:
                <span class="ml-1 font-mono text-cyan-300">
                  {{ gatewaySummary.recommendedSceneryId ?? '-' }}
                </span>
              </div>
              <div v-if="gatewaySummary.sceneryCount !== null" class="text-gray-400">
                {{ t('map.gateway.submissions', { count: gatewaySummary.sceneryCount }) }}
              </div>
              <div v-if="gatewaySummary.recommendedArtist" class="text-gray-400">
                {{ t('map.gateway.artist') }}: {{ gatewaySummary.recommendedArtist }}
              </div>
              <div v-if="gatewaySummary.recommendedAcceptedAt" class="text-gray-500">
                {{ t('map.gateway.acceptedAt') }}:
                {{ formatGatewayDate(gatewaySummary.recommendedAcceptedAt) }}
              </div>
              <div v-if="gatewaySceneryLoading" class="mt-1 text-gray-500">
                {{ t('map.gateway.sceneryLoading') }}
              </div>
              <template v-else-if="gatewayScenery">
                <div v-if="gatewayScenery.status" class="text-gray-400">
                  {{ t('map.gateway.status') }}: {{ gatewayScenery.status }}
                </div>
                <div v-if="gatewayScenery.features.length > 0" class="text-gray-400">
                  {{ t('map.gateway.features') }}: {{ gatewayScenery.features.join(' · ') }}
                </div>
                <div v-if="gatewayScenery.comment" class="mt-1 line-clamp-2 text-gray-500">
                  {{ t('map.gateway.comments') }}: {{ gatewayScenery.comment }}
                </div>
              </template>
            </template>
            <div v-else class="text-gray-500">{{ t('map.gateway.unavailable') }}</div>
          </div>
        </template>
        <div v-else class="mt-1 text-sm text-gray-500">{{ t('map.noAirportSelected') }}</div>
      </div>

      <div
        v-if="mapStore.selectedAirport"
        class="rounded-xl border border-gray-700/70 bg-slate-900/90 backdrop-blur-md p-3 shadow-xl text-xs text-gray-300"
      >
        <div class="mb-2 flex items-center justify-between text-gray-400">
          <span>{{ t('map.procedures.title') }}</span>
          <button
            class="rounded border border-gray-600 px-2 py-0.5 text-[11px] hover:bg-slate-700/70 disabled:opacity-50"
            :disabled="proceduresLoading"
            @click="refreshProceduresForSelected"
          >
            {{ t('map.refresh') }}
          </button>
        </div>

        <div v-if="proceduresLoading" class="text-[11px] text-gray-500">
          {{ t('map.procedures.loading') }}
        </div>
        <template v-else>
          <div class="mb-2 grid grid-cols-3 gap-1">
            <button
              v-for="tab in procedureTabs"
              :key="tab.key"
              class="rounded border px-2 py-1 text-[11px] transition-colors"
              :class="activeProcedureTab === tab.key
                ? 'border-blue-500/50 bg-blue-500/20 text-blue-200'
                : 'border-gray-700 bg-slate-800 text-gray-300 hover:bg-slate-700/70'"
              @click="setActiveProcedureTab(tab.key)"
            >
              {{ t(tab.label) }} ({{ procedureCounts[tab.key] }})
            </button>
          </div>

          <div v-if="activeProcedureGroups.length === 0" class="text-[11px] text-gray-500">
            {{ t('map.procedures.empty') }}
          </div>
          <div v-else class="max-h-44 space-y-1 overflow-y-auto pr-1">
            <div
              v-for="group in activeProcedureGroups"
              :key="`${activeProcedureTab}-${group.name}`"
              class="rounded border border-gray-700/70 bg-slate-800/70"
            >
              <button
                class="flex w-full items-center justify-between px-2 py-1.5 text-left"
                @click="toggleProcedureGroup(group.name)"
              >
                <span class="font-mono text-[11px] text-blue-200">{{ group.name }}</span>
                <span class="text-[10px] text-gray-400">{{ group.variants.length }}</span>
              </button>

              <div
                v-if="expandedProcedureGroup === group.name"
                class="space-y-1 border-t border-gray-700/70 px-2 py-1.5"
              >
                <div
                  v-for="(variant, idx) in group.variants"
                  :key="`${group.name}-${idx}-${variant.runway || ''}-${variant.transition || ''}`"
                  class="rounded border border-gray-700/70 bg-slate-900/60 px-2 py-1"
                >
                  <div class="flex items-center justify-between gap-2 text-[10px] text-gray-300">
                    <span class="truncate">{{ formatProcedureVariant(variant) }}</span>
                    <span class="shrink-0 text-gray-500">{{ variant.waypointCount }} {{ t('map.procedures.legs') }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
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
  mapGetAirportProcedures,
  mapFetchGatewayAirport,
  mapFetchGatewayScenery,
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
  MapAirportProcedures,
  MapAirportFilters,
  MapLayerVisibility,
  MapNavSnapshot,
  MapPlaneState,
  MapPlaneStreamStatus,
  MapProcedure,
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
const airportProcedures = ref<MapAirportProcedures | null>(null)
const proceduresLoading = ref(false)
const activeProcedureTab = ref<'sids' | 'stars' | 'approaches'>('sids')
const expandedProcedureGroup = ref<string | null>(null)
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
const gatewayLoading = ref(false)
const gatewaySummary = ref<GatewaySummary | null>(null)
const gatewaySceneryLoading = ref(false)
const gatewayScenery = ref<GatewaySceneryDetail | null>(null)
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

const procedureTabs: Array<{ key: ProcedureTabKey; label: string }> = [
  { key: 'sids', label: 'map.procedures.tabs.sid' },
  { key: 'stars', label: 'map.procedures.tabs.star' },
  { key: 'approaches', label: 'map.procedures.tabs.approach' },
]

const procedureCounts = computed(() => ({
  sids: airportProcedures.value?.sids.length || 0,
  stars: airportProcedures.value?.stars.length || 0,
  approaches: airportProcedures.value?.approaches.length || 0,
}))

const activeProcedureGroups = computed<ProcedureGroup[]>(() => {
  const source = airportProcedures.value?.[activeProcedureTab.value] || []
  const grouped = new Map<string, MapProcedure[]>()

  for (const procedure of source) {
    const list = grouped.get(procedure.name) || []
    list.push(procedure)
    grouped.set(procedure.name, list)
  }

  return Array.from(grouped.entries())
    .map(([name, variants]) => ({
      name,
      variants: [...variants].sort((a, b) =>
        (a.runway || '')
          .localeCompare(b.runway || '')
          || (a.transition || '').localeCompare(b.transition || ''),
      ),
    }))
    .sort((a, b) => a.name.localeCompare(b.name))
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
let proceduresSeq = 0
let gatewaySeq = 0
let gatewayScenerySeq = 0
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

type GatewaySummary = {
  icao: string
  airportName: string | null
  sceneryCount: number | null
  recommendedSceneryId: number | null
  recommendedArtist: string | null
  recommendedAcceptedAt: string | null
}

type ProcedureTabKey = 'sids' | 'stars' | 'approaches'

type ProcedureGroup = {
  name: string
  variants: MapProcedure[]
}

type GatewaySceneryDetail = {
  sceneryId: number
  status: string | null
  artist: string | null
  approvedDate: string | null
  comment: string | null
  features: string[]
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

const EARTH_RADIUS_M = 6_371_000
const RUNWAY_MARKING_VISUAL = 1
const RUNWAY_MARKING_NON_PRECISION = 2
const RUNWAY_MARKING_PRECISION = 3

const SURFACE_COLOR_BY_CODE: Record<number, string> = {
  1: '#2a2a2a',
  2: '#a0a0a0',
  3: '#3d6b35',
  4: '#8b6914',
  5: '#9e9e9e',
  12: '#c4a35a',
  13: '#1e5799',
  14: '#e8e8e8',
  15: '#cbd5e1',
  20: '#252525',
  21: '#2f2f2f',
  22: '#3a3a3a',
  23: '#5e5e5e',
  24: '#707070',
  25: '#7a7a7a',
  26: '#3d6b35',
  27: '#4a7d40',
  28: '#5c8f4c',
  29: '#7a5c3a',
  30: '#8b6914',
  31: '#a8a8a8',
  32: '#8e8e8e',
  33: '#787878',
  34: '#c4a35a',
  35: '#b39550',
  36: '#d4d4d4',
  37: '#c0c0c0',
  38: '#b8d4e8',
  50: '#2a2a2a',
  51: '#6a6a6a',
  52: '#2a2a2a',
  53: '#333333',
  54: '#5a5a5a',
  55: '#4a4a4a',
  56: '#656565',
  57: '#2e2e2e',
}

type RunwayLike = MapAirportDetail['runways'][number]

function toRadians(value: number): number {
  return (value * Math.PI) / 180
}

function toDegrees(value: number): number {
  return (value * 180) / Math.PI
}

function normalizeHeading(value: number): number {
  const normalized = value % 360
  return normalized < 0 ? normalized + 360 : normalized
}

function isFiniteLatLon(lat: number, lon: number): boolean {
  return Number.isFinite(lat)
    && Number.isFinite(lon)
    && lat >= -90
    && lat <= 90
    && lon >= -180
    && lon <= 180
}

function haversineDistanceMeters(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const phi1 = toRadians(lat1)
  const phi2 = toRadians(lat2)
  const deltaPhi = toRadians(lat2 - lat1)
  const deltaLambda = toRadians(lon2 - lon1)

  const a = Math.sin(deltaPhi / 2) ** 2
    + Math.cos(phi1) * Math.cos(phi2) * Math.sin(deltaLambda / 2) ** 2
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a))
  return EARTH_RADIUS_M * c
}

function calculateBearing(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const phi1 = toRadians(lat1)
  const phi2 = toRadians(lat2)
  const deltaLambda = toRadians(lon2 - lon1)

  const y = Math.sin(deltaLambda) * Math.cos(phi2)
  const x = Math.cos(phi1) * Math.sin(phi2)
    - Math.sin(phi1) * Math.cos(phi2) * Math.cos(deltaLambda)

  return normalizeHeading(toDegrees(Math.atan2(y, x)))
}

function destinationPoint(lat: number, lon: number, distanceMeters: number, bearing: number): [number, number] {
  const angularDistance = distanceMeters / EARTH_RADIUS_M
  const theta = toRadians(normalizeHeading(bearing))
  const phi1 = toRadians(lat)
  const lambda1 = toRadians(lon)

  const phi2 = Math.asin(
    Math.sin(phi1) * Math.cos(angularDistance)
      + Math.cos(phi1) * Math.sin(angularDistance) * Math.cos(theta),
  )

  const lambda2 = lambda1 + Math.atan2(
    Math.sin(theta) * Math.sin(angularDistance) * Math.cos(phi1),
    Math.cos(angularDistance) - Math.sin(phi1) * Math.sin(phi2),
  )

  return [toDegrees(lambda2), toDegrees(phi2)]
}

function createRectPolygon(
  centerLat: number,
  centerLon: number,
  lengthMeters: number,
  widthMeters: number,
  heading: number,
): Array<[number, number]> {
  const head = normalizeHeading(heading)
  const halfLength = Math.max(0, lengthMeters / 2)
  const halfWidth = Math.max(0, widthMeters / 2)

  const frontCenter = destinationPoint(centerLat, centerLon, halfLength, head)
  const backCenter = destinationPoint(centerLat, centerLon, halfLength, head + 180)

  const frontLeft = destinationPoint(frontCenter[1], frontCenter[0], halfWidth, head - 90)
  const frontRight = destinationPoint(frontCenter[1], frontCenter[0], halfWidth, head + 90)
  const backRight = destinationPoint(backCenter[1], backCenter[0], halfWidth, head + 90)
  const backLeft = destinationPoint(backCenter[1], backCenter[0], halfWidth, head - 90)

  return [frontLeft, frontRight, backRight, backLeft, frontLeft]
}

function parseRunwayHeadingFromName(name: string): number | null {
  const numberText = name.trim().toUpperCase().replace(/[LCR]$/, '')
  const number = Number.parseInt(numberText, 10)
  if (!Number.isFinite(number) || number <= 0 || number > 36) return null
  return normalizeHeading(number * 10)
}

function getRunwayHeading(runway: RunwayLike): number {
  if (
    isFiniteLatLon(runway.end1Lat, runway.end1Lon)
    && isFiniteLatLon(runway.end2Lat, runway.end2Lon)
  ) {
    return calculateBearing(runway.end1Lat, runway.end1Lon, runway.end2Lat, runway.end2Lon)
  }
  return parseRunwayHeadingFromName(runway.end1Name)
    ?? parseRunwayHeadingFromName(runway.end2Name)
    ?? 0
}

function runwayWidthMeters(runway: RunwayLike): number {
  const width = Number(runway.widthM || 0)
  if (!Number.isFinite(width) || width <= 0) {
    return 45
  }
  return Math.max(8, width)
}

function defaultShoulderWidth(runwayWidth: number): number {
  if (runwayWidth < 30) return 3
  if (runwayWidth <= 45) return 4
  return 5
}

function createRunwayPolygon(
  runway: RunwayLike,
  extraWidthMeters = 0,
): Array<[number, number]> | null {
  if (
    !isFiniteLatLon(runway.end1Lat, runway.end1Lon)
    || !isFiniteLatLon(runway.end2Lat, runway.end2Lon)
  ) {
    return null
  }

  const heading = getRunwayHeading(runway)
  const halfWidth = runwayWidthMeters(runway) / 2 + Math.max(0, extraWidthMeters)

  const left1 = destinationPoint(runway.end1Lat, runway.end1Lon, halfWidth, heading - 90)
  const right1 = destinationPoint(runway.end1Lat, runway.end1Lon, halfWidth, heading + 90)
  const right2 = destinationPoint(runway.end2Lat, runway.end2Lon, halfWidth, heading + 90)
  const left2 = destinationPoint(runway.end2Lat, runway.end2Lon, halfWidth, heading - 90)

  return [left1, right1, right2, left2, left1]
}

function formatRunwayNumberLabel(name: string): string {
  const normalized = name.trim().toUpperCase()
  if (!normalized) return ''

  const suffix = normalized.match(/[LCR]$/)?.[0] || ''
  const numberText = normalized.replace(/[LCR]$/, '')
  const number = Number.parseInt(numberText, 10)
  const normalizedNumber = Number.isFinite(number) ? String(number) : numberText

  return suffix ? `${normalizedNumber}\n${suffix}` : normalizedNumber
}

function buildSurfaceColorExpression(propertyName: string): maplibregl.ExpressionSpecification {
  const expression: Array<string | number | unknown[]> = ['match', ['to-number', ['get', propertyName]]]
  Object.entries(SURFACE_COLOR_BY_CODE).forEach(([surfaceCode, color]) => {
    expression.push(Number(surfaceCode), color)
  })
  expression.push('#3a3a3a')
  return expression as maplibregl.ExpressionSpecification
}

function pushThresholdBars(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  runwayWidth: number,
) {
  const numBars = 8
  const barLength = Math.min(45, runwayWidth * 0.4)
  const barWidth = 1.8
  const barSpacing = 1.8
  const startOffset = 6

  for (let i = 0; i < numBars; i += 1) {
    const sideOffset = i < numBars / 2
      ? -(barSpacing * (numBars / 4 - i - 0.5) + barWidth / 2)
      : barSpacing * (i - numBars / 2 + 0.5) - barWidth / 2

    const barCenter = destinationPoint(lat, lon, startOffset + barLength / 2, heading)
    const centerOffset = destinationPoint(
      barCenter[1],
      barCenter[0],
      Math.abs(sideOffset),
      sideOffset < 0 ? heading - 90 : heading + 90,
    )

    features.push({
      type: 'Feature',
      properties: { type: 'threshold' },
      geometry: {
        type: 'Polygon',
        coordinates: [createRectPolygon(centerOffset[1], centerOffset[0], barLength, barWidth, heading)],
      },
    })
  }
}

function pushAimingPoints(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  runwayWidth: number,
) {
  const distance = 300
  const length = 45
  const width = 10
  const sideOffset = runwayWidth * 0.25

  for (const side of [-1, 1]) {
    const center = destinationPoint(lat, lon, distance, heading)
    const offsetCenter = destinationPoint(center[1], center[0], sideOffset, heading + side * 90)
    features.push({
      type: 'Feature',
      properties: { type: 'aiming' },
      geometry: {
        type: 'Polygon',
        coordinates: [createRectPolygon(offsetCenter[1], offsetCenter[0], length, width, heading)],
      },
    })
  }
}

function pushTouchdownZoneMarks(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  runwayWidth: number,
) {
  const distances = [150, 300, 450, 600, 750, 900]
  const pairCounts = [3, 3, 2, 2, 1, 1]
  const markLength = 22.5
  const markWidth = 3
  const sideOffset = runwayWidth * 0.15

  distances.forEach((distance, index) => {
    const pairs = pairCounts[index] ?? 1
    for (let pair = 0; pair < pairs; pair += 1) {
      const pairOffset = pair * 1.5
      for (const side of [-1, 1]) {
        const center = destinationPoint(lat, lon, distance, heading)
        const offsetCenter = destinationPoint(
          center[1],
          center[0],
          sideOffset + pairOffset,
          heading + side * 90,
        )
        features.push({
          type: 'Feature',
          properties: { type: 'tdz' },
          geometry: {
            type: 'Polygon',
            coordinates: [createRectPolygon(offsetCenter[1], offsetCenter[0], markLength, markWidth, heading)],
          },
        })
      }
    }
  })
}

function getApproachBarIndex(distance: number): number {
  return Math.min(29, Math.max(0, Math.floor(distance / 30) - 1))
}

function pushApproachLight(
  features: Array<Record<string, unknown>>,
  lon: number,
  lat: number,
  distance: number,
  isRed: boolean,
  intensity = 1,
) {
  features.push({
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: [lon, lat],
    },
    properties: {
      type: 'approach',
      dist: distance,
      isRed,
      intensity,
      barIndex: getApproachBarIndex(distance),
    },
  })
}

function pushRAILLights(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  startDistance = 450,
  endDistance = 730,
) {
  const spacing = (endDistance - startDistance) / 4
  for (let i = 0; i < 5; i += 1) {
    const distance = startDistance + i * spacing
    const point = destinationPoint(lat, lon, distance, heading)
    pushApproachLight(features, point[0], point[1], distance, false, 1)
  }
}

function pushALSFLights(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  withRedSidebars: boolean,
) {
  const totalLength = 730

  for (let distance = 30; distance <= totalLength; distance += 30) {
    const center = destinationPoint(lat, lon, distance, heading)
    const isRed = distance <= 60
    for (let offset = -6; offset <= 6; offset += 3) {
      const point = offset === 0
        ? center
        : destinationPoint(center[1], center[0], Math.abs(offset), offset < 0 ? heading - 90 : heading + 90)
      pushApproachLight(features, point[0], point[1], distance, isRed, 1)
    }
  }

  for (const distance of [60, 150, 240, 330, 450]) {
    const center = destinationPoint(lat, lon, distance, heading)
    const width = distance <= 150 ? 10 : 15
    for (let offset = -width; offset <= width; offset += 2.5) {
      if (Math.abs(offset) < 4) continue
      const point = destinationPoint(center[1], center[0], Math.abs(offset), offset < 0 ? heading - 90 : heading + 90)
      pushApproachLight(features, point[0], point[1], distance, distance <= 60, 0.95)
    }
  }

  if (withRedSidebars) {
    for (let distance = 30; distance <= 300; distance += 30) {
      const center = destinationPoint(lat, lon, distance, heading)
      for (const side of [-1, 1]) {
        const point = destinationPoint(center[1], center[0], 7, heading + side * 90)
        pushApproachLight(features, point[0], point[1], distance, true, 0.9)
      }
    }
  }

  pushRAILLights(features, lat, lon, heading, 450, 730)
}

function pushCalvertLights(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  isCalvert2: boolean,
) {
  const totalLength = 900
  for (let distance = 30; distance <= totalLength; distance += 30) {
    const point = destinationPoint(lat, lon, distance, heading)
    pushApproachLight(features, point[0], point[1], distance, distance <= 90, 1)
  }

  const crossbars = isCalvert2
    ? [90, 150, 300, 450, 600, 750]
    : [150, 300, 450, 600]

  for (const distance of crossbars) {
    const center = destinationPoint(lat, lon, distance, heading)
    const width = Math.min(distance / 10, 25)
    for (let offset = -width; offset <= width; offset += 3) {
      if (Math.abs(offset) < 3) continue
      const point = destinationPoint(center[1], center[0], Math.abs(offset), offset < 0 ? heading - 90 : heading + 90)
      pushApproachLight(features, point[0], point[1], distance, false, 0.9)
    }
  }
}

function pushSSALLights(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  hasRail: boolean,
) {
  const totalLength = 425
  for (let distance = 60; distance <= totalLength; distance += 60) {
    const center = destinationPoint(lat, lon, distance, heading)
    for (let offset = -3; offset <= 3; offset += 3) {
      const point = offset === 0
        ? center
        : destinationPoint(center[1], center[0], Math.abs(offset), offset < 0 ? heading - 90 : heading + 90)
      pushApproachLight(features, point[0], point[1], distance, distance <= 60, 1)
    }
  }

  const thresholdBar = destinationPoint(lat, lon, 30, heading)
  for (let offset = -12; offset <= 12; offset += 3) {
    const point = destinationPoint(
      thresholdBar[1],
      thresholdBar[0],
      Math.abs(offset),
      offset < 0 ? heading - 90 : heading + 90,
    )
    pushApproachLight(features, point[0], point[1], 30, true, 1)
  }

  if (hasRail) {
    pushRAILLights(features, lat, lon, heading, 425, 730)
  }
}

function pushMALSLights(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  hasRail: boolean,
) {
  const totalLength = 425
  for (let distance = 60; distance <= totalLength; distance += 60) {
    const center = destinationPoint(lat, lon, distance, heading)
    for (let offset = -3; offset <= 3; offset += 3) {
      const point = offset === 0
        ? center
        : destinationPoint(center[1], center[0], Math.abs(offset), offset < 0 ? heading - 90 : heading + 90)
      pushApproachLight(features, point[0], point[1], distance, false, 0.9)
    }
  }

  for (const distance of [60, 180, 300]) {
    const center = destinationPoint(lat, lon, distance, heading)
    for (let offset = -12; offset <= 12; offset += 3) {
      if (Math.abs(offset) < 4) continue
      const point = destinationPoint(center[1], center[0], Math.abs(offset), offset < 0 ? heading - 90 : heading + 90)
      pushApproachLight(features, point[0], point[1], distance, false, 0.85)
    }
  }

  if (hasRail) {
    pushRAILLights(features, lat, lon, heading, 425, 730)
  }
}

function pushODALSLights(features: Array<Record<string, unknown>>, lat: number, lon: number, heading: number) {
  for (let i = 0; i < 5; i += 1) {
    const distance = 90 + i * 90
    const point = destinationPoint(lat, lon, distance, heading)
    pushApproachLight(features, point[0], point[1], distance, false, 1)
  }
}

function pushGenericApproachLights(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
) {
  for (let distance = 60; distance <= 300; distance += 60) {
    const point = destinationPoint(lat, lon, distance, heading)
    pushApproachLight(features, point[0], point[1], distance, distance <= 60, 0.8)
  }
}

function pushApproachLightsByType(
  features: Array<Record<string, unknown>>,
  lat: number,
  lon: number,
  heading: number,
  lightingType: number,
) {
  switch (lightingType) {
    case 1:
    case 2:
      pushALSFLights(features, lat, lon, heading, lightingType === 2)
      break
    case 3:
    case 4:
      pushCalvertLights(features, lat, lon, heading, lightingType === 4)
      break
    case 5:
    case 6:
    case 7:
      pushSSALLights(features, lat, lon, heading, lightingType === 5)
      break
    case 8:
    case 9:
    case 10:
      pushMALSLights(features, lat, lon, heading, lightingType === 8)
      break
    case 11:
      pushODALSLights(features, lat, lon, heading)
      break
    case 12:
      pushRAILLights(features, lat, lon, heading)
      break
    default:
      pushGenericApproachLights(features, lat, lon, heading)
      break
  }
}

function toAirportRunwayShoulderFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features = detail.runways
    .map((runway) => {
      const shoulderCode = Number(runway.shoulderSurfaceCode || 0)
      if (!Number.isFinite(shoulderCode) || shoulderCode <= 0) return null

      const shoulderWidth = runway.shoulderWidthM && runway.shoulderWidthM > 0
        ? runway.shoulderWidthM
        : defaultShoulderWidth(runwayWidthMeters(runway))

      const polygon = createRunwayPolygon(runway, shoulderWidth)
      if (!polygon) return null

      return {
        type: 'Feature',
        properties: {
          surfaceCode: shoulderCode,
          runwayName: runway.name,
        },
        geometry: {
          type: 'Polygon',
          coordinates: [polygon],
        },
      }
    })
    .filter((item): item is Record<string, unknown> => Boolean(item))

  return { type: 'FeatureCollection', features }
}

function toAirportRunwayFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features = detail.runways
    .map((runway) => {
      const polygon = createRunwayPolygon(runway)
      if (!polygon) return null

      return {
        type: 'Feature',
        properties: {
          name: runway.name,
          surfaceCode: Number(runway.surfaceCode || 0),
          surfaceType: runway.surfaceType || '',
          widthM: runwayWidthMeters(runway),
          heading: getRunwayHeading(runway),
        },
        geometry: {
          type: 'Polygon',
          coordinates: [polygon],
        },
      }
    })
    .filter((item): item is Record<string, unknown> => Boolean(item))

  return { type: 'FeatureCollection', features }
}

function toAirportRunwayCenterlineFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features = detail.runways
    .filter((runway) =>
      isFiniteLatLon(runway.end1Lat, runway.end1Lon)
      && isFiniteLatLon(runway.end2Lat, runway.end2Lon),
    )
    .map((runway) => ({
      type: 'Feature',
      properties: {
        name: runway.name,
      },
      geometry: {
        type: 'LineString',
        coordinates: [
          [runway.end1Lon, runway.end1Lat],
          [runway.end2Lon, runway.end2Lat],
        ],
      },
    }))

  return { type: 'FeatureCollection', features }
}

function toAirportRunwayMarkingFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features: Array<Record<string, unknown>> = []
  for (const runway of detail.runways) {
    if (Number(runway.surfaceCode || 0) === 13) {
      continue
    }

    if (
      !isFiniteLatLon(runway.end1Lat, runway.end1Lon)
      || !isFiniteLatLon(runway.end2Lat, runway.end2Lon)
    ) {
      continue
    }

    const heading1 = getRunwayHeading(runway)
    const heading2 = normalizeHeading(heading1 + 180)
    const width = runwayWidthMeters(runway)

    const marking1 = Number(runway.end1Marking ?? 0)
    if (marking1 >= RUNWAY_MARKING_VISUAL) {
      pushThresholdBars(features, runway.end1Lat, runway.end1Lon, heading1, width)
    }
    if (marking1 >= RUNWAY_MARKING_NON_PRECISION) {
      pushAimingPoints(features, runway.end1Lat, runway.end1Lon, heading1, width)
    }
    if (marking1 >= RUNWAY_MARKING_PRECISION) {
      pushTouchdownZoneMarks(features, runway.end1Lat, runway.end1Lon, heading1, width)
    }

    const marking2 = Number(runway.end2Marking ?? 0)
    if (marking2 >= RUNWAY_MARKING_VISUAL) {
      pushThresholdBars(features, runway.end2Lat, runway.end2Lon, heading2, width)
    }
    if (marking2 >= RUNWAY_MARKING_NON_PRECISION) {
      pushAimingPoints(features, runway.end2Lat, runway.end2Lon, heading2, width)
    }
    if (marking2 >= RUNWAY_MARKING_PRECISION) {
      pushTouchdownZoneMarks(features, runway.end2Lat, runway.end2Lon, heading2, width)
    }
  }

  return { type: 'FeatureCollection', features }
}

function toAirportRunwayNumberFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features: Array<Record<string, unknown>> = []
  for (const runway of detail.runways) {
    if (Number(runway.surfaceCode || 0) === 13) {
      continue
    }

    if (
      !isFiniteLatLon(runway.end1Lat, runway.end1Lon)
      || !isFiniteLatLon(runway.end2Lat, runway.end2Lon)
    ) {
      continue
    }

    const heading1 = getRunwayHeading(runway)
    const heading2 = normalizeHeading(heading1 + 180)
    const pos1 = destinationPoint(runway.end1Lat, runway.end1Lon, 300, heading1)
    const pos2 = destinationPoint(runway.end2Lat, runway.end2Lon, 300, heading2)

    features.push({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: pos1,
      },
      properties: {
        number: formatRunwayNumberLabel(runway.end1Name || ''),
        rotation: heading1,
        name: runway.end1Name || '',
      },
    })

    features.push({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: pos2,
      },
      properties: {
        number: formatRunwayNumberLabel(runway.end2Name || ''),
        rotation: heading2,
        name: runway.end2Name || '',
      },
    })
  }

  return { type: 'FeatureCollection', features }
}

function toAirportRunwayLightFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features: Array<Record<string, unknown>> = []
  for (const runway of detail.runways) {
    if (
      !isFiniteLatLon(runway.end1Lat, runway.end1Lon)
      || !isFiniteLatLon(runway.end2Lat, runway.end2Lon)
    ) {
      continue
    }

    const heading1 = getRunwayHeading(runway)
    const heading2 = normalizeHeading(heading1 + 180)
    const width = runwayWidthMeters(runway)
    const length = haversineDistanceMeters(runway.end1Lat, runway.end1Lon, runway.end2Lat, runway.end2Lon)

    if (runway.edgeLights) {
      for (let distance = 0; distance <= length; distance += 60) {
        const ratio = length > 0 ? distance / length : 0
        const lat = runway.end1Lat + ratio * (runway.end2Lat - runway.end1Lat)
        const lon = runway.end1Lon + ratio * (runway.end2Lon - runway.end1Lon)

        const left = destinationPoint(lat, lon, width / 2, heading1 - 90)
        const right = destinationPoint(lat, lon, width / 2, heading1 + 90)
        const isYellowZone = distance < 600 || distance > length - 600

        features.push({
          type: 'Feature',
          geometry: { type: 'Point', coordinates: left },
          properties: {
            type: 'edge',
            isYellowZone,
          },
        })

        features.push({
          type: 'Feature',
          geometry: { type: 'Point', coordinates: right },
          properties: {
            type: 'edge',
            isYellowZone,
          },
        })
      }
    }

    for (let offset = -width / 2; offset <= width / 2; offset += 3) {
      const threshold1 = destinationPoint(
        runway.end1Lat,
        runway.end1Lon,
        Math.abs(offset),
        offset < 0 ? heading1 - 90 : heading1 + 90,
      )
      const threshold2 = destinationPoint(
        runway.end2Lat,
        runway.end2Lon,
        Math.abs(offset),
        offset < 0 ? heading2 - 90 : heading2 + 90,
      )

      features.push({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: threshold1 },
        properties: { type: 'threshold' },
      })
      features.push({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: threshold2 },
        properties: { type: 'threshold' },
      })

      const end1 = destinationPoint(threshold1[1], threshold1[0], 4, heading1 + 180)
      const end2 = destinationPoint(threshold2[1], threshold2[0], 4, heading2 + 180)

      features.push({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: end1 },
        properties: { type: 'end' },
      })
      features.push({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: end2 },
        properties: { type: 'end' },
      })
    }

    if (runway.centerlineLights) {
      for (let distance = 30; distance < length - 30; distance += 15) {
        const ratio = length > 0 ? distance / length : 0
        const lat = runway.end1Lat + ratio * (runway.end2Lat - runway.end1Lat)
        const lon = runway.end1Lon + ratio * (runway.end2Lon - runway.end1Lon)
        const distFromEnd = Math.min(distance, length - distance)

        features.push({
          type: 'Feature',
          geometry: {
            type: 'Point',
            coordinates: [lon, lat],
          },
          properties: {
            type: 'centerline',
            isRedZone: distFromEnd < 300,
            isYellowZone: distFromEnd >= 300 && distFromEnd < 900,
          },
        })
      }
    }

    const end1Lighting = Number(runway.end1Lighting || 0)
    if (end1Lighting > 0) {
      pushApproachLightsByType(
        features,
        runway.end1Lat,
        runway.end1Lon,
        heading2,
        end1Lighting,
      )
    }

    const end2Lighting = Number(runway.end2Lighting || 0)
    if (end2Lighting > 0) {
      pushApproachLightsByType(
        features,
        runway.end2Lat,
        runway.end2Lon,
        heading1,
        end2Lighting,
      )
    }
  }

  return {
    type: 'FeatureCollection',
    features,
  }
}

function toAirportRunwayEndFeatureCollection(detail: MapAirportDetail | null): FeatureCollection {
  if (!detail) {
    return { type: 'FeatureCollection', features: [] }
  }

  const features: Array<Record<string, unknown>> = []
  detail.runways.forEach((runway) => {
    if (isFiniteLatLon(runway.end1Lat, runway.end1Lon)) {
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
    if (isFiniteLatLon(runway.end2Lat, runway.end2Lon)) {
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
      .filter((helipad) => isFiniteLatLon(helipad.lat, helipad.lon))
      .map((helipad) => ({
        type: 'Feature',
        properties: {
          name: helipad.name,
          heading: helipad.heading || 0,
          widthM: helipad.widthM || 20,
          lengthM: helipad.lengthM || helipad.widthM || 20,
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
      .filter((gate) => isFiniteLatLon(gate.lat, gate.lon))
      .map((gate) => ({
        type: 'Feature',
        properties: {
          name: gate.name,
          heading: gate.heading || 0,
          locationType: gate.locationType || '',
          operationType: gate.operationType || '',
          widthCode: gate.widthCode || '',
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

function setActiveProcedureTab(tab: ProcedureTabKey) {
  activeProcedureTab.value = tab
  expandedProcedureGroup.value = null
}

function toggleProcedureGroup(name: string) {
  expandedProcedureGroup.value = expandedProcedureGroup.value === name ? null : name
}

function formatProcedureVariant(procedure: MapProcedure): string {
  const parts: string[] = []
  if (procedure.runway) {
    parts.push(`${t('map.procedures.runway')}: ${procedure.runway}`)
  }
  if (procedure.transition) {
    parts.push(`${t('map.procedures.transition')}: ${procedure.transition}`)
  }
  return parts.length > 0 ? parts.join(' · ') : t('map.procedures.common')
}

function resetAirportProceduresState() {
  proceduresSeq += 1
  proceduresLoading.value = false
  airportProcedures.value = null
  activeProcedureTab.value = 'sids'
  expandedProcedureGroup.value = null
}

async function refreshAirportProcedures(airport: MapAirport) {
  const xplanePath = appStore.xplanePath
  if (!xplanePath) return

  const seq = ++proceduresSeq
  proceduresLoading.value = true
  airportProcedures.value = null
  expandedProcedureGroup.value = null

  try {
    const procedures = await mapGetAirportProcedures(xplanePath, airport.icao)
    if (seq !== proceduresSeq) return

    airportProcedures.value = procedures

    if ((procedures.sids || []).length > 0) {
      activeProcedureTab.value = 'sids'
    } else if ((procedures.stars || []).length > 0) {
      activeProcedureTab.value = 'stars'
    } else if ((procedures.approaches || []).length > 0) {
      activeProcedureTab.value = 'approaches'
    } else {
      activeProcedureTab.value = 'sids'
    }
  } catch (error) {
    if (seq !== proceduresSeq) return
    airportProcedures.value = {
      icao: airport.icao,
      sids: [],
      stars: [],
      approaches: [],
    }
    logError(`Failed to fetch airport procedures for ${airport.icao}: ${error}`, 'map')
  } finally {
    if (seq === proceduresSeq) {
      proceduresLoading.value = false
    }
  }
}

async function refreshProceduresForSelected() {
  const airport = mapStore.selectedAirport
  if (!airport) return
  await refreshAirportProcedures(airport)
}

async function refreshGatewayDataForSelected() {
  const airport = mapStore.selectedAirport
  if (!airport) return
  await refreshGatewayAirport(airport)
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

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null
}

function pickString(record: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const raw = record[key]
    if (typeof raw !== 'string') continue
    const text = raw.trim()
    if (text) return text
  }
  return null
}

function pickNumber(record: Record<string, unknown>, keys: string[]): number | null {
  for (const key of keys) {
    const raw = record[key]
    if (typeof raw === 'number' && Number.isFinite(raw)) return raw
    if (typeof raw === 'string') {
      const parsed = Number(raw.trim())
      if (Number.isFinite(parsed)) return parsed
    }
  }
  return null
}

function pickRecord(record: Record<string, unknown>, keys: string[]): Record<string, unknown> | null {
  for (const key of keys) {
    const child = asRecord(record[key])
    if (child) return child
  }
  return null
}

function pickArray(record: Record<string, unknown>, keys: string[]): unknown[] | null {
  for (const key of keys) {
    const child = record[key]
    if (Array.isArray(child)) return child
  }
  return null
}

function pickArrayLength(record: Record<string, unknown>, keys: string[]): number | null {
  const list = pickArray(record, keys)
  return list ? list.length : null
}

function parseGatewaySummary(payload: unknown, fallbackIcao: string): GatewaySummary | null {
  const root = asRecord(payload)
  if (!root) return null

  const airport = pickRecord(root, ['airport', 'Airport', 'data']) ?? root
  const airportCode = (
    pickString(airport, ['icao', 'ICAO', 'airportCode', 'AirportCode', 'code', 'ident'])
    || pickString(root, ['icao', 'ICAO', 'airportCode', 'AirportCode'])
    || fallbackIcao
  )
    .trim()
    .toUpperCase()

  if (!airportCode) return null

  const airportName = pickString(airport, ['name', 'airportName', 'AirportName', 'Name'])
  const sceneryList = pickArray(root, ['sceneries', 'Sceneries', 'scenery', 'results', 'items']) ?? []

  const rootRecommended = pickRecord(root, ['recommendedScenery', 'RecommendedScenery'])
  const airportRecommended = pickRecord(airport, ['recommendedScenery', 'RecommendedScenery'])

  const recommendedSceneryId = (
    pickNumber(
      airport,
      ['recommendedSceneryId', 'RecommendedSceneryId', 'recommended_scenery_id'],
    )
    ?? pickNumber(
      root,
      ['recommendedSceneryId', 'RecommendedSceneryId', 'recommended_scenery_id'],
    )
    ?? (rootRecommended
      ? pickNumber(rootRecommended, ['id', 'sceneryId', 'SceneryId'])
      : null)
    ?? (airportRecommended
      ? pickNumber(airportRecommended, ['id', 'sceneryId', 'SceneryId'])
      : null)
  )

  let recommended = rootRecommended ?? airportRecommended
  if (!recommended && sceneryList.length > 0) {
    if (recommendedSceneryId !== null) {
      recommended = sceneryList
        .map((entry) => asRecord(entry))
        .find((entry) => entry && pickNumber(entry, ['id', 'sceneryId', 'SceneryId']) === recommendedSceneryId)
        ?? null
    }
    if (!recommended) {
      recommended = asRecord(sceneryList[0])
    }
  }

  const sceneryCount = (
    pickNumber(airport, ['sceneryCount', 'SceneryCount', 'totalSceneries'])
    ?? pickNumber(root, ['sceneryCount', 'SceneryCount', 'totalSceneries'])
    ?? (sceneryList.length > 0 ? sceneryList.length : null)
  )

  let recommendedArtist = recommended
    ? pickString(recommended, ['userName', 'username', 'authorName', 'author', 'artist', 'submittedBy'])
    : null
  if (!recommendedArtist && recommended) {
    const user = pickRecord(recommended, ['user', 'User'])
    if (user) {
      recommendedArtist = pickString(user, ['name', 'username', 'displayName', 'userName'])
    }
  }

  const recommendedAcceptedAt = recommended
    ? pickString(
      recommended,
      [
        'dateAccepted',
        'acceptedAt',
        'accepted',
        'approvalDate',
        'approvedAt',
        'date',
        'updatedAt',
      ],
    )
    : null

  if (
    recommendedSceneryId === null
    && sceneryCount === null
    && !airportName
    && !recommendedArtist
    && !recommendedAcceptedAt
  ) {
    return null
  }

  return {
    icao: airportCode,
    airportName,
    sceneryCount,
    recommendedSceneryId,
    recommendedArtist,
    recommendedAcceptedAt,
  }
}

function parseGatewaySceneryDetail(payload: unknown, fallbackSceneryId: number): GatewaySceneryDetail | null {
  const root = asRecord(payload)
  if (!root) return null

  const detail = pickRecord(root, ['scenery', 'Scenery', 'data']) ?? root
  const sceneryId = pickNumber(detail, ['id', 'sceneryId', 'SceneryId']) ?? fallbackSceneryId

  let artist = pickString(
    detail,
    ['artist', 'artistName', 'author', 'authorName', 'userName', 'username', 'submittedBy'],
  )
  const user = pickRecord(detail, ['user', 'User'])
  if (!artist && user) {
    artist = pickString(user, ['name', 'username', 'displayName', 'userName'])
  }

  const status = pickString(
    detail,
    ['status', 'approvalStatus', 'submissionStatus', 'state', 'gatewayStatus'],
  )
  const approvedDate = pickString(
    detail,
    ['dateAccepted', 'acceptedAt', 'approvedDate', 'approvalDate', 'updatedAt'],
  )
  const comment = pickString(
    detail,
    ['artistComments', 'comments', 'comment', 'description', 'notes'],
  )

  const runwayCount = (
    pickNumber(detail, ['runwayCount', 'runwaysCount'])
    ?? pickArrayLength(detail, ['runways', 'Runways'])
  )
  const gateCount = (
    pickNumber(detail, ['gateCount', 'gatesCount', 'startupCount'])
    ?? pickArrayLength(detail, ['gates', 'startupLocations', 'ramps'])
  )
  const taxiwayCount = (
    pickNumber(detail, ['taxiwayCount', 'taxiwaysCount'])
    ?? pickArrayLength(detail, ['taxiways', 'taxiwayEdges'])
  )

  const rawFeatures = pickArray(detail, ['features', 'featureFlags', 'tags']) ?? []
  const tagFeatures = rawFeatures
    .map((item) => (typeof item === 'string' ? item.trim() : ''))
    .filter((item) => item.length > 0)
    .slice(0, 5)

  const countFeatures: string[] = []
  if (runwayCount !== null) countFeatures.push(`RWY ${runwayCount}`)
  if (gateCount !== null) countFeatures.push(`Gates ${gateCount}`)
  if (taxiwayCount !== null) countFeatures.push(`Taxiway ${taxiwayCount}`)

  const features = [...countFeatures, ...tagFeatures]
  if (
    !status
    && !artist
    && !approvedDate
    && !comment
    && features.length === 0
    && sceneryId <= 0
  ) {
    return null
  }

  return {
    sceneryId,
    status,
    artist,
    approvedDate,
    comment,
    features,
  }
}

function formatGatewayDate(input: string): string {
  const timestamp = Date.parse(input)
  if (Number.isNaN(timestamp)) {
    return input
  }
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(new Date(timestamp))
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
    { id: 'airport-runway-shoulders', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runways-fill', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-centerline', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-labels', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-threshold-bars', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-aiming-points', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-tdz-marks', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-numbers', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-edge-lights', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-threshold-lights', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-centerline-lights', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-end-lights', visible: mapStore.layerVisibility.airports },
    { id: 'airport-approach-lights', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-ends-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-runway-ends-label', visible: mapStore.layerVisibility.airports },
    { id: 'airport-helipads-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-gates-circle', visible: mapStore.layerVisibility.airports },
    { id: 'airport-gates-label', visible: mapStore.layerVisibility.airports },
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
  const addGeoJsonSource = (id: string) => {
    if (map.getSource(id)) return
    map.addSource(id, {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
    })
  }

  const sourceIds = [
    'airports',
    'navaids',
    'waypoints',
    'airways',
    'ils',
    'airspaces',
    'plane',
    'vatsim',
    'airport-runway-shoulders',
    'airport-runways',
    'airport-runway-centerlines',
    'airport-runway-markings',
    'airport-runway-numbers',
    'airport-runway-lights',
    'airport-runway-ends',
    'airport-helipads',
    'airport-gates',
    'airport-taxiways',
    'airport-tower',
    'airport-beacon',
    'airport-windsocks',
    'airport-signs',
    'simbrief-route',
  ]
  sourceIds.forEach((id) => addGeoJsonSource(id))

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
        'line-width': ['interpolate', ['linear'], ['zoom'], 4, 0.4, 9, 1.2],
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
        'line-width': ['interpolate', ['linear'], ['zoom'], 4, 1, 10, 2.5],
        'line-opacity': 0.75,
      },
    })
  }

  if (!map.getLayer('airport-runway-shoulders')) {
    map.addLayer({
      id: 'airport-runway-shoulders',
      type: 'fill',
      source: 'airport-runway-shoulders',
      minzoom: 10,
      paint: {
        'fill-color': buildSurfaceColorExpression('surfaceCode'),
        'fill-opacity': 1,
      },
    })
  }

  if (!map.getLayer('airport-runways-fill')) {
    map.addLayer({
      id: 'airport-runways-fill',
      type: 'fill',
      source: 'airport-runways',
      minzoom: 10,
      paint: {
        'fill-color': buildSurfaceColorExpression('surfaceCode'),
        'fill-opacity': 1,
      },
    })
  }

  if (!map.getLayer('airport-runway-centerline')) {
    map.addLayer({
      id: 'airport-runway-centerline',
      type: 'line',
      source: 'airport-runway-centerlines',
      minzoom: 12,
      paint: {
        'line-color': '#ffffff',
        'line-width': ['interpolate', ['linear'], ['zoom'], 12, 1, 16, 3, 20, 5],
        'line-dasharray': [10, 10],
        'line-opacity': 0.9,
      },
    })
  }

  if (!map.getLayer('airport-runway-labels')) {
    map.addLayer({
      id: 'airport-runway-labels',
      type: 'symbol',
      source: 'airport-runways',
      minzoom: 14,
      layout: {
        'text-field': ['get', 'name'],
        'text-size': ['interpolate', ['linear'], ['zoom'], 14, 11, 18, 18],
        'text-font': ['Noto Sans Bold'],
      },
      paint: {
        'text-color': '#FFFFFF',
        'text-halo-color': '#000000',
        'text-halo-width': 1.8,
      },
    })
  }

  if (!map.getLayer('airport-runway-threshold-bars')) {
    map.addLayer({
      id: 'airport-runway-threshold-bars',
      type: 'fill',
      source: 'airport-runway-markings',
      filter: ['==', ['get', 'type'], 'threshold'],
      minzoom: 14,
      paint: {
        'fill-color': '#FFFFFF',
        'fill-opacity': 0.95,
      },
    })
  }

  if (!map.getLayer('airport-runway-aiming-points')) {
    map.addLayer({
      id: 'airport-runway-aiming-points',
      type: 'fill',
      source: 'airport-runway-markings',
      filter: ['==', ['get', 'type'], 'aiming'],
      minzoom: 14,
      paint: {
        'fill-color': '#FFFFFF',
        'fill-opacity': 0.95,
      },
    })
  }

  if (!map.getLayer('airport-runway-tdz-marks')) {
    map.addLayer({
      id: 'airport-runway-tdz-marks',
      type: 'fill',
      source: 'airport-runway-markings',
      filter: ['==', ['get', 'type'], 'tdz'],
      minzoom: 15,
      paint: {
        'fill-color': '#FFFFFF',
        'fill-opacity': 0.9,
      },
    })
  }

  if (!map.getLayer('airport-runway-numbers')) {
    map.addLayer({
      id: 'airport-runway-numbers',
      type: 'symbol',
      source: 'airport-runway-numbers',
      minzoom: 13,
      layout: {
        'text-field': ['get', 'number'],
        'text-font': ['Noto Sans Bold'],
        'text-size': ['interpolate', ['linear'], ['zoom'], 13, 16, 15, 28, 17, 48, 19, 72],
        'text-rotate': ['get', 'rotation'],
        'text-rotation-alignment': 'map',
        'text-pitch-alignment': 'map',
        'text-allow-overlap': true,
        'text-ignore-placement': true,
      },
      paint: {
        'text-color': '#FFFFFF',
        'text-halo-color': '#000000',
        'text-halo-width': 1,
        'text-opacity': ['interpolate', ['linear'], ['zoom'], 13, 0.7, 15, 1],
      },
    })
  }

  if (!map.getLayer('airport-runway-edge-lights')) {
    map.addLayer({
      id: 'airport-runway-edge-lights',
      type: 'circle',
      source: 'airport-runway-lights',
      filter: ['==', ['get', 'type'], 'edge'],
      minzoom: 15,
      paint: {
        'circle-color': ['case', ['get', 'isYellowZone'], '#FFD700', '#FFFFFF'],
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 15, 0.8, 17, 1.5, 20, 2.5],
        'circle-blur': 0.2,
        'circle-opacity': 0.85,
      },
    })
  }

  if (!map.getLayer('airport-runway-threshold-lights')) {
    map.addLayer({
      id: 'airport-runway-threshold-lights',
      type: 'circle',
      source: 'airport-runway-lights',
      filter: ['==', ['get', 'type'], 'threshold'],
      minzoom: 15,
      paint: {
        'circle-color': '#00FF00',
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 15, 1, 17, 2, 20, 3],
        'circle-blur': 0.3,
        'circle-opacity': 0.9,
      },
    })
  }

  if (!map.getLayer('airport-runway-end-lights')) {
    map.addLayer({
      id: 'airport-runway-end-lights',
      type: 'circle',
      source: 'airport-runway-lights',
      filter: ['==', ['get', 'type'], 'end'],
      minzoom: 15,
      paint: {
        'circle-color': '#FF0000',
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 15, 1, 17, 2, 20, 3],
        'circle-blur': 0.3,
        'circle-opacity': 0.9,
      },
    })
  }

  if (!map.getLayer('airport-runway-centerline-lights')) {
    map.addLayer({
      id: 'airport-runway-centerline-lights',
      type: 'circle',
      source: 'airport-runway-lights',
      filter: ['==', ['get', 'type'], 'centerline'],
      minzoom: 16,
      paint: {
        'circle-color': [
          'case',
          ['get', 'isRedZone'],
          '#FF0000',
          ['get', 'isYellowZone'],
          '#FFD700',
          '#FFFFFF',
        ],
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 16, 0.5, 18, 1.5, 20, 2],
        'circle-blur': 0.2,
        'circle-opacity': 0.75,
      },
    })
  }

  if (!map.getLayer('airport-approach-lights')) {
    map.addLayer({
      id: 'airport-approach-lights',
      type: 'circle',
      source: 'airport-runway-lights',
      filter: ['==', ['get', 'type'], 'approach'],
      minzoom: 13,
      paint: {
        'circle-color': ['case', ['get', 'isRed'], '#FF0000', '#FFFFFF'],
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 13, 0.8, 16, 1.5, 18, 2.5],
        'circle-blur': 0.2,
        'circle-opacity': 0.9,
      },
    })
  }

  if (!map.getLayer('airport-runway-ends-circle')) {
    map.addLayer({
      id: 'airport-runway-ends-circle',
      type: 'circle',
      source: 'airport-runway-ends',
      minzoom: 13,
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 13, 4, 15, 6, 17, 10, 19, 14],
        'circle-color': '#3b82f6',
        'circle-opacity': 0.85,
        'circle-stroke-width': 2,
        'circle-stroke-color': '#ffffff',
      },
    })
  }

  if (!map.getLayer('airport-runway-ends-label')) {
    map.addLayer({
      id: 'airport-runway-ends-label',
      type: 'symbol',
      source: 'airport-runway-ends',
      minzoom: 14,
      layout: {
        'text-field': ['get', 'name'],
        'text-font': ['Noto Sans Bold'],
        'text-size': ['interpolate', ['linear'], ['zoom'], 14, 10, 17, 14],
        'text-anchor': 'center',
      },
      paint: {
        'text-color': '#ffffff',
        'text-halo-color': '#3b82f6',
        'text-halo-width': 0,
      },
    })
  }

  if (!map.getLayer('airport-helipads-circle')) {
    map.addLayer({
      id: 'airport-helipads-circle',
      type: 'circle',
      source: 'airport-helipads',
      minzoom: 11,
      paint: {
        'circle-color': '#334155',
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 11, 4, 14, 8],
        'circle-stroke-color': '#cbd5e1',
        'circle-stroke-width': 1.2,
      },
    })
  }

  if (!map.getLayer('airport-gates-circle')) {
    map.addLayer({
      id: 'airport-gates-circle',
      type: 'circle',
      source: 'airport-gates',
      minzoom: 14,
      paint: {
        'circle-color': '#64748b',
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 14, 3, 18, 7],
        'circle-stroke-color': '#e2e8f0',
        'circle-stroke-width': 1,
        'circle-opacity': 0.75,
      },
    })
  }

  if (!map.getLayer('airport-gates-label')) {
    map.addLayer({
      id: 'airport-gates-label',
      type: 'symbol',
      source: 'airport-gates',
      minzoom: 14,
      layout: {
        'text-field': ['get', 'name'],
        'text-font': ['Noto Sans Bold'],
        'text-size': ['interpolate', ['linear'], ['zoom'], 14, 9, 18, 13],
        'text-offset': [0, 1.5],
        'text-anchor': 'top',
      },
      paint: {
        'text-color': '#e2e8f0',
        'text-halo-color': '#0f172a',
        'text-halo-width': 1.5,
      },
    })
  }

  if (!map.getLayer('airport-taxiways-line')) {
    map.addLayer({
      id: 'airport-taxiways-line',
      type: 'line',
      source: 'airport-taxiways',
      minzoom: 12,
      paint: {
        'line-color': '#fbbf24',
        'line-width': ['interpolate', ['linear'], ['zoom'], 12, 0.8, 16, 2],
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
      minzoom: 14,
      layout: {
        'symbol-placement': 'line-center',
        'text-field': ['get', 'name'],
        'text-size': 10,
        'text-font': ['Noto Sans Bold'],
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
      minzoom: 13,
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
      minzoom: 14,
      layout: {
        'text-field': ['get', 'name'],
        'text-size': 10,
        'text-font': ['Noto Sans Bold'],
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
      minzoom: 12,
      paint: {
        'circle-color': '#facc15',
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
      minzoom: 14,
      paint: {
        'circle-color': ['case', ['to-boolean', ['get', 'illuminated']], '#00FF88', '#FF8800'],
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 14, 4, 18, 8],
        'circle-stroke-width': 2,
        'circle-stroke-color': '#FFFFFF',
        'circle-blur': ['case', ['to-boolean', ['get', 'illuminated']], 0.3, 0],
      },
    })
  }

  if (!map.getLayer('airport-signs-circle')) {
    map.addLayer({
      id: 'airport-signs-circle',
      type: 'circle',
      source: 'airport-signs',
      minzoom: 16,
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
      minzoom: 16,
      layout: {
        'text-field': ['get', 'text'],
        'text-size': 9,
        'text-font': ['Noto Sans Bold'],
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
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 3, 1.5, 7, 4, 12, 7],
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
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 4, 1, 10, 3],
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

function resetGatewaySceneryState() {
  gatewayScenerySeq += 1
  gatewaySceneryLoading.value = false
  gatewayScenery.value = null
}

async function refreshGatewayScenery(
  sceneryId: number,
  airportIcao: string,
  airportGatewaySeq: number,
) {
  if (!Number.isFinite(sceneryId) || sceneryId <= 0) {
    resetGatewaySceneryState()
    return
  }

  const scenerySeq = ++gatewayScenerySeq
  gatewaySceneryLoading.value = true
  gatewayScenery.value = null

  try {
    const payload = await mapFetchGatewayScenery(sceneryId)
    if (airportGatewaySeq !== gatewaySeq || scenerySeq !== gatewayScenerySeq) return
    gatewayScenery.value = parseGatewaySceneryDetail(payload, sceneryId)
  } catch (error) {
    if (airportGatewaySeq !== gatewaySeq || scenerySeq !== gatewayScenerySeq) return
    gatewayScenery.value = null
    logError(`Failed to fetch Gateway scenery ${sceneryId} for ${airportIcao}: ${error}`, 'map')
  } finally {
    if (airportGatewaySeq === gatewaySeq && scenerySeq === gatewayScenerySeq) {
      gatewaySceneryLoading.value = false
    }
  }
}

async function refreshGatewayAirport(airport: MapAirport) {
  const seq = ++gatewaySeq
  gatewayLoading.value = true
  gatewaySummary.value = null
  resetGatewaySceneryState()

  try {
    const payload = await mapFetchGatewayAirport(airport.icao)
    if (seq !== gatewaySeq) return
    const summary = parseGatewaySummary(payload, airport.icao)
    gatewaySummary.value = summary

    const recommendedId = summary?.recommendedSceneryId
    if (recommendedId && recommendedId > 0) {
      await refreshGatewayScenery(recommendedId, airport.icao, seq)
    }
  } catch (error) {
    if (seq !== gatewaySeq) return
    gatewaySummary.value = null
    resetGatewaySceneryState()
    logError(`Failed to fetch Gateway data for ${airport.icao}: ${error}`, 'map')
  } finally {
    if (seq === gatewaySeq) {
      gatewayLoading.value = false
    }
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
  resetAirportProceduresState()
  gatewaySummary.value = null
  gatewayLoading.value = false
  resetGatewaySceneryState()

  const map = mapRef.value
  if (map) {
    map.easeTo({
      center: [airport.lon, airport.lat],
      zoom: Math.max(map.getZoom(), 8),
      duration: 500,
    })
  }

  await Promise.allSettled([
    refreshAirportWeather(airport),
    refreshAirportDetail(airport),
    refreshAirportProcedures(airport),
    refreshGatewayAirport(airport),
  ])
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
  updateGeoJsonSource('airport-runway-shoulders', toAirportRunwayShoulderFeatureCollection(detail))
  updateGeoJsonSource('airport-runways', toAirportRunwayFeatureCollection(detail))
  updateGeoJsonSource('airport-runway-centerlines', toAirportRunwayCenterlineFeatureCollection(detail))
  updateGeoJsonSource('airport-runway-markings', toAirportRunwayMarkingFeatureCollection(detail))
  updateGeoJsonSource('airport-runway-numbers', toAirportRunwayNumberFeatureCollection(detail))
  updateGeoJsonSource('airport-runway-lights', toAirportRunwayLightFeatureCollection(detail))
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
  resetAirportProceduresState()
  gatewaySeq += 1
  gatewayLoading.value = false
  gatewaySummary.value = null
  resetGatewaySceneryState()
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
