import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

export interface FlightPlanWaypoint {
  ident: string
  name: string
  type: string
  latitude: number
  longitude: number
  altitude: number
  stage: string
  windDir: number
  windSpd: number
  oat: number
  distFromDep: number
  distToNext: number
  timeFromDep: number
  fuelRemaining: number
  fuelUsed: number
}

export interface FlightPlanOFP {
  callsign: string
  airline: string
  flightNumber: string
  departure: string
  arrival: string
  alternate: string
  cruiseAltitude: number
  cruiseMach: string
  costIndex: number
  route: string
  distance: number
  ete: string
  // Weights (kg)
  zfw: number
  tow: number
  lw: number
  // Fuel breakdown (kg)
  blockFuel: number
  taxiFuel: number
  tripFuel: number
  contingencyFuel: number
  alternateFuel: number
  reserveFuel: number
  extraFuel: number
  // Performance
  takeoffRunway: string
  landingRunway: string
  // Navlog
  waypoints: FlightPlanWaypoint[]
  // Raw
  rawOFP: Record<string, unknown> | null
}

export const useFlightPlanStore = defineStore('flightPlan', () => {
  const ofp = ref<FlightPlanOFP | null>(null)
  const isLoading = ref(false)

  const hasData = computed(() => ofp.value !== null)

  function parseSimBriefOFP(data: Record<string, unknown>): FlightPlanOFP {
    const asStr = (v: unknown): string => (v != null ? String(v) : '')
    const asNum = (v: unknown): number => {
      const n = Number(v)
      return Number.isFinite(n) ? n : 0
    }
    const asRec = (v: unknown): Record<string, unknown> =>
      v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {}

    const general = asRec(data.general)
    const origin = asRec(data.origin)
    const destination = asRec(data.destination)
    const alternate = asRec(data.alternate)
    const fuel = asRec(data.fuel)
    const weights = asRec(data.weights)
    const atc = asRec(data.atc)
    const times = asRec(data.times)

    // Parse waypoints from navlog
    const navlog = asRec(data.navlog)
    const fixesRaw = Array.isArray(navlog.fix)
      ? navlog.fix
      : Array.isArray(navlog.fixes)
        ? navlog.fixes
        : []
    const waypoints: FlightPlanWaypoint[] = []

    for (const fixItem of fixesRaw) {
      if (!fixItem || typeof fixItem !== 'object') continue
      const fix = fixItem as Record<string, unknown>
      const lat = asNum(fix.pos_lat ?? fix.lat ?? fix.latitude)
      const lon = asNum(fix.pos_long ?? fix.lon ?? fix.longitude)
      if (!Number.isFinite(lat) || !Number.isFinite(lon)) continue

      waypoints.push({
        ident: asStr(fix.ident ?? fix.name ?? fix.fix),
        name: asStr(fix.name ?? fix.ident),
        type: asStr(fix.type ?? fix.fix_type),
        latitude: lat,
        longitude: lon,
        altitude: asNum(fix.altitude_feet ?? fix.altitude ?? fix.plan_alt),
        stage: asStr(fix.stage ?? fix.phase).toUpperCase(),
        windDir: asNum(fix.wind_dir ?? fix.oat_wind_dir),
        windSpd: asNum(fix.wind_spd ?? fix.oat_wind_spd),
        oat: asNum(fix.oat ?? fix.air_temp),
        distFromDep: asNum(fix.distance ?? fix.dist_from_dep ?? fix.cumulative_distance),
        distToNext: asNum(fix.dist_leg ?? fix.dist_to_next ?? fix.leg_distance),
        timeFromDep: asNum(fix.time_total ?? fix.time_from_dep ?? fix.cumulative_time),
        fuelRemaining: asNum(fix.fuel_plan_onboard ?? fix.fuel_remaining),
        fuelUsed: asNum(fix.fuel_totalused ?? fix.fuel_used),
      })
    }

    // Format ETE
    const eteMin = asNum(times.est_time_enroute ?? general.air_time ?? times.flight_time)
    const eteHrs = Math.floor(eteMin / 60)
    const eteMins = Math.round(eteMin % 60)
    const ete = eteMin > 0 ? `${eteHrs}h${String(eteMins).padStart(2, '0')}m` : ''

    return {
      callsign: asStr(atc.callsign ?? general.icao_airline),
      airline: asStr(general.icao_airline),
      flightNumber: asStr(atc.flightplan_id ?? general.flight_number),
      departure: asStr(origin.icao_code ?? origin.icao),
      arrival: asStr(destination.icao_code ?? destination.icao),
      alternate: asStr(alternate.icao_code ?? alternate.icao ?? data.alternate_icao),
      cruiseAltitude: asNum(general.initial_altitude ?? general.cruise_altitude),
      cruiseMach: asStr(general.cruise_mach ?? general.avg_mach),
      costIndex: asNum(general.costindex ?? general.cost_index),
      route: asStr(general.route ?? atc.route),
      distance: asNum(general.air_distance ?? general.route_distance ?? general.gc_distance),
      ete,
      zfw: asNum(weights.est_zfw ?? weights.zfw),
      tow: asNum(weights.est_tow ?? weights.tow),
      lw: asNum(weights.est_ldw ?? weights.ldw ?? weights.landing_weight),
      blockFuel: asNum(fuel.plan_ramp ?? fuel.block_fuel ?? fuel.total),
      taxiFuel: asNum(fuel.taxi ?? fuel.taxi_fuel),
      tripFuel: asNum(fuel.enroute_burn ?? fuel.trip_fuel ?? fuel.route),
      contingencyFuel: asNum(fuel.contingency ?? fuel.cont_fuel),
      alternateFuel: asNum(fuel.alternate_burn ?? fuel.alternate_fuel ?? fuel.altn),
      reserveFuel: asNum(fuel.reserve ?? fuel.final_reserve ?? fuel.rsv),
      extraFuel: asNum(fuel.extra ?? fuel.extra_fuel),
      takeoffRunway: asStr(origin.plan_rwy ?? origin.runway),
      landingRunway: asStr(destination.plan_rwy ?? destination.runway),
      waypoints,
      rawOFP: data,
    }
  }

  function setFromSimBrief(data: Record<string, unknown>) {
    ofp.value = parseSimBriefOFP(data)
  }

  function clear() {
    ofp.value = null
  }

  return {
    ofp,
    isLoading,
    hasData,
    setFromSimBrief,
    clear,
  }
})
