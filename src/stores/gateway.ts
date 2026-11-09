import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { CommandError } from '@/services/api'
import {
  gatewayCheckUpdates,
  gatewayCheckInstallWarning,
  gatewayGetAirport,
  gatewayGetScenery,
  gatewayInstallScenery,
  gatewayListInstalled,
  gatewayResolveReleaseContext,
  gatewaySearchAirports,
  gatewayUninstallAirport,
} from '@/services/gateway-api'
import type {
  GatewayAirportDetail,
  GatewayAirportSearchResult,
  GatewayInstalledAirport,
  GatewayReleaseContext,
  GatewaySceneryDetail,
} from '@/types'

export interface GatewayInstallSelection {
  airportIcao: string
  sceneryId: number
}

function normalizeIcaoKey(value: string): string {
  return value.trim().toUpperCase()
}

export const useGatewayStore = defineStore('gateway', () => {
  const searchQuery = ref('')
  const searchResults = ref<GatewayAirportSearchResult[]>([])
  const installed = ref<GatewayInstalledAirport[]>([])
  const airportDetail = ref<GatewayAirportDetail | null>(null)
  const sceneryDetail = ref<GatewaySceneryDetail | null>(null)
  const selectedSceneryId = ref<number | null>(null)
  const selectedAirportIcao = ref('')
  const releaseContext = ref<GatewayReleaseContext | null>(null)

  const isSearching = ref(false)
  const isLoadingAirport = ref(false)
  const isLoadingScenery = ref(false)
  const isLoadingInstalled = ref(false)
  const isCheckingUpdates = ref(false)
  const isLoadingReleaseContext = ref(false)
  const installingIcao = ref<string | null>(null)
  const uninstallingIcao = ref<string | null>(null)

  let searchSeq = 0
  let airportSeq = 0
  let scenerySeq = 0
  let installedRequestSeq = 0
  let loadInstalledSeq = 0
  let checkUpdatesSeq = 0

  const installedByIcao = computed(() => {
    const map = new Map<string, GatewayInstalledAirport>()
    for (const item of installed.value) {
      map.set(normalizeIcaoKey(item.airportIcao), item)
    }
    return map
  })

  const selectedInstalledRecord = computed(() =>
    airportDetail.value
      ? (installedByIcao.value.get(normalizeIcaoKey(airportDetail.value.icao)) ?? null)
      : null,
  )

  const selectedInstallSelection = computed<GatewayInstallSelection | null>(() => {
    const airportIcao = airportDetail.value?.icao
    const sceneryId = selectedSceneryId.value
    if (!airportIcao || sceneryId === null) return null

    return {
      airportIcao: normalizeIcaoKey(airportIcao),
      sceneryId,
    }
  })

  const selectedScenerySummary = computed(
    () =>
      airportDetail.value?.sceneries.find((item) => item.sceneryId === selectedSceneryId.value) ??
      null,
  )

  const updatesCount = computed(
    () => installed.value.filter((item) => item.updateAvailable === true).length,
  )

  const releaseVersion = computed(() =>
    releaseContext.value?.comparisonAvailable ? releaseContext.value.matchedReleaseVersion : null,
  )

  async function loadReleaseContext(xplanePath: string | null | undefined) {
    if (!xplanePath) {
      releaseContext.value = null
      isLoadingReleaseContext.value = false
      return null
    }

    isLoadingReleaseContext.value = true
    try {
      const context = await gatewayResolveReleaseContext(xplanePath)
      releaseContext.value = context
      return context
    } finally {
      isLoadingReleaseContext.value = false
    }
  }

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
      const results = await gatewaySearchAirports(trimmed, 20, releaseVersion.value)
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
    const requestSeq = ++installedRequestSeq
    const loadingSeq = ++loadInstalledSeq

    if (!xplanePath) {
      installed.value = []
      isLoadingInstalled.value = false
      return installed.value
    }

    isLoadingInstalled.value = true
    try {
      const nextInstalled = await gatewayListInstalled(xplanePath, releaseVersion.value)
      if (requestSeq === installedRequestSeq) {
        installed.value = nextInstalled
      }
      return installed.value
    } finally {
      if (loadingSeq === loadInstalledSeq) {
        isLoadingInstalled.value = false
      }
    }
  }

  async function checkUpdates(xplanePath: string) {
    const requestSeq = ++installedRequestSeq
    const checkingSeq = ++checkUpdatesSeq
    isCheckingUpdates.value = true
    try {
      const nextInstalled = await gatewayCheckUpdates(xplanePath, releaseVersion.value)
      if (requestSeq === installedRequestSeq) {
        installed.value = nextInstalled
      }
      return installed.value
    } finally {
      if (checkingSeq === checkUpdatesSeq) {
        isCheckingUpdates.value = false
      }
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
      const detail = await gatewayGetAirport(normalized, releaseVersion.value)
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

  function gatewaySelectionError(message: string, code: 'validation_failed' | 'conflict_exists') {
    return new CommandError(message, {
      code,
      message,
      reportable: false,
    })
  }

  function upsertInstalledRecord(record: GatewayInstalledAirport) {
    const key = normalizeIcaoKey(record.airportIcao)
    const existing = installed.value.find((item) => normalizeIcaoKey(item.airportIcao) === key)
    const merged = existing ? { ...existing, ...record } : record
    installed.value = [
      ...installed.value.filter((item) => normalizeIcaoKey(item.airportIcao) !== key),
      merged,
    ].sort((left, right) => left.airportIcao.localeCompare(right.airportIcao))
  }

  async function refreshInstalledAfterMutation(xplanePath: string) {
    try {
      await loadInstalled(xplanePath)
    } catch (error) {
      console.warn('Failed to refresh installed Gateway airports after a completed action:', error)
    }
  }

  async function installSelection(
    xplanePath: string,
    selection: GatewayInstallSelection,
    autoSortScenery = false,
    ignoreExternalConflict = false,
  ) {
    const airportIcao = normalizeIcaoKey(selection.airportIcao)
    if (!airportIcao || !Number.isSafeInteger(selection.sceneryId) || selection.sceneryId <= 0) {
      throw gatewaySelectionError('No Gateway scenery selected', 'validation_failed')
    }
    if (installingIcao.value !== null) {
      throw gatewaySelectionError(
        'Another Gateway installation is already in progress',
        'conflict_exists',
      )
    }

    installingIcao.value = airportIcao
    try {
      const installedRecord = await gatewayInstallScenery({
        xplanePath,
        icao: airportIcao,
        sceneryId: selection.sceneryId,
        autoSortScenery,
        ignoreExternalConflict,
      })
      upsertInstalledRecord(installedRecord)
      await refreshInstalledAfterMutation(xplanePath)
      return installedRecord
    } finally {
      installingIcao.value = null
    }
  }

  async function checkInstallWarning(xplanePath: string, airportIcao?: string | null) {
    const icao = airportIcao ?? airportDetail.value?.icao
    if (!icao) {
      throw new Error('No Gateway airport selected')
    }

    return gatewayCheckInstallWarning(xplanePath, icao)
  }

  async function uninstallAirportByIcao(xplanePath: string, airportIcao: string) {
    const normalizedIcao = normalizeIcaoKey(airportIcao)
    uninstallingIcao.value = normalizedIcao
    try {
      await gatewayUninstallAirport(xplanePath, normalizedIcao)
      installed.value = installed.value.filter(
        (item) => normalizeIcaoKey(item.airportIcao) !== normalizedIcao,
      )
      await refreshInstalledAfterMutation(xplanePath)
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
    isLoadingReleaseContext,
    installingIcao,
    uninstallingIcao,
    releaseContext,
    installedByIcao,
    selectedInstalledRecord,
    selectedInstallSelection,
    selectedScenerySummary,
    updatesCount,
    releaseVersion,
    searchAirports,
    clearSearch,
    loadReleaseContext,
    loadInstalled,
    checkUpdates,
    openAirport,
    selectScenery,
    installSelection,
    checkInstallWarning,
    uninstallAirportByIcao,
    resetAirportSelection,
  }
})
