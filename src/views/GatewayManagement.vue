<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6 gap-4">
    <div class="flex flex-col lg:flex-row lg:items-end lg:justify-between gap-3">
      <div>
        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ $t('gatewayManager.title') }}
        </h1>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
          {{ $t('gatewayManager.subtitle') }}
        </p>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <button
          class="text-sm px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors disabled:opacity-50"
          :disabled="!appStore.xplanePath || store.isLoadingInstalled"
          @click="handleReloadInstalled"
        >
          <span v-if="store.isLoadingInstalled">{{ $t('common.loading') }}</span>
          <span v-else>{{ $t('map.gateway.refresh') }}</span>
        </button>
        <button
          class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50"
          :disabled="!appStore.xplanePath || store.isCheckingUpdates || store.installed.length === 0"
          @click="handleCheckUpdates"
        >
          <span v-if="store.isCheckingUpdates">{{ $t('gatewayManager.checkingUpdates') }}</span>
          <span v-else>{{ $t('gatewayManager.checkUpdates') }}</span>
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
      <div class="rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 px-4 py-3">
        <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
          {{ $t('gatewayManager.installedCount') }}
        </div>
        <div class="mt-1 text-2xl font-semibold text-gray-900 dark:text-white">
          {{ store.installed.length }}
        </div>
      </div>
      <div class="rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 px-4 py-3">
        <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
          {{ $t('gatewayManager.updatesAvailable') }}
        </div>
        <div class="mt-1 text-2xl font-semibold text-amber-600 dark:text-amber-400">
          {{ store.updatesCount }}
        </div>
      </div>
      <div class="rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 px-4 py-3">
        <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
          {{ $t('gatewayManager.results') }}
        </div>
        <div class="mt-1 text-2xl font-semibold text-gray-900 dark:text-white">
          {{ store.searchResults.length }}
        </div>
      </div>
    </div>

    <div
      v-if="!appStore.xplanePath"
      class="rounded-2xl border border-amber-200 dark:border-amber-900/50 bg-amber-50 dark:bg-amber-950/30 px-4 py-3 text-sm text-amber-800 dark:text-amber-200"
    >
      {{ $t('gatewayManager.pathRequiredHint') }}
    </div>

    <div class="flex-1 min-h-0 grid grid-cols-1 xl:grid-cols-[360px_minmax(0,1fr)] gap-4">
      <div class="min-h-0 flex flex-col gap-4">
        <section class="rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 p-4 flex flex-col min-h-[240px]">
          <div class="flex items-center justify-between gap-2">
            <div>
              <div class="text-sm font-semibold text-gray-900 dark:text-white">
                {{ $t('gatewayManager.results') }}
              </div>
              <div class="text-xs text-gray-500 dark:text-gray-400">
                {{ $t('gatewayManager.searchHint') }}
              </div>
            </div>
            <div
              v-if="store.isSearching"
              class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
            ></div>
          </div>

          <div class="mt-3 relative">
            <input
              v-model="searchText"
              type="text"
              :placeholder="$t('gatewayManager.searchPlaceholder')"
              class="w-full px-3 py-2 pr-9 rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/40 text-sm text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
            <button
              v-if="searchText"
              class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
              @click="clearSearch"
            >
              &#x2715;
            </button>
          </div>

          <div class="mt-3 flex-1 overflow-y-auto space-y-2">
            <button
              v-for="airport in store.searchResults"
              :key="airport.icao"
              class="w-full text-left rounded-xl border px-3 py-2.5 transition-colors"
              :class="
                store.airportDetail?.icao === airport.icao
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-500/10 dark:border-blue-400'
                  : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50'
              "
              @click="handleSelectAirport(airport.icao)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="text-sm font-semibold text-gray-900 dark:text-white">
                    {{ airport.icao }}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400 truncate">
                    {{ airport.airportName || airport.icao }}
                  </div>
                </div>
                <div class="text-right text-[11px] text-gray-500 dark:text-gray-400">
                  <div>{{ airport.sceneryCount ?? 0 }}</div>
                  <div>{{ $t('map.gateway.submissions', { count: airport.sceneryCount ?? 0 }) }}</div>
                </div>
              </div>
            </button>

            <div
              v-if="!store.isSearching && searchText && store.searchResults.length === 0"
              class="text-sm text-gray-500 dark:text-gray-400 py-6 text-center"
            >
              {{ $t('gatewayManager.searchEmpty') }}
            </div>
          </div>
        </section>

        <section class="rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 p-4 flex flex-col min-h-[280px]">
          <div class="flex items-center justify-between gap-2">
            <div>
              <div class="text-sm font-semibold text-gray-900 dark:text-white">
                {{ $t('gatewayManager.installedTitle') }}
              </div>
              <div class="text-xs text-gray-500 dark:text-gray-400">
                {{ appStore.xplanePath || $t('gatewayManager.pathRequired') }}
              </div>
            </div>
            <span
              class="px-2 py-1 rounded-full bg-gray-100 dark:bg-gray-900/50 text-xs text-gray-600 dark:text-gray-300"
            >
              {{ store.installed.length }}
            </span>
          </div>

          <div class="mt-3 flex-1 overflow-y-auto space-y-2">
            <button
              v-for="airport in store.installed"
              :key="airport.id"
              class="w-full text-left rounded-xl border px-3 py-2.5 transition-colors"
              :class="
                store.airportDetail?.icao === airport.airportIcao
                  ? 'border-emerald-500 bg-emerald-50 dark:bg-emerald-500/10 dark:border-emerald-400'
                  : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50'
              "
              @click="handleSelectAirport(airport.airportIcao, airport.sceneryId)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="text-sm font-semibold text-gray-900 dark:text-white">
                    {{ airport.airportIcao }}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400 truncate">
                    {{ airport.airportName }}
                  </div>
                  <div class="mt-1 text-[11px] text-gray-400 dark:text-gray-500">
                    {{ $t('gatewayManager.localVersion') }} #{{ airport.sceneryId }}
                  </div>
                </div>

                <div class="flex items-center gap-2">
                  <span
                    v-if="airport.updateAvailable === true"
                    class="px-2 py-1 rounded-full bg-amber-100 dark:bg-amber-500/10 text-[11px] text-amber-700 dark:text-amber-300"
                  >
                    {{ $t('gatewayManager.updatesAvailable') }}
                  </span>
                  <button
                    class="px-2 py-1 rounded-lg border border-red-200 dark:border-red-800 text-[11px] text-red-600 dark:text-red-300 hover:bg-red-50 dark:hover:bg-red-950/20 transition-colors disabled:opacity-50"
                    :disabled="store.uninstallingIcao === airport.airportIcao"
                    @click.stop="confirmUninstall(airport)"
                  >
                    {{ $t('csl.uninstall') }}
                  </button>
                </div>
              </div>
            </button>

            <div
              v-if="!store.isLoadingInstalled && store.installed.length === 0"
              class="text-sm text-gray-500 dark:text-gray-400 py-6 text-center"
            >
              {{ appStore.xplanePath ? $t('gatewayManager.installedEmpty') : $t('gatewayManager.installedEmptyHint') }}
            </div>
          </div>
        </section>
      </div>

      <section class="min-h-0 rounded-2xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 p-5 flex flex-col">
        <template v-if="store.airportDetail">
          <div class="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">
                  {{ store.airportDetail.icao }}
                </h2>
                <span
                  v-if="store.selectedInstalledRecord"
                  class="px-2 py-1 rounded-full bg-emerald-100 dark:bg-emerald-500/10 text-xs text-emerald-700 dark:text-emerald-300"
                >
                  {{ $t('gatewayManager.localVersion') }} #{{ store.selectedInstalledRecord.sceneryId }}
                </span>
              </div>
              <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
                {{ store.airportDetail.airportName || store.airportDetail.icao }}
              </p>
              <div class="mt-3 flex flex-wrap items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
                <span
                  class="px-2 py-1 rounded-full bg-gray-100 dark:bg-gray-900/50 text-gray-700 dark:text-gray-300"
                >
                  {{ $t('map.gateway.submissions', { count: store.airportDetail.sceneryCount ?? store.airportDetail.sceneries.length }) }}
                </span>
                <span
                  v-if="store.airportDetail.recommendedSceneryId"
                  class="px-2 py-1 rounded-full bg-blue-100 dark:bg-blue-500/10 text-blue-700 dark:text-blue-300"
                >
                  {{ $t('map.gateway.recommended') }} #{{ store.airportDetail.recommendedSceneryId }}
                </span>
              </div>
            </div>

            <button
              class="self-start text-sm px-4 py-2 rounded-xl bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50"
              :disabled="installDisabled"
              @click="handleInstall"
            >
              <span v-if="store.installingIcao === store.airportDetail.icao">{{ $t('common.loading') }}</span>
              <span v-else>{{ installButtonText }}</span>
            </button>
          </div>

          <div class="mt-5 min-h-0 grid grid-cols-1 lg:grid-cols-[320px_minmax(0,1fr)] gap-4 flex-1">
            <div class="min-h-0 rounded-2xl border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/20 p-3 flex flex-col">
              <div class="text-sm font-semibold text-gray-900 dark:text-white">
                {{ $t('gatewayManager.results') }}
              </div>
              <div class="mt-3 flex-1 overflow-y-auto space-y-2 pr-1">
                <button
                  v-for="scenery in store.airportDetail.sceneries"
                  :key="scenery.sceneryId"
                  class="w-full text-left rounded-xl border px-3 py-2.5 transition-colors"
                  :class="
                    store.selectedSceneryId === scenery.sceneryId
                      ? 'border-blue-500 bg-white dark:bg-blue-500/10 dark:border-blue-400'
                      : 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/20 hover:bg-gray-50 dark:hover:bg-gray-800/40'
                  "
                  @click="handleSelectScenery(scenery.sceneryId)"
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="text-sm font-semibold text-gray-900 dark:text-white">
                        #{{ scenery.sceneryId }}
                      </div>
                      <div class="text-xs text-gray-500 dark:text-gray-400 truncate">
                        {{ scenery.artist || $t('common.unknown') }}
                      </div>
                    </div>
                    <div class="flex flex-col items-end gap-1">
                      <span
                        v-if="scenery.recommended"
                        class="px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-500/10 text-[11px] text-blue-700 dark:text-blue-300"
                      >
                        {{ $t('map.gateway.recommended') }}
                      </span>
                      <span
                        v-if="store.selectedInstalledRecord?.sceneryId === scenery.sceneryId"
                        class="px-2 py-0.5 rounded-full bg-emerald-100 dark:bg-emerald-500/10 text-[11px] text-emerald-700 dark:text-emerald-300"
                      >
                        {{ $t('gatewayManager.localVersion') }}
                      </span>
                    </div>
                  </div>
                  <div class="mt-2 text-[11px] text-gray-500 dark:text-gray-400">
                    {{ scenery.approvedDate ? formatDateTime(scenery.approvedDate) : $t('common.notSet') }}
                  </div>
                </button>

                <div
                  v-if="store.airportDetail.sceneries.length === 0"
                  class="text-sm text-gray-500 dark:text-gray-400 py-8 text-center"
                >
                  {{ $t('gatewayManager.historyEmpty') }}
                </div>
              </div>
            </div>

            <div class="min-h-0 rounded-2xl border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/20 p-4 overflow-y-auto">
              <div v-if="store.isLoadingScenery" class="flex items-center justify-center h-full">
                <div class="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
              </div>

              <template v-else-if="store.sceneryDetail">
                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                  <div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/30 px-4 py-3">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
                      {{ $t('gatewayManager.localVersion') }}
                    </div>
                    <div class="mt-1 text-lg font-semibold text-gray-900 dark:text-white">
                      #{{ store.sceneryDetail.sceneryId }}
                    </div>
                  </div>
                  <div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/30 px-4 py-3">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
                      {{ $t('gatewayManager.folderName') }}
                    </div>
                    <div class="mt-1 text-sm font-medium text-gray-900 dark:text-white break-all">
                      {{ store.selectedInstalledRecord?.folderName || $t('common.notSet') }}
                    </div>
                  </div>
                </div>

                <div class="mt-4 grid grid-cols-1 md:grid-cols-2 gap-3">
                  <div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/30 px-4 py-3">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
                      {{ $t('map.gateway.artist') }}
                    </div>
                    <div class="mt-1 text-sm text-gray-900 dark:text-white">
                      {{ store.sceneryDetail.artist || $t('common.unknown') }}
                    </div>
                  </div>
                  <div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/30 px-4 py-3">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
                      {{ $t('map.gateway.acceptedAt') }}
                    </div>
                    <div class="mt-1 text-sm text-gray-900 dark:text-white">
                      {{ store.sceneryDetail.approvedDate ? formatDateTime(store.sceneryDetail.approvedDate) : $t('common.notSet') }}
                    </div>
                  </div>
                  <div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/30 px-4 py-3">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">
                      {{ $t('map.gateway.status') }}
                    </div>
                    <div class="mt-1 text-sm text-gray-900 dark:text-white">
                      {{ store.sceneryDetail.status || $t('common.unknown') }}
                    </div>
                  </div>
                  <div
                    v-if="store.selectedInstalledRecord?.updateAvailable === true"
                    class="rounded-xl border border-amber-200 dark:border-amber-900/50 bg-amber-50 dark:bg-amber-950/20 px-4 py-3"
                  >
                    <div class="text-xs uppercase tracking-wide text-amber-500 dark:text-amber-300">
                      {{ $t('gatewayManager.updatesAvailable') }}
                    </div>
                    <div class="mt-1 text-sm text-amber-800 dark:text-amber-100">
                      #{{ store.selectedInstalledRecord.latestSceneryId || '?' }}
                    </div>
                  </div>
                </div>

                <div v-if="store.sceneryDetail.features.length > 0" class="mt-4">
                  <div class="text-sm font-semibold text-gray-900 dark:text-white">
                    {{ $t('map.gateway.features') }}
                  </div>
                  <div class="mt-2 flex flex-wrap gap-2">
                    <span
                      v-for="feature in store.sceneryDetail.features"
                      :key="feature"
                      class="px-2.5 py-1 rounded-full bg-gray-200 dark:bg-gray-800 text-xs text-gray-700 dark:text-gray-300"
                    >
                      {{ feature }}
                    </span>
                  </div>
                </div>

                <div v-if="store.sceneryDetail.comment" class="mt-4">
                  <div class="text-sm font-semibold text-gray-900 dark:text-white">
                    {{ $t('map.gateway.comments') }}
                  </div>
                  <div class="mt-2 rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900/30 px-4 py-3 text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap">
                    {{ store.sceneryDetail.comment }}
                  </div>
                </div>
              </template>

              <div
                v-else
                class="h-full flex items-center justify-center text-sm text-gray-500 dark:text-gray-400"
              >
                {{ $t('gatewayManager.noSelection') }}
              </div>
            </div>
          </div>
        </template>

        <div
          v-else
          class="flex-1 flex items-center justify-center text-sm text-gray-500 dark:text-gray-400"
        >
          {{ $t('gatewayManager.noSelection') }}
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { CommandError } from '@/services/api'
import { useGatewayStore } from '@/stores/gateway'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { getErrorMessage, type GatewayInstalledAirport } from '@/types'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const appStore = useAppStore()
const store = useGatewayStore()
const modal = useModalStore()
const toast = useToastStore()

const searchText = ref('')
let searchTimer: ReturnType<typeof setTimeout> | null = null

const installDisabled = computed(() => {
  if (!appStore.xplanePath) return true
  if (!store.airportDetail || store.selectedSceneryId === null) return true
  if (store.installingIcao === store.airportDetail.icao) return true
  return store.selectedInstalledRecord?.sceneryId === store.selectedSceneryId
})

const installButtonText = computed(() => {
  if (!store.airportDetail) return t('common.install')
  if (store.selectedInstalledRecord?.sceneryId === store.selectedSceneryId) {
    return t('gatewayManager.localVersion')
  }
  return t('gatewayManager.installSelected')
})

watch(
  () => appStore.xplanePath,
  async (path) => {
    try {
      await store.loadInstalled(path)
    } catch (error) {
      modal.showError(`${t('gatewayManager.loadInstalledFailed')}: ${getErrorMessage(error)}`)
    }
  },
  { immediate: true },
)

watch(searchText, (value) => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }

  searchTimer = setTimeout(async () => {
    try {
      await store.searchAirports(value)
    } catch (error) {
      toast.error(`${t('gatewayManager.loadAirportFailed')}: ${getErrorMessage(error)}`)
    }
  }, 250)
})

onBeforeUnmount(() => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }
})

function clearSearch() {
  searchText.value = ''
  store.clearSearch()
}

function formatDateTime(value: string): string {
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) {
    return value
  }
  return parsed.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

async function handleSelectAirport(icao: string, preferredSceneryId?: number) {
  try {
    await store.openAirport(icao, preferredSceneryId)
  } catch (error) {
    modal.showError(`${t('gatewayManager.loadAirportFailed')}: ${getErrorMessage(error)}`)
  }
}

async function handleSelectScenery(sceneryId: number) {
  try {
    await store.selectScenery(sceneryId)
  } catch (error) {
    modal.showError(`${t('gatewayManager.loadSceneryFailed')}: ${getErrorMessage(error)}`)
  }
}

async function handleReloadInstalled() {
  if (!appStore.xplanePath) return
  try {
    await store.loadInstalled(appStore.xplanePath)
  } catch (error) {
    modal.showError(`${t('gatewayManager.loadInstalledFailed')}: ${getErrorMessage(error)}`)
  }
}

async function handleCheckUpdates() {
  if (!appStore.xplanePath) {
    modal.showError(t('gatewayManager.pathRequiredHint'), t('gatewayManager.pathRequired'), {
      hideReport: true,
    })
    return
  }

  try {
    await store.checkUpdates(appStore.xplanePath)
  } catch (error) {
    modal.showError(`${t('gatewayManager.updateCheckFailed')}: ${getErrorMessage(error)}`)
  }
}

async function handleInstall() {
  if (!appStore.xplanePath) {
    modal.showError(t('gatewayManager.pathRequiredHint'), t('gatewayManager.pathRequired'), {
      hideReport: true,
    })
    return
  }

  try {
    const installedRecord = await store.installSelected(appStore.xplanePath, appStore.autoSortScenery)
    toast.success(
      t('gatewayManager.installSuccess', {
        icao: installedRecord.airportIcao,
      }),
    )
  } catch (error) {
    if (error instanceof CommandError && error.code === 'conflict_exists') {
      modal.showError(error.message, t('gatewayManager.installBlocked'), { hideReport: true })
      return
    }
    modal.showError(`${t('gatewayManager.installFailed')}: ${getErrorMessage(error)}`)
  }
}

function confirmUninstall(record: GatewayInstalledAirport) {
  modal.showConfirm({
    title: t('csl.uninstall'),
    message: t('gatewayManager.uninstallConfirm', { icao: record.airportIcao }),
    confirmText: t('csl.uninstall'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: async () => {
      if (!appStore.xplanePath) return
      try {
        await store.uninstallAirportByIcao(appStore.xplanePath, record.airportIcao)
        toast.success(t('gatewayManager.uninstallSuccess', { icao: record.airportIcao }))
      } catch (error) {
        modal.showError(`${t('gatewayManager.uninstallFailed')}: ${getErrorMessage(error)}`)
      }
    },
    onCancel: () => {},
  })
}
</script>
