<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6 gap-4">
    <section
      class="relative overflow-hidden rounded-3xl border border-sky-200/80 dark:border-sky-900/40 bg-[radial-gradient(circle_at_top_left,_rgba(14,165,233,0.14),_transparent_42%),linear-gradient(135deg,_rgba(249,250,251,0.96),_rgba(239,246,255,0.96))] dark:bg-[radial-gradient(circle_at_top_left,_rgba(56,189,248,0.14),_transparent_38%),linear-gradient(135deg,_rgba(15,23,42,0.96),_rgba(2,6,23,0.98))] p-5 shadow-sm"
    >
      <div class="absolute inset-y-0 right-0 w-1/2 opacity-30 pointer-events-none">
        <div
          class="absolute inset-0 bg-[linear-gradient(transparent_0%,transparent_47%,rgba(14,165,233,0.14)_48%,transparent_49%,transparent_100%)] bg-[length:100%_24px]"
        ></div>
      </div>
      <div class="relative flex flex-col lg:flex-row lg:items-end lg:justify-between gap-4">
        <div class="max-w-3xl">
          <p class="text-xs font-semibold tracking-[0.28em] uppercase text-sky-600/80 dark:text-sky-300/75">
            {{ $t('airportFlatten.navTitle') }}
          </p>
          <h1 class="mt-2 text-2xl font-bold text-gray-900 dark:text-white">
            {{ $t('airportFlatten.title') }}
          </h1>
          <p class="mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">
            {{ $t('airportFlatten.subtitle') }}
          </p>
        </div>
        <div class="flex flex-wrap gap-2 text-xs">
          <span
            class="inline-flex items-center gap-2 rounded-full border border-white/70 dark:border-white/10 bg-white/75 dark:bg-white/5 px-3 py-1.5 text-gray-600 dark:text-gray-300 backdrop-blur"
          >
            <span class="font-semibold text-sky-600 dark:text-sky-300">{{
              searchResults.length
            }}</span>
            {{ $t('airportFlatten.results') }}
          </span>
          <span
            class="inline-flex items-center gap-2 rounded-full border border-white/70 dark:border-white/10 bg-white/75 dark:bg-white/5 px-3 py-1.5 text-gray-600 dark:text-gray-300 backdrop-blur"
          >
            <span class="font-semibold text-amber-600 dark:text-amber-300">{{
              targets.length
            }}</span>
            {{ $t('airportFlatten.targets') }}
          </span>
        </div>
      </div>
    </section>

    <div
      v-if="!appStore.xplanePath"
      class="rounded-2xl border border-amber-200 dark:border-amber-900/50 bg-amber-50 dark:bg-amber-950/30 px-4 py-3 text-sm text-amber-800 dark:text-amber-200"
    >
      {{ $t('airportFlatten.pathRequiredHint') }}
    </div>

    <section class="flex items-center gap-3">
      <div class="relative flex-1">
        <input
          v-model="searchText"
          type="text"
          :placeholder="$t('airportFlatten.searchPlaceholder')"
          class="w-full rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-4 py-3 pr-10 text-sm text-gray-900 dark:text-white shadow-sm outline-none transition focus:border-sky-400 focus:ring-4 focus:ring-sky-500/10"
          :disabled="!appStore.xplanePath"
        />
        <button
          v-if="searchText"
          class="absolute right-3 top-1/2 -translate-y-1/2 rounded-full p-1 text-gray-400 transition hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-700 dark:hover:text-gray-200"
          @click="clearSearch"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <div v-if="isSearching" class="flex items-center gap-2 text-sm text-sky-600 dark:text-sky-300">
        <div class="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
        {{ $t('common.loading') }}
      </div>
    </section>

    <p class="text-xs text-gray-500 dark:text-gray-400">
      {{ $t('airportFlatten.searchHint') }}
    </p>

    <div class="grid flex-1 min-h-0 grid-cols-1 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] gap-4">
      <section class="min-h-0 rounded-3xl border border-gray-200 dark:border-gray-700 bg-white/90 dark:bg-gray-900/40 p-4 shadow-sm">
        <div class="flex items-center justify-between gap-3 mb-3">
          <h2 class="text-sm font-semibold tracking-wide text-gray-900 dark:text-white uppercase">
            {{ $t('airportFlatten.results') }}
          </h2>
          <span class="text-xs text-gray-500 dark:text-gray-400">{{ searchResults.length }}</span>
        </div>

        <div class="h-full min-h-0 overflow-y-auto pr-1 space-y-2">
          <button
            v-for="airport in searchResults"
            :key="airport.icao"
            class="w-full rounded-2xl border px-4 py-3 text-left transition shadow-sm"
            :class="
              selectedIcao === airport.icao
                ? 'border-sky-400 bg-sky-50 dark:bg-sky-950/30 dark:border-sky-700'
                : 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/40 hover:border-sky-300 dark:hover:border-sky-700 hover:bg-sky-50/40 dark:hover:bg-sky-950/20'
            "
            :disabled="!appStore.xplanePath"
            @click="selectAirport(airport)"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="font-mono text-base font-bold text-gray-900 dark:text-white">
                    {{ airport.icao }}
                  </span>
                  <span
                    v-if="airport.hasDefaultSource"
                    class="rounded-full bg-gray-100 dark:bg-gray-800 px-2 py-0.5 text-[10px] font-medium text-gray-600 dark:text-gray-300"
                  >
                    {{ $t('airportFlatten.defaultSource') }}
                  </span>
                  <span
                    v-if="airport.customSourceCount > 0"
                    class="rounded-full bg-emerald-100 dark:bg-emerald-900/30 px-2 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-300"
                  >
                    {{ $t('airportFlatten.customCount', { count: airport.customSourceCount }) }}
                  </span>
                </div>
                <p class="mt-1 truncate text-sm text-gray-600 dark:text-gray-300">
                  {{ airport.airportName }}
                </p>
              </div>
              <svg
                class="w-4 h-4 mt-1 flex-shrink-0 text-gray-300 dark:text-gray-600"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
            </div>
          </button>

          <div
            v-if="!isSearching && searchText && searchResults.length === 0"
            class="rounded-2xl border border-dashed border-gray-200 dark:border-gray-700 px-4 py-10 text-center text-sm text-gray-500 dark:text-gray-400"
          >
            {{ $t('airportFlatten.searchEmpty') }}
          </div>
        </div>
      </section>

      <section class="min-h-0 rounded-3xl border border-gray-200 dark:border-gray-700 bg-white/90 dark:bg-gray-900/40 p-4 shadow-sm">
        <div class="flex items-start justify-between gap-3 mb-4">
          <div>
            <h2 class="text-sm font-semibold tracking-wide text-gray-900 dark:text-white uppercase">
              {{ $t('airportFlatten.targets') }}
            </h2>
            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
              {{
                selectedIcao
                  ? $t('airportFlatten.targetSubtitle', { icao: selectedIcao })
                  : $t('airportFlatten.noSelection')
              }}
            </p>
          </div>
          <div
            v-if="selectedIcao"
            class="rounded-2xl border border-sky-200 dark:border-sky-800/60 bg-sky-50 dark:bg-sky-950/20 px-3 py-1.5 text-right"
          >
            <div class="font-mono text-sm font-semibold text-sky-700 dark:text-sky-200">
              {{ selectedIcao }}
            </div>
            <div class="text-[11px] text-sky-700/80 dark:text-sky-300/80 max-w-[16rem] truncate">
              {{ selectedAirportName }}
            </div>
          </div>
        </div>

        <div
          v-if="targets.length > 1"
          class="mb-4 rounded-2xl border border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/20 px-4 py-3 text-sm text-amber-800 dark:text-amber-200"
        >
          {{ $t('airportFlatten.multiSourceHint') }}
        </div>

        <div v-if="isLoadingTargets" class="flex h-full items-center justify-center">
          <div class="flex items-center gap-3 text-sm text-sky-600 dark:text-sky-300">
            <div class="w-5 h-5 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
            {{ $t('airportFlatten.loadingTargets') }}
          </div>
        </div>

        <div v-else-if="targets.length > 0" class="h-full min-h-0 overflow-y-auto pr-1 space-y-3">
          <article
            v-for="target in targets"
            :key="targetKey(target)"
            class="rounded-3xl border border-gray-200 dark:border-gray-700 bg-[linear-gradient(135deg,rgba(248,250,252,0.95),rgba(255,255,255,0.98))] dark:bg-[linear-gradient(135deg,rgba(17,24,39,0.8),rgba(2,6,23,0.92))] p-4 shadow-sm"
          >
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span
                    class="rounded-full px-2.5 py-1 text-[11px] font-semibold uppercase tracking-wide"
                    :class="
                      target.sourceKind === 'default'
                        ? 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-200'
                        : 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300'
                    "
                  >
                    {{
                      target.sourceKind === 'default'
                        ? $t('airportFlatten.defaultSource')
                        : $t('airportFlatten.customSource')
                    }}
                  </span>
                  <span
                    class="rounded-full px-2.5 py-1 text-[11px] font-medium"
                    :class="
                      target.flattened
                        ? 'bg-sky-100 text-sky-700 dark:bg-sky-900/30 dark:text-sky-300'
                        : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-300'
                    "
                  >
                    {{
                      target.flattened
                        ? $t('airportFlatten.stateOn')
                        : $t('airportFlatten.stateOff')
                    }}
                  </span>
                </div>
                <h3 class="mt-3 text-base font-semibold text-gray-900 dark:text-white">
                  {{ target.sourceLabel }}
                </h3>
                <p class="mt-1 text-xs leading-5 text-gray-500 dark:text-gray-400 break-all">
                  <span class="font-semibold text-gray-600 dark:text-gray-300">
                    {{ $t('airportFlatten.pathLabel') }}:
                  </span>
                  {{ target.sourcePath }}
                </p>
              </div>

              <button
                class="inline-flex items-center justify-center gap-2 rounded-2xl px-4 py-2 text-sm font-medium text-white shadow-sm transition disabled:opacity-60 disabled:cursor-not-allowed"
                :class="
                  target.flattened
                    ? 'bg-rose-500 hover:bg-rose-600'
                    : 'bg-sky-600 hover:bg-sky-700'
                "
                :disabled="busyTargetKeys.has(targetKey(target))"
                @click="setTargetState(target, !target.flattened)"
              >
                <div
                  v-if="busyTargetKeys.has(targetKey(target))"
                  class="w-4 h-4 border-2 border-white/70 border-t-transparent rounded-full animate-spin"
                ></div>
                <span v-else>{{
                  target.flattened ? $t('airportFlatten.disable') : $t('airportFlatten.enable')
                }}</span>
              </button>
            </div>
          </article>
        </div>

        <div
          v-else
          class="flex h-full items-center justify-center rounded-2xl border border-dashed border-gray-200 dark:border-gray-700 text-sm text-gray-500 dark:text-gray-400 px-6 text-center"
        >
          {{ selectedIcao ? $t('airportFlatten.noTargets') : $t('airportFlatten.noSelection') }}
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import {
  airportFlattenGetTargets,
  airportFlattenSearchAirports,
  airportFlattenSetState,
} from '@/services/airport-flatten-api'
import type { AirportFlattenSearchResult, AirportFlattenTarget } from '@/types'
import { getErrorMessage } from '@/types'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const modalStore = useModalStore()
const toastStore = useToastStore()

const searchText = ref('')
const searchResults = ref<AirportFlattenSearchResult[]>([])
const targets = ref<AirportFlattenTarget[]>([])
const selectedIcao = ref('')
const selectedAirportName = ref('')
const isSearching = ref(false)
const isLoadingTargets = ref(false)
const busyTargetKeys = ref<Set<string>>(new Set())

let searchTimer: ReturnType<typeof setTimeout> | null = null
let searchSeq = 0
let targetSeq = 0

function targetKey(target: AirportFlattenTarget) {
  return `${target.sourceKind}:${target.folderName ?? 'global'}:${target.icao}`
}

function syncRouteIcao(icao: string) {
  const current = typeof route.query.icao === 'string' ? route.query.icao : ''
  if (current === icao) return

  router.replace({
    path: '/airport-flatten',
    query: icao ? { icao } : {},
  })
}

async function runSearch(query: string) {
  const trimmed = query.trim()
  const seq = ++searchSeq

  if (!appStore.xplanePath || !trimmed) {
    searchResults.value = []
    isSearching.value = false
    return
  }

  isSearching.value = true
  try {
    const results = await airportFlattenSearchAirports(appStore.xplanePath, trimmed, 20)
    if (seq !== searchSeq) return
    searchResults.value = results

    const selected = results.find((item) => item.icao === selectedIcao.value)
    if (selected) {
      selectedAirportName.value = selected.airportName
    }
  } catch (error) {
    if (seq === searchSeq) {
      modalStore.showError(`${t('airportFlatten.loadSearchFailed')}: ${getErrorMessage(error)}`)
    }
  } finally {
    if (seq === searchSeq) {
      isSearching.value = false
    }
  }
}

async function loadTargets(icao: string, airportName?: string) {
  const normalized = icao.trim().toUpperCase()
  const seq = ++targetSeq

  selectedIcao.value = normalized
  if (airportName) {
    selectedAirportName.value = airportName
  } else if (!selectedAirportName.value) {
    selectedAirportName.value = normalized
  }

  if (!appStore.xplanePath || !normalized) {
    targets.value = []
    isLoadingTargets.value = false
    return
  }

  isLoadingTargets.value = true
  try {
    const nextTargets = await airportFlattenGetTargets(appStore.xplanePath, normalized)
    if (seq !== targetSeq) return
    targets.value = nextTargets
    if (nextTargets[0]) {
      selectedAirportName.value = nextTargets[0].airportName
    }
  } catch (error) {
    if (seq === targetSeq) {
      targets.value = []
      modalStore.showError(`${t('airportFlatten.loadTargetsFailed')}: ${getErrorMessage(error)}`)
    }
  } finally {
    if (seq === targetSeq) {
      isLoadingTargets.value = false
    }
  }
}

async function selectAirport(airport: AirportFlattenSearchResult) {
  selectedIcao.value = airport.icao
  selectedAirportName.value = airport.airportName
  syncRouteIcao(airport.icao)
  await loadTargets(airport.icao, airport.airportName)
}

async function setTargetState(target: AirportFlattenTarget, enabled: boolean) {
  if (!appStore.xplanePath) {
    modalStore.showError(t('airportFlatten.pathRequiredHint'))
    return
  }

  const key = targetKey(target)
  const nextBusy = new Set(busyTargetKeys.value)
  nextBusy.add(key)
  busyTargetKeys.value = nextBusy

  try {
    const updated = await airportFlattenSetState({
      xplanePath: appStore.xplanePath,
      icao: target.icao,
      sourceKind: target.sourceKind,
      folderName: target.folderName,
      enabled,
    })

    targets.value = targets.value.map((item) => (targetKey(item) === key ? updated : item))
    selectedAirportName.value = updated.airportName
    toastStore.success(
      t(enabled ? 'airportFlatten.enableSuccess' : 'airportFlatten.disableSuccess', {
        icao: updated.icao,
      }),
    )
  } catch (error) {
    modalStore.showError(`${t('airportFlatten.toggleFailed')}: ${getErrorMessage(error)}`)
  } finally {
    const doneBusy = new Set(busyTargetKeys.value)
    doneBusy.delete(key)
    busyTargetKeys.value = doneBusy
  }
}

function clearSearch() {
  searchSeq += 1
  targetSeq += 1
  searchText.value = ''
  searchResults.value = []
  selectedIcao.value = ''
  selectedAirportName.value = ''
  targets.value = []
  isSearching.value = false
  isLoadingTargets.value = false
  syncRouteIcao('')
}

watch(searchText, (value) => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }

  searchTimer = setTimeout(() => {
    void runSearch(value)
  }, 220)
})

watch(
  () => route.query.icao,
  (value) => {
    const icao = typeof value === 'string' ? value.trim().toUpperCase() : ''
    if (!icao || icao === selectedIcao.value) return
    searchText.value = icao
    void runSearch(icao)
    void loadTargets(icao)
  },
)

watch(
  () => appStore.xplanePath,
  (value) => {
    if (!value) {
      searchResults.value = []
      targets.value = []
      selectedIcao.value = ''
      selectedAirportName.value = ''
      return
    }

    if (searchText.value.trim()) {
      void runSearch(searchText.value)
    }
    if (selectedIcao.value) {
      void loadTargets(selectedIcao.value)
    }
  },
)

onMounted(() => {
  const initialIcao = typeof route.query.icao === 'string' ? route.query.icao.trim().toUpperCase() : ''
  if (initialIcao) {
    searchText.value = initialIcao
    void loadTargets(initialIcao)
  }
})

onBeforeUnmount(() => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }
})

const selectedAirport = computed(() =>
  searchResults.value.find((item) => item.icao === selectedIcao.value) ?? null,
)

watch(selectedAirport, (value) => {
  if (value) {
    selectedAirportName.value = value.airportName
  }
})
</script>
