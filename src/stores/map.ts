import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'
import type { MapAirport, MapAirportFilters, MapLayerVisibility, MapPlaneState } from '@/types/map'

const DEFAULT_MAP_STYLE_URL = 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json'

const DEFAULT_LAYER_VISIBILITY: MapLayerVisibility = {
  airports: true,
  navaids: true,
  waypoints: true,
  airways: true,
  ils: true,
  airspaces: true,
  plane: true,
  vatsim: false,
  weatherRadar: false,
}

const DEFAULT_AIRPORT_FILTERS: MapAirportFilters = {
  showLand: true,
  showSeaplane: true,
  showHeliport: true,
  onlyCustom: false,
  minRunwayCount: 0,
}

export const useMapStore = defineStore('map', () => {
  const isInitialized = ref(false)

  const mapStyleUrl = ref(DEFAULT_MAP_STYLE_URL)
  const navRadiusNm = ref(60)
  const vatsimRefreshInterval = ref(15)
  const simbriefPilotId = ref('')
  const followPlane = ref(true)
  const weightUnit = ref<'kg' | 'lbs'>('kg')

  const layerVisibility = ref<MapLayerVisibility>({ ...DEFAULT_LAYER_VISIBILITY })
  const airportFilters = ref<MapAirportFilters>({ ...DEFAULT_AIRPORT_FILTERS })

  const selectedAirport = ref<MapAirport | null>(null)
  const planeConnected = ref(false)
  const planeState = ref<MapPlaneState | null>(null)

  async function initStore() {
    if (isInitialized.value) return

    const [
      savedStyle,
      savedRadius,
      savedVatsimInterval,
      savedLayers,
      savedFollowPlane,
      savedSimbriefPilotId,
      savedAirportFilters,
      savedWeightUnit,
    ] = await Promise.all([
      getItem<string>(STORAGE_KEYS.MAP_STYLE_URL),
      getItem<number>(STORAGE_KEYS.MAP_NAV_RADIUS_NM),
      getItem<number>(STORAGE_KEYS.MAP_VATSIM_REFRESH_INTERVAL),
      getItem<MapLayerVisibility>(STORAGE_KEYS.MAP_LAYER_VISIBILITY),
      getItem<boolean>(STORAGE_KEYS.MAP_FOLLOW_PLANE),
      getItem<string>(STORAGE_KEYS.MAP_SIMBRIEF_PILOT_ID),
      getItem<MapAirportFilters>(STORAGE_KEYS.MAP_AIRPORT_FILTERS),
      getItem<string>(STORAGE_KEYS.MAP_WEIGHT_UNIT),
    ])

    if (savedStyle && typeof savedStyle === 'string') {
      mapStyleUrl.value = savedStyle
    }

    if (typeof savedRadius === 'number' && Number.isFinite(savedRadius)) {
      navRadiusNm.value = Math.min(200, Math.max(10, savedRadius))
    }

    if (typeof savedVatsimInterval === 'number' && Number.isFinite(savedVatsimInterval)) {
      vatsimRefreshInterval.value = Math.min(120, Math.max(5, savedVatsimInterval))
    }

    if (savedLayers && typeof savedLayers === 'object') {
      layerVisibility.value = {
        ...DEFAULT_LAYER_VISIBILITY,
        ...savedLayers,
      }
    }

    if (typeof savedFollowPlane === 'boolean') {
      followPlane.value = savedFollowPlane
    }

    if (typeof savedSimbriefPilotId === 'string') {
      simbriefPilotId.value = savedSimbriefPilotId
    }

    if (savedAirportFilters && typeof savedAirportFilters === 'object') {
      airportFilters.value = {
        ...DEFAULT_AIRPORT_FILTERS,
        ...savedAirportFilters,
        minRunwayCount: Math.max(0, Number(savedAirportFilters.minRunwayCount || 0)),
      }
    }

    if (savedWeightUnit === 'kg' || savedWeightUnit === 'lbs') {
      weightUnit.value = savedWeightUnit
    }

    isInitialized.value = true
  }

  async function setMapStyleUrl(url: string) {
    mapStyleUrl.value = url
    await setItem(STORAGE_KEYS.MAP_STYLE_URL, url)
  }

  async function setNavRadiusNm(radius: number) {
    navRadiusNm.value = Math.min(200, Math.max(10, radius))
    await setItem(STORAGE_KEYS.MAP_NAV_RADIUS_NM, navRadiusNm.value)
  }

  async function setVatsimRefreshInterval(seconds: number) {
    vatsimRefreshInterval.value = Math.min(120, Math.max(5, seconds))
    await setItem(STORAGE_KEYS.MAP_VATSIM_REFRESH_INTERVAL, vatsimRefreshInterval.value)
  }

  async function setLayerVisibility(next: Partial<MapLayerVisibility>) {
    layerVisibility.value = {
      ...layerVisibility.value,
      ...next,
    }
    await setItem(STORAGE_KEYS.MAP_LAYER_VISIBILITY, layerVisibility.value)
  }

  async function toggleLayer(layer: keyof MapLayerVisibility) {
    await setLayerVisibility({ [layer]: !layerVisibility.value[layer] })
  }

  async function setFollowPlane(value: boolean) {
    followPlane.value = value
    await setItem(STORAGE_KEYS.MAP_FOLLOW_PLANE, value)
  }

  async function setSimbriefPilotId(value: string) {
    simbriefPilotId.value = value
    await setItem(STORAGE_KEYS.MAP_SIMBRIEF_PILOT_ID, value)
  }

  async function setWeightUnit(value: 'kg' | 'lbs') {
    weightUnit.value = value
    await setItem(STORAGE_KEYS.MAP_WEIGHT_UNIT, value)
  }

  async function setAirportFilters(next: Partial<MapAirportFilters>) {
    airportFilters.value = {
      ...airportFilters.value,
      ...next,
      minRunwayCount: Math.max(
        0,
        Number(next.minRunwayCount ?? airportFilters.value.minRunwayCount),
      ),
    }
    await setItem(STORAGE_KEYS.MAP_AIRPORT_FILTERS, airportFilters.value)
  }

  async function resetAirportFilters() {
    airportFilters.value = { ...DEFAULT_AIRPORT_FILTERS }
    await setItem(STORAGE_KEYS.MAP_AIRPORT_FILTERS, airportFilters.value)
  }

  function setSelectedAirport(airport: MapAirport | null) {
    selectedAirport.value = airport
  }

  function setPlaneConnection(value: boolean) {
    planeConnected.value = value
  }

  function setPlaneState(value: MapPlaneState | null) {
    planeState.value = value
  }

  const onlineLayerEnabled = computed(
    () => layerVisibility.value.vatsim || layerVisibility.value.weatherRadar,
  )

  return {
    isInitialized,
    mapStyleUrl,
    navRadiusNm,
    vatsimRefreshInterval,
    simbriefPilotId,
    followPlane,
    weightUnit,
    layerVisibility,
    airportFilters,
    selectedAirport,
    planeConnected,
    planeState,
    onlineLayerEnabled,
    initStore,
    setMapStyleUrl,
    setNavRadiusNm,
    setVatsimRefreshInterval,
    setLayerVisibility,
    toggleLayer,
    setFollowPlane,
    setSimbriefPilotId,
    setWeightUnit,
    setAirportFilters,
    resetAirportFilters,
    setSelectedAirport,
    setPlaneConnection,
    setPlaneState,
  }
})
