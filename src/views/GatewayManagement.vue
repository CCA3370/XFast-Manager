<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-1">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">
        {{ $t('gatewayManager.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <button
          class="text-sm px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors disabled:opacity-50 flex items-center gap-2"
          :disabled="!appStore.xplanePath || store.isLoadingInstalled"
          @click="handleReloadInstalled"
        >
          <div
            v-if="store.isLoadingInstalled"
            class="w-4 h-4 border-2 border-gray-400 border-t-transparent rounded-full animate-spin"
          ></div>
          {{ store.isLoadingInstalled ? $t('common.loading') : $t('map.gateway.refresh') }}
        </button>
        <button
          class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-2"
          :disabled="
            !appStore.xplanePath || store.isCheckingUpdates || store.installed.length === 0
          "
          @click="handleCheckUpdates"
        >
          <span v-if="store.isCheckingUpdates">{{ $t('gatewayManager.checkingUpdates') }}</span>
          <span v-else>{{ $t('gatewayManager.checkUpdates') }}</span>
        </button>
      </div>
    </div>

    <!-- Stats -->
    <div class="flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400 mb-3">
      <span
        >{{ $t('gatewayManager.installedCount') }}:
        <strong class="text-gray-900 dark:text-white">{{ store.installed.length }}</strong></span
      >
      <span
        >{{ $t('gatewayManager.updatesAvailable') }}:
        <strong class="text-amber-600 dark:text-amber-400">{{ store.updatesCount }}</strong></span
      >
      <span
        >{{ $t('gatewayManager.results') }}:
        <strong class="text-gray-900 dark:text-white">{{
          store.searchResults.length
        }}</strong></span
      >
    </div>

    <div
      v-if="!appStore.xplanePath"
      class="rounded-xl border border-amber-200 dark:border-amber-900/50 bg-amber-50 dark:bg-amber-950/30 px-4 py-3 text-sm text-amber-800 dark:text-amber-200 mb-3"
    >
      {{ $t('gatewayManager.pathRequiredHint') }}
    </div>

    <!-- Search bar -->
    <div class="flex items-center gap-2 mb-3">
      <div class="flex-1 relative">
        <input
          v-model="searchText"
          type="text"
          :placeholder="$t('gatewayManager.searchPlaceholder')"
          class="w-full px-3 py-1.5 pr-8 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
        />
        <button
          v-if="searchText"
          class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-sm"
          @click="clearSearch"
        >
          &#x2715;
        </button>
      </div>
    </div>

    <!-- Filter tabs -->
    <div class="flex items-center gap-1 mb-3">
      <button
        class="px-3 py-1.5 rounded-lg text-sm transition-colors"
        :class="
          activeTab === 'installed'
            ? 'bg-blue-600 text-white'
            : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
        "
        @click="activeTab = 'installed'"
      >
        {{ $t('gatewayManager.installedTitle') }}
        <span class="ml-1 text-xs opacity-70">({{ filteredInstalled.length }})</span>
      </button>
      <button
        class="px-3 py-1.5 rounded-lg text-sm transition-colors"
        :class="
          activeTab === 'search'
            ? 'bg-blue-600 text-white'
            : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
        "
        @click="activeTab = 'search'"
      >
        {{ $t('gatewayManager.results') }}
        <span class="ml-1 text-xs opacity-70">({{ store.searchResults.length }})</span>
      </button>
      <div v-if="store.isSearching && activeTab === 'search'" class="ml-2">
        <div
          class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
        ></div>
      </div>
    </div>

    <!-- Main Content -->
    <div
      class="flex-1 min-h-0 bg-gray-50/50 dark:bg-gray-900/20 rounded-xl border border-gray-200 dark:border-gray-700 p-4 overflow-y-auto"
    >
      <!-- Installed Tab -->
      <div
        v-if="activeTab === 'installed'"
        class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-3"
      >
        <div
          v-for="airport in filteredInstalled"
          :key="airport.id"
          class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 p-3 transition-colors hover:border-blue-300 dark:hover:border-blue-700/50 hover:bg-blue-50/30 dark:hover:bg-blue-900/10 cursor-pointer flex flex-col gap-2 shadow-sm hover:shadow-md"
          @click="openAirportModal(airport.airportIcao, airport.sceneryId)"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="min-w-0 flex flex-1 items-center gap-2 overflow-hidden">
              <span class="font-bold text-gray-900 dark:text-white text-base flex-shrink-0">{{
                airport.airportIcao
              }}</span>
              <div class="min-w-0 truncate text-[13px] text-gray-500 dark:text-gray-400">
                {{ airport.airportName }}
              </div>
              <span
                v-if="airport.updateAvailable === true"
                class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300"
                >{{ $t('gatewayManager.updatesAvailable') }}</span
              >
            </div>
            <div
              class="text-[10px] font-medium text-gray-400 dark:text-gray-500 flex-shrink-0 ml-2 bg-gray-100 dark:bg-gray-800 px-1.5 py-0.5 rounded"
            >
              #{{ airport.sceneryId }}
            </div>
          </div>
          <div
            class="flex justify-between items-center mt-auto pt-2 border-t border-gray-100 dark:border-gray-700/50"
          >
            <button
              class="text-[11px] px-2.5 py-1 rounded-md border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors shadow-sm"
              @click.stop="openAirportModal(airport.airportIcao, airport.sceneryId)"
            >
              {{ $t('gatewayManager.details') }}
            </button>
            <button
              class="text-[11px] px-2.5 py-1 rounded-md text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors disabled:opacity-50"
              :disabled="store.uninstallingIcao === airport.airportIcao"
              @click.stop="confirmUninstall(airport)"
            >
              {{ $t('csl.uninstall') }}
            </button>
          </div>
        </div>

        <div
          v-if="!store.isLoadingInstalled && filteredInstalled.length === 0"
          class="col-span-full text-sm text-gray-400 dark:text-gray-500 py-12 text-center"
        >
          <template v-if="searchText">
            {{ $t('gatewayManager.searchEmpty') }}
          </template>
          <template v-else>
            {{
              appStore.xplanePath
                ? $t('gatewayManager.installedEmpty')
                : $t('gatewayManager.installedEmptyHint')
            }}
          </template>
        </div>
      </div>

      <!-- Search Tab -->
      <div
        v-if="activeTab === 'search'"
        class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-3"
      >
        <div
          v-for="airport in store.searchResults"
          :key="airport.icao"
          class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 p-4 transition-colors hover:border-blue-300 dark:hover:border-blue-700/50 hover:bg-blue-50/30 dark:hover:bg-blue-900/10 cursor-pointer flex flex-col gap-3 shadow-sm hover:shadow-md"
          @click="openAirportModal(airport.icao)"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="min-w-0 flex flex-1 items-center gap-2 overflow-hidden">
              <span class="font-bold text-gray-900 dark:text-white text-lg flex-shrink-0">{{
                airport.icao
              }}</span>
              <div class="min-w-0 truncate text-sm text-gray-500 dark:text-gray-400">
                {{ airport.airportName || airport.icao }}
              </div>
            </div>
            <div
              class="text-[11px] text-gray-500 dark:text-gray-400 text-right flex-shrink-0 ml-2 bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded-md"
            >
              <div class="font-bold text-gray-700 dark:text-gray-300 text-xs">
                {{ airport.sceneryCount ?? 0 }}
              </div>
              <div>
                {{
                  $t('map.gateway.submissions', { count: airport.sceneryCount ?? 0 })
                    .replace((airport.sceneryCount ?? 0).toString(), '')
                    .trim()
                }}
              </div>
            </div>
          </div>
          <div
            class="flex justify-start items-center mt-auto pt-3 border-t border-gray-100 dark:border-gray-700/50"
          >
            <button
              class="text-xs px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors shadow-sm"
              @click.stop="openAirportModal(airport.icao)"
            >
              {{ $t('gatewayManager.details') }}
            </button>
          </div>
        </div>

        <div
          v-if="!store.isSearching && store.searchResults.length === 0"
          class="col-span-full text-sm text-gray-400 dark:text-gray-500 py-12 text-center"
        >
          <template v-if="searchText">{{ $t('gatewayManager.searchEmpty') }}</template>
          <template v-else>{{ $t('gatewayManager.searchHint') }}</template>
        </div>
      </div>
    </div>

    <!-- Airport Details Modal -->
    <Teleport to="body">
      <div
        v-if="showAirportModal"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 sm:p-6"
        @click.self="closeAirportModal"
      >
        <div
          class="bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-full max-w-5xl overflow-hidden flex flex-col max-h-full"
        >
          <!-- Modal Header -->
          <div
            class="flex items-center justify-between px-5 py-3 border-b border-gray-200 dark:border-gray-800"
          >
            <div class="flex items-start gap-3 min-w-0">
              <template v-if="store.airportDetail">
                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-2">
                    <h2 class="text-lg font-bold text-gray-900 dark:text-white">
                      {{ store.airportDetail.icao }}
                    </h2>
                    <span
                      v-if="store.selectedInstalledRecord"
                      class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300"
                    >
                      {{ $t('gatewayManager.localVersion') }} #{{
                        store.selectedInstalledRecord.sceneryId
                      }}
                    </span>
                    <span
                      v-if="store.airportDetail.recommendedSceneryId"
                      class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300"
                    >
                      {{ $t('map.gateway.recommended') }} #{{
                        store.airportDetail.recommendedSceneryId
                      }}
                    </span>
                  </div>
                  <div class="min-w-0 text-sm text-gray-700 dark:text-gray-300 font-medium truncate">
                    {{ selectedAirportName }}
                  </div>
                  <div class="text-[11px] text-gray-500 dark:text-gray-400 mt-0.5">
                    {{ $t('map.gateway.submissions', { count: selectedSubmissionCount }) }}
                  </div>
                </div>
              </template>
              <h2 v-else class="text-lg font-bold text-gray-900 dark:text-white">
                {{ $t('common.loading') }}...
              </h2>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-base h-7 w-7 flex items-center justify-center rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              @click="closeAirportModal"
            >
              &#x2715;
            </button>
          </div>

          <div class="flex-1 min-h-0 flex flex-col p-4 sm:p-5 bg-gray-50/50 dark:bg-gray-900/20">
            <div v-if="!store.airportDetail" class="h-full flex items-center justify-center">
              <div
                class="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
              ></div>
            </div>
            <template v-else>
              <!-- Action Bar -->
              <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5 mb-3">
                <div
                  class="min-w-0 flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px] text-gray-600 dark:text-gray-300"
                >
                  <span class="font-semibold text-gray-900 dark:text-white">
                    {{ $t('management.currentVersionLabel') }}
                    {{ formatGatewayVersion(store.selectedSceneryId) }}
                  </span>
                  <span>
                    {{ $t('gatewayManager.localVersion') }}
                    {{ selectedInstalledVersionLabel }}
                  </span>
                  <span>
                    {{ $t('map.gateway.recommended') }}
                    {{ formatGatewayVersion(store.airportDetail.recommendedSceneryId) }}
                  </span>
                  <span
                    v-if="store.selectedInstalledRecord?.updateAvailable === true"
                    class="text-amber-700 dark:text-amber-300"
                  >
                    {{ $t('gatewayManager.updatesAvailable') }}
                    {{ formatGatewayVersion(store.selectedInstalledRecord.latestSceneryId) }}
                  </span>
                </div>
                <button
                  class="text-[13px] px-3.5 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center justify-center gap-1.5 flex-shrink-0 shadow-sm shadow-blue-600/20"
                  :disabled="installDisabled"
                  @click="handleInstall()"
                >
                  <div
                    v-if="store.installingIcao === store.airportDetail.icao"
                    class="w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin"
                  ></div>
                  {{
                    store.installingIcao === store.airportDetail.icao
                      ? $t('common.loading')
                      : installButtonText
                  }}
                </button>
              </div>

              <!-- Two Column Grid inside Modal -->
              <div
                class="min-h-0 flex-1 grid grid-cols-[minmax(190px,36%)_minmax(0,1fr)] sm:grid-cols-[260px_minmax(0,1fr)] gap-3 sm:gap-4"
              >
                <!-- Scenery List -->
                <div
                  class="min-h-0 flex flex-col bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 py-2.5 shadow-sm"
                >
                  <div
                    class="px-3 pb-2 text-[11px] font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b border-gray-100 dark:border-gray-700/50 mb-1.5"
                  >
                    {{
                      $t('map.gateway.submissions', {
                        count:
                          store.airportDetail.sceneryCount ?? store.airportDetail.sceneries.length,
                      })
                    }}
                  </div>
                  <div class="flex-1 overflow-y-auto px-1.5 space-y-1">
                    <button
                      v-for="scenery in store.airportDetail.sceneries"
                      :key="scenery.sceneryId"
                      class="w-full text-left rounded-lg border px-2.5 py-2 transition-colors flex flex-col gap-1"
                      :class="
                        store.selectedSceneryId === scenery.sceneryId
                          ? 'border-blue-500 bg-blue-50/50 dark:bg-blue-500/10 dark:border-blue-400 shadow-sm'
                          : 'border-transparent hover:border-gray-200 dark:hover:border-gray-700 bg-transparent hover:bg-gray-50 dark:hover:bg-gray-800/60'
                      "
                      @click="handleSelectScenery(scenery.sceneryId)"
                    >
                      <div class="flex items-center justify-between w-full">
                        <span class="font-semibold text-gray-900 dark:text-white text-[13px]"
                          >#{{ scenery.sceneryId }}</span
                        >
                        <div class="flex gap-1 items-center">
                          <span
                            v-if="scenery.recommended"
                            class="px-1.5 py-0.5 rounded text-[9px] font-bold tracking-wide bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300"
                          >
                            REC
                          </span>
                          <span
                            v-if="store.selectedInstalledRecord?.sceneryId === scenery.sceneryId"
                            class="px-1.5 py-0.5 rounded text-[9px] font-bold tracking-wide bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300"
                          >
                            LOCAL
                          </span>
                          <span
                            v-if="isLatestUpdateScenery(scenery.sceneryId)"
                            class="px-1.5 py-0.5 rounded text-[9px] font-bold tracking-wide bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300"
                          >
                            UPDATE
                          </span>
                        </div>
                      </div>
                      <div
                        class="flex items-center justify-between w-full text-[11px] text-gray-500 dark:text-gray-400"
                      >
                        <span class="truncate pr-2">{{
                          scenery.artist || $t('common.unknown')
                        }}</span>
                        <span class="flex-shrink-0">{{
                          scenery.approvedDate ? formatDate(scenery.approvedDate) : ''
                        }}</span>
                      </div>
                    </button>

                    <div
                      v-if="store.airportDetail.sceneries.length === 0"
                      class="text-[11px] text-gray-400 dark:text-gray-500 py-6 text-center"
                    >
                      {{ $t('gatewayManager.historyEmpty') }}
                    </div>
                  </div>
                </div>

                <!-- Scenery Details -->
                <div
                  class="min-h-0 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 overflow-y-auto shadow-sm"
                >
                  <div
                    v-if="store.isLoadingScenery"
                    class="flex items-center justify-center h-full"
                  >
                    <div
                      class="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
                    ></div>
                  </div>

                  <template v-else-if="store.sceneryDetail">
                    <div class="space-y-4">
                      <div
                        class="rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50/70 dark:bg-gray-900/40 p-4"
                      >
                        <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
                          <div>
                            <div
                              class="text-[11px] font-semibold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400"
                            >
                              {{ $t('management.versionInfo') }}
                            </div>
                            <div class="mt-2 flex flex-wrap gap-1.5">
                              <span
                                v-if="selectedSceneryIsInstalled"
                                class="px-2 py-0.5 rounded-md text-[10px] font-semibold tracking-wide bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300"
                              >
                                LOCAL
                              </span>
                              <span
                                v-if="selectedSceneryIsRecommended"
                                class="px-2 py-0.5 rounded-md text-[10px] font-semibold tracking-wide bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300"
                              >
                                REC
                              </span>
                              <span
                                v-if="selectedSceneryIsLatestUpdate"
                                class="px-2 py-0.5 rounded-md text-[10px] font-semibold tracking-wide bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300"
                              >
                                UPDATE
                              </span>
                            </div>
                          </div>
                        </div>
                        <div class="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-3 text-[13px]">
                          <div
                            class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/70 px-3 py-2.5"
                          >
                            <div class="text-[11px] text-gray-500 dark:text-gray-400">
                              {{ $t('management.currentVersionLabel') }}
                            </div>
                            <div class="font-medium text-gray-900 dark:text-white mt-1">
                              {{ formatGatewayVersion(store.sceneryDetail.sceneryId) }}
                            </div>
                          </div>
                          <div
                            class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/70 px-3 py-2.5"
                          >
                            <div class="text-[11px] text-gray-500 dark:text-gray-400">
                              {{ $t('map.gateway.artist') }}
                            </div>
                            <div class="font-medium text-gray-900 dark:text-white mt-1">
                              {{ store.sceneryDetail.artist || $t('common.unknown') }}
                            </div>
                          </div>
                          <div
                            class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/70 px-3 py-2.5"
                          >
                            <div class="text-[11px] text-gray-500 dark:text-gray-400">
                              {{ $t('map.gateway.acceptedAt') }}
                            </div>
                            <div class="font-medium text-gray-900 dark:text-white mt-1">
                              {{
                                store.sceneryDetail.approvedDate
                                  ? formatDateTime(store.sceneryDetail.approvedDate)
                                  : $t('common.notSet')
                              }}
                            </div>
                          </div>
                          <div
                            class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/70 px-3 py-2.5"
                          >
                            <div class="text-[11px] text-gray-500 dark:text-gray-400">
                              {{ $t('map.gateway.status') }}
                            </div>
                            <div class="font-medium text-gray-900 dark:text-white mt-1">
                              {{ store.sceneryDetail.status || $t('common.unknown') }}
                            </div>
                          </div>
                        </div>
                      </div>

                      <div
                        class="rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50/70 dark:bg-gray-900/40 p-4"
                      >
                        <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
                          <div class="min-w-0 flex-1">
                            <div
                              class="text-[11px] font-semibold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400"
                            >
                              {{ $t('gatewayManager.folderName') }}
                            </div>
                            <div
                              class="font-medium text-gray-900 dark:text-white mt-1 text-[13px] break-all"
                            >
                              {{ selectedInstalledFolderLabel }}
                            </div>
                          </div>
                          <div v-if="hasInstalledFolderActions" class="flex items-center gap-2">
                            <button
                              class="text-xs px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                              @click="handleOpenInstalledFolder"
                            >
                              {{ $t('diskUsage.openFolder') }}
                            </button>
                            <button
                              class="text-xs px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                              @click="handleCopyInstalledFolder"
                            >
                              {{ $t('copy.copy') }}
                            </button>
                          </div>
                        </div>
                        <div class="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-3 text-[13px]">
                          <div
                            class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/70 px-3 py-2.5"
                          >
                            <div class="text-[11px] text-gray-500 dark:text-gray-400">
                              {{ $t('gatewayManager.localVersion') }}
                            </div>
                            <div class="font-medium text-gray-900 dark:text-white mt-1">
                              {{ selectedInstalledVersionLabel }}
                            </div>
                          </div>
                          <div
                            v-if="store.selectedInstalledRecord?.updateAvailable === true"
                            class="rounded-xl border border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/20 px-3 py-2.5"
                          >
                            <div class="text-[11px] text-amber-700 dark:text-amber-300">
                              {{ $t('gatewayManager.updatesAvailable') }}
                            </div>
                            <div class="font-medium text-gray-900 dark:text-white mt-1">
                              {{
                                formatGatewayVersion(store.selectedInstalledRecord.latestSceneryId)
                              }}
                            </div>
                            <div
                              v-if="
                                store.selectedInstalledRecord.latestArtist ||
                                store.selectedInstalledRecord.latestApprovedDate
                              "
                              class="text-[11px] text-amber-700/80 dark:text-amber-200/80 mt-1"
                            >
                              {{
                                [
                                  store.selectedInstalledRecord.latestArtist || $t('common.unknown'),
                                  store.selectedInstalledRecord.latestApprovedDate
                                    ? formatDate(store.selectedInstalledRecord.latestApprovedDate)
                                    : null,
                                ]
                                  .filter(Boolean)
                                  .join(' · ')
                              }}
                            </div>
                          </div>
                        </div>
                      </div>

                      <div
                        v-if="store.sceneryDetail.features.length > 0"
                        class="rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50/70 dark:bg-gray-900/40 p-4"
                      >
                        <div
                          class="text-[11px] font-semibold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-2"
                        >
                          {{ $t('map.gateway.features') }}
                        </div>
                        <div class="flex flex-wrap gap-1.5">
                          <span
                            v-for="feature in store.sceneryDetail.features"
                            :key="feature"
                            class="px-2 py-0.5 rounded-md bg-gray-100 dark:bg-gray-700/50 text-[10px] font-medium text-gray-700 dark:text-gray-300"
                          >
                            {{ feature }}
                          </span>
                        </div>
                      </div>

                      <div
                        v-if="store.sceneryDetail.comment"
                        class="rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50/70 dark:bg-gray-900/40 p-4"
                      >
                        <div
                          class="text-[11px] font-semibold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-2"
                        >
                          {{ $t('map.gateway.comments') }}
                        </div>
                        <div
                          class="rounded-xl bg-white dark:bg-gray-800/70 border border-gray-200 dark:border-gray-700 px-3 py-2 text-[12px] text-gray-700 dark:text-gray-300 whitespace-pre-wrap leading-relaxed max-h-56 overflow-y-auto"
                        >
                          {{ store.sceneryDetail.comment }}
                        </div>
                      </div>
                    </div>
                  </template>

                  <div
                    v-else
                    class="h-full flex items-center justify-center text-[13px] text-gray-400 dark:text-gray-500"
                  >
                    {{ $t('gatewayManager.noSelection') }}
                  </div>
                </div>
              </div>
            </template>
          </div>
        </div>
      </div>
    </Teleport>

    <ConfirmModal
      :show="showInstallWarning"
      :title="t('gatewayManager.installBlocked')"
      :message="installWarningMessage"
      :warning="t('gatewayManager.installBlockedWarning')"
      :confirm-text="t('gatewayManager.ignoreWarningInstall')"
      :cancel-text="t('common.cancel')"
      variant="warning"
      :hide-close-button="true"
      @update:show="showInstallWarning = $event"
      @confirm="confirmInstallWarning"
      @cancel="cancelInstallWarning"
    />
  </div>
</template>

<script setup lang="ts">
import ConfirmModal from '@/components/ConfirmModal.vue'
import { CommandError, invokeVoidCommand } from '@/services/api'
import { useGatewayStore } from '@/stores/gateway'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { getErrorMessage, type GatewayInstalledAirport } from '@/types'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const GATEWAY_EXTERNAL_CONFLICT_MARKER = 'contains a non-Gateway airport'
const GATEWAY_EXTERNAL_CONFLICT_KIND = 'external_airport_conflict'
const { t } = useI18n()
const appStore = useAppStore()
const store = useGatewayStore()
const modal = useModalStore()
const toast = useToastStore()

const activeTab = ref<'installed' | 'search'>('installed')
const searchText = ref('')
const showAirportModal = ref(false)
const showInstallWarning = ref(false)
const installWarningMessage = ref('')
let searchTimer: ReturnType<typeof setTimeout> | null = null

const filteredInstalled = computed(() => {
  if (!searchText.value) return store.installed
  const q = searchText.value.toLowerCase()
  return store.installed.filter(
    (a) => a.airportIcao.toLowerCase().includes(q) || a.airportName?.toLowerCase().includes(q),
  )
})

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

const selectedAirportName = computed(
  () => store.airportDetail?.airportName || store.airportDetail?.icao || '',
)

const selectedSubmissionCount = computed(() => {
  if (!store.airportDetail) return 0
  return store.airportDetail.sceneryCount ?? store.airportDetail.sceneries.length
})

const selectedInstalledVersionLabel = computed(() =>
  store.selectedInstalledRecord?.sceneryId !== null && store.selectedInstalledRecord?.sceneryId !== undefined
    ? `#${store.selectedInstalledRecord.sceneryId}`
    : t('gatewayManager.notInstalled'),
)

const selectedInstalledFolderLabel = computed(
  () => store.selectedInstalledRecord?.folderName || t('gatewayManager.notInstalled'),
)

const hasInstalledFolderActions = computed(
  () => Boolean(appStore.xplanePath && store.selectedInstalledRecord?.folderName),
)

const selectedSceneryIsInstalled = computed(
  () => store.selectedSceneryId !== null && isInstalledScenery(store.selectedSceneryId),
)

const selectedSceneryIsRecommended = computed(
  () => store.selectedSceneryId !== null && isRecommendedScenery(store.selectedSceneryId),
)

const selectedSceneryIsLatestUpdate = computed(
  () => store.selectedSceneryId !== null && isLatestUpdateScenery(store.selectedSceneryId),
)

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

  if (activeTab.value === 'search') {
    searchTimer = setTimeout(async () => {
      try {
        await store.searchAirports(value)
      } catch (error) {
        toast.error(`${t('gatewayManager.loadAirportFailed')}: ${getErrorMessage(error)}`)
      }
    }, 250)
  }
})

watch(activeTab, (tab) => {
  if (tab === 'search' && searchText.value) {
    if (searchTimer) clearTimeout(searchTimer)
    store.searchAirports(searchText.value).catch((error) => {
      toast.error(`${t('gatewayManager.loadAirportFailed')}: ${getErrorMessage(error)}`)
    })
  }
})

onBeforeUnmount(() => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }
})

function clearSearch() {
  searchText.value = ''
  if (activeTab.value === 'search') {
    store.clearSearch()
  }
}

function formatGatewayVersion(value: number | null | undefined): string {
  return value !== null && value !== undefined ? `#${value}` : t('common.notSet')
}

function formatDate(value: string): string {
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
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

async function openAirportModal(icao: string, preferredSceneryId?: number) {
  showAirportModal.value = true
  try {
    await store.openAirport(icao, preferredSceneryId)
  } catch (error) {
    modal.showError(`${t('gatewayManager.loadAirportFailed')}: ${getErrorMessage(error)}`)
    showAirportModal.value = false
  }
}

function closeAirportModal() {
  showAirportModal.value = false
}

function isInstalledScenery(sceneryId: number): boolean {
  return store.selectedInstalledRecord?.sceneryId === sceneryId
}

function isRecommendedScenery(sceneryId: number): boolean {
  return store.airportDetail?.recommendedSceneryId === sceneryId
}

function isLatestUpdateScenery(sceneryId: number): boolean {
  return (
    store.selectedInstalledRecord?.updateAvailable === true &&
    store.selectedInstalledRecord.latestSceneryId === sceneryId
  )
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

async function handleOpenInstalledFolder() {
  if (!appStore.xplanePath || !store.selectedInstalledRecord?.folderName) return

  try {
    await invokeVoidCommand('open_scenery_folder', {
      xplanePath: appStore.xplanePath,
      folderName: store.selectedInstalledRecord.folderName,
    })
  } catch (error) {
    modal.showError(`${t('management.openFolderFailed')}: ${getErrorMessage(error)}`)
  }
}

async function handleCopyInstalledFolder() {
  if (!store.selectedInstalledRecord?.folderName) return

  try {
    await navigator.clipboard.writeText(store.selectedInstalledRecord.folderName)
    toast.success(t('copy.copied'))
  } catch {
    toast.error(t('copy.copyFailed'))
  }
}

async function installSelectedGateway(ignoreExternalConflict = false) {
  if (!appStore.xplanePath) return

  const installedRecord = await store.installSelected(
    appStore.xplanePath,
    appStore.autoSortScenery,
    ignoreExternalConflict,
  )
  toast.success(
    t('gatewayManager.installSuccess', {
      icao: installedRecord.airportIcao,
    }),
  )
}

function showInstallBlockedWarning(message: string) {
  installWarningMessage.value = message
  showInstallWarning.value = true
}

function cancelInstallWarning() {
  showInstallWarning.value = false
}

function confirmInstallWarning() {
  showInstallWarning.value = false
  void handleInstall(true)
}

function getGatewayExternalConflictMessage(error: unknown): string | null {
  const message = getErrorMessage(error)
  if (!message.includes(GATEWAY_EXTERNAL_CONFLICT_MARKER)) {
    return null
  }

  if (error instanceof CommandError && error.code && error.code !== 'conflict_exists') {
    return null
  }

  return message
}

async function handleInstall(ignoreExternalConflict = false) {
  if (!appStore.xplanePath) {
    modal.showError(t('gatewayManager.pathRequiredHint'), t('gatewayManager.pathRequired'), {
      hideReport: true,
    })
    return
  }

  if (!ignoreExternalConflict) {
    try {
      const warning = await store.checkInstallWarning(appStore.xplanePath)
      if (warning?.kind === GATEWAY_EXTERNAL_CONFLICT_KIND) {
        showInstallBlockedWarning(warning.message)
        return
      }
    } catch {
      // Fall back to install-time conflict handling if warning precheck is unavailable.
    }
  }

  try {
    await installSelectedGateway(ignoreExternalConflict)
  } catch (error) {
    const externalConflictMessage = getGatewayExternalConflictMessage(error)
    if (externalConflictMessage && !ignoreExternalConflict) {
      showInstallBlockedWarning(externalConflictMessage)
      return
    }
    if (error instanceof CommandError && error.code === 'conflict_exists' && !ignoreExternalConflict) {
      showInstallBlockedWarning(error.message)
      return
    }
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
