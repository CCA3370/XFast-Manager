<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-1">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">
        {{ $t('csl.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <button
          v-if="appStore.isXcslDev"
          class="text-sm px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors disabled:opacity-50 flex items-center gap-2"
          :disabled="isAnyLoading || isAnyInstalling"
          @click="openServerSettingsDialog"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h8" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 6h4" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 12h3" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 12h9" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 18h10" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18 18h2" />
            <circle cx="14" cy="6" r="2" stroke-width="2" />
            <circle cx="9" cy="12" r="2" stroke-width="2" />
            <circle cx="16" cy="18" r="2" stroke-width="2" />
          </svg>
          {{ $t('csl.serverSettings') }}
        </button>
        <button
          class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-2"
          :disabled="isAnyLoading || isAnyInstalling"
          @click="scanAll"
        >
          <div
            v-if="isAnyLoading"
            class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
          ></div>
          {{ isAnyLoading ? $t('csl.scanning') : $t('csl.refresh') }}
        </button>
      </div>
    </div>
    <!-- Stats -->
    <div
      v-if="hasPackages"
      class="flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400 mb-3"
    >
      <span
        >{{ $t('csl.totalPackages') }}:
        <strong class="text-gray-900 dark:text-white">{{ combinedTotalPackages }}</strong></span
      >
      <span
        >{{ $t('csl.installed') }}:
        <strong class="text-green-600 dark:text-green-400">{{
          combinedInstalledCount
        }}</strong></span
      >
      <span
        >{{ $t('csl.updatesAvailable') }}:
        <strong class="text-amber-600 dark:text-amber-400">{{ combinedUpdatesCount }}</strong></span
      >
      <span
        >{{ $t('csl.totalSize') }}:
        <strong class="text-gray-900 dark:text-white">{{
          formatSize(totalSizeBytes)
        }}</strong></span
      >
    </div>
    <div v-else class="mb-3"></div>

    <!-- Search bar + Paths button -->
    <div v-if="hasPackages" class="flex items-center gap-2 mb-3">
      <div class="flex-1 relative">
        <input
          v-model="store.searchQuery"
          type="text"
          :placeholder="$t('csl.searchPlaceholder')"
          class="w-full px-3 py-1.5 pr-8 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
        />
        <button
          v-if="store.searchQuery"
          class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-sm"
          @click="store.searchQuery = ''"
        >
          &#x2715;
        </button>
      </div>
      <button
        v-if="store.paths.length > 0 || store.customPaths.length > 0"
        class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors flex items-center gap-1.5 flex-shrink-0"
        @click="showPathsDialog = true"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
          />
        </svg>
        {{ $t('csl.cslPaths') }}
        <span class="text-xs text-gray-400 dark:text-gray-500"
          >({{ store.paths.length + store.customPaths.length }})</span
        >
      </button>
    </div>

    <!-- Filter tabs -->
    <div v-if="hasPackages" class="flex items-center gap-1 mb-3">
      <button
        v-for="f in filters"
        :key="f.value"
        class="px-3 py-1.5 rounded-lg text-sm transition-colors"
        :class="
          activeFilter === f.value
            ? 'bg-blue-600 text-white'
            : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
        "
        @click="activeFilter = f.value"
      >
        {{ f.label }}
        <span v-if="f.count !== undefined" class="ml-1 text-xs opacity-70">({{ f.count }})</span>
      </button>

      <!-- Batch actions -->
      <div class="ml-auto flex items-center gap-2">
        <button
          v-if="combinedNotUpToDateCount > 0"
          class="text-xs px-3 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50"
          :disabled="isAnyInstalling"
          @click="installAll"
        >
          {{ $t('csl.installAll') }} ({{ combinedNotUpToDateCount }})
        </button>
      </div>
    </div>

    <!-- Package list (ALTITUDE first, then CSL) -->
    <div v-if="hasPackages" ref="packageListRef" class="flex-1 overflow-y-auto space-y-1.5">
      <div
        v-for="pkg in filteredPackages"
        :key="pkg.source + '-' + pkg.name"
        :data-package-name="pkg.name"
        :data-package-source="pkg.source"
        :data-csl-description-pending="
          pkg.source === 'csl' && isDescriptionPending(pkg.name) ? 'true' : 'false'
        "
        class="flex items-center gap-3 px-4 py-2.5 rounded-lg border transition-colors"
        :class="
          pkg.source === 'altitude'
            ? 'border-indigo-200 dark:border-indigo-800/60 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/10'
            : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/30'
        "
      >
        <!-- Package info -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span
              v-if="pkg.source === 'altitude'"
              class="px-1.5 py-0.5 rounded text-xs font-medium bg-indigo-100 dark:bg-indigo-900/40 text-indigo-700 dark:text-indigo-300"
              >ALTITUDE</span
            >
            <span
              v-if="pkg.source !== 'altitude'"
              class="font-medium text-gray-900 dark:text-white text-sm"
              >{{ pkg.name }}</span
            >
            <span
              v-if="pkg.description && pkg.description !== pkg.name && pkg.source !== 'altitude'"
              class="text-xs text-gray-500 dark:text-gray-400 ml-1"
              >{{ pkg.description }}</span
            >
            <span
              v-else-if="pkg.source !== 'altitude' && isDescriptionPending(pkg.name)"
              class="ml-1 inline-flex items-center gap-2"
            >
              <span class="h-3 w-20 rounded-full bg-gray-200 dark:bg-gray-700 animate-pulse"></span>
              <span
                class="h-3 w-3 rounded-full border-2 border-gray-300 dark:border-gray-600 border-t-transparent animate-spin"
              ></span>
            </span>
            <span
              class="px-1.5 py-0.5 rounded text-xs font-medium"
              :class="statusClass(pkg.status)"
              >{{ statusLabel(pkg.status) }}</span
            >
            <span class="text-xs text-gray-400 dark:text-gray-500">{{
              formatSize(pkg.total_size_bytes)
            }}</span>
            <span
              v-if="pkg.status === 'needs_update'"
              class="text-xs text-amber-500 dark:text-amber-400"
            >
              ↓ {{ formatSize(pkg.update_size_bytes) }}
            </span>
          </div>
        </div>

        <!-- Active install -->
        <div
          v-if="isInstalling(pkg)"
          class="flex items-center gap-2 text-xs flex-shrink-0"
          :class="
            pkg.source === 'altitude'
              ? 'text-indigo-600 dark:text-indigo-400'
              : 'text-blue-600 dark:text-blue-400'
          "
        >
          <div
            class="w-4 h-4 border-2 border-t-transparent rounded-full animate-spin"
            :class="pkg.source === 'altitude' ? 'border-indigo-600' : 'border-blue-600'"
          ></div>
          <span v-if="getProgress(pkg)"
            >{{ getProgress(pkg)!.current_file }}/{{ getProgress(pkg)!.total_files }}</span
          >
          <button
            class="px-2 py-1 rounded border text-xs transition-colors disabled:opacity-50"
            :class="
              pkg.source === 'altitude'
                ? 'border-indigo-200 dark:border-indigo-700 text-indigo-700 dark:text-indigo-300 hover:bg-indigo-50 dark:hover:bg-indigo-900/20'
                : 'border-blue-200 dark:border-blue-700 text-blue-700 dark:text-blue-300 hover:bg-blue-50 dark:hover:bg-blue-900/20'
            "
            :disabled="isCancelling(pkg)"
            @click="cancelInstall(pkg)"
          >
            {{ $t('common.cancel') }}
          </button>
        </div>

        <!-- Queued -->
        <div
          v-else-if="isQueued(pkg)"
          class="flex items-center gap-2 text-xs flex-shrink-0 text-gray-500 dark:text-gray-400"
        >
          <span class="px-2 py-1 rounded bg-gray-100 dark:bg-gray-800">{{
            $t('home.waiting')
          }}</span>
          <button
            class="px-2 py-1 rounded border border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            @click="cancelInstall(pkg)"
          >
            {{ $t('common.cancel') }}
          </button>
        </div>

        <!-- Actions -->
        <div v-else class="flex items-center gap-2 flex-shrink-0">
          <button
            v-if="pkg.status === 'not_installed'"
            class="text-xs px-3 py-1.5 rounded-lg text-white transition-colors"
            :class="
              pkg.source === 'altitude'
                ? 'bg-indigo-600 hover:bg-indigo-700'
                : 'bg-blue-600 hover:bg-blue-700'
            "
            @click="handleInstall(pkg)"
          >
            {{ $t('csl.install') }}
          </button>

          <button
            v-if="pkg.status === 'needs_update'"
            class="text-xs px-3 py-1.5 rounded-lg bg-amber-600 text-white hover:bg-amber-700 transition-colors"
            @click="handleInstall(pkg)"
          >
            {{ $t('csl.update') }}
          </button>

          <button
            v-if="pkg.status !== 'not_installed'"
            class="text-xs px-3 py-1.5 rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors disabled:opacity-50"
            :disabled="isAnyInstalling"
            @click="handleUninstall(pkg)"
          >
            {{ $t('csl.uninstall') }}
          </button>
        </div>
      </div>

      <div
        v-if="filteredPackages.length === 0"
        class="text-center text-gray-400 dark:text-gray-500 py-12 text-sm"
      >
        {{ $t('csl.noPackages') }}
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!isAnyLoading" class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto text-gray-300 dark:text-gray-600 mb-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
          />
        </svg>
        <p class="text-gray-500 dark:text-gray-400 text-sm mb-3">{{ $t('csl.noPathsDetected') }}</p>
        <button
          class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors"
          @click="scanAll"
        >
          {{ $t('csl.refresh') }}
        </button>
      </div>
    </div>

    <!-- Paths Dialog -->
    <Teleport to="body">
      <div
        v-if="showPathsDialog"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click.self="showPathsDialog = false"
      >
        <div
          class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden"
        >
          <div
            class="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-700"
          >
            <h3 class="text-base font-semibold text-gray-900 dark:text-white">
              {{ $t('csl.cslPaths') }}
            </h3>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-lg"
              @click="showPathsDialog = false"
            >
              &#x2715;
            </button>
          </div>
          <div class="px-5 py-4 space-y-2 max-h-80 overflow-y-auto">
            <div v-for="p in store.paths" :key="p.path" class="flex items-center gap-2 text-sm">
              <span
                class="px-1.5 py-0.5 rounded text-xs bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 flex-shrink-0"
                >{{ $t('csl.autoDetected') }}</span
              >
              <span v-if="p.plugin_name" class="text-gray-500 dark:text-gray-400 flex-shrink-0"
                >{{ p.plugin_name }}:</span
              >
              <span class="text-gray-700 dark:text-gray-300 truncate">{{
                relativePath(p.path)
              }}</span>
            </div>
            <div v-for="cp in store.customPaths" :key="cp" class="flex items-center gap-2 text-sm">
              <span
                class="px-1.5 py-0.5 rounded text-xs bg-purple-100 dark:bg-purple-900/40 text-purple-700 dark:text-purple-300 flex-shrink-0"
                >{{ $t('csl.custom') }}</span
              >
              <span class="text-gray-700 dark:text-gray-300 truncate flex-1">{{
                relativePath(cp)
              }}</span>
              <button
                class="text-red-500 hover:text-red-600 text-xs flex-shrink-0"
                @click="store.removeCustomPath(cp)"
              >
                {{ $t('csl.removePath') }}
              </button>
            </div>
            <div
              v-if="store.paths.length === 0 && store.customPaths.length === 0"
              class="text-sm text-gray-400 dark:text-gray-500 text-center py-4"
            >
              {{ $t('csl.noPathsDetected') }}
            </div>
          </div>
          <div class="px-5 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-between">
            <button
              class="text-sm text-blue-600 dark:text-blue-400 hover:underline"
              @click="addCustomPath"
            >
              + {{ $t('csl.addPath') }}
            </button>
            <button
              class="text-sm px-4 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
              @click="showPathsDialog = false"
            >
              {{ $t('common.close') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div
        v-if="showServerSettingsDialog"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-sm"
        @click.self="closeServerSettingsDialog"
      >
        <div
          class="bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-full max-w-xl mx-4 overflow-hidden border border-gray-200/80 dark:border-gray-800"
        >
          <div
            class="flex items-start justify-between px-4 py-4 border-b border-gray-200 dark:border-gray-800"
          >
            <div class="min-w-0">
              <h3 class="text-base font-semibold text-gray-900 dark:text-white">
                {{ $t('csl.serverSettingsTitle') }}
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {{ $t('csl.serverSettingsHint') }}
              </p>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-lg h-8 w-8 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors flex items-center justify-center"
              @click="closeServerSettingsDialog"
            >
              &#x2715;
            </button>
          </div>
          <div class="px-4 py-4 space-y-3 max-h-[22rem] overflow-y-auto">
            <div
              v-for="entry in serverEntries"
              :key="entry.id"
              class="rounded-xl border p-3 transition-all cursor-pointer"
              :class="
                isServerEntrySelected(entry.id)
                  ? 'border-blue-300 bg-blue-50/60 dark:border-blue-700 dark:bg-blue-950/20'
                  : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
              "
              @click="selectServerEntry(entry.id)"
            >
              <div class="flex items-center gap-3">
                <input
                  :id="`csl-server-${entry.id}`"
                  v-model="selectedServerEntryId"
                  type="radio"
                  class="h-4 w-4 border-gray-300 text-blue-600 focus:ring-blue-500 flex-shrink-0"
                  :value="entry.id"
                  @click.stop
                />
                <input
                  v-model="entry.url"
                  type="url"
                  :placeholder="$t('csl.serverBaseUrlPlaceholder')"
                  class="flex-1 min-w-0 px-3 py-2 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                  @click.stop
                />
                <button
                  class="h-9 px-3 rounded-lg border border-red-200 dark:border-red-800/60 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors disabled:opacity-50 flex items-center justify-center text-xs"
                  :disabled="serverEntries.length <= 1"
                  @click.stop="removeServerEntry(entry.id)"
                >
                  {{ $t('csl.removeServer') }}
                </button>
              </div>
            </div>

            <button
              class="w-full rounded-xl border border-dashed border-gray-300 dark:border-gray-700 px-4 py-3 text-sm font-medium text-blue-600 dark:text-blue-400 hover:border-blue-300 hover:bg-blue-50/50 dark:hover:border-blue-700 dark:hover:bg-blue-950/20 transition-colors disabled:opacity-50"
              :disabled="serverEntries.length >= MAX_CSL_SERVER_BASE_URLS"
              @click="addServerEntry"
            >
              + {{ $t('csl.addServer') }}
            </button>

            <p
              v-if="serverSettingsError"
              class="rounded-xl border border-red-200 dark:border-red-900/60 bg-red-50 dark:bg-red-950/20 px-3 py-2 text-sm text-red-600 dark:text-red-300"
            >
              {{ serverSettingsError }}
            </p>
          </div>
          <div
            class="px-4 py-3 border-t border-gray-200 dark:border-gray-800 flex items-center justify-end gap-2 bg-gray-50/70 dark:bg-gray-950/20"
          >
            <button
              class="text-sm px-4 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
              @click="closeServerSettingsDialog"
            >
              {{ $t('common.close') }}
            </button>
            <button
              class="text-sm px-4 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors shadow-sm shadow-blue-600/20"
              @click="saveServerSettings"
            >
              {{ $t('common.save') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useCslStore } from '@/stores/csl'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { getErrorMessage, type CslPackageInfo, type CslProgress } from '@/types'

interface DisplayPackage extends CslPackageInfo {
  source: 'csl' | 'altitude'
}

interface EditableServerEntry {
  id: number
  url: string
}

const MAX_CSL_SERVER_BASE_URLS = 4

const { t } = useI18n()
const store = useCslStore()
const appStore = useAppStore()
const modal = useModalStore()

const activeFilter = ref<string>('all')
const showPathsDialog = ref(false)
const showServerSettingsDialog = ref(false)
const packageListRef = ref<HTMLElement | null>(null)
const serverEntries = ref<EditableServerEntry[]>([])
const selectedServerEntryId = ref<number | null>(null)
const serverSettingsError = ref('')

let nextServerEntryId = 0
let descriptionObserver: IntersectionObserver | null = null

/** Show path relative to X-Plane root if it starts with xplanePath */
function relativePath(fullPath: string): string {
  const xp = appStore.xplanePath
  if (!xp) return fullPath
  // Normalize separators for comparison
  const norm = (s: string) => s.replace(/\\/g, '/').replace(/\/+$/, '')
  const normXp = norm(xp)
  const normFull = norm(fullPath)
  if (normFull.startsWith(normXp + '/')) {
    return normFull.slice(normXp.length + 1)
  }
  return fullPath
}

function createServerEntry(url: string): EditableServerEntry {
  nextServerEntryId += 1
  return {
    id: nextServerEntryId,
    url,
  }
}

let unlistenProgress: UnlistenFn | null = null
let unlistenAltitudeProgress: UnlistenFn | null = null

// Combined packages: ALTITUDE first, then CSL
const combinedPackages = computed<DisplayPackage[]>(() => {
  const altPkgs: DisplayPackage[] = store.altitudePackages.map((p) => ({
    ...p,
    source: 'altitude',
  }))
  const cslPkgs: DisplayPackage[] = store.packages.map((p) => ({ ...p, source: 'csl' }))
  return [...altPkgs, ...cslPkgs]
})

const hasPackages = computed(() => store.allScansDone && combinedPackages.value.length > 0)
const isAnyLoading = computed(() => store.isLoading || store.altitudeLoading)
const isAnyInstalling = computed(() => store.hasPendingInstalls)

const combinedTotalPackages = computed(() => store.totalPackages + store.altitudeTotalPackages)
const combinedInstalledCount = computed(() => store.installedCount + store.altitudeInstalledCount)
const combinedUpdatesCount = computed(() => store.updatesCount + store.altitudeUpdatesCount)
const combinedNotUpToDateCount = computed(
  () => store.notUpToDateCount + store.altitudeNotUpToDateCount,
)

const filters = computed(() => [
  { value: 'all', label: t('csl.filterAll'), count: combinedTotalPackages.value },
  {
    value: 'not_installed',
    label: t('csl.filterNotInstalled'),
    count: combinedPackages.value.filter((p) => p.status === 'not_installed').length,
  },
  { value: 'needs_update', label: t('csl.filterNeedsUpdate'), count: combinedUpdatesCount.value },
  {
    value: 'up_to_date',
    label: t('csl.filterUpToDate'),
    count: combinedPackages.value.filter((p) => p.status === 'up_to_date').length,
  },
])

const filteredPackages = computed(() => {
  let result = combinedPackages.value
  if (activeFilter.value !== 'all') {
    result = result.filter((p) => p.status === activeFilter.value)
  }
  const q = store.searchQuery.trim().toLowerCase()
  if (q) {
    result = result.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        (p.description && p.description.toLowerCase().includes(q)),
    )
  }
  return result
})

const totalSizeBytes = computed(() =>
  combinedPackages.value.reduce((sum, p) => sum + p.total_size_bytes, 0),
)

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + ' ' + units[i]
}

function statusClass(status: string): string {
  switch (status) {
    case 'not_installed':
      return 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400'
    case 'needs_update':
      return 'bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300'
    case 'up_to_date':
      return 'bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-300'
    default:
      return 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400'
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'not_installed':
      return t('csl.notInstalled')
    case 'needs_update':
      return t('csl.needsUpdate')
    case 'up_to_date':
      return t('csl.upToDate')
    default:
      return status
  }
}

function isInstalling(pkg: DisplayPackage): boolean {
  if (pkg.source === 'altitude') {
    return store.altitudeInstallingPackages.includes(pkg.name)
  }
  return store.installingPackages.includes(pkg.name)
}

function isQueued(pkg: DisplayPackage): boolean {
  if (pkg.source === 'altitude') {
    return store.queuedAltitudePackages.includes(pkg.name)
  }
  return store.queuedPackages.includes(pkg.name)
}

function isCancelling(pkg: DisplayPackage): boolean {
  if (pkg.source === 'altitude') {
    return store.cancellingAltitudePackages.includes(pkg.name)
  }
  return store.cancellingPackages.includes(pkg.name)
}

function getProgress(pkg: DisplayPackage): CslProgress | undefined {
  if (pkg.source === 'altitude') {
    return store.altitudeProgressMap[pkg.name]
  }
  return store.progressMap[pkg.name]
}

function isDescriptionPending(packageName: string): boolean {
  return store.isDescriptionPending(packageName)
}

function handleInstall(pkg: DisplayPackage) {
  if (pkg.source === 'altitude') {
    store.installAltitudePackage()
  } else {
    store.installPackage(pkg.name)
  }
}

function cancelInstall(pkg: DisplayPackage) {
  void store.cancelInstall(pkg.source, pkg.name)
}

function handleUninstall(pkg: DisplayPackage) {
  const i18nPrefix = pkg.source === 'altitude' ? 'altitude' : 'csl'
  modal.showConfirm({
    title: t(`${i18nPrefix}.uninstall`),
    message: t(`${i18nPrefix}.confirmUninstall`, { name: pkg.name }),
    confirmText: t(`${i18nPrefix}.uninstall`),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: () => {
      if (pkg.source === 'altitude') {
        store.uninstallAltitudePackage()
      } else {
        store.uninstallPackage(pkg.name)
      }
    },
    onCancel: () => {},
  })
}

function scanAll() {
  store.scanPackages()
  store.scanAltitudePackages()
}

function installAll() {
  store.installAllCombined(
    combinedPackages.value
      .filter((pkg) => pkg.status !== 'up_to_date')
      .map((pkg) => ({ source: pkg.source, name: pkg.name })),
  )
}

async function addCustomPath() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('csl.selectTargetPath'),
  })

  if (selected && typeof selected === 'string') {
    store.addCustomPath(selected)
  }
}

async function openServerSettingsDialog() {
  await store.ensureServerConfigLoaded()

  serverEntries.value = store.serverBaseUrls.map((url) => createServerEntry(url))
  selectedServerEntryId.value =
    serverEntries.value.find((entry) => entry.url === store.activeServerBaseUrl)?.id ??
    serverEntries.value[0]?.id ??
    null
  serverSettingsError.value = ''
  showServerSettingsDialog.value = true
}

function closeServerSettingsDialog() {
  showServerSettingsDialog.value = false
  serverSettingsError.value = ''
}

function addServerEntry() {
  if (serverEntries.value.length >= MAX_CSL_SERVER_BASE_URLS) {
    return
  }

  const entry = createServerEntry('')
  serverEntries.value = [...serverEntries.value, entry]
  selectedServerEntryId.value ??= entry.id
  serverSettingsError.value = ''
}

function removeServerEntry(entryId: number) {
  if (serverEntries.value.length <= 1) {
    return
  }

  const wasSelected = selectedServerEntryId.value === entryId
  serverEntries.value = serverEntries.value.filter((entry) => entry.id !== entryId)

  if (wasSelected) {
    selectedServerEntryId.value = serverEntries.value[0]?.id ?? null
  }

  serverSettingsError.value = ''
}

function selectServerEntry(entryId: number) {
  selectedServerEntryId.value = entryId
}

function isServerEntrySelected(entryId: number): boolean {
  return selectedServerEntryId.value === entryId
}

async function saveServerSettings() {
  const selectedEntry = serverEntries.value.find(
    (entry) => entry.id === selectedServerEntryId.value,
  )

  if (!selectedEntry) {
    serverSettingsError.value = t('csl.serverSelectionRequired')
    return
  }

  try {
    await store.saveServerConfig(
      serverEntries.value.map((entry) => entry.url),
      selectedEntry.url,
    )
    closeServerSettingsDialog()
    scanAll()
  } catch (error) {
    serverSettingsError.value = getErrorMessage(error)
  }
}

function observeVisibleDescriptions() {
  if (!descriptionObserver || !packageListRef.value) {
    return
  }

  descriptionObserver.disconnect()

  const pendingNodes = packageListRef.value.querySelectorAll<HTMLElement>(
    '[data-csl-description-pending="true"]',
  )

  for (const node of pendingNodes) {
    descriptionObserver.observe(node)
  }
}

function setupDescriptionObserver() {
  if (!packageListRef.value) {
    return
  }

  descriptionObserver?.disconnect()
  descriptionObserver = new IntersectionObserver(
    (entries) => {
      const namesToLoad = entries
        .filter((entry) => entry.isIntersecting)
        .map((entry) => {
          descriptionObserver?.unobserve(entry.target)
          return (entry.target as HTMLElement).dataset.packageName
        })
        .filter((name): name is string => Boolean(name))

      if (namesToLoad.length > 0) {
        store.queuePackageDescriptions(namesToLoad)
      }
    },
    {
      root: packageListRef.value,
      threshold: 0.1,
    },
  )

  observeVisibleDescriptions()
}

watch(
  filteredPackages,
  async () => {
    await nextTick()
    if (!descriptionObserver) {
      setupDescriptionObserver()
      return
    }
    observeVisibleDescriptions()
  },
  { flush: 'post' },
)

onMounted(async () => {
  await store.ensureServerConfigLoaded()

  // Listen for CSL progress events
  unlistenProgress = await listen<CslProgress>('csl-progress', (event) => {
    store.updateProgress(event.payload)
  })

  // Listen for ALTITUDE progress events
  unlistenAltitudeProgress = await listen<CslProgress>('altitude-progress', (event) => {
    store.updateAltitudeProgress(event.payload)
  })

  // Auto-scan on mount
  if (store.packages.length === 0) {
    store.scanPackages()
  }
  if (store.altitudePackages.length === 0) {
    store.scanAltitudePackages()
  }

  await nextTick()
  setupDescriptionObserver()
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
  if (unlistenAltitudeProgress) {
    unlistenAltitudeProgress()
  }
  if (descriptionObserver) {
    descriptionObserver.disconnect()
    descriptionObserver = null
  }
})
</script>
