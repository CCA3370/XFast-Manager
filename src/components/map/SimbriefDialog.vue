<script setup lang="ts">
import { ref, computed } from 'vue'
import { useFlightPlanStore, type FlightPlanWaypoint } from '@/stores/flightPlan'
import { useMapStore } from '@/stores/map'

const flightPlanStore = useFlightPlanStore()
const mapStore = useMapStore()

const emit = defineEmits<{
  (e: 'close'): void
}>()

type Tab = 'flight' | 'navlog' | 'performance' | 'fuel' | 'weights'
const activeTab = ref<Tab>('flight')
const expandedFixes = ref<Set<number>>(new Set())

const ofp = computed(() => flightPlanStore.ofp)

function toggleFix(idx: number) {
  if (expandedFixes.value.has(idx)) {
    expandedFixes.value.delete(idx)
  } else {
    expandedFixes.value.add(idx)
  }
}

// Format altitude as FL or feet
function fmtAlt(alt: number): string {
  if (alt >= 10000) return `FL${Math.round(alt / 100)}`
  return `${Math.round(alt)} ft`
}

// Format time in minutes to H:MM
function fmtTime(minutes: number): string {
  if (minutes <= 0) return '-'
  const h = Math.floor(minutes / 60)
  const m = Math.round(minutes % 60)
  return `${h}:${String(m).padStart(2, '0')}`
}

// Weight unit helpers
const isLbs = computed(() => mapStore.weightUnit === 'lbs')
const wUnit = computed(() => isLbs.value ? 'lbs' : 'kg')

function toDisplayWeight(kg: number): number {
  return isLbs.value ? kg * 2.20462 : kg
}

// Format fuel in display unit
function fmtFuel(kg: number): string {
  const val = toDisplayWeight(kg)
  if (val >= 1000) return `${(val / 1000).toFixed(1)}t`
  return `${Math.round(val)} ${wUnit.value}`
}

// Format weight with thousands separator
function fmtWeight(kg: number): string {
  return Math.round(toDisplayWeight(kg)).toLocaleString()
}

// Stage color
function stageColor(stage: string): string {
  switch (stage) {
    case 'CLB': return 'text-emerald-400'
    case 'CRZ': return 'text-cyan-400'
    case 'DSC': return 'text-amber-400'
    default: return 'text-gray-400'
  }
}

// Stage badge bg
function stageBg(stage: string): string {
  switch (stage) {
    case 'CLB': return 'bg-emerald-900/40 text-emerald-300'
    case 'CRZ': return 'bg-cyan-900/40 text-cyan-300'
    case 'DSC': return 'bg-amber-900/40 text-amber-300'
    default: return 'bg-slate-700 text-gray-400'
  }
}

// Wind component (head/tail relative to heading)
function windComponent(wp: FlightPlanWaypoint): string {
  if (!wp.windSpd) return '-'
  return `${Math.round(wp.windDir)}°/${Math.round(wp.windSpd)}kt`
}

// Top of Climb / Top of Descent detection
const tocIndex = computed(() => {
  if (!ofp.value) return -1
  const wps = ofp.value.waypoints
  for (let i = 1; i < wps.length; i++) {
    if (wps[i - 1].stage === 'CLB' && wps[i].stage === 'CRZ') return i
  }
  return -1
})

const todIndex = computed(() => {
  if (!ofp.value) return -1
  const wps = ofp.value.waypoints
  for (let i = 1; i < wps.length; i++) {
    if (wps[i - 1].stage === 'CRZ' && wps[i].stage === 'DSC') return i
  }
  return -1
})

// Total distance
const totalDistance = computed(() => {
  if (!ofp.value) return 0
  return ofp.value.distance
})

// Fuel breakdown for fuel tab
const fuelBreakdown = computed(() => {
  if (!ofp.value) return []
  const o = ofp.value
  const items = [
    { label: 'Block Fuel', value: o.blockFuel, color: 'bg-blue-500' },
    { label: 'Taxi Fuel', value: o.taxiFuel, color: 'bg-slate-500' },
    { label: 'Trip Fuel', value: o.tripFuel, color: 'bg-cyan-500' },
    { label: 'Contingency', value: o.contingencyFuel, color: 'bg-amber-500' },
    { label: 'Alternate', value: o.alternateFuel, color: 'bg-purple-500' },
    { label: 'Reserve', value: o.reserveFuel, color: 'bg-red-500' },
    { label: 'Extra', value: o.extraFuel, color: 'bg-emerald-500' },
  ]
  return items.filter(i => i.value > 0)
})

// Max fuel for percentage bars
const maxFuelValue = computed(() => {
  if (!ofp.value) return 1
  return ofp.value.blockFuel || 1
})

// Vertical profile data
const profileData = computed(() => {
  if (!ofp.value || ofp.value.waypoints.length < 2) return []
  return ofp.value.waypoints.map((wp) => ({
    ident: wp.ident,
    dist: wp.distFromDep,
    alt: wp.altitude,
    stage: wp.stage,
    wind: `${Math.round(wp.windDir)}°/${Math.round(wp.windSpd)}kt`,
    oat: wp.oat,
  }))
})

// SVG vertical profile
const profileSvgWidth = 720
const profileSvgHeight = 180
const profilePadding = { top: 20, right: 20, bottom: 30, left: 50 }

const profilePath = computed(() => {
  const data = profileData.value
  if (data.length < 2) return ''
  const maxDist = data[data.length - 1].dist || 1
  const maxAlt = Math.max(...data.map(d => d.alt), 1000)
  const w = profileSvgWidth - profilePadding.left - profilePadding.right
  const h = profileSvgHeight - profilePadding.top - profilePadding.bottom

  const points = data.map(d => {
    const x = profilePadding.left + (d.dist / maxDist) * w
    const y = profilePadding.top + h - (d.alt / (maxAlt * 1.1)) * h
    return `${x},${y}`
  })

  return `M${points.join(' L')}`
})

const profileFillPath = computed(() => {
  const data = profileData.value
  if (data.length < 2) return ''
  const maxDist = data[data.length - 1].dist || 1
  const maxAlt = Math.max(...data.map(d => d.alt), 1000)
  const w = profileSvgWidth - profilePadding.left - profilePadding.right
  const h = profileSvgHeight - profilePadding.top - profilePadding.bottom
  const baseline = profilePadding.top + h

  const points = data.map(d => {
    const x = profilePadding.left + (d.dist / maxDist) * w
    const y = profilePadding.top + h - (d.alt / (maxAlt * 1.1)) * h
    return `${x},${y}`
  })

  const firstX = profilePadding.left + (data[0].dist / maxDist) * w
  const lastX = profilePadding.left + (data[data.length - 1].dist / maxDist) * w

  return `M${firstX},${baseline} L${points.join(' L')} L${lastX},${baseline} Z`
})

// TOC/TOD x positions for profile markers
const tocX = computed(() => {
  const idx = tocIndex.value
  if (idx < 0 || !ofp.value) return -1
  const data = profileData.value
  if (data.length < 2) return -1
  const maxDist = data[data.length - 1].dist || 1
  const w = profileSvgWidth - profilePadding.left - profilePadding.right
  return profilePadding.left + (data[idx].dist / maxDist) * w
})

const todX = computed(() => {
  const idx = todIndex.value
  if (idx < 0 || !ofp.value) return -1
  const data = profileData.value
  if (data.length < 2) return -1
  const maxDist = data[data.length - 1].dist || 1
  const w = profileSvgWidth - profilePadding.left - profilePadding.right
  return profilePadding.left + (data[idx].dist / maxDist) * w
})

// Y-axis labels
const yAxisLabels = computed(() => {
  const data = profileData.value
  if (data.length < 2) return []
  const maxAlt = Math.max(...data.map(d => d.alt), 1000)
  const ceil = Math.ceil(maxAlt * 1.1 / 10000) * 10000
  const labels: Array<{ value: string; y: number }> = []
  const h = profileSvgHeight - profilePadding.top - profilePadding.bottom
  for (let a = 0; a <= ceil; a += 10000) {
    const y = profilePadding.top + h - (a / ceil) * h
    labels.push({ value: a >= 10000 ? `FL${a / 100}` : `${a}`, y })
  }
  return labels
})
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[900px] max-w-[95vw] max-h-[85vh] rounded-xl border border-gray-700/70 bg-slate-900 shadow-2xl flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-gray-700/70 px-4 py-3">
        <div class="flex items-center gap-3">
          <h2 class="text-lg text-gray-100">SimBrief OFP</h2>
          <template v-if="ofp">
            <span class="font-mono text-sm text-blue-300">{{ ofp.callsign || ofp.flightNumber }}</span>
            <span class="text-sm text-gray-400">{{ ofp.departure }} → {{ ofp.arrival }}</span>
          </template>
        </div>
        <div class="flex items-center gap-2">
          <!-- Weight unit toggle -->
          <button
            class="rounded border border-gray-700 bg-slate-800 px-2 py-1 text-[10px] text-gray-400 hover:text-gray-200 transition-colors"
            @click="mapStore.setWeightUnit(isLbs ? 'kg' : 'lbs')"
          >
            {{ isLbs ? 'LBS' : 'KG' }}
          </button>
          <button class="text-gray-400 hover:text-white" @click="emit('close')">
          <svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
          </svg>
        </button>
      </div>

      <!-- No data state -->
      <div v-if="!ofp" class="flex-1 flex items-center justify-center py-12">
        <div class="text-center text-gray-500">
          <div class="text-lg mb-2">No Flight Plan Loaded</div>
          <div class="text-sm">Fetch a SimBrief flight plan from the sidebar first.</div>
        </div>
      </div>

      <!-- Tabs + Content -->
      <template v-else>
        <!-- Quick stats bar -->
        <div class="flex items-center gap-4 border-b border-gray-700/70 px-4 py-2 text-[11px] text-gray-400 bg-slate-800/50">
          <div>
            <span class="text-gray-500">ETE</span>
            <span class="ml-1 font-mono text-gray-200">{{ ofp.ete || '-' }}</span>
          </div>
          <div>
            <span class="text-gray-500">FL</span>
            <span class="ml-1 font-mono text-gray-200">{{ ofp.cruiseAltitude >= 10000 ? `FL${Math.round(ofp.cruiseAltitude / 100)}` : ofp.cruiseAltitude }}</span>
          </div>
          <div>
            <span class="text-gray-500">Mach</span>
            <span class="ml-1 font-mono text-gray-200">{{ ofp.cruiseMach || '-' }}</span>
          </div>
          <div>
            <span class="text-gray-500">CI</span>
            <span class="ml-1 font-mono text-gray-200">{{ ofp.costIndex }}</span>
          </div>
          <div>
            <span class="text-gray-500">Dist</span>
            <span class="ml-1 font-mono text-gray-200">{{ Math.round(ofp.distance) }} nm</span>
          </div>
          <div v-if="ofp.alternate">
            <span class="text-gray-500">Alt</span>
            <span class="ml-1 font-mono text-gray-200">{{ ofp.alternate }}</span>
          </div>
        </div>

        <!-- Tabs -->
        <div class="flex border-b border-gray-700/70 px-4">
          <button
            v-for="tab in (['flight', 'navlog', 'performance', 'fuel', 'weights'] as const)"
            :key="tab"
            class="px-4 py-2 text-sm transition-colors"
            :class="activeTab === tab ? 'text-blue-300 border-b-2 border-blue-400' : 'text-gray-400 hover:text-gray-200'"
            @click="activeTab = tab"
          >
            {{ tab === 'flight' ? 'Flight' : tab === 'navlog' ? 'Navlog' : tab === 'performance' ? 'Performance' : tab === 'fuel' ? 'Fuel' : 'Weights' }}
          </button>
        </div>

        <!-- Body -->
        <div class="flex-1 overflow-y-auto p-4">

          <!-- Flight Tab -->
          <div v-if="activeTab === 'flight'" class="space-y-4">
            <!-- Route -->
            <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
              <div class="text-[11px] text-gray-500 mb-1">Route</div>
              <div class="font-mono text-[11px] text-gray-200 leading-relaxed break-all">{{ ofp.route || '-' }}</div>
            </div>

            <!-- Vertical Profile -->
            <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
              <div class="text-[11px] text-gray-500 mb-2">Vertical Profile</div>
              <svg
                v-if="profileData.length >= 2"
                :viewBox="`0 0 ${profileSvgWidth} ${profileSvgHeight}`"
                class="w-full h-auto"
                preserveAspectRatio="xMidYMid meet"
              >
                <defs>
                  <linearGradient id="profile-fill-grad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.3" />
                    <stop offset="100%" stop-color="#3b82f6" stop-opacity="0.02" />
                  </linearGradient>
                </defs>
                <!-- Y axis labels -->
                <text
                  v-for="label in yAxisLabels"
                  :key="label.value"
                  :x="profilePadding.left - 6"
                  :y="label.y + 3"
                  text-anchor="end"
                  class="fill-gray-500"
                  font-size="9"
                >{{ label.value }}</text>
                <!-- Y grid lines -->
                <line
                  v-for="label in yAxisLabels"
                  :key="'g' + label.value"
                  :x1="profilePadding.left"
                  :x2="profileSvgWidth - profilePadding.right"
                  :y1="label.y"
                  :y2="label.y"
                  stroke="#334155"
                  stroke-width="0.5"
                  stroke-dasharray="4 4"
                />
                <!-- Fill area -->
                <path :d="profileFillPath" fill="url(#profile-fill-grad)" />
                <!-- Profile line -->
                <path :d="profilePath" fill="none" stroke="#3b82f6" stroke-width="1.5" />
                <!-- TOC marker -->
                <template v-if="tocX > 0">
                  <line
                    :x1="tocX" :x2="tocX"
                    :y1="profilePadding.top" :y2="profileSvgHeight - profilePadding.bottom"
                    stroke="#22c55e" stroke-width="1" stroke-dasharray="4 3"
                  />
                  <text :x="tocX" :y="profilePadding.top - 4" text-anchor="middle" class="fill-emerald-400" font-size="8">T/C</text>
                </template>
                <!-- TOD marker -->
                <template v-if="todX > 0">
                  <line
                    :x1="todX" :x2="todX"
                    :y1="profilePadding.top" :y2="profileSvgHeight - profilePadding.bottom"
                    stroke="#f59e0b" stroke-width="1" stroke-dasharray="4 3"
                  />
                  <text :x="todX" :y="profilePadding.top - 4" text-anchor="middle" class="fill-amber-400" font-size="8">T/D</text>
                </template>
              </svg>
              <div v-else class="text-center text-gray-500 text-sm py-4">No profile data</div>
              <!-- Legend -->
              <div class="flex items-center gap-4 mt-2 text-[10px] text-gray-500">
                <span class="flex items-center gap-1">
                  <span class="inline-block w-3 h-0.5 bg-blue-500"></span> Flight Path
                </span>
                <span class="flex items-center gap-1">
                  <span class="inline-block w-3 h-0.5 bg-emerald-500" style="border-top:1px dashed"></span> T/C
                </span>
                <span class="flex items-center gap-1">
                  <span class="inline-block w-3 h-0.5 bg-amber-500" style="border-top:1px dashed"></span> T/D
                </span>
                <span class="ml-auto">Total: {{ Math.round(totalDistance) }} nm</span>
              </div>
            </div>

            <!-- Summary cards -->
            <div class="grid grid-cols-3 gap-3">
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
                <div class="text-[11px] text-gray-500 mb-1">Fuel Summary</div>
                <div class="text-sm font-mono text-gray-200">{{ fmtWeight(ofp.blockFuel) }} {{ wUnit }}</div>
                <div class="text-[10px] text-gray-500">Block Fuel</div>
                <div class="mt-1 text-sm font-mono text-gray-200">{{ fmtWeight(ofp.tripFuel) }} {{ wUnit }}</div>
                <div class="text-[10px] text-gray-500">Trip Fuel</div>
              </div>
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
                <div class="text-[11px] text-gray-500 mb-1">Weights</div>
                <div class="text-sm font-mono text-gray-200">{{ fmtWeight(ofp.zfw) }} {{ wUnit }}</div>
                <div class="text-[10px] text-gray-500">ZFW</div>
                <div class="mt-1 text-sm font-mono text-gray-200">{{ fmtWeight(ofp.tow) }} {{ wUnit }}</div>
                <div class="text-[10px] text-gray-500">TOW</div>
              </div>
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
                <div class="text-[11px] text-gray-500 mb-1">Runways</div>
                <div class="text-sm font-mono text-gray-200">{{ ofp.takeoffRunway || '-' }}</div>
                <div class="text-[10px] text-gray-500">Takeoff RWY</div>
                <div class="mt-1 text-sm font-mono text-gray-200">{{ ofp.landingRunway || '-' }}</div>
                <div class="text-[10px] text-gray-500">Landing RWY</div>
              </div>
            </div>
          </div>

          <!-- Navlog Tab -->
          <div v-if="activeTab === 'navlog'">
            <!-- Header row -->
            <div class="grid grid-cols-[1fr_80px_90px_80px_80px_80px_40px] gap-1 text-[10px] text-gray-500 border-b border-gray-700/70 pb-1 mb-1 px-1">
              <div>Fix</div>
              <div class="text-right">Altitude</div>
              <div class="text-right">Wind</div>
              <div class="text-right">ETA</div>
              <div class="text-right">Dist</div>
              <div class="text-right">Fuel Rem</div>
              <div></div>
            </div>

            <!-- Waypoint rows -->
            <div class="max-h-[50vh] overflow-y-auto space-y-0.5">
              <div v-for="(wp, idx) in ofp.waypoints" :key="idx">
                <!-- Main row -->
                <button
                  class="w-full grid grid-cols-[1fr_80px_90px_80px_80px_80px_40px] gap-1 items-center text-[11px] rounded px-1 py-1 transition-colors"
                  :class="expandedFixes.has(idx) ? 'bg-slate-800' : 'hover:bg-slate-800/50'"
                  @click="toggleFix(idx)"
                >
                  <div class="flex items-center gap-1.5 min-w-0">
                    <span class="font-mono text-gray-100 truncate">{{ wp.ident }}</span>
                    <span v-if="idx === tocIndex" class="rounded bg-emerald-900/50 px-1 py-0 text-[9px] text-emerald-300 shrink-0">T/C</span>
                    <span v-if="idx === todIndex" class="rounded bg-amber-900/50 px-1 py-0 text-[9px] text-amber-300 shrink-0">T/D</span>
                    <span :class="stageBg(wp.stage)" class="rounded px-1 py-0 text-[9px] shrink-0">{{ wp.stage }}</span>
                  </div>
                  <div class="text-right font-mono" :class="stageColor(wp.stage)">{{ fmtAlt(wp.altitude) }}</div>
                  <div class="text-right font-mono text-gray-400">{{ windComponent(wp) }}</div>
                  <div class="text-right font-mono text-gray-300">{{ fmtTime(wp.timeFromDep) }}</div>
                  <div class="text-right font-mono text-gray-400">{{ Math.round(wp.distFromDep) }} nm</div>
                  <div class="text-right font-mono text-cyan-300">{{ fmtFuel(wp.fuelRemaining) }}</div>
                  <div class="text-center text-gray-500">
                    <svg class="h-3 w-3 inline transition-transform" :class="expandedFixes.has(idx) ? 'rotate-180' : ''" viewBox="0 0 20 20" fill="currentColor">
                      <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                    </svg>
                  </div>
                </button>

                <!-- Expanded details -->
                <div v-if="expandedFixes.has(idx)" class="grid grid-cols-4 gap-2 px-3 py-2 bg-slate-800/80 rounded-b text-[10px] mb-1">
                  <div>
                    <span class="text-gray-500">Position</span>
                    <div class="font-mono text-gray-300">{{ wp.latitude.toFixed(4) }}°, {{ wp.longitude.toFixed(4) }}°</div>
                  </div>
                  <div>
                    <span class="text-gray-500">OAT</span>
                    <div class="font-mono text-gray-300">{{ wp.oat }}°C</div>
                  </div>
                  <div>
                    <span class="text-gray-500">Leg Dist</span>
                    <div class="font-mono text-gray-300">{{ Math.round(wp.distToNext) }} nm</div>
                  </div>
                  <div>
                    <span class="text-gray-500">Fuel Used</span>
                    <div class="font-mono text-gray-300">{{ fmtFuel(wp.fuelUsed) }}</div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Footer -->
            <div class="flex items-center justify-between border-t border-gray-700/70 pt-2 mt-2 text-[10px] text-gray-500">
              <span>{{ ofp.waypoints.length }} waypoints</span>
              <span>Total: {{ Math.round(totalDistance) }} nm</span>
            </div>
          </div>

          <!-- Performance Tab -->
          <div v-if="activeTab === 'performance'" class="space-y-4">
            <div class="grid grid-cols-2 gap-3">
              <!-- Takeoff -->
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
                <div class="text-[11px] text-gray-500 mb-2">Takeoff</div>
                <div class="text-lg font-mono text-gray-200 mb-1">{{ ofp.departure }} / {{ ofp.takeoffRunway || '-' }}</div>
                <div class="grid grid-cols-2 gap-2 text-[11px] mt-3">
                  <div>
                    <span class="text-gray-500">Altitude</span>
                    <div class="font-mono text-gray-200">{{ fmtAlt(ofp.cruiseAltitude) }}</div>
                  </div>
                  <div>
                    <span class="text-gray-500">Cost Index</span>
                    <div class="font-mono text-gray-200">{{ ofp.costIndex }}</div>
                  </div>
                </div>
              </div>

              <!-- Landing -->
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
                <div class="text-[11px] text-gray-500 mb-2">Landing</div>
                <div class="text-lg font-mono text-gray-200 mb-1">{{ ofp.arrival }} / {{ ofp.landingRunway || '-' }}</div>
                <div class="grid grid-cols-2 gap-2 text-[11px] mt-3">
                  <div>
                    <span class="text-gray-500">Landing Weight</span>
                    <div class="font-mono text-gray-200">{{ fmtWeight(ofp.lw) }} {{ wUnit }}</div>
                  </div>
                  <div v-if="ofp.alternate">
                    <span class="text-gray-500">Alternate</span>
                    <div class="font-mono text-gray-200">{{ ofp.alternate }}</div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Cruise performance -->
            <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
              <div class="text-[11px] text-gray-500 mb-2">Cruise Performance</div>
              <div class="grid grid-cols-4 gap-3 text-[11px]">
                <div>
                  <span class="text-gray-500">Initial FL</span>
                  <div class="font-mono text-lg text-blue-300">{{ ofp.cruiseAltitude >= 10000 ? `FL${Math.round(ofp.cruiseAltitude / 100)}` : ofp.cruiseAltitude }}</div>
                </div>
                <div>
                  <span class="text-gray-500">Cost Index</span>
                  <div class="font-mono text-lg text-gray-200">{{ ofp.costIndex }}</div>
                </div>
                <div>
                  <span class="text-gray-500">Cruise Mach</span>
                  <div class="font-mono text-lg text-gray-200">{{ ofp.cruiseMach || '-' }}</div>
                </div>
                <div>
                  <span class="text-gray-500">Distance</span>
                  <div class="font-mono text-lg text-gray-200">{{ Math.round(ofp.distance) }} nm</div>
                </div>
              </div>
            </div>
          </div>

          <!-- Fuel Tab -->
          <div v-if="activeTab === 'fuel'" class="space-y-4">
            <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
              <div class="text-[11px] text-gray-500 mb-3">Fuel Breakdown</div>
              <div class="space-y-2.5">
                <div v-for="item in fuelBreakdown" :key="item.label" class="flex items-center gap-3">
                  <span class="w-24 text-[11px] text-gray-400 truncate">{{ item.label }}</span>
                  <div class="flex-1 h-4 rounded-full bg-slate-700/80 overflow-hidden">
                    <div
                      class="h-full rounded-full transition-all"
                      :class="item.color"
                      :style="{ width: `${Math.min((item.value / maxFuelValue) * 100, 100)}%` }"
                    ></div>
                  </div>
                  <span class="w-20 text-right font-mono text-[11px] text-gray-200">{{ fmtWeight(item.value) }} {{ wUnit }}</span>
                </div>
              </div>
            </div>

            <!-- Fuel summary card -->
            <div class="grid grid-cols-3 gap-3">
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3 text-center">
                <div class="text-[10px] text-gray-500">Block Fuel</div>
                <div class="text-lg font-mono text-blue-300">{{ fmtWeight(ofp.blockFuel) }}</div>
                <div class="text-[10px] text-gray-500">{{ wUnit }}</div>
              </div>
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3 text-center">
                <div class="text-[10px] text-gray-500">Trip Fuel</div>
                <div class="text-lg font-mono text-cyan-300">{{ fmtWeight(ofp.tripFuel) }}</div>
                <div class="text-[10px] text-gray-500">{{ wUnit }}</div>
              </div>
              <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3 text-center">
                <div class="text-[10px] text-gray-500">Reserve + Alt</div>
                <div class="text-lg font-mono text-amber-300">{{ fmtWeight(ofp.reserveFuel + ofp.alternateFuel) }}</div>
                <div class="text-[10px] text-gray-500">{{ wUnit }}</div>
              </div>
            </div>
          </div>

          <!-- Weights Tab -->
          <div v-if="activeTab === 'weights'" class="space-y-4">
            <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
              <div class="text-[11px] text-gray-500 mb-3">Weight Summary</div>
              <div class="space-y-3">
                <!-- ZFW -->
                <div>
                  <div class="flex items-center justify-between text-[11px] mb-1">
                    <span class="text-gray-400">Zero Fuel Weight</span>
                    <span class="font-mono text-gray-200">{{ fmtWeight(ofp.zfw) }} {{ wUnit }}</span>
                  </div>
                  <div class="h-2 rounded-full bg-slate-700/80 overflow-hidden">
                    <div class="h-full rounded-full bg-emerald-500" :style="{ width: ofp.tow > 0 ? `${(ofp.zfw / ofp.tow) * 100}%` : '0%' }"></div>
                  </div>
                </div>
                <!-- TOW -->
                <div>
                  <div class="flex items-center justify-between text-[11px] mb-1">
                    <span class="text-gray-400">Takeoff Weight</span>
                    <span class="font-mono text-gray-200">{{ fmtWeight(ofp.tow) }} {{ wUnit }}</span>
                  </div>
                  <div class="h-2 rounded-full bg-slate-700/80 overflow-hidden">
                    <div class="h-full rounded-full bg-blue-500 w-full"></div>
                  </div>
                </div>
                <!-- LW -->
                <div>
                  <div class="flex items-center justify-between text-[11px] mb-1">
                    <span class="text-gray-400">Landing Weight</span>
                    <span class="font-mono text-gray-200">{{ fmtWeight(ofp.lw) }} {{ wUnit }}</span>
                  </div>
                  <div class="h-2 rounded-full bg-slate-700/80 overflow-hidden">
                    <div class="h-full rounded-full bg-purple-500" :style="{ width: ofp.tow > 0 ? `${(ofp.lw / ofp.tow) * 100}%` : '0%' }"></div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Fuel weight in context -->
            <div class="rounded-lg border border-gray-700/70 bg-slate-800/60 p-3">
              <div class="text-[11px] text-gray-500 mb-2">Weight Composition</div>
              <div class="grid grid-cols-2 gap-3 text-[11px]">
                <div>
                  <span class="text-gray-500">Payload (TOW - ZFW - Fuel)</span>
                  <div class="font-mono text-gray-200">{{ fmtWeight(Math.max(ofp.tow - ofp.zfw - ofp.blockFuel, 0)) }} {{ wUnit }}</div>
                </div>
                <div>
                  <span class="text-gray-500">Block Fuel</span>
                  <div class="font-mono text-gray-200">{{ fmtWeight(ofp.blockFuel) }} {{ wUnit }}</div>
                </div>
                <div>
                  <span class="text-gray-500">Trip Burn</span>
                  <div class="font-mono text-gray-200">{{ fmtWeight(ofp.tripFuel) }} {{ wUnit }}</div>
                </div>
                <div>
                  <span class="text-gray-500">Weight at Landing</span>
                  <div class="font-mono text-gray-200">{{ fmtWeight(ofp.lw) }} {{ wUnit }}</div>
                </div>
              </div>
            </div>
          </div>

        </div>
      </template>

      <!-- Footer -->
      <div class="border-t border-gray-700/70 px-4 py-3 flex items-center justify-end">
        <button
          class="rounded bg-slate-700 px-4 py-1.5 text-sm text-gray-200 hover:bg-slate-600"
          @click="emit('close')"
        >
          Close
        </button>
      </div>
    </div>
  </div>
</template>
