<template>
  <div class="h-full flex flex-col px-5 pt-3 pb-5 gap-2 max-w-4xl mx-auto w-full">
    <header
      class="flex items-center justify-between pb-1.5 border-b border-gray-200 dark:border-gray-800"
    >
      <h1 class="text-lg font-bold text-gray-900 dark:text-white">
        {{ $t('airportFlatten.title') }}
      </h1>
      <div class="flex items-center gap-2 text-xs">
        <span
          class="rounded-full bg-gray-100 dark:bg-gray-800 px-2.5 py-0.5 text-gray-600 dark:text-gray-300"
        >
          <span class="font-semibold text-gray-900 dark:text-white">{{
            searchResults.length
          }}</span>
          {{ $t('airportFlatten.results') }}
        </span>
      </div>
    </header>

    <div
      v-if="!appStore.xplanePath"
      class="rounded-lg border border-amber-200 dark:border-amber-900/50 bg-amber-50 dark:bg-amber-950/30 px-3 py-1.5 text-xs text-amber-800 dark:text-amber-200"
    >
      {{ $t('airportFlatten.pathRequiredHint') }}
    </div>

    <section class="flex items-center gap-2 shrink-0">
      <div class="relative flex-1">
        <input
          v-model="searchText"
          type="text"
          :placeholder="$t('airportFlatten.searchPlaceholder')"
          class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 pr-8 text-sm text-gray-900 dark:text-white shadow-sm outline-none transition focus:border-sky-400 focus:ring-4 focus:ring-sky-500/10"
          :disabled="!appStore.xplanePath"
        />
        <button
          v-if="searchText"
          class="absolute right-3 top-1/2 -translate-y-1/2 rounded-full p-1 text-gray-400 transition hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-700 dark:hover:text-gray-200"
          @click="clearSearch"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>
      <div
        v-if="isSearching"
        class="flex items-center gap-2 text-sm text-sky-600 dark:text-sky-300"
      >
        <div
          class="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin"
        ></div>
        {{ $t('common.loading') }}
      </div>
    </section>

    <div class="flex-1 min-h-0 overflow-y-auto pr-2 space-y-2">
      <div
        v-for="airport in searchResults"
        :key="airport.icao"
        class="rounded-lg border transition shadow-sm overflow-hidden"
        :class="
          selectedIcao === airport.icao
            ? 'border-sky-300 bg-sky-50/30 dark:bg-sky-950/10 dark:border-sky-800/80'
            : 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/40 hover:border-sky-300 dark:hover:border-sky-700'
        "
      >
        <button
          class="w-full px-3 py-2 text-left flex items-center justify-between gap-3 focus:outline-none"
          :disabled="!appStore.xplanePath"
          @click="selectAirport(airport)"
        >
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
            <p class="mt-0.5 truncate text-xs text-gray-600 dark:text-gray-300">
              {{ airport.airportName }}
            </p>
          </div>
          <svg
            class="w-4 h-4 flex-shrink-0 text-gray-400 transition-transform duration-200"
            :class="selectedIcao === airport.icao ? 'rotate-180' : ''"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </button>

        <div
          v-if="selectedIcao === airport.icao"
          class="border-t border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-900/50 p-2.5"
        >
          <div
            v-if="isLoadingTargets"
            class="flex items-center justify-center py-3 text-sm text-sky-600 dark:text-sky-300"
          >
            <div
              class="w-5 h-5 border-2 border-current border-t-transparent rounded-full animate-spin mr-3"
            ></div>
            {{ $t('airportFlatten.loadingTargets') }}
          </div>

          <div v-else-if="targets.length > 0" class="space-y-2">
            <div
              v-if="targets.length > 1"
              class="mb-3 rounded-lg border border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/20 px-3 py-2 text-xs text-amber-800 dark:text-amber-200"
            >
              {{ $t('airportFlatten.multiSourceHint') }}
            </div>

            <article
              v-for="target in targets"
              :key="targetKey(target)"
              class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-2.5 shadow-sm"
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
                  <h3 class="mt-2.5 text-sm font-semibold text-gray-900 dark:text-white">
                    {{ target.sourceLabel }}
                  </h3>
                  <p class="mt-1 text-[11px] leading-5 text-gray-500 dark:text-gray-400 break-all">
                    <span class="font-semibold text-gray-600 dark:text-gray-300">
                      {{ $t('airportFlatten.pathLabel') }}:
                    </span>
                    {{ target.sourcePath }}
                  </p>
                </div>

                <button
                  class="inline-flex items-center justify-center gap-2 rounded-xl px-2.5 py-1 text-xs font-medium text-white shadow-sm transition disabled:opacity-60 disabled:cursor-not-allowed"
                  :class="
                    target.flattened
                      ? 'bg-rose-500 hover:bg-rose-600'
                      : 'bg-sky-600 hover:bg-sky-700'
                  "
                  :disabled="busyTargetKeys.has(targetKey(target))"
                  @click.stop="setTargetState(target, !target.flattened)"
                >
                  <div
                    v-if="busyTargetKeys.has(targetKey(target))"
                    class="w-3 h-3 border-2 border-white/70 border-t-transparent rounded-full animate-spin"
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
            class="flex items-center justify-center rounded-xl border border-dashed border-gray-200 dark:border-gray-700 py-6 text-sm text-gray-500 dark:text-gray-400 text-center"
          >
            {{ $t('airportFlatten.noTargets') }}
          </div>
        </div>
      </div>

      <div
        v-if="!isSearching && searchText && searchResults.length === 0"
        class="rounded-2xl border border-dashed border-gray-200 dark:border-gray-700 px-4 py-6 text-center text-sm text-gray-500 dark:text-gray-400"
      >
        {{ $t('airportFlatten.searchEmpty') }}
      </div>
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
  if (selectedIcao.value === airport.icao) {
    selectedIcao.value = ''
    selectedAirportName.value = ''
    targets.value = []
    syncRouteIcao('')
    return
  }

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
  const initialIcao =
    typeof route.query.icao === 'string' ? route.query.icao.trim().toUpperCase() : ''
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

const selectedAirport = computed(
  () => searchResults.value.find((item) => item.icao === selectedIcao.value) ?? null,
)

watch(selectedAirport, (value) => {
  if (value) {
    selectedAirportName.value = value.airportName
  }
})
</script>
