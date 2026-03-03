export interface MapBounds {
  north: number
  south: number
  east: number
  west: number
}

export type MapAirportType = 'land' | 'seaplane' | 'heliport' | 'unknown'

export interface MapAirport {
  icao: string
  name: string
  lat: number
  lon: number
  airportType: MapAirportType
  isCustom: boolean
  elevation?: number
  runwayCount?: number
  surfaceType?: string
}

export interface MapAirportDetailRunway {
  name: string
  widthM?: number
  surfaceCode?: number
  surfaceType?: string
  shoulderSurfaceCode?: number
  shoulderSurfaceType?: string
  shoulderWidthM?: number
  centerlineLights?: boolean
  edgeLights?: boolean
  end1Name: string
  end1Lat: number
  end1Lon: number
  end1Marking?: number
  end1Lighting?: number
  end1TdzLighting?: boolean
  end1Reil?: number
  end2Name: string
  end2Lat: number
  end2Lon: number
  end2Marking?: number
  end2Lighting?: number
  end2TdzLighting?: boolean
  end2Reil?: number
}

export interface MapAirportDetailHelipad {
  name: string
  lat: number
  lon: number
  heading?: number
  lengthM?: number
  widthM?: number
  surfaceCode?: number
  surfaceType?: string
}

export interface MapAirportDetailGate {
  name: string
  lat: number
  lon: number
  heading?: number
  locationType?: string
  operationType?: string
  widthCode?: string
  airlines: string[]
  isLegacy: boolean
}

export interface MapAirportDetailTower {
  lat: number
  lon: number
  heightM?: number
  name?: string
}

export interface MapAirportDetailBeacon {
  lat: number
  lon: number
  beaconType?: number
  name?: string
}

export interface MapAirportDetailWindsock {
  lat: number
  lon: number
  illuminated: boolean
  name?: string
}

export interface MapAirportDetailSign {
  lat: number
  lon: number
  heading?: number
  size?: number
  text: string
}

export interface MapAirportDetailTaxiway {
  name: string
  fromLat: number
  fromLon: number
  toLat: number
  toLon: number
}

export interface MapAirportDetail {
  icao: string
  name: string
  airportType: string
  isCustom: boolean
  runways: MapAirportDetailRunway[]
  helipads: MapAirportDetailHelipad[]
  gates: MapAirportDetailGate[]
  tower?: MapAirportDetailTower
  beacon?: MapAirportDetailBeacon
  windsocks: MapAirportDetailWindsock[]
  signs: MapAirportDetailSign[]
  taxiways: MapAirportDetailTaxiway[]
}

export interface MapProcedureWaypoint {
  fixId: string
  fixRegion: string
  fixType: string
  pathTerminator: string
}

export interface MapProcedure {
  procedureType: string
  name: string
  runway?: string | null
  transition?: string | null
  waypointCount: number
  waypoints: MapProcedureWaypoint[]
}

export interface MapAirportProcedures {
  icao: string
  sids: MapProcedure[]
  stars: MapProcedure[]
  approaches: MapProcedure[]
}

export interface MapNavaid {
  id: string
  name: string
  lat: number
  lon: number
  navaidType: string
  frequency?: number
}

export interface MapWaypoint {
  id: string
  region?: string
  lat: number
  lon: number
}

export interface MapAirwaySegment {
  name: string
  fromId: string
  toId: string
  fromLat: number
  fromLon: number
  toLat: number
  toLon: number
  isHigh: boolean
  baseFl?: number
  topFl?: number
}

export interface MapIls {
  id: string
  name: string
  lat: number
  lon: number
  course?: number
  airport?: string
  runway?: string
}

export interface MapAirspace {
  name: string
  classCode: string
  upperLimit?: string
  lowerLimit?: string
  coordinates: Array<[number, number]>
}

export interface MapNavSnapshot {
  navaids: MapNavaid[]
  waypoints: MapWaypoint[]
  airways: MapAirwaySegment[]
  ils: MapIls[]
  airspaces: MapAirspace[]
}

export interface MapLayerRequest {
  lat: number
  lon: number
  radiusNm: number
  includeNavaids?: boolean
  includeWaypoints?: boolean
  includeAirways?: boolean
  includeIls?: boolean
  includeAirspaces?: boolean
}

export interface MapDataStatus {
  loaded: boolean
  xplanePath?: string
  airportCount: number
  navaidCount: number
  waypointCount: number
  airwayCount: number
  ilsCount: number
  airspaceCount: number
  lastLoadedMs?: number
}

export interface MapPlaneState {
  latitude: number
  longitude: number
  altitudeMSL?: number
  altitudeAGL?: number
  heading?: number
  groundspeed?: number
  indicatedAirspeed?: number
  verticalSpeed?: number
}

export interface MapPlaneStreamStatus {
  running: boolean
  connected: boolean
  port: number
}

export interface MapVatsimPilot {
  callsign: string
  name?: string
  latitude: number
  longitude: number
  altitude?: number
  groundspeed?: number
  heading?: number
  departure?: string
  arrival?: string
}

export interface RainViewerFrame {
  time: number
  path: string
}

export interface RainViewerManifest {
  host: string
  radar: {
    past: RainViewerFrame[]
    nowcast: RainViewerFrame[]
  }
}

export interface MapLayerVisibility {
  airports: boolean
  navaids: boolean
  waypoints: boolean
  airways: boolean
  ils: boolean
  airspaces: boolean
  plane: boolean
  vatsim: boolean
  weatherRadar: boolean
}

export interface MapAirportFilters {
  showLand: boolean
  showSeaplane: boolean
  showHeliport: boolean
  onlyCustom: boolean
  minRunwayCount: number
}

export interface MapVatsimEventRoute {
  departure?: string
  arrival?: string
}

export interface MapVatsimEvent {
  id: number
  name: string
  startTime: string
  endTime: string
  routes: MapVatsimEventRoute[]
}
