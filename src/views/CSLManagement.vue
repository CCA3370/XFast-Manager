<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-1">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">
        {{ $t('csl.title') }}
      </h1>
      <button
        class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-2"
        :disabled="store.isLoading"
        @click="store.scanPackages()"
      >
        <div
          v-if="store.isLoading"
          class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
        ></div>
        {{ store.isLoading ? $t('csl.scanning') : $t('csl.refresh') }}
      </button>
    </div>
    <!-- Stats -->
    <div v-if="store.packages.length > 0" class="flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400 mb-3">
      <span>{{ $t('csl.totalPackages') }}: <strong class="text-gray-900 dark:text-white">{{ store.totalPackages }}</strong></span>
      <span>{{ $t('csl.installed') }}: <strong class="text-green-600 dark:text-green-400">{{ store.installedCount }}</strong></span>
      <span>{{ $t('csl.updatesAvailable') }}: <strong class="text-amber-600 dark:text-amber-400">{{ store.updatesCount }}</strong></span>
      <span>{{ $t('csl.totalSize') }}: <strong class="text-gray-900 dark:text-white">{{ formatSize(totalSizeBytes) }}</strong></span>
    </div>
    <div v-else class="mb-3"></div>

    <!-- CSL Paths -->
    <div
      v-if="store.paths.length > 0 || store.customPaths.length > 0"
      class="mb-3"
    >
      <details class="rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
        <summary class="flex items-center justify-between px-3 py-2 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
          <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ $t('csl.cslPaths') }}</span>
          <span class="text-xs text-gray-400 dark:text-gray-500">{{ store.paths.length + store.customPaths.length }}</span>
        </summary>
        <div class="border-t border-gray-200 dark:border-gray-700 px-4 py-2 space-y-1.5">
          <div
            v-for="p in store.paths"
            :key="p.path"
            class="flex items-center gap-2 text-sm"
          >
            <span class="px-1.5 py-0.5 rounded text-xs bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300">{{ $t('csl.autoDetected') }}</span>
            <span v-if="p.plugin_name" class="text-gray-500 dark:text-gray-400">{{ p.plugin_name }}:</span>
            <span class="text-gray-700 dark:text-gray-300 truncate">{{ p.path }}</span>
          </div>
          <div
            v-for="cp in store.customPaths"
            :key="cp"
            class="flex items-center gap-2 text-sm"
          >
            <span class="px-1.5 py-0.5 rounded text-xs bg-purple-100 dark:bg-purple-900/40 text-purple-700 dark:text-purple-300">{{ $t('csl.custom') }}</span>
            <span class="text-gray-700 dark:text-gray-300 truncate flex-1">{{ cp }}</span>
            <button
              class="text-red-500 hover:text-red-600 text-xs"
              @click="store.removeCustomPath(cp)"
            >{{ $t('csl.removePath') }}</button>
          </div>
          <button
            class="text-xs text-blue-600 dark:text-blue-400 hover:underline mt-1"
            @click="addCustomPath"
          >+ {{ $t('csl.addPath') }}</button>
        </div>
      </details>
    </div>

    <!-- Filter tabs -->
    <div
      v-if="store.packages.length > 0"
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
          v-if="store.notUpToDateCount > 0"
          class="text-xs px-3 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50"
          :disabled="store.installingPackages.length > 0"
          @click="installAll"
        >{{ $t('csl.installAll') }} ({{ store.notUpToDateCount }})</button>
      </div>
    </div>

    <!-- Package list -->
    <div
      v-if="store.packages.length > 0"
      class="flex-1 overflow-y-auto space-y-1.5"
    >
      <div
        v-for="pkg in filteredPackages"
        :key="pkg.name"
        class="flex items-center gap-3 px-4 py-2.5 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/30 transition-colors"
      >
        <!-- Package info -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="font-medium text-gray-900 dark:text-white text-sm">{{ pkg.name }}</span>
            <span
              class="px-1.5 py-0.5 rounded text-xs font-medium"
              :class="statusClass(pkg.status)"
            >{{ statusLabel(pkg.status) }}</span>
            <span class="text-xs text-gray-400 dark:text-gray-500">{{ pkg.file_count }} {{ $t('csl.files') }} · {{ formatSize(pkg.total_size_bytes) }}</span>
            <span v-if="pkg.status === 'needs_update'" class="text-xs text-amber-500 dark:text-amber-400">
              {{ $t('csl.downloadSize') }}: {{ formatSize(pkg.update_size_bytes) }}
            </span>
          </div>
        </div>

        <!-- Progress -->
        <div
          v-if="store.installingPackages.includes(pkg.name) && store.progressMap[pkg.name]"
          class="flex items-center gap-2 text-xs text-blue-600 dark:text-blue-400"
        >
          <div class="w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
          <span>{{ store.progressMap[pkg.name].current_file }}/{{ store.progressMap[pkg.name].total_files }}</span>
        </div>

        <!-- Spinner (no progress yet) -->
        <div
          v-else-if="store.installingPackages.includes(pkg.name)"
          class="flex items-center gap-2 text-xs text-blue-600 dark:text-blue-400"
        >
          <div class="w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
        </div>

        <!-- Actions -->
        <div v-else class="flex items-center gap-2 flex-shrink-0">
          <button
            v-if="pkg.status === 'not_installed'"
            class="text-xs px-3 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors"
            @click="installPackage(pkg)"
          >{{ $t('csl.install') }}</button>

          <button
            v-if="pkg.status === 'needs_update'"
            class="text-xs px-3 py-1.5 rounded-lg bg-amber-600 text-white hover:bg-amber-700 transition-colors"
            @click="installPackage(pkg)"
          >{{ $t('csl.update') }}</button>

          <button
            v-if="pkg.status !== 'not_installed'"
            class="text-xs px-3 py-1.5 rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors disabled:opacity-50"
            :disabled="store.installingPackages.length > 0"
            @click="confirmUninstall(pkg)"
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
      v-else-if="!store.isLoading"
      class="flex-1 flex items-center justify-center"
    >
      <div class="text-center">
        <svg class="w-16 h-16 mx-auto text-gray-300 dark:text-gray-600 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
        </svg>
        <p class="text-gray-500 dark:text-gray-400 text-sm mb-3">{{ $t('csl.noPathsDetected') }}</p>
        <button
          class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors"
          @click="store.scanPackages()"
        >{{ $t('csl.refresh') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useCslStore } from '@/stores/csl'
import { useModalStore } from '@/stores/modal'
import type { CslPackageInfo, CslProgress } from '@/types'

const { t } = useI18n()
const store = useCslStore()
const modal = useModalStore()

const activeFilter = ref<string>('all')

let unlistenProgress: UnlistenFn | null = null

const filters = computed(() => [
  { value: 'all', label: t('csl.filterAll'), count: store.totalPackages },
  { value: 'not_installed', label: t('csl.filterNotInstalled'), count: store.packages.filter((p) => p.status === 'not_installed').length },
  { value: 'needs_update', label: t('csl.filterNeedsUpdate'), count: store.updatesCount },
  { value: 'up_to_date', label: t('csl.filterUpToDate'), count: store.packages.filter((p) => p.status === 'up_to_date').length },
])

const filteredPackages = computed(() => {
  if (activeFilter.value === 'all') return store.packages
  return store.packages.filter((p) => p.status === activeFilter.value)
})

const totalSizeBytes = computed(() =>
  store.packages.reduce((sum, p) => sum + p.total_size_bytes, 0),
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

function getDefaultInstallPath(): string {
  if (store.paths.length > 0) {
    return store.paths[0].path
  }
  if (store.customPaths.length > 0) {
    return store.customPaths[0]
  }
  return ''
}

function installPackage(pkg: CslPackageInfo) {
  const targetPath = getDefaultInstallPath()
  if (!targetPath) return
  store.installPackage(pkg.name, targetPath)
}

function confirmUninstall(pkg: CslPackageInfo) {
  modal.showConfirm({
    title: t('csl.uninstall'),
    message: t('csl.confirmUninstall', { name: pkg.name }),
    confirmText: t('csl.uninstall'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: () => store.uninstallPackage(pkg.name),
    onCancel: () => {},
  })
}

function installAll() {
  const targetPath = getDefaultInstallPath()
  if (!targetPath) return
  store.installAll(targetPath)
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
  // Listen for progress events from backend
  unlistenProgress = await listen<CslProgress>('csl-progress', (event) => {
    store.updateProgress(event.payload)
  })

  // Auto-scan on mount
  if (store.packages.length === 0) {
    store.scanPackages()
  }
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
})
</script>
