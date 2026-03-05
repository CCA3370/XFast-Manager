import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import type { ScannedAircraft } from '@/types/map'

export const useLaunchStore = defineStore('launch', () => {
  const aircraftList = ref<ScannedAircraft[]>([])
  const isScanning = ref(false)
  const selectedAircraftIndex = ref<number | null>(null)
  const selectedLiveryIndex = ref(0)
  const fuelPercents = ref<number[]>([])
  const payloadWeights = ref<number[]>([])
  const timeHours = ref<number | null>(null)
  const dayOfYear = ref<number | null>(null)
  const weatherPreset = ref('clear')
  const startPosition = ref<string | null>(null)
  const startIsRunway = ref(false)

  const selectedAircraft = computed(() => {
    if (selectedAircraftIndex.value === null) return null
    return aircraftList.value[selectedAircraftIndex.value] || null
  })

  function selectAircraft(index: number) {
    selectedAircraftIndex.value = index
    selectedLiveryIndex.value = 0
    const acf = aircraftList.value[index]
    if (acf) {
      fuelPercents.value = acf.tankRatios.map(() => 50)
      payloadWeights.value = acf.payloadStations.map(() => 0)
    }
  }

  function reset() {
    selectedAircraftIndex.value = null
    selectedLiveryIndex.value = 0
    fuelPercents.value = []
    payloadWeights.value = []
    timeHours.value = null
    dayOfYear.value = null
    weatherPreset.value = 'clear'
    startPosition.value = null
    startIsRunway.value = false
  }

  return {
    aircraftList,
    isScanning,
    selectedAircraftIndex,
    selectedLiveryIndex,
    fuelPercents,
    payloadWeights,
    timeHours,
    dayOfYear,
    weatherPreset,
    startPosition,
    startIsRunway,
    selectedAircraft,
    selectAircraft,
    reset,
  }
})
