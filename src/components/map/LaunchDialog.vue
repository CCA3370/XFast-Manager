<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { useLaunchStore } from '@/stores/launch'
import { useToastStore } from '@/stores/toast'
import { mapScanAircraft, mapGetAircraftImage, mapLaunchFlight } from '@/services/map-api'

const { t } = useI18n()
const appStore = useAppStore()
const launchStore = useLaunchStore()
const toast = useToastStore()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const searchQuery = ref('')
const isLaunching = ref(false)
const aircraftImages = ref<Map<string, string>>(new Map())
type Tab = 'aircraft' | 'fuel' | 'payload' | 'config'
const activeTab = ref<Tab>('aircraft')

const filteredAircraft = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  if (!q) return launchStore.aircraftList
  return launchStore.aircraftList.filter(
    (a) =>
      a.name.toLowerCase().includes(q) ||
      a.icao.toLowerCase().includes(q) ||
      a.manufacturer.toLowerCase().includes(q) ||
      a.description.toLowerCase().includes(q),
  )
})

const selectedAircraft = computed(() => launchStore.selectedAircraft)

async function scanAircraft() {
  const xpPath = appStore.xplanePath
  if (!xpPath) {
    toast.warning('X-Plane path not set')
    return
  }
  launchStore.isScanning = true
  try {
    const results = await mapScanAircraft(xpPath)
    launchStore.aircraftList = results
  } catch (error) {
    toast.error(`Failed to scan aircraft: ${error}`)
  } finally {
    launchStore.isScanning = false
  }
}

async function loadPreviewImage(path: string) {
  if (aircraftImages.value.has(path)) return
  try {
    const dataUrl = await mapGetAircraftImage(path)
    aircraftImages.value.set(path, dataUrl)
  } catch {
    // Ignore
  }
}

function selectAircraft(index: number) {
  const realIndex = launchStore.aircraftList.indexOf(filteredAircraft.value[index])
  if (realIndex >= 0) {
    launchStore.selectAircraft(realIndex)
    activeTab.value = 'fuel'
    const acf = launchStore.aircraftList[realIndex]
    if (acf.previewImage) {
      loadPreviewImage(acf.previewImage)
    }
  }
}

const totalFuelKg = computed(() => {
  if (!selectedAircraft.value) return 0
  const acf = selectedAircraft.value
  return launchStore.fuelPercents.reduce((sum, pct, i) => {
    const ratio = acf.tankRatios[i] || 0
    const maxKg = (acf.maxFuelLbs * ratio * 0.453592)
    return sum + (maxKg * pct) / 100
  }, 0)
})

const totalPayloadKg = computed(() => {
  return launchStore.payloadWeights.reduce((sum, w) => sum + w, 0)
})

async function launchFlight() {
  if (!selectedAircraft.value || !appStore.xplanePath) return
  const acf = selectedAircraft.value

  isLaunching.value = true
  try {
    // Calculate fuel weights in kg
    const fuelWeightsKg = launchStore.fuelPercents.map((pct, i) => {
      const ratio = acf.tankRatios[i] || 0
      const maxKg = acf.maxFuelLbs * ratio * 0.453592
      return (maxKg * pct) / 100
    })

    await mapLaunchFlight({
      xplanePath: appStore.xplanePath,
      aircraftPath: acf.path,
      liveryFolder: acf.liveries[launchStore.selectedLiveryIndex]?.folder || undefined,
      airportIcao: launchStore.startPosition ? '' : 'KLAX',
      startPosition: launchStore.startPosition || undefined,
      startIsRunway: launchStore.startIsRunway,
      fuelWeightsKg,
      payloadWeightsKg: launchStore.payloadWeights,
      timeHours: launchStore.timeHours || undefined,
      dayOfYear: launchStore.dayOfYear || undefined,
      weatherPreset: launchStore.weatherPreset,
    })
    toast.success('Flight launched!')
    emit('close')
  } catch (error) {
    toast.error(`Launch failed: ${error}`)
  } finally {
    isLaunching.value = false
  }
}

onMounted(() => {
  if (launchStore.aircraftList.length === 0) {
    scanAircraft()
  }
})
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[800px] max-w-[95vw] max-h-[85vh] rounded-xl border border-gray-700/70 bg-slate-900 shadow-2xl flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-gray-700/70 px-4 py-3">
        <h2 class="text-lg text-gray-100">Flight Launcher</h2>
        <button class="text-gray-400 hover:text-white" @click="emit('close')">
          <svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
          </svg>
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-gray-700/70 px-4">
        <button
          v-for="tab in (['aircraft', 'fuel', 'payload', 'config'] as const)"
          :key="tab"
          class="px-4 py-2 text-sm transition-colors"
          :class="activeTab === tab ? 'text-blue-300 border-b-2 border-blue-400' : 'text-gray-400 hover:text-gray-200'"
          @click="activeTab = tab"
        >
          {{ tab === 'aircraft' ? 'Aircraft' : tab === 'fuel' ? 'Fuel' : tab === 'payload' ? 'Payload' : 'Config' }}
        </button>
      </div>

      <!-- Body -->
      <div class="flex-1 overflow-y-auto p-4">
        <!-- Aircraft Tab -->
        <div v-if="activeTab === 'aircraft'">
          <div class="flex items-center gap-2 mb-3">
            <input
              v-model="searchQuery"
              type="text"
              class="flex-1 rounded border border-gray-700 bg-slate-800 px-3 py-1.5 text-sm text-gray-200 placeholder-gray-500 outline-none focus:border-blue-500"
              placeholder="Search aircraft..."
            >
            <button
              class="rounded bg-slate-700 px-3 py-1.5 text-sm text-gray-200 hover:bg-slate-600 disabled:opacity-50"
              :disabled="launchStore.isScanning"
              @click="scanAircraft"
            >
              {{ launchStore.isScanning ? 'Scanning...' : 'Rescan' }}
            </button>
          </div>

          <div v-if="launchStore.isScanning" class="text-center py-8 text-gray-500">
            Scanning X-Plane Aircraft folder...
          </div>
          <div v-else-if="filteredAircraft.length === 0" class="text-center py-8 text-gray-500">
            No aircraft found
          </div>
          <div v-else class="grid gap-1.5 max-h-[50vh] overflow-y-auto pr-1">
            <button
              v-for="(acf, idx) in filteredAircraft"
              :key="acf.path"
              class="flex items-start gap-3 rounded-lg p-2 text-left transition-colors"
              :class="selectedAircraft?.path === acf.path
                ? 'bg-blue-900/30 border border-blue-500/40'
                : 'bg-slate-800/60 hover:bg-slate-700/60 border border-transparent'"
              @click="selectAircraft(idx)"
            >
              <div class="min-w-0 flex-1">
                <div class="flex items-baseline gap-2">
                  <span class="text-sm font-medium text-gray-100">{{ acf.name }}</span>
                  <span v-if="acf.icao" class="font-mono text-[10px] text-cyan-300">{{ acf.icao }}</span>
                </div>
                <div class="text-[11px] text-gray-400 truncate">
                  {{ acf.manufacturer }}{{ acf.studio ? ` · ${acf.studio}` : '' }}
                </div>
                <div class="mt-0.5 flex flex-wrap gap-1 text-[9px]">
                  <span v-if="acf.isHelicopter" class="rounded bg-purple-900/40 px-1 py-0.5 text-purple-300">Heli</span>
                  <span class="rounded bg-slate-700 px-1 py-0.5 text-gray-400">{{ acf.engineCount }}eng</span>
                  <span class="rounded bg-slate-700 px-1 py-0.5 text-gray-400">{{ acf.liveries.length }} liveries</span>
                </div>
              </div>
            </button>
          </div>
        </div>

        <!-- Fuel Tab -->
        <div v-if="activeTab === 'fuel' && selectedAircraft">
          <div class="mb-3 text-sm text-gray-300">
            <span class="text-gray-400">Aircraft:</span> {{ selectedAircraft.name }}
            <span class="ml-2 text-gray-500">Max fuel: {{ Math.round(selectedAircraft.maxFuelLbs) }} lbs</span>
          </div>

          <!-- Livery selection -->
          <div v-if="selectedAircraft.liveries.length > 1" class="mb-4">
            <label class="text-[11px] text-gray-400 block mb-1">Livery</label>
            <select
              :value="launchStore.selectedLiveryIndex"
              class="w-full rounded border border-gray-700 bg-slate-800 px-2 py-1.5 text-sm text-gray-200 outline-none"
              @change="launchStore.selectedLiveryIndex = Number(($event.target as HTMLSelectElement).value)"
            >
              <option v-for="(liv, idx) in selectedAircraft.liveries" :key="liv.folder" :value="idx">
                {{ liv.name }}
              </option>
            </select>
          </div>

          <!-- Fuel sliders -->
          <div class="space-y-3">
            <div v-for="(tankName, i) in selectedAircraft.tankNames" :key="i" class="flex items-center gap-3">
              <span class="w-24 text-[11px] text-gray-400 truncate">{{ tankName }}</span>
              <input
                type="range"
                min="0"
                max="100"
                :value="launchStore.fuelPercents[i] || 0"
                class="flex-1 accent-blue-500"
                @input="launchStore.fuelPercents[i] = Number(($event.target as HTMLInputElement).value)"
              >
              <span class="w-12 text-right text-[11px] text-gray-300">{{ launchStore.fuelPercents[i] || 0 }}%</span>
            </div>
          </div>

          <div class="mt-3 text-[11px] text-gray-400">
            Total fuel: {{ Math.round(totalFuelKg) }} kg ({{ Math.round(totalFuelKg * 2.20462) }} lbs)
          </div>
        </div>

        <!-- Payload Tab -->
        <div v-if="activeTab === 'payload' && selectedAircraft">
          <div class="mb-3 text-sm text-gray-300">
            <span class="text-gray-400">Empty weight:</span> {{ Math.round(selectedAircraft.emptyWeightLbs) }} lbs
            <span class="ml-2 text-gray-400">Max:</span> {{ Math.round(selectedAircraft.maxWeightLbs) }} lbs
          </div>

          <div v-if="selectedAircraft.payloadStations.length === 0" class="text-gray-500 text-sm py-4 text-center">
            No payload stations defined
          </div>
          <div v-else class="space-y-3">
            <div v-for="(station, i) in selectedAircraft.payloadStations" :key="i" class="flex items-center gap-3">
              <span class="w-28 text-[11px] text-gray-400 truncate">{{ station.name }}</span>
              <input
                type="range"
                min="0"
                :max="Math.round(station.maxWeightLbs * 0.453592)"
                :value="launchStore.payloadWeights[i] || 0"
                class="flex-1 accent-emerald-500"
                @input="launchStore.payloadWeights[i] = Number(($event.target as HTMLInputElement).value)"
              >
              <span class="w-16 text-right text-[11px] text-gray-300">{{ launchStore.payloadWeights[i] || 0 }} kg</span>
            </div>
          </div>

          <div class="mt-3 text-[11px] text-gray-400">
            Total payload: {{ Math.round(totalPayloadKg) }} kg ({{ Math.round(totalPayloadKg * 2.20462) }} lbs)
          </div>
        </div>

        <!-- Config Tab -->
        <div v-if="activeTab === 'config'" class="space-y-4">
          <div>
            <label class="text-[11px] text-gray-400 block mb-1">Weather Preset</label>
            <select
              v-model="launchStore.weatherPreset"
              class="w-full rounded border border-gray-700 bg-slate-800 px-2 py-1.5 text-sm text-gray-200 outline-none"
            >
              <option value="real">Real Weather</option>
              <option value="clear">Clear</option>
              <option value="cloudy">Cloudy</option>
              <option value="rainy">Rainy</option>
              <option value="stormy">Stormy</option>
              <option value="snowy">Snowy</option>
              <option value="foggy">Foggy</option>
            </select>
          </div>

          <div>
            <label class="text-[11px] text-gray-400 block mb-1">Time of Day (hours, 0-24)</label>
            <input
              type="number"
              min="0"
              max="24"
              step="0.5"
              :value="launchStore.timeHours ?? ''"
              class="w-full rounded border border-gray-700 bg-slate-800 px-2 py-1.5 text-sm text-gray-200 outline-none"
              placeholder="Default (current time)"
              @input="launchStore.timeHours = ($event.target as HTMLInputElement).value ? Number(($event.target as HTMLInputElement).value) : null"
            >
          </div>
        </div>

        <div v-if="activeTab !== 'aircraft' && !selectedAircraft" class="text-center py-8 text-gray-500">
          Select an aircraft first
        </div>
      </div>

      <!-- Footer -->
      <div class="border-t border-gray-700/70 px-4 py-3 flex items-center justify-between">
        <div class="text-[11px] text-gray-400">
          <template v-if="selectedAircraft">
            {{ selectedAircraft.name }}
            <span v-if="selectedAircraft.liveries[launchStore.selectedLiveryIndex]?.name !== 'Default'" class="text-gray-500">
              · {{ selectedAircraft.liveries[launchStore.selectedLiveryIndex]?.name }}
            </span>
          </template>
          <template v-else>No aircraft selected</template>
        </div>
        <div class="flex gap-2">
          <button
            class="rounded bg-slate-700 px-4 py-1.5 text-sm text-gray-200 hover:bg-slate-600"
            @click="emit('close')"
          >
            Cancel
          </button>
          <button
            class="rounded bg-blue-600 px-4 py-1.5 text-sm text-white hover:bg-blue-500 disabled:opacity-50"
            :disabled="!selectedAircraft || isLaunching"
            @click="launchFlight"
          >
            {{ isLaunching ? 'Launching...' : 'Launch Flight' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
