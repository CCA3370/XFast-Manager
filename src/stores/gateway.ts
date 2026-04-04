import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  gatewayCheckUpdates,
  gatewayGetAirport,
  gatewayGetScenery,
  gatewayInstallScenery,
  gatewayListInstalled,
  gatewaySearchAirports,
  gatewayUninstallAirport,
} from '@/services/gateway-api'
import type {
  GatewayAirportDetail,
  GatewayAirportSearchResult,
  GatewayInstalledAirport,
  GatewaySceneryDetail,
} from '@/types'

export const useGatewayStore = defineStore('gateway', () => {
  const searchQuery = ref('')
  const searchResults = ref<GatewayAirportSearchResult[]>([])
  const installed = ref<GatewayInstalledAirport[]>([])
  const airportDetail = ref<GatewayAirportDetail | null>(null)
  const sceneryDetail = ref<GatewaySceneryDetail | null>(null)
  const selectedSceneryId = ref<number | null>(null)
  const selectedAirportIcao = ref('')

  const isSearching = ref(false)
  const isLoadingAirport = ref(false)
  const isLoadingScenery = ref(false)
  const isLoadingInstalled = ref(false)
  const isCheckingUpdates = ref(false)
  const installingIcao = ref<string | null>(null)
  const uninstallingIcao = ref<string | null>(null)

  let searchSeq = 0
  let airportSeq = 0
  let scenerySeq = 0

  const installedByIcao = computed(() => {
    const map = new Map<string, GatewayInstalledAirport>()
    for (const item of installed.value) {
      map.set(item.airportIcao, item)
    }
    return map
  })

  const selectedInstalledRecord = computed(() =>
    airportDetail.value ? (installedByIcao.value.get(airportDetail.value.icao) ?? null) : null,
  )

  const selectedScenerySummary = computed(
    () =>
      airportDetail.value?.sceneries.find((item) => item.sceneryId === selectedSceneryId.value) ??
      null,
  )

  const updatesCount = computed(
    () => installed.value.filter((item) => item.updateAvailable === true).length,
  )

  async function searchAirports(query: string) {
    const trimmed = query.trim()
    searchQuery.value = query
    const seq = ++searchSeq

    if (!trimmed) {
      searchResults.value = []
      isSearching.value = false
      return
    }

    isSearching.value = true
    try {
      const results = await gatewaySearchAirports(trimmed)
      if (seq !== searchSeq) return
      searchResults.value = results
    } finally {
      if (seq === searchSeq) {
        isSearching.value = false
      }
    }
  }

  function clearSearch() {
    searchSeq += 1
    searchQuery.value = ''
    searchResults.value = []
    isSearching.value = false
  }

  async function loadInstalled(xplanePath: string | null | undefined) {
    if (!xplanePath) {
      installed.value = []
      isLoadingInstalled.value = false
      return
    }

    isLoadingInstalled.value = true
    try {
      installed.value = await gatewayListInstalled(xplanePath)
    } finally {
      isLoadingInstalled.value = false
    }
  }

  async function checkUpdates(xplanePath: string) {
    isCheckingUpdates.value = true
    try {
      installed.value = await gatewayCheckUpdates(xplanePath)
      return installed.value
    } finally {
      isCheckingUpdates.value = false
    }
  }

  function resetAirportSelection() {
    airportSeq += 1
    scenerySeq += 1
    selectedAirportIcao.value = ''
    selectedSceneryId.value = null
    airportDetail.value = null
    sceneryDetail.value = null
    isLoadingAirport.value = false
    isLoadingScenery.value = false
  }

  async function openAirport(icao: string, preferredSceneryId?: number | null) {
    const seq = ++airportSeq
    const normalized = icao.trim().toUpperCase()
    selectedAirportIcao.value = normalized
    isLoadingAirport.value = true
    airportDetail.value = null
    sceneryDetail.value = null
    selectedSceneryId.value = null

    try {
      const detail = await gatewayGetAirport(normalized)
      if (seq !== airportSeq) return

      airportDetail.value = detail
      const installedSceneryId = installedByIcao.value.get(detail.icao)?.sceneryId ?? null
      const nextSceneryId =
        preferredSceneryId ??
        installedSceneryId ??
        detail.recommendedSceneryId ??
        detail.sceneries[0]?.sceneryId ??
        null

      if (nextSceneryId !== null) {
        await selectScenery(nextSceneryId, seq)
      }
    } finally {
      if (seq === airportSeq) {
        isLoadingAirport.value = false
      }
    }
  }

  async function selectScenery(sceneryId: number, expectedAirportSeq?: number) {
    if (!airportDetail.value) return

    selectedSceneryId.value = sceneryId
    const seq = ++scenerySeq
    isLoadingScenery.value = true
    sceneryDetail.value = null

    try {
      const detail = await gatewayGetScenery(sceneryId)
      if (expectedAirportSeq && expectedAirportSeq !== airportSeq) return
      if (seq !== scenerySeq) return
      sceneryDetail.value = detail
    } finally {
      if (seq === scenerySeq && (!expectedAirportSeq || expectedAirportSeq === airportSeq)) {
        isLoadingScenery.value = false
      }
    }
  }

  async function installSelected(
    xplanePath: string,
    autoSortScenery = false,
    ignoreExternalConflict = false,
  ) {
    if (!airportDetail.value || selectedSceneryId.value === null) {
      throw new Error('No Gateway scenery selected')
    }

    installingIcao.value = airportDetail.value.icao
    try {
      const installedRecord = await gatewayInstallScenery({
        xplanePath,
        icao: airportDetail.value.icao,
        sceneryId: selectedSceneryId.value,
        autoSortScenery,
        ignoreExternalConflict,
      })
      await loadInstalled(xplanePath)
      await openAirport(installedRecord.airportIcao, installedRecord.sceneryId)
      return installedRecord
    } finally {
      installingIcao.value = null
    }
  }

  async function uninstallAirportByIcao(xplanePath: string, airportIcao: string) {
    uninstallingIcao.value = airportIcao
    try {
      await gatewayUninstallAirport(xplanePath, airportIcao)
      await loadInstalled(xplanePath)
      if (airportDetail.value?.icao === airportIcao) {
        await openAirport(airportIcao)
      }
    } finally {
      uninstallingIcao.value = null
    }
  }

  return {
    searchQuery,
    searchResults,
    installed,
    airportDetail,
    sceneryDetail,
    selectedSceneryId,
    selectedAirportIcao,
    isSearching,
    isLoadingAirport,
    isLoadingScenery,
    isLoadingInstalled,
    isCheckingUpdates,
    installingIcao,
    uninstallingIcao,
    installedByIcao,
    selectedInstalledRecord,
    selectedScenerySummary,
    updatesCount,
    searchAirports,
    clearSearch,
    loadInstalled,
    checkUpdates,
    openAirport,
    selectScenery,
    installSelected,
    uninstallAirportByIcao,
    resetAirportSelection,
  }
})
