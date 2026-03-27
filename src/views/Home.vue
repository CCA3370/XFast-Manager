<template>
  <div
    class="home-view h-full flex flex-col p-4 animate-fade-in relative overflow-hidden select-none"
  >
    <!-- Background Decor (Dark Mode Only for deep glow) -->
    <div
      class="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none z-0 opacity-0 dark:opacity-100 transition-opacity duration-500"
    >
      <div class="absolute top-1/4 left-1/4 w-48 h-48 bg-blue-500/5 rounded-full blur-3xl"></div>
      <div
        class="absolute bottom-1/4 right-1/4 w-64 h-64 bg-purple-500/5 rounded-full blur-3xl"
      ></div>
    </div>

    <div
      class="w-full z-10 flex flex-col flex-1 min-h-0 gap-3 overflow-y-auto pr-1 custom-scrollbar"
    >
      <!-- Update Banner -->
      <UpdateBanner
        :visible="updateStore.showUpdateBanner"
        :update-info="updateStore.updateInfo"
        :is-downloading="updateStore.isDownloading"
        :download-progress="updateStore.downloadProgress"
        :update-phase="updateStore.updatePhase"
        :update-error="updateStore.updateError"
        @view-release="updateStore.openReleaseUrl"
        @dismiss="updateStore.dismissUpdate"
        @update="updateStore.performUpdate"
        @retry="updateStore.performUpdate"
      />

      <!-- Warning Alert (Compact) -->
      <transition name="slide-down">
        <div
          v-if="!store.xplanePath"
          class="flex-shrink-0 bg-yellow-50/90 dark:bg-yellow-500/10 backdrop-blur-md border border-yellow-200 dark:border-yellow-500/20 rounded-xl p-3 flex items-center space-x-3 shadow-lg shadow-yellow-500/5 transition-colors duration-300"
        >
          <div class="p-2 bg-yellow-100 dark:bg-yellow-500/20 rounded-lg flex-shrink-0">
            <svg
              class="w-5 h-5 text-yellow-600 dark:text-yellow-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
              ></path>
            </svg>
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-semibold text-yellow-800 dark:text-yellow-100">
              <AnimatedText>{{ $t('home.setPathFirst') }}</AnimatedText>
            </p>
            <p class="text-xs text-yellow-700 dark:text-yellow-200/70">
              <AnimatedText>{{ $t('home.pathNotSetDesc') }}</AnimatedText>
            </p>
          </div>
          <router-link
            to="/settings"
            class="flex-shrink-0 inline-flex items-center px-3 py-1.5 bg-yellow-200 dark:bg-yellow-500/20 hover:bg-yellow-300 dark:hover:bg-yellow-500/30 text-yellow-900 dark:text-yellow-100 text-xs font-bold rounded-xl transition-all duration-200 border border-yellow-300 dark:border-yellow-500/30 shadow-sm"
          >
            <AnimatedText>{{ $t('home.goToSettings') }}</AnimatedText>
            <svg class="w-3.5 h-3.5 ml-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 5l7 7-7 7"
              ></path>
            </svg>
          </router-link>
        </div>
      </transition>

      <!-- Dashboard Content -->
      <div class="grid grid-cols-1 md:grid-cols-4 gap-3 pb-4">
        <!-- Stats Cards -->
        <div
          class="bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-3.5 shadow-sm transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <div class="flex justify-between items-start">
            <div class="p-2 bg-blue-100 dark:bg-blue-500/20 rounded-xl">
              <svg
                class="w-6 h-6 text-blue-600 dark:text-blue-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                />
              </svg>
            </div>
            <div
              v-if="managementStore.aircraftUpdateCount > 0"
              class="px-2 py-0.5 bg-blue-500 text-white text-[10px] font-bold rounded-full animate-pulse"
            >
              {{ $t('dashboard.stats.updates', { count: managementStore.aircraftUpdateCount }) }}
            </div>
          </div>
          <div class="mt-2.5">
            <div class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ managementStore.aircraftTotalCount }}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400 font-medium">
              {{ $t('dashboard.stats.aircraft') }}
            </div>
          </div>
        </div>

        <div
          class="bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-3.5 shadow-sm transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <div class="flex justify-between items-start">
            <div class="p-2 bg-purple-100 dark:bg-purple-500/20 rounded-xl">
              <svg
                class="w-6 h-6 text-purple-600 dark:text-purple-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                />
              </svg>
            </div>
            <div
              v-if="managementStore.pluginsUpdateCount > 0"
              class="px-2 py-0.5 bg-purple-500 text-white text-[10px] font-bold rounded-full animate-pulse"
            >
              {{ $t('dashboard.stats.updates', { count: managementStore.pluginsUpdateCount }) }}
            </div>
          </div>
          <div class="mt-2.5">
            <div class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ managementStore.pluginsTotalCount }}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400 font-medium">
              {{ $t('dashboard.stats.plugins') }}
            </div>
          </div>
        </div>

        <div
          class="bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-3.5 shadow-sm transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <div class="flex justify-between items-start">
            <div class="p-2 bg-emerald-100 dark:bg-emerald-500/20 rounded-xl">
              <svg
                class="w-6 h-6 text-emerald-600 dark:text-emerald-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </div>
            <div
              v-if="sceneryStore.duplicatesCount > 0"
              class="px-2 py-0.5 bg-amber-500 text-white text-[10px] font-bold rounded-full"
            >
              {{ $t('dashboard.stats.duplicates', { count: sceneryStore.duplicatesCount }) }}
            </div>
            <div
              v-else-if="sceneryStore.missingDepsCount > 0"
              class="px-2 py-0.5 bg-red-500 text-white text-[10px] font-bold rounded-full"
            >
              {{ $t('dashboard.stats.missingDeps', { count: sceneryStore.missingDepsCount }) }}
            </div>
          </div>
          <div class="mt-2.5">
            <div class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ sceneryStore.totalCount }}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400 font-medium">
              {{ $t('dashboard.stats.scenery') }}
            </div>
          </div>
        </div>

        <div
          class="bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-3.5 shadow-sm transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <div class="flex justify-between items-start">
            <div class="p-2 bg-pink-100 dark:bg-pink-500/20 rounded-xl">
              <svg
                class="w-6 h-6 text-pink-600 dark:text-pink-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
            </div>
          </div>
          <div class="mt-2.5">
            <div class="text-2xl font-bold text-gray-900 dark:text-white">
              {{ managementStore.navdataTotalCount }}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400 font-medium">
              {{ $t('dashboard.stats.navdata') }}
            </div>
          </div>
        </div>

        <!-- Main Dashboard Row 2 -->
        <!-- Disk Usage -->
        <div
          class="md:col-span-2 bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-4.5 shadow-sm flex flex-col items-center justify-center min-h-[260px] transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <h3
            class="w-full text-base font-bold text-gray-900 dark:text-white mb-3 flex items-center gap-2"
          >
            <svg
              class="w-4 h-4 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"
              />
            </svg>
            {{ $t('dashboard.diskUsage') }}
          </h3>
          <div
            v-if="diskUsageStore.isScanning"
            class="flex flex-col items-center justify-center gap-2.5"
          >
            <svg class="w-7 h-7 animate-spin text-blue-500" fill="none" viewBox="0 0 24 24">
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
            <div class="text-xs text-gray-500 dark:text-gray-400">
              {{ $t('diskUsage.scanning') }}
            </div>
          </div>
          <DiskUsageChart
            v-else-if="diskUsageStore.report"
            :categories="diskCategories"
            :total-bytes="diskUsageStore.report.totalBytes"
            :size="170"
            :is-dark="store.theme === 'dark'"
          />
          <div v-else class="text-xs text-gray-500 dark:text-gray-400">
            {{ $t('diskUsage.empty') }}
          </div>
        </div>

        <!-- Quick Actions -->
        <div
          class="md:col-span-2 bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-4.5 shadow-sm flex flex-col transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <h3
            class="text-base font-bold text-gray-900 dark:text-white mb-4 flex items-center gap-2"
          >
            <svg
              class="w-4 h-4 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M13 10V3L4 14h7v7l9-11h-7z"
              />
            </svg>
            {{ $t('dashboard.quickActions') }}
          </h3>
          <div class="grid grid-cols-2 gap-3 flex-1">
            <button
              :disabled="isLaunchingXPlane || isXPlaneRunning"
              class="flex flex-col items-center justify-center gap-2.5 p-3.5 bg-white/50 dark:bg-gray-700/50 hover:bg-blue-50 dark:hover:bg-blue-500/20 rounded-2xl border border-white/40 dark:border-gray-600/50 transition-all hover:scale-[1.02] active:scale-[0.98] group disabled:opacity-50"
              @click="handleLaunchXPlane"
            >
              <div
                class="w-10 h-10 rounded-full bg-blue-100 dark:bg-blue-500/20 flex items-center justify-center group-hover:scale-110 transition-transform"
              >
                <svg
                  v-if="isLaunchingXPlane"
                  class="w-5 h-5 animate-spin text-blue-600 dark:text-blue-400"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                  ></circle>
                  <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  ></path>
                </svg>
                <svg
                  v-else
                  class="w-5 h-5 text-blue-600 dark:text-blue-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                  />
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </div>
              <span class="text-xs font-bold text-gray-700 dark:text-gray-200">{{
                isXPlaneRunning ? $t('home.xplaneRunning') : $t('home.launchXPlane')
              }}</span>
            </button>

            <button
              class="flex flex-col items-center justify-center gap-2.5 p-3.5 bg-white/50 dark:bg-gray-700/50 hover:bg-emerald-50 dark:hover:bg-emerald-500/20 rounded-2xl border border-white/40 dark:border-gray-600/50 transition-all hover:scale-[1.02] active:scale-[0.98] group"
              @click="handleFixScenery"
            >
              <div
                class="w-10 h-10 rounded-full bg-emerald-100 dark:bg-emerald-500/20 flex items-center justify-center group-hover:scale-110 transition-transform"
              >
                <svg
                  class="w-5 h-5 text-emerald-600 dark:text-emerald-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"
                  />
                </svg>
              </div>
              <span class="text-xs font-bold text-gray-700 dark:text-gray-200">{{
                $t('dashboard.fixScenery')
              }}</span>
            </button>

            <button
              class="flex flex-col items-center justify-center gap-2.5 p-3.5 bg-white/50 dark:bg-gray-700/50 hover:bg-pink-50 dark:hover:bg-pink-500/20 rounded-2xl border border-white/40 dark:border-gray-600/50 transition-all hover:scale-[1.02] active:scale-[0.98] group"
              @click="router.push('/activity')"
            >
              <div
                class="w-10 h-10 rounded-full bg-pink-100 dark:bg-pink-500/20 flex items-center justify-center group-hover:scale-110 transition-transform"
              >
                <svg
                  class="w-5 h-5 text-pink-600 dark:text-pink-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </div>
              <span class="text-xs font-bold text-gray-700 dark:text-gray-200">{{
                $t('activityLog.navTitle')
              }}</span>
            </button>

            <button
              class="flex flex-col items-center justify-center gap-2.5 p-3.5 bg-white/50 dark:bg-gray-700/50 hover:bg-amber-50 dark:hover:bg-amber-500/20 rounded-2xl border border-white/40 dark:border-gray-600/50 transition-all hover:scale-[1.02] active:scale-[0.98] group"
              @click="router.push('/log-analysis')"
            >
              <div
                class="w-10 h-10 rounded-full bg-amber-100 dark:bg-amber-500/20 flex items-center justify-center group-hover:scale-110 transition-transform"
              >
                <svg
                  class="w-5 h-5 text-amber-600 dark:text-amber-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                  />
                </svg>
              </div>
              <span class="text-xs font-bold text-gray-700 dark:text-gray-200">{{
                $t('dashboard.openLogs')
              }}</span>
            </button>
          </div>
        </div>

        <!-- Row 3: Activity & System -->
        <!-- Recent Activity -->
        <div
          class="md:col-span-3 bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-4.5 shadow-sm transition-all hover:bg-white/60 dark:hover:bg-gray-800/60"
        >
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-base font-bold text-gray-900 dark:text-white flex items-center gap-2">
              <svg
                class="w-4 h-4 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              {{ $t('dashboard.recentActivity') }}
            </h3>
            <button
              class="text-xs font-bold text-blue-500 hover:text-blue-600 transition-colors"
              @click="router.push('/activity')"
            >
              {{ $t('dashboard.viewAll') }}
            </button>
          </div>
          <div class="space-y-2.5">
            <div
              v-for="entry in activityLogStore.entries.slice(0, 5)"
              :key="entry.id"
              class="flex items-center gap-3.5 p-2.5 rounded-2xl bg-white/50 dark:bg-gray-700/30 border border-white/40 dark:border-gray-600/30 transition-all hover:bg-white/80 dark:hover:bg-gray-700/50"
            >
              <div
                class="w-9 h-9 rounded-full flex items-center justify-center shrink-0"
                :class="
                  entry.success
                    ? 'bg-emerald-100 dark:bg-emerald-500/20 text-emerald-600 dark:text-emerald-400'
                    : 'bg-red-100 dark:bg-red-500/20 text-red-600 dark:text-red-400'
                "
              >
                <svg
                  v-if="entry.success"
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </div>
              <div class="min-w-0 flex-1">
                <div class="text-sm font-bold text-gray-900 dark:text-white truncate">
                  {{ entry.itemName || entry.operation }}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400 truncate">
                  {{ entry.operation }} • {{ entry.itemType }}
                </div>
              </div>
              <div class="text-[10px] text-gray-400 dark:text-gray-500 whitespace-nowrap">
                {{ formatRelativeTime(entry.timestamp) }}
              </div>
            </div>
            <div v-if="activityLogStore.entries.length === 0" class="py-12 text-center">
              <div class="text-sm text-gray-400">{{ $t('dashboard.noActivity') }}</div>
            </div>
          </div>
        </div>

        <!-- System Status -->
        <div
          class="bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-2xl p-4.5 shadow-sm transition-all hover:bg-white/60 dark:hover:bg-gray-800/60 flex flex-col"
        >
          <h3
            class="text-base font-bold text-gray-900 dark:text-white mb-4 flex items-center gap-2"
          >
            <svg
              class="w-4 h-4 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
              />
            </svg>
            {{ $t('dashboard.systemStatus') }}
          </h3>
          <div class="space-y-3 flex-1">
            <div class="space-y-1">
              <div class="text-[9px] uppercase tracking-wider font-bold text-gray-400">
                {{ $t('dashboard.xplaneStatus') }}
              </div>
              <div class="flex items-center gap-2">
                <div
                  class="w-1.5 h-1.5 rounded-full"
                  :class="isXPlaneRunning ? 'bg-emerald-500 animate-pulse' : 'bg-gray-400'"
                ></div>
                <div class="text-xs font-bold text-gray-700 dark:text-gray-200">
                  {{ isXPlaneRunning ? $t('home.xplaneRunning') : $t('common.offline') }}
                </div>
              </div>
            </div>
            <div class="space-y-1">
              <div class="text-[9px] uppercase tracking-wider font-bold text-gray-400">
                {{ $t('dashboard.appUpdate') }}
              </div>
              <div
                v-if="updateStore.showUpdateBanner"
                class="flex items-center gap-2 text-blue-500"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                  />
                </svg>
                <div class="text-xs font-bold">{{ $t('update.newVersionAvailable') }}</div>
              </div>
              <div v-else class="text-xs font-bold text-gray-700 dark:text-gray-200">
                v{{ updateStore.currentVersion }} ({{ $t('common.latest') }})
              </div>
            </div>
            <div class="space-y-1">
              <div class="text-[9px] uppercase tracking-wider font-bold text-gray-400">
                {{ $t('dashboard.xplanePath') }}
              </div>
              <div
                class="text-[10px] font-mono p-1.5 bg-black/5 dark:bg-black/20 rounded-lg text-gray-600 dark:text-gray-400 break-all border border-black/5"
              >
                {{ store.xplanePath || $t('common.notSet') }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Drag Overlay -->
    <transition name="fade">
      <div
        v-if="isDragging && !store.isInstalling"
        class="fixed inset-0 z-[100] flex flex-col items-center justify-center bg-blue-500/20 backdrop-blur-md transition-all duration-300 pointer-events-none"
      >
        <div
          class="p-12 rounded-[3rem] bg-white/90 dark:bg-gray-800/90 shadow-2xl border-4 border-dashed border-blue-500 flex flex-col items-center gap-6 animate-bounce-in"
        >
          <div
            class="w-24 h-24 rounded-full bg-blue-500 flex items-center justify-center text-white shadow-lg shadow-blue-500/40"
          >
            <svg class="w-12 h-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M12 4v16m8-8H4"
              />
            </svg>
          </div>
          <div class="text-center">
            <h2 class="text-3xl font-black text-gray-900 dark:text-white">
              {{ $t('dashboard.dropToInstall') }}
            </h2>
            <p class="text-gray-500 dark:text-gray-400 mt-2 font-medium">
              {{ $t('home.supportedFormats') }}
            </p>
          </div>
        </div>
      </div>
    </transition>

    <!-- Progress Overlays -->
    <transition name="fade" mode="out-in">
      <AnalyzingOverlay v-if="store.isAnalyzing" key="analyzing" />

      <InstallProgressOverlay
        v-else-if="store.isInstalling || store.showCompletion"
        key="installing"
        :percentage="progressStore.formatted.percentage"
        :task-name="progressStore.formatted.taskName"
        :processed-m-b="progressStore.formatted.processedMB"
        :total-m-b="progressStore.formatted.totalMB"
        :task-progress="progressStore.formatted.taskProgress"
        :tasks="store.installingTasks"
        :current-task-index="progressStore.progress?.currentTaskIndex ?? 0"
        :current-task-percentage="progressStore.formatted.currentTaskPercentage"
        :current-task-processed-m-b="progressStore.formatted.currentTaskProcessedMB"
        :current-task-total-m-b="progressStore.formatted.currentTaskTotalMB"
        :is-complete="store.showCompletion"
        :install-result="store.installResult"
        :active-tasks="progressStore.activeTasks"
        :completed-task-count="progressStore.completedTaskCount"
        :completed-task-ids="progressStore.completedTaskIds"
        @skip="handleSkipTask"
        @cancel="handleCancelInstallation"
        @confirm="handleCompletionConfirm"
      />
    </transition>

    <ConfirmationModal
      v-if="showConfirmation"
      @close="showConfirmation = false"
      @confirm="handleInstall"
    />
    <PasswordModal
      v-if="showPasswordModal"
      :archive-paths="passwordRequiredPaths"
      :error-message="passwordErrorMessage"
      @confirm="handlePasswordSubmit"
      @cancel="handlePasswordCancel"
    />

    <!-- Launch X-Plane Confirmation Dialog -->
    <transition name="fade">
      <div
        v-if="showLaunchConfirmDialog"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
        @click.self="cancelLaunchDialog"
      >
        <div
          class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-md w-full mx-4 overflow-hidden animate-bounce-in"
        >
          <!-- Header -->
          <div
            class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center gap-3"
          >
            <div class="p-2 bg-amber-100 dark:bg-amber-500/20 rounded-lg">
              <svg
                class="w-5 h-5 text-amber-600 dark:text-amber-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                ></path>
              </svg>
            </div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
              {{ $t('home.launchConfirmTitle') }}
            </h3>
          </div>

          <!-- Content -->
          <div class="px-6 py-4">
            <p class="text-gray-600 dark:text-gray-300">
              {{ $t('home.launchConfirmMessage') }}
            </p>
          </div>

          <!-- Actions -->
          <div
            class="px-6 py-4 bg-gray-50 dark:bg-gray-800/50 flex flex-col sm:flex-row gap-3 sm:justify-end"
          >
            <button
              :disabled="isLaunchingXPlane"
              class="px-4 py-2 text-gray-700 dark:text-gray-300 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              @click="cancelLaunchDialog"
            >
              {{ $t('common.cancel') }}
            </button>
            <button
              :disabled="isLaunchingXPlane"
              class="px-4 py-2 text-blue-700 dark:text-blue-300 bg-blue-100 dark:bg-blue-500/20 hover:bg-blue-200 dark:hover:bg-blue-500/30 rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="launchXPlane"
            >
              <svg
                v-if="isLaunchingXPlane"
                class="w-4 h-4 animate-spin"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              {{ $t('home.launchDirectly') }}
            </button>
            <button
              :disabled="isLaunchingXPlane"
              class="px-4 py-2 text-white bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 rounded-lg font-medium transition-all shadow-md disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              @click="applyAndLaunch"
            >
              <svg
                v-if="isLaunchingXPlane"
                class="w-4 h-4 animate-spin"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              {{ $t('home.applyAndLaunch') }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore, type ConfirmOptions } from '@/stores/modal'
import { useProgressStore } from '@/stores/progress'
import { useUpdateStore } from '@/stores/update'
import { useSceneryStore } from '@/stores/scenery'
import { useManagementStore } from '@/stores/management'
import { useActivityLogStore } from '@/stores/activityLog'
import { useDiskUsageStore } from '@/stores/diskUsage'
import { useLockStore } from '@/stores/lock'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import ConfirmationModal from '@/components/ConfirmationModal.vue'
import PasswordModal from '@/components/PasswordModal.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import UpdateBanner from '@/components/UpdateBanner.vue'
import InstallProgressOverlay from '@/components/InstallProgressOverlay.vue'
import AnalyzingOverlay from '@/components/AnalyzingOverlay.vue'
import DiskUsageChart from '@/components/DiskUsageChart.vue'
import type { AnalysisResult, InstallProgress, InstallResult } from '@/types'
import { AddonType } from '@/types'
import { getErrorMessage } from '@/types'
import { logOperation, logError, logDebug, logBasic } from '@/services/logger'
import { setTrackedTimeout } from '@/utils/timeout'

const { t } = useI18n()
const router = useRouter()

const store = useAppStore()
const toast = useToastStore()
const modal = useModalStore()
const updateStore = useUpdateStore()
const progressStore = useProgressStore()
const sceneryStore = useSceneryStore()
const managementStore = useManagementStore()
const activityLogStore = useActivityLogStore()
const diskUsageStore = useDiskUsageStore()
const lockStore = useLockStore()

const isDragging = ref(false)
const showConfirmation = ref(false)
const showLaunchConfirmDialog = ref(false)
const isLaunchingXPlane = ref(false)
const isXPlaneRunning = ref(false)
let xplaneCheckInterval: number | null = null
const debugDropFlash = ref(false)

// Dashboard data loading state
const isInitialLoading = ref(true)

// Sync confirmation modal state with store for exit confirmation
watch(showConfirmation, (value) => {
  store.setConfirmationOpen(value)
})

// Password modal state
const showPasswordModal = ref(false)
const passwordRequiredPaths = ref<string[]>([])
const pendingAnalysisPaths = ref<string[]>([])
const collectedPasswords = ref<Record<string, string>>({})
const passwordRetryCount = ref(0)
const passwordErrorMessage = ref('')
const MAX_PASSWORD_RETRIES = 3

// Password rate limiting
const passwordAttemptTimestamps = ref<number[]>([])
const MIN_PASSWORD_ATTEMPT_DELAY_MS = 1000 // 1 second between attempts
const PASSWORD_RATE_LIMIT_WINDOW_MS = 10000 // 10 second window for rate limiting
const DEBUG_DROP_FLASH_DURATION_MS = 800 // Duration for debug drop flash visual feedback
const DROP_ZONE_CLICK_SUPPRESS_AFTER_FOCUS_MS = 350 // Prevent activation click from opening picker
const COMPLETION_ANIMATION_DELAY_MS = 100 // Brief delay before hiding progress to allow animation to start
const suppressDropZoneClickUntil = ref(0)
const windowWasBlurred = ref(!document.hasFocus())

const DISK_CATEGORY_KEY_MAP: Record<string, string> = {
  aircraft: 'diskUsage.categoryAircraft',
  plugin: 'diskUsage.categoryPlugins',
  plugins: 'diskUsage.categoryPlugins',
  scenery: 'diskUsage.categoryScenery',
  navdata: 'diskUsage.categoryNavdata',
  screenshot: 'diskUsage.categoryScreenshots',
  screenshots: 'diskUsage.categoryScreenshots',
}

function diskCategoryLabel(category: string): string {
  const key = DISK_CATEGORY_KEY_MAP[category.trim().toLowerCase()]
  return key ? t(key) : category
}

// Disk usage categories for the widget
const diskCategories = computed(() => {
  if (!diskUsageStore.report) return []
  const colors = ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ef4444', '#6b7280']
  return diskUsageStore.report.categories.map((cat, i) => ({
    name: diskCategoryLabel(cat.category),
    bytes: cat.totalBytes,
    color: colors[i % colors.length],
  }))
})

// Timer tracking for cleanup on unmount to prevent memory leaks
const activeTimeoutIds = new Set<ReturnType<typeof setTimeout>>()

// Tauri drag-drop event unsubscribe function
let unlistenDragDrop: UnlistenFn | null = null
let unlistenProgress: UnlistenFn | null = null
let unlistenDeletionSkipped: UnlistenFn | null = null

// Watch for pending CLI args changes
watch(
  () => store.pendingCliArgs,
  async (args) => {
    if (args && args.length > 0) {
      if (store.isAnalyzeInProgress || store.isAnalyzing || store.isInstalling) {
        logDebug('Analysis in progress, re-queueing args for later', 'app')
        store.addCliArgsToBatch(args)
        store.clearPendingCliArgs()
        return
      }

      store.isAnalyzeInProgress = true
      logDebug(`Processing pending CLI args from watcher: ${args.join(', ')}`, 'app')
      const argsCopy = [...args]
      store.clearPendingCliArgs()
      try {
        await analyzeFiles(argsCopy)
      } catch (error) {
        logError(`Failed to process CLI args: ${error}`, 'app')
        modal.showError(getErrorMessage(error))
      } finally {
        store.isAnalyzeInProgress = false
      }
    }
  },
)

// Global listeners for drag/drop visual feedback
function onWindowDragOver(e: DragEvent) {
  e.preventDefault()
  if (store.isInstalling) return
  isDragging.value = true
}

function onWindowDragLeave(e: DragEvent) {
  if (store.isInstalling) return
  if (!e.relatedTarget) isDragging.value = false
}

function onWindowDrop(e: DragEvent) {
  e.preventDefault()
  if (store.isInstalling) return
  isDragging.value = false
  debugDropFlash.value = true
  setTrackedTimeout(
    () => (debugDropFlash.value = false),
    DEBUG_DROP_FLASH_DURATION_MS,
    activeTimeoutIds,
  )
}

function onWindowFocus() {
  if (windowWasBlurred.value) {
    suppressDropZoneClickUntil.value = Date.now() + DROP_ZONE_CLICK_SUPPRESS_AFTER_FOCUS_MS
  }
  windowWasBlurred.value = false
}

function onWindowBlur() {
  windowWasBlurred.value = true
  suppressDropZoneClickUntil.value = 0
}

async function handleDropZoneClick() {
  if (store.isInstalling || store.isAnalyzing || store.showCompletion) return

  if (!document.hasFocus() || Date.now() < suppressDropZoneClickUntil.value) {
    logDebug('Ignoring drop-zone click while window is inactive/just activated', 'drag-drop')
    return
  }

  try {
    const selected = await open({
      multiple: true,
      title: t('home.dropFilesHere'),
      filters: [
        {
          name: 'Addon Files',
          extensions: ['zip', '7z', 'rar', 'lua'],
        },
        {
          name: 'All Files',
          extensions: ['*'],
        },
      ],
    })

    const paths = Array.isArray(selected) ? selected : selected ? [selected] : []
    if (paths.length === 0) return

    await analyzeFiles(paths)
  } catch (error) {
    logError(`Failed to open file picker: ${error}`, 'drag-drop')
    modal.showError(getErrorMessage(error))
  }
}

function formatRelativeTime(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (seconds < 60) return t('activityLog.justNow')
  if (minutes < 60) return t('activityLog.minutesAgo', { n: minutes })
  if (hours < 24) return t('activityLog.hoursAgo', { n: hours })
  return t('activityLog.daysAgo', { n: days })
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

onMounted(async () => {
  window.addEventListener('dragover', onWindowDragOver)
  window.addEventListener('dragleave', onWindowDragLeave)
  window.addEventListener('drop', onWindowDrop)
  window.addEventListener('focus', onWindowFocus)
  window.addEventListener('blur', onWindowBlur)

  // Initialize dashboard data
  if (store.xplanePath) {
    Promise.all([
      managementStore.loadAircraft(),
      managementStore.loadPlugins(),
      managementStore.loadNavdata(),
      sceneryStore.loadData(),
      activityLogStore.loadRecent(),
      diskUsageStore.scan(),
    ]).finally(() => {
      isInitialLoading.value = false
    })
  } else {
    isInitialLoading.value = false
  }

  // Use Tauri 2's native drag-drop event
  try {
    const webview = getCurrentWebviewWindow()
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      if (store.isInstalling) return

      if (event.payload.type === 'over') {
        isDragging.value = true
      } else if (event.payload.type === 'leave') {
        isDragging.value = false
      } else if (event.payload.type === 'drop') {
        isDragging.value = false
        debugDropFlash.value = true
        setTrackedTimeout(
          () => (debugDropFlash.value = false),
          DEBUG_DROP_FLASH_DURATION_MS,
          activeTimeoutIds,
        )

        if (store.showCompletion) {
          store.clearInstallResult()
        }

        const paths = event.payload.paths
        if (paths && paths.length > 0) {
          try {
            await analyzeFiles(paths)
          } catch (error) {
            logError(`Failed to analyze dropped files: ${error}`, 'drag-drop')
            modal.showError(getErrorMessage(error))
          }
        }
      }
    })
  } catch (error) {
    logError(`Failed to setup Tauri drag-drop listener: ${error}`, 'drag-drop')
  }

  // Listen for installation progress events
  try {
    unlistenProgress = await listen<InstallProgress>('install-progress', (event) => {
      progressStore.update(event.payload)
    })
  } catch (error) {
    logError(`Failed to setup progress listener: ${error}`, 'install')
  }

  // Listen for source deletion skipped events
  try {
    unlistenDeletionSkipped = await listen<string>('source-deletion-skipped', (event) => {
      const path = event.payload
      toast.info(t('home.sourceDeletionSkipped', { path }))
    })
  } catch (error) {
    logError(`Failed to setup source deletion skipped listener: ${error}`, 'install')
  }

  // Check if X-Plane is running
  const checkXPlaneRunning = async () => {
    try {
      isXPlaneRunning.value = await invoke<boolean>('is_xplane_running')
    } catch (error) {
      logDebug(`Failed to check X-Plane running status: ${error}`, 'app')
    }
  }
  await checkXPlaneRunning()
  xplaneCheckInterval = window.setInterval(checkXPlaneRunning, 3000)
})

onBeforeUnmount(() => {
  window.removeEventListener('dragover', onWindowDragOver)
  window.removeEventListener('dragleave', onWindowDragLeave)
  window.removeEventListener('drop', onWindowDrop)
  window.removeEventListener('focus', onWindowFocus)
  window.removeEventListener('blur', onWindowBlur)

  activeTimeoutIds.forEach((id) => clearTimeout(id))
  activeTimeoutIds.clear()

  if (unlistenDragDrop) unlistenDragDrop()
  if (unlistenProgress) unlistenProgress()
  if (unlistenDeletionSkipped) unlistenDeletionSkipped()
  if (xplaneCheckInterval !== null) clearInterval(xplaneCheckInterval)
})

async function analyzeFiles(paths: string[], passwords?: Record<string, string>) {
  logOperation(t('log.filesDropped'), t('log.fileCount', { count: paths.length }))

  if (!passwords || Object.keys(passwords).length === 0) {
    passwordRetryCount.value = 0
  }

  if (!store.xplanePath) {
    toast.warning(t('home.pathNotSet'))
    return
  }

  store.isAnalyzing = true
  try {
    const result = await invoke<AnalysisResult>('analyze_addons', {
      paths,
      xplanePath: store.xplanePath,
      passwords: passwords || null,
      verificationPreferences: store.verificationPreferences,
    })

    const nestedRequiredPaths = result.nestedPasswordRequired
      ? Object.keys(result.nestedPasswordRequired)
      : []
    const allRequiredPaths = [...(result.passwordRequired || []), ...nestedRequiredPaths]

    if (allRequiredPaths.length > 0) {
      pendingAnalysisPaths.value = paths
      passwordRequiredPaths.value = allRequiredPaths
      if (passwords) collectedPasswords.value = { ...passwords }
      showPasswordModal.value = true
      store.isAnalyzing = false
      return
    }

    if (result.errors.length > 0) {
      const passwordErrors = result.errors.filter(
        (err) => err.includes('Wrong password') || err.toLowerCase().includes('wrong password'),
      )
      if (passwordErrors.length > 0 && passwords && Object.keys(passwords).length > 0) {
        passwordRetryCount.value++
        if (passwordRetryCount.value >= MAX_PASSWORD_RETRIES) {
          modal.showError(t('password.maxRetries') + '\n\n' + result.errors.join('\n'))
          resetPasswordState()
          store.isAnalyzing = false
          return
        }
        const wrongPasswordPaths = extractWrongPasswordPaths(passwordErrors)
        if (wrongPasswordPaths.length > 0) passwordRequiredPaths.value = wrongPasswordPaths
        passwordErrorMessage.value = t('password.wrongPassword')
        showPasswordModal.value = true
        store.isAnalyzing = false
        return
      }

      if (result.tasks.length > 0) {
        toast.warning(
          `${t('home.partialAnalysisWarning', { count: result.errors.length })} ${t('home.partialAnalysisHint')}`,
        )
      } else {
        modal.showError(result.errors.join('\n'))
        return
      }
    }

    if (result.tasks.length > 0) {
      const allowedTasks = result.tasks.filter((task) => {
        const effectiveType = task.type === AddonType.LuaScript ? AddonType.Plugin : task.type
        return store.installPreferences[effectiveType]
      })
      const ignoredCount = result.tasks.length - allowedTasks.length

      if (ignoredCount > 0) toast.info(t('home.ignoredTasks', { count: ignoredCount }))

      if (allowedTasks.length > 0) {
        if (showConfirmation.value) {
          const addedCount = store.appendTasks(allowedTasks)
          if (addedCount > 0) toast.success(t('home.tasksAppended', { count: addedCount }))
          else toast.info(t('home.duplicateTasksIgnored'))
        } else {
          store.setCurrentTasks(allowedTasks)
          showConfirmation.value = true
        }
        resetPasswordState()
      } else if (ignoredCount > 0) {
        toast.warning(t('home.allIgnored'))
      } else {
        toast.warning(t('home.noValidAddons'))
      }
    } else {
      toast.warning(t('home.noValidAddons'))
    }
  } catch (error) {
    logError(`${t('log.analysisFailed')}: ${error}`, 'analysis')
    modal.showError(t('home.failedToAnalyze') + ': ' + getErrorMessage(error))
  } finally {
    store.isAnalyzing = false
  }
}

async function handlePasswordSubmit(passwords: Record<string, string>) {
  const now = Date.now()
  const recentAttempts = passwordAttemptTimestamps.value.filter(
    (t) => now - t < PASSWORD_RATE_LIMIT_WINDOW_MS,
  )
  if (recentAttempts.length > 0) {
    const lastAttempt = Math.max(...recentAttempts)
    if (now - lastAttempt < MIN_PASSWORD_ATTEMPT_DELAY_MS) {
      toast.warning(t('password.tooFast'))
      return
    }
  }
  passwordAttemptTimestamps.value.push(now)
  showPasswordModal.value = false
  passwordErrorMessage.value = ''
  const allPasswords = { ...collectedPasswords.value, ...passwords }
  await analyzeFiles(pendingAnalysisPaths.value, allPasswords)
}

async function handlePasswordCancel() {
  showPasswordModal.value = false
  const nonPasswordPaths = pendingAnalysisPaths.value.filter(
    (p) => !passwordRequiredPaths.value.includes(p),
  )
  resetPasswordState()
  if (nonPasswordPaths.length > 0) await analyzeFiles(nonPasswordPaths)
}

function extractWrongPasswordPaths(errors: string[]): string[] {
  const paths: string[] = []
  for (const err of errors) {
    const match = err.match(/Wrong password for archive:\s*(.+)$/i)
    if (match && match[1]) paths.push(match[1].trim())
  }
  return paths.length > 0 ? paths : passwordRequiredPaths.value
}

function resetPasswordState() {
  pendingAnalysisPaths.value = []
  passwordRequiredPaths.value = []
  collectedPasswords.value = {}
  passwordRetryCount.value = 0
  passwordErrorMessage.value = ''
}

async function handleInstall() {
  showConfirmation.value = false
  const enabledTasks = store.currentTasks.filter((task) => store.getTaskEnabled(task.id))
  if (enabledTasks.length === 0) {
    toast.warning(t('home.noTasksEnabled'))
    return
  }

  store.setInstallingTasks(enabledTasks)
  store.isInstalling = true

  try {
    if (!lockStore.isInitialized) await lockStore.initStore()
    const allTasksWithSettings = store.getTasksWithOverwrite()
    const tasksWithOverwrite = allTasksWithSettings.filter((task) => store.getTaskEnabled(task.id))

    const result = await invoke<InstallResult>('install_addons', {
      tasks: tasksWithOverwrite,
      atomicInstallEnabled: store.atomicInstallEnabled,
      xplanePath: store.xplanePath,
      deleteSourceAfterInstall: store.deleteSourceAfterInstall,
      autoSortScenery: store.autoSortScenery,
      lockedSceneryFolderNames: lockStore.getLockedItems('scenery'),
      parallelEnabled: store.parallelInstallEnabled,
      maxParallel: store.maxParallelTasks,
    })

    progressStore.setPercentage(100)
    store.setInstallResult(result)
    setTrackedTimeout(
      () => {
        store.isInstalling = false
        progressStore.reset()
        // Refresh dashboard data after install
        activityLogStore.loadRecent()
        managementStore.loadAircraft()
        managementStore.loadPlugins()
        sceneryStore.loadData()
        diskUsageStore.scan()
      },
      COMPLETION_ANIMATION_DELAY_MS,
      activeTimeoutIds,
    )
  } catch (error) {
    logError(`${t('log.installationFailed')}: ${error}`, 'installation')
    modal.showError(t('home.installationFailed') + ': ' + getErrorMessage(error))
    store.isInstalling = false
    progressStore.reset()
  }
}

async function handleSkipTask() {
  const confirmed = await showConfirmDialog({
    title: t('taskControl.skipConfirmTitle'),
    message: t('taskControl.skipConfirmMessage'),
    warning: t('taskControl.skipWarningClean'),
    confirmText: t('taskControl.confirmSkip'),
    cancelText: t('common.cancel'),
    type: 'warning',
  })
  if (confirmed) {
    try {
      await invoke('skip_current_task')
      toast.info(t('taskControl.taskSkipped'))
    } catch (error) {
      modal.showError(getErrorMessage(error))
    }
  }
}

async function handleCancelInstallation() {
  const confirmed = await showConfirmDialog({
    title: t('taskControl.cancelConfirmTitle'),
    message: t('taskControl.cancelConfirmMessage'),
    warning: t('taskControl.cancelWarningClean'),
    confirmText: t('taskControl.confirmCancel'),
    cancelText: t('common.cancel'),
    type: 'danger',
  })
  if (confirmed) {
    try {
      await invoke('cancel_installation')
      toast.info(t('taskControl.tasksCancelled'))
    } catch (error) {
      modal.showError(getErrorMessage(error))
    }
  }
}

function showConfirmDialog(
  options: Omit<ConfirmOptions, 'onConfirm' | 'onCancel'>,
): Promise<boolean> {
  return new Promise((resolve) => {
    modal.showConfirm({
      ...options,
      onConfirm: () => resolve(true),
      onCancel: () => resolve(false),
    })
  })
}

async function handleLaunchXPlane() {
  if (isLaunchingXPlane.value || isXPlaneRunning.value) return
  await sceneryStore.loadData()
  if (sceneryStore.hasChanges) showLaunchConfirmDialog.value = true
  else await launchXPlane()
}

async function launchXPlane() {
  if (isLaunchingXPlane.value || isXPlaneRunning.value) return
  isLaunchingXPlane.value = true
  try {
    const args = store.xplaneLaunchArgs ? store.xplaneLaunchArgs.split(/\s+/).filter(Boolean) : []
    await invoke('launch_xplane', {
      xplanePath: store.xplanePath,
      args: args.length > 0 ? args : null,
    })
    await new Promise((resolve) => setTimeout(resolve, 5000))
    showLaunchConfirmDialog.value = false
  } catch (error) {
    modal.showError(t('home.launchFailed') + ': ' + getErrorMessage(error))
  } finally {
    isLaunchingXPlane.value = false
  }
}

async function applyAndLaunch() {
  if (isLaunchingXPlane.value) return
  isLaunchingXPlane.value = true
  try {
    await sceneryStore.applyChanges()
    isLaunchingXPlane.value = false
    await launchXPlane()
  } catch (error) {
    modal.showError(getErrorMessage(error))
    isLaunchingXPlane.value = false
  }
}

function cancelLaunchDialog() {
  showLaunchConfirmDialog.value = false
}

function handleCompletionConfirm() {
  store.clearInstallResult()
  store.clearTasks()
}

async function handleFixScenery() {
  try {
    toast.info(t('settings.sorting'))
    await invoke('sort_scenery_packs', { xplanePath: store.xplanePath })
    toast.success(t('settings.scenerySorted'))
    await sceneryStore.loadData()
  } catch (error) {
    modal.showError(t('settings.scenerySortFailed') + ': ' + getErrorMessage(error))
  }
}
</script>

<style scoped>
/* Custom Scrollbar for dashboard */
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(156, 163, 175, 0.3);
  border-radius: 10px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(156, 163, 175, 0.5);
}

/* debug drop visual */
.debug-drop {
  border-color: #10b981 !important; /* emerald */
  box-shadow: 0 0 30px rgba(16, 185, 129, 0.15) !important;
}

/* Animations */
@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes bounce-in {
  0% {
    opacity: 0;
    transform: scale(0.3);
  }
  50% {
    opacity: 1;
    transform: scale(1.05);
  }
  70% {
    transform: scale(0.9);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

.animate-fade-in {
  animation: fade-in 0.6s ease-out;
}

.animate-bounce-in {
  animation: bounce-in 0.8s ease-out;
}

/* Transition for warnings */
.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.3s ease-out;
}

.slide-down-enter-from,
.slide-down-leave-to {
  opacity: 0;
  transform: translateY(-20px);
}

/* Fade transition */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
