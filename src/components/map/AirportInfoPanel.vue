<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MapAirportDetail, MapAirportDetailGate, MapAirportDetailRunway, MapAirportDetailHelipad } from '@/types/map'

const { t } = useI18n()

const props = defineProps<{
  detail: MapAirportDetail | null
  metarText: string
  tafText: string
  collapsed: boolean
}>()

const emit = defineEmits<{
  (e: 'selectGate', gate: MapAirportDetailGate): void
  (e: 'selectRunwayEnd', runway: MapAirportDetailRunway, end: 'end1' | 'end2'): void
  (e: 'selectHelipad', helipad: MapAirportDetailHelipad): void
  (e: 'toggle-collapse'): void
}>()

type TabKey = 'info' | 'start' | 'procedures'
const activeTab = ref<TabKey>('info')

// Start position sub-tab
type StartSubTab = 'gates' | 'runways' | 'helipads'
const startSubTab = ref<StartSubTab>('gates')
const gateSearch = ref('')
const selectedStartIndex = ref<number | null>(null)
const selectedStartType = ref<string>('')

const tabs = computed<Array<{ key: TabKey; label: string; count?: number }>>(() => {
  const d = props.detail
  if (!d) return []
  return [
    { key: 'info', label: 'Info' },
    { key: 'start', label: 'Start', count: (d.gates?.length || 0) + (d.runways?.length || 0) + (d.helipads?.length || 0) },
  ]
})

const filteredGates = computed(() => {
  if (!props.detail?.gates) return []
  const q = gateSearch.value.toLowerCase().trim()
  if (!q) return props.detail.gates
  return props.detail.gates.filter((g) =>
    g.name.toLowerCase().includes(q)
    || (g.operationType || '').toLowerCase().includes(q)
    || (g.airlines || []).some((a) => a.toLowerCase().includes(q)),
  )
})

const runwayPairs = computed(() => {
  if (!props.detail?.runways) return []
  return props.detail.runways.map((rwy) => ({
    runway: rwy,
    label: `${rwy.end1Name || '?'}/${rwy.end2Name || '?'}`,
    surface: rwy.surfaceType || 'Unknown',
    widthM: rwy.widthM || 0,
  }))
})

function gateWidthLabel(widthCode: string | undefined): string {
  switch (widthCode) {
    case 'A': return 'S'
    case 'B': return 'M'
    case 'C': return 'N'
    case 'D': return 'W'
    case 'E': return 'H'
    case 'F': return 'SH'
    default: return ''
  }
}

function gateOpLabel(op: string | undefined): string {
  if (!op) return ''
  const lower = op.toLowerCase()
  if (lower.includes('airline') || lower === 'airline') return 'AIR'
  if (lower.includes('cargo')) return 'CRG'
  if (lower.includes('ga') || lower.includes('general')) return 'GA'
  if (lower.includes('military') || lower.includes('mil')) return 'MIL'
  return op.substring(0, 3).toUpperCase()
}

function selectGate(gate: MapAirportDetailGate, idx: number) {
  selectedStartIndex.value = idx
  selectedStartType.value = 'gate'
  emit('selectGate', gate)
}

function selectRunwayEnd(runway: MapAirportDetailRunway, end: 'end1' | 'end2', idx: number) {
  selectedStartIndex.value = idx
  selectedStartType.value = `rwy-${end}`
  emit('selectRunwayEnd', runway, end)
}

function selectHelipad(helipad: MapAirportDetailHelipad, idx: number) {
  selectedStartIndex.value = idx
  selectedStartType.value = 'helipad'
  emit('selectHelipad', helipad)
}

watch(() => props.detail?.icao, () => {
  selectedStartIndex.value = null
  selectedStartType.value = ''
  gateSearch.value = ''
  activeTab.value = 'info'
})
</script>

<template>
  <div class="rounded-xl border border-gray-200/50 dark:border-gray-700/70 bg-white/90 dark:bg-slate-900/90 backdrop-blur-md shadow-xl overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between px-3 py-2 border-b border-gray-200/50 dark:border-gray-700/70 cursor-pointer" @click="emit('toggle-collapse')">
      <div v-if="detail" class="min-w-0 flex-1">
        <div class="flex items-baseline gap-2">
          <span class="font-mono text-base text-blue-600 dark:text-blue-300">{{ detail.icao }}</span>
          <span class="text-[10px] text-gray-500">{{ detail.airportType }}</span>
          <span v-if="detail.isCustom" class="text-[10px] text-emerald-400">Custom</span>
        </div>
        <div class="text-sm text-gray-700 dark:text-gray-200 truncate">{{ detail.name }}</div>
      </div>
      <div v-else class="text-sm text-gray-500">{{ t('map.noAirportSelected') }}</div>
      <svg class="h-4 w-4 text-gray-500 transition-transform shrink-0 ml-2" :class="{ 'rotate-180': !collapsed }" viewBox="0 0 20 20" fill="currentColor">
        <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
      </svg>
    </div>

    <!-- Body (collapsible) -->
    <div v-if="!collapsed && detail" class="max-h-[60vh] overflow-y-auto">
      <!-- Tab buttons -->
      <div class="flex border-b border-gray-200/50 dark:border-gray-700/70 px-2 pt-1 gap-1">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="px-3 py-1.5 text-[11px] rounded-t transition-colors"
          :class="activeTab === tab.key
            ? 'bg-gray-100 dark:bg-slate-800 text-blue-600 dark:text-blue-200 border-b-2 border-blue-400'
            : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200'"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
          <span v-if="tab.count != null" class="ml-1 text-gray-500">({{ tab.count }})</span>
        </button>
      </div>

      <!-- Info Tab -->
      <div v-if="activeTab === 'info'" class="p-3 space-y-2 text-xs text-gray-600 dark:text-gray-300">
        <!-- Stats summary -->
        <div class="text-[11px] text-gray-400 flex flex-wrap gap-x-2">
          <span>RWY: {{ detail.runways.length }}</span>
          <span>Gates: {{ detail.gates?.length || 0 }}</span>
          <span>Helipads: {{ detail.helipads?.length || 0 }}</span>
          <span>TWY: {{ detail.taxiways?.length || 0 }}</span>
        </div>

        <!-- Weather -->
        <div class="space-y-1">
          <div class="text-[10px] text-gray-500 uppercase tracking-wide">METAR</div>
          <div class="font-mono text-[11px] leading-relaxed text-gray-700 dark:text-gray-200 break-all">{{ metarText || 'N/A' }}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-1">TAF</div>
          <div class="font-mono text-[11px] leading-relaxed text-gray-400 break-all">{{ tafText || 'N/A' }}</div>
        </div>

        <!-- Runway list -->
        <div v-if="detail.runways.length > 0">
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mb-1">Runways</div>
          <div class="space-y-0.5">
            <div
              v-for="rwy in detail.runways"
              :key="rwy.name"
              class="flex items-center justify-between rounded bg-gray-100 dark:bg-slate-800/60 px-2 py-1"
            >
              <span class="font-mono text-blue-600 dark:text-blue-200">{{ rwy.end1Name || '?' }}/{{ rwy.end2Name || '?' }}</span>
              <div class="flex items-center gap-2 text-[10px] text-gray-400">
                <span v-if="rwy.widthM">{{ Math.round(rwy.widthM) }}m</span>
                <span>{{ rwy.surfaceType || 'Unknown' }}</span>
                <span v-if="rwy.centerlineLights" class="text-yellow-400">CL</span>
                <span v-if="rwy.edgeLights" class="text-yellow-400">EDGE</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Start Position Tab -->
      <div v-if="activeTab === 'start'" class="p-2">
        <!-- Sub-tabs -->
        <div class="flex gap-1 mb-2">
          <button
            v-for="st in ['gates', 'runways', 'helipads'] as const"
            :key="st"
            class="px-2 py-1 rounded text-[10px] transition-colors"
            :class="startSubTab === st
              ? 'bg-gray-200 dark:bg-slate-700 text-gray-900 dark:text-white'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-slate-800'"
            @click="startSubTab = st"
          >
            {{ st === 'gates' ? `Gates (${detail.gates?.length || 0})` : st === 'runways' ? `Runways (${detail.runways.length})` : `Helipads (${detail.helipads?.length || 0})` }}
          </button>
        </div>

        <!-- Gates sub-tab -->
        <template v-if="startSubTab === 'gates'">
          <input
            v-if="(detail.gates?.length || 0) > 8"
            v-model="gateSearch"
            type="text"
            class="w-full mb-2 rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-slate-800 px-2 py-1 text-[11px] text-gray-700 dark:text-gray-200 placeholder-gray-500 outline-none focus:border-blue-500"
            placeholder="Search gates..."
          >
          <div class="max-h-52 space-y-0.5 overflow-y-auto pr-1">
            <button
              v-for="(gate, idx) in filteredGates"
              :key="gate.name"
              class="flex w-full items-center justify-between rounded px-2 py-1.5 text-left transition-colors"
              :class="selectedStartType === 'gate' && selectedStartIndex === idx
                ? 'bg-emerald-100 dark:bg-emerald-900/40 border border-emerald-500/40 text-emerald-700 dark:text-emerald-200'
                : 'bg-gray-100 dark:bg-slate-800/60 hover:bg-gray-200 dark:hover:bg-slate-700/60 text-gray-700 dark:text-gray-300'"
              @click="selectGate(gate, idx)"
            >
              <div class="min-w-0">
                <div class="font-mono text-[11px]">{{ gate.name }}</div>
                <div v-if="gate.airlines?.length" class="text-[9px] text-gray-500 truncate">{{ gate.airlines.join(', ') }}</div>
              </div>
              <div class="flex items-center gap-1 shrink-0">
                <span v-if="gateWidthLabel(gate.widthCode)" class="rounded bg-gray-200 dark:bg-slate-700 px-1 py-0.5 text-[9px] text-gray-600 dark:text-gray-300">{{ gateWidthLabel(gate.widthCode) }}</span>
                <span v-if="gateOpLabel(gate.operationType)" class="rounded bg-gray-200 dark:bg-slate-700 px-1 py-0.5 text-[9px] text-gray-600 dark:text-gray-300">{{ gateOpLabel(gate.operationType) }}</span>
              </div>
            </button>
          </div>
          <div v-if="filteredGates.length === 0" class="text-[11px] text-gray-500 text-center py-2">No gates found</div>
        </template>

        <!-- Runways sub-tab -->
        <template v-if="startSubTab === 'runways'">
          <div class="space-y-1">
            <div
              v-for="(pair, rIdx) in runwayPairs"
              :key="pair.label"
              class="rounded border border-gray-200/50 dark:border-gray-700/70 bg-gray-100 dark:bg-slate-800/60 p-1.5"
            >
              <div class="text-[10px] text-gray-400 mb-1">{{ pair.surface }} · {{ Math.round(pair.widthM) }}m</div>
              <div class="grid grid-cols-2 gap-1">
                <button
                  class="rounded px-2 py-1.5 text-center font-mono text-[11px] transition-colors"
                  :class="selectedStartType === 'rwy-end1' && selectedStartIndex === rIdx
                    ? 'bg-emerald-100 dark:bg-emerald-900/40 border border-emerald-500/40 text-emerald-700 dark:text-emerald-200'
                    : 'bg-gray-200 dark:bg-slate-700/70 hover:bg-gray-300 dark:hover:bg-slate-600/70 text-blue-600 dark:text-blue-200'"
                  @click="selectRunwayEnd(pair.runway, 'end1', rIdx)"
                >
                  {{ pair.runway.end1Name || '?' }}
                </button>
                <button
                  class="rounded px-2 py-1.5 text-center font-mono text-[11px] transition-colors"
                  :class="selectedStartType === 'rwy-end2' && selectedStartIndex === rIdx
                    ? 'bg-emerald-100 dark:bg-emerald-900/40 border border-emerald-500/40 text-emerald-700 dark:text-emerald-200'
                    : 'bg-gray-200 dark:bg-slate-700/70 hover:bg-gray-300 dark:hover:bg-slate-600/70 text-blue-600 dark:text-blue-200'"
                  @click="selectRunwayEnd(pair.runway, 'end2', rIdx)"
                >
                  {{ pair.runway.end2Name || '?' }}
                </button>
              </div>
            </div>
          </div>
        </template>

        <!-- Helipads sub-tab -->
        <template v-if="startSubTab === 'helipads'">
          <div class="space-y-0.5">
            <button
              v-for="(hp, idx) in detail.helipads"
              :key="hp.name"
              class="flex w-full items-center justify-between rounded px-2 py-1.5 text-left transition-colors"
              :class="selectedStartType === 'helipad' && selectedStartIndex === idx
                ? 'bg-emerald-100 dark:bg-emerald-900/40 border border-emerald-500/40 text-emerald-700 dark:text-emerald-200'
                : 'bg-gray-100 dark:bg-slate-800/60 hover:bg-gray-200 dark:hover:bg-slate-700/60 text-gray-700 dark:text-gray-300'"
              @click="selectHelipad(hp, idx)"
            >
              <span class="font-mono text-[11px]">{{ hp.name }}</span>
              <span v-if="hp.widthM || hp.lengthM" class="text-[9px] text-gray-500">
                {{ Math.round(hp.widthM || 0) }}x{{ Math.round(hp.lengthM || hp.widthM || 0) }}m
              </span>
            </button>
          </div>
          <div v-if="detail.helipads.length === 0" class="text-[11px] text-gray-500 text-center py-2">No helipads</div>
        </template>
      </div>
    </div>
  </div>
</template>
