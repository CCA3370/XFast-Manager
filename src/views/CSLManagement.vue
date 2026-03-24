<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-1">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">
        {{ $t('csl.title') }}
      </h1>
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
    <!-- Stats -->
    <div v-if="hasPackages" class="flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400 mb-3">
      <span>{{ $t('csl.totalPackages') }}: <strong class="text-gray-900 dark:text-white">{{ combinedTotalPackages }}</strong></span>
      <span>{{ $t('csl.installed') }}: <strong class="text-green-600 dark:text-green-400">{{ combinedInstalledCount }}</strong></span>
      <span>{{ $t('csl.updatesAvailable') }}: <strong class="text-amber-600 dark:text-amber-400">{{ combinedUpdatesCount }}</strong></span>
      <span>{{ $t('csl.totalSize') }}: <strong class="text-gray-900 dark:text-white">{{ formatSize(totalSizeBytes) }}</strong></span>
    </div>
    <div v-else class="mb-3"></div>

    <!-- Search bar + Paths button -->
    <div v-if="hasPackages" class="flex items-center gap-2 mb-3">
      <div class="flex-1 relative">
        <input
          v-model="store.searchQuery"
          type="text"
          :placeholder="$t('csl.searchPlaceholder')"
          class="w-full px-3 py-1.5 pr-8 rounded-lg border border-gray-200 dark:border-gray-700
                 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-white
                 placeholder-gray-400 dark:placeholder-gray-500
                 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
        />
        <button
          v-if="store.searchQuery"
          class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-sm"
          @click="store.searchQuery = ''"
        >&#x2715;</button>
      </div>
      <button
        v-if="store.paths.length > 0 || store.customPaths.length > 0"
        class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700
               text-sm text-gray-700 dark:text-gray-300
               hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors
               flex items-center gap-1.5 flex-shrink-0"
        @click="showPathsDialog = true"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        {{ $t('csl.cslPaths') }}
        <span class="text-xs text-gray-400 dark:text-gray-500">({{ store.paths.length + store.customPaths.length }})</span>
      </button>
    </div>

    <!-- Filter tabs -->
    <div
      v-if="hasPackages"
      class="flex items-center gap-1 mb-3"
    >
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
        <span
          v-if="f.count !== undefined"
          class="ml-1 text-xs opacity-70"
        >({{ f.count }})</span>
      </button>

      <!-- Batch actions -->
      <div class="ml-auto flex items-center gap-2">
        <button
          v-if="combinedNotUpToDateCount > 0"
          class="text-xs px-3 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50"
          :disabled="isAnyInstalling"
          @click="installAll"
        >{{ $t('csl.installAll') }} ({{ combinedNotUpToDateCount }})</button>
      </div>
    </div>

    <!-- Package list (ALTITUDE first, then CSL) -->
    <div
      v-if="hasPackages"
      class="flex-1 overflow-y-auto space-y-1.5"
    >
      <div
        v-for="pkg in filteredPackages"
        :key="pkg.source + '-' + pkg.name"
        class="flex items-center gap-3 px-4 py-2.5 rounded-lg border transition-colors"
        :class="pkg.source === 'altitude'
          ? 'border-indigo-200 dark:border-indigo-800/60 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/10'
          : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/30'"
      >
        <!-- Package info -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span
              v-if="pkg.source === 'altitude'"
              class="px-1.5 py-0.5 rounded text-xs font-medium bg-indigo-100 dark:bg-indigo-900/40 text-indigo-700 dark:text-indigo-300"
            >ALTITUDE</span>
            <span v-if="pkg.source !== 'altitude'" class="font-medium text-gray-900 dark:text-white text-sm">{{ pkg.name }}</span>
            <span
              v-if="pkg.description && pkg.description !== pkg.name && pkg.source !== 'altitude'"
              class="text-xs text-gray-500 dark:text-gray-400 ml-1"
            >{{ pkg.description }}</span>
            <span
              class="px-1.5 py-0.5 rounded text-xs font-medium"
              :class="statusClass(pkg.status)"
            >{{ statusLabel(pkg.status) }}</span>
            <span class="text-xs text-gray-400 dark:text-gray-500">{{ formatSize(pkg.total_size_bytes) }}</span>
            <span v-if="pkg.status === 'needs_update'" class="text-xs text-amber-500 dark:text-amber-400">
              ↓ {{ formatSize(pkg.update_size_bytes) }}
            </span>
          </div>
        </div>

        <!-- Active install -->
        <div
          v-if="isInstalling(pkg)"
          class="flex items-center gap-2 text-xs flex-shrink-0"
          :class="pkg.source === 'altitude' ? 'text-indigo-600 dark:text-indigo-400' : 'text-blue-600 dark:text-blue-400'"
        >
          <div
            class="w-4 h-4 border-2 border-t-transparent rounded-full animate-spin"
            :class="pkg.source === 'altitude' ? 'border-indigo-600' : 'border-blue-600'"
          ></div>
          <span v-if="getProgress(pkg)">{{ getProgress(pkg)!.current_file }}/{{ getProgress(pkg)!.total_files }}</span>
          <button
            class="px-2 py-1 rounded border text-xs transition-colors disabled:opacity-50"
            :class="pkg.source === 'altitude'
              ? 'border-indigo-200 dark:border-indigo-700 text-indigo-700 dark:text-indigo-300 hover:bg-indigo-50 dark:hover:bg-indigo-900/20'
              : 'border-blue-200 dark:border-blue-700 text-blue-700 dark:text-blue-300 hover:bg-blue-50 dark:hover:bg-blue-900/20'"
            :disabled="isCancelling(pkg)"
            @click="cancelInstall(pkg)"
          >{{ $t('common.cancel') }}</button>
        </div>

        <!-- Queued -->
        <div
          v-else-if="isQueued(pkg)"
          class="flex items-center gap-2 text-xs flex-shrink-0 text-gray-500 dark:text-gray-400"
        >
          <span class="px-2 py-1 rounded bg-gray-100 dark:bg-gray-800">{{ $t('home.waiting') }}</span>
          <button
            class="px-2 py-1 rounded border border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            @click="cancelInstall(pkg)"
          >{{ $t('common.cancel') }}</button>
        </div>

        <!-- Actions -->
        <div v-else class="flex items-center gap-2 flex-shrink-0">
          <button
            v-if="pkg.status === 'not_installed'"
            class="text-xs px-3 py-1.5 rounded-lg text-white transition-colors"
            :class="pkg.source === 'altitude' ? 'bg-indigo-600 hover:bg-indigo-700' : 'bg-blue-600 hover:bg-blue-700'"
            @click="handleInstall(pkg)"
          >{{ $t('csl.install') }}</button>

          <button
            v-if="pkg.status === 'needs_update'"
            class="text-xs px-3 py-1.5 rounded-lg bg-amber-600 text-white hover:bg-amber-700 transition-colors"
            @click="handleInstall(pkg)"
          >{{ $t('csl.update') }}</button>

          <button
            v-if="pkg.status !== 'not_installed'"
            class="text-xs px-3 py-1.5 rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors disabled:opacity-50"
            :disabled="isAnyInstalling"
            @click="handleUninstall(pkg)"
          >{{ $t('csl.uninstall') }}</button>
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
    <div
      v-else-if="!isAnyLoading"
      class="flex-1 flex items-center justify-center"
    >
      <div class="text-center">
        <svg class="w-16 h-16 mx-auto text-gray-300 dark:text-gray-600 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
        </svg>
        <p class="text-gray-500 dark:text-gray-400 text-sm mb-3">{{ $t('csl.noPathsDetected') }}</p>
        <button
          class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors"
          @click="scanAll"
        >{{ $t('csl.refresh') }}</button>
      </div>
    </div>

    <!-- Paths Dialog -->
    <Teleport to="body">
    <div
      v-if="showPathsDialog"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="showPathsDialog = false"
    >
      <div class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden">
        <div class="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-700">
          <h3 class="text-base font-semibold text-gray-900 dark:text-white">{{ $t('csl.cslPaths') }}</h3>
          <button
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-lg"
            @click="showPathsDialog = false"
          >&#x2715;</button>
        </div>
        <div class="px-5 py-4 space-y-2 max-h-80 overflow-y-auto">
          <div
            v-for="p in store.paths"
            :key="p.path"
            class="flex items-center gap-2 text-sm"
          >
            <span class="px-1.5 py-0.5 rounded text-xs bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 flex-shrink-0">{{ $t('csl.autoDetected') }}</span>
            <span v-if="p.plugin_name" class="text-gray-500 dark:text-gray-400 flex-shrink-0">{{ p.plugin_name }}:</span>
            <span class="text-gray-700 dark:text-gray-300 truncate">{{ relativePath(p.path) }}</span>
          </div>
          <div
            v-for="cp in store.customPaths"
            :key="cp"
            class="flex items-center gap-2 text-sm"
          >
            <span class="px-1.5 py-0.5 rounded text-xs bg-purple-100 dark:bg-purple-900/40 text-purple-700 dark:text-purple-300 flex-shrink-0">{{ $t('csl.custom') }}</span>
            <span class="text-gray-700 dark:text-gray-300 truncate flex-1">{{ relativePath(cp) }}</span>
            <button
              class="text-red-500 hover:text-red-600 text-xs flex-shrink-0"
              @click="store.removeCustomPath(cp)"
            >{{ $t('csl.removePath') }}</button>
          </div>
          <div v-if="store.paths.length === 0 && store.customPaths.length === 0"
               class="text-sm text-gray-400 dark:text-gray-500 text-center py-4">
            {{ $t('csl.noPathsDetected') }}
          </div>
        </div>
        <div class="px-5 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-between">
          <button
            class="text-sm text-blue-600 dark:text-blue-400 hover:underline"
            @click="addCustomPath"
          >+ {{ $t('csl.addPath') }}</button>
          <button
            class="text-sm px-4 py-1.5 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
            @click="showPathsDialog = false"
          >{{ $t('common.close') }}</button>
        </div>
      </div>
    </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useCslStore } from '@/stores/csl'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import type { CslPackageInfo, CslProgress } from '@/types'

interface DisplayPackage extends CslPackageInfo {
  source: 'csl' | 'altitude'
}

const { t } = useI18n()
const store = useCslStore()
const appStore = useAppStore()
const modal = useModalStore()

const activeFilter = ref<string>('all')
const showPathsDialog = ref(false)

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

let unlistenProgress: UnlistenFn | null = null
let unlistenAltitudeProgress: UnlistenFn | null = null

// Combined packages: ALTITUDE first, then CSL
const combinedPackages = computed<DisplayPackage[]>(() => {
  const altPkgs: DisplayPackage[] = store.altitudePackages.map(p => ({ ...p, source: 'altitude' }))
  const cslPkgs: DisplayPackage[] = store.packages.map(p => ({ ...p, source: 'csl' }))
  return [...altPkgs, ...cslPkgs]
})

const hasPackages = computed(() => store.allScansDone && combinedPackages.value.length > 0)
const isAnyLoading = computed(() => store.isLoading || store.altitudeLoading)
const isAnyInstalling = computed(() => store.hasPendingInstalls)

const combinedTotalPackages = computed(() => store.totalPackages + store.altitudeTotalPackages)
const combinedInstalledCount = computed(() => store.installedCount + store.altitudeInstalledCount)
const combinedUpdatesCount = computed(() => store.updatesCount + store.altitudeUpdatesCount)
const combinedNotUpToDateCount = computed(() => store.notUpToDateCount + store.altitudeNotUpToDateCount)

const filters = computed(() => [
  { value: 'all', label: t('csl.filterAll'), count: combinedTotalPackages.value },
  { value: 'not_installed', label: t('csl.filterNotInstalled'), count: combinedPackages.value.filter(p => p.status === 'not_installed').length },
  { value: 'needs_update', label: t('csl.filterNeedsUpdate'), count: combinedUpdatesCount.value },
  { value: 'up_to_date', label: t('csl.filterUpToDate'), count: combinedPackages.value.filter(p => p.status === 'up_to_date').length },
])

const filteredPackages = computed(() => {
  let result = combinedPackages.value
  if (activeFilter.value !== 'all') {
    result = result.filter(p => p.status === activeFilter.value)
  }
  const q = store.searchQuery.trim().toLowerCase()
  if (q) {
    result = result.filter(p =>
      p.name.toLowerCase().includes(q) ||
      (p.description && p.description.toLowerCase().includes(q))
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

onMounted(async () => {
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
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
  if (unlistenAltitudeProgress) {
    unlistenAltitudeProgress()
  }
})
</script>
