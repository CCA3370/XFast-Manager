import { invokeCommand } from '@/services/api'
import type {
  MapAirport,
  MapAirportDetail,
  MapAirportProcedures,
  MapBounds,
  MapDataStatus,
  MapLayerRequest,
  MapNavSnapshot,
  MapPlaneStreamStatus,
  RainViewerManifest,
  ScannedAircraft,
} from '@/types/map'

export async function mapPrepareDataIndex(xplanePath: string): Promise<MapDataStatus> {
  return invokeCommand<MapDataStatus>('map_prepare_data_index', { xplanePath })
}

export async function mapGetDataStatus(): Promise<MapDataStatus> {
  return invokeCommand<MapDataStatus>('map_get_data_status')
}

export async function mapSearchAirports(
  xplanePath: string,
  query: string,
  limit = 20,
): Promise<MapAirport[]> {
  return invokeCommand<MapAirport[]>('map_search_airports', {
    xplanePath,
    query,
    limit,
  })
}

export async function mapGetAirportsInBounds(
  xplanePath: string,
  bounds: MapBounds,
  limit = 1200,
): Promise<MapAirport[]> {
  return invokeCommand<MapAirport[]>('map_get_airports_in_bounds', {
    xplanePath,
    bounds,
    limit,
  })
}

export async function mapGetAirportDetail(
  xplanePath: string,
  icao: string,
): Promise<MapAirportDetail> {
  return invokeCommand<MapAirportDetail>('map_get_airport_detail', {
    xplanePath,
    icao,
  })
}

export async function mapGetAirportProcedures(
  xplanePath: string,
  icao: string,
): Promise<MapAirportProcedures> {
  return invokeCommand<MapAirportProcedures>('map_get_airport_procedures', {
    xplanePath,
    icao,
  })
}

export async function mapGetNavSnapshot(
  xplanePath: string,
  request: MapLayerRequest,
): Promise<MapNavSnapshot> {
  return invokeCommand<MapNavSnapshot>('map_get_nav_snapshot', {
    xplanePath,
    request,
  })
}

export async function mapFetchMetar(icao: string): Promise<string> {
  return invokeCommand<string>('map_fetch_metar', { icao })
}

export async function mapFetchTaf(icao: string): Promise<string> {
  return invokeCommand<string>('map_fetch_taf', { icao })
}

export async function mapFetchVatsimData(): Promise<Record<string, unknown>> {
  return invokeCommand<Record<string, unknown>>('map_fetch_vatsim_data')
}

export async function mapFetchVatsimEvents(): Promise<Record<string, unknown>> {
  return invokeCommand<Record<string, unknown>>('map_fetch_vatsim_events')
}

export async function mapFetchVatsimMetar(icao: string): Promise<string> {
  return invokeCommand<string>('map_fetch_vatsim_metar', { icao })
}

export async function mapFetchRainviewerManifest(): Promise<RainViewerManifest> {
  return invokeCommand<RainViewerManifest>('map_fetch_rainviewer_manifest')
}

export async function mapFetchSimbriefLatest(pilotId: string): Promise<Record<string, unknown>> {
  return invokeCommand<Record<string, unknown>>('map_fetch_simbrief_latest', { pilotId })
}

export async function mapFetchGatewayAirport(icao: string): Promise<Record<string, unknown>> {
  return invokeCommand<Record<string, unknown>>('map_fetch_gateway_airport', { icao })
}

export async function mapFetchGatewayScenery(sceneryId: number): Promise<Record<string, unknown>> {
  return invokeCommand<Record<string, unknown>>('map_fetch_gateway_scenery', { sceneryId })
}

export async function mapStartPlaneStream(port = 8086): Promise<boolean> {
  return invokeCommand<boolean>('map_start_plane_stream', { port })
}

export async function mapStopPlaneStream(): Promise<boolean> {
  return invokeCommand<boolean>('map_stop_plane_stream')
}

export async function mapGetPlaneStreamStatus(): Promise<MapPlaneStreamStatus> {
  return invokeCommand<MapPlaneStreamStatus>('map_get_plane_stream_status')
}

export async function mapScanAircraft(xplanePath: string): Promise<ScannedAircraft[]> {
  return invokeCommand<ScannedAircraft[]>('map_scan_aircraft', { xplanePath })
}

export async function mapGetAircraftImage(imagePath: string): Promise<string> {
  return invokeCommand<string>('map_get_aircraft_image', { imagePath })
}

export async function mapLaunchFlight(request: {
  xplanePath: string
  aircraftPath: string
  liveryFolder?: string
  airportIcao: string
  startPosition?: string
  startIsRunway: boolean
  fuelWeightsKg: number[]
  payloadWeightsKg: number[]
  timeHours?: number
  dayOfYear?: number
  weatherPreset?: string
}): Promise<boolean> {
  return invokeCommand<boolean>('map_launch_flight', { request })
}

export async function xplaneIsApiAvailable(port?: number): Promise<boolean> {
  return invokeCommand<boolean>('xplane_is_api_available', { port: port ?? null })
}

export async function xplaneGetDataref(
  name: string,
  port?: number,
  index?: number,
): Promise<unknown> {
  return invokeCommand<unknown>('xplane_get_dataref', {
    name,
    port: port ?? null,
    index: index ?? null,
  })
}

export async function xplaneSetDataref(
  name: string,
  value: unknown,
  port?: number,
  index?: number,
): Promise<boolean> {
  return invokeCommand<boolean>('xplane_set_dataref', {
    name,
    value,
    port: port ?? null,
    index: index ?? null,
  })
}

export async function xplaneActivateCommand(
  name: string,
  port?: number,
  duration?: number,
): Promise<boolean> {
  return invokeCommand<boolean>('xplane_activate_command', {
    name,
    port: port ?? null,
    duration: duration ?? null,
  })
}
