<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ $t('diskUsage.title') }}
        </h1>
        <p v-if="store.report" class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
          {{ $t('diskUsage.scannedIn', { ms: store.report.scanDurationMs }) }}
        </p>
      </div>
      <button
        class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-2"
        :disabled="store.isScanning"
        @click="store.scan()"
      >
        <div
          v-if="store.isScanning"
          class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
        ></div>
        {{ store.isScanning ? $t('diskUsage.scanning') : $t('diskUsage.scan') }}
      </button>
    </div>

    <!-- Content -->
    <div v-if="store.report" class="flex-1 overflow-y-auto">
      <!-- Chart + total -->
      <div class="flex items-center gap-8 mb-6">
        <DiskUsageChart
          :categories="chartData"
          :total-bytes="store.report.totalBytes"
          :size="180"
          :is-dark="isDark"
        />
        <div class="space-y-2">
          <div
            v-for="cat in store.report.categories"
            :key="cat.category"
            class="flex items-center gap-2 text-sm"
          >
            <div class="w-3 h-3 rounded-sm flex-shrink-0" :style="{ backgroundColor: categoryColor(cat.category) }"></div>
            <span class="text-gray-700 dark:text-gray-300">{{ cat.category }}</span>
            <span class="text-gray-400 dark:text-gray-500 ml-auto">{{ formatSize(cat.totalBytes) }}</span>
          </div>
        </div>
      </div>

      <!-- Category details -->
      <div class="space-y-3">
        <details
          v-for="cat in store.report.categories"
          :key="cat.category"
          class="rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden"
        >
          <summary
            class="flex items-center justify-between px-4 py-3 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
          >
            <div class="flex items-center gap-2">
              <div class="w-2.5 h-2.5 rounded-sm" :style="{ backgroundColor: categoryColor(cat.category) }"></div>
              <span class="font-medium text-gray-900 dark:text-white">{{ cat.category }}</span>
              <span class="text-xs text-gray-400 dark:text-gray-500">
                {{ cat.itemCount }} {{ $t('diskUsage.items') }}
              </span>
            </div>
            <span class="text-sm text-gray-600 dark:text-gray-400 font-mono">
              {{ formatSize(cat.totalBytes) }}
            </span>
          </summary>

          <div class="border-t border-gray-200 dark:border-gray-700">
            <div
              v-for="item in cat.items"
              :key="item.folderName"
              class="flex items-center gap-3 px-4 py-2 hover:bg-gray-50 dark:hover:bg-gray-800/30 transition-colors"
            >
              <div class="flex-1 min-w-0">
                <p class="text-sm text-gray-800 dark:text-gray-200 truncate">
                  {{ item.displayName }}
                </p>
                <div class="flex items-center gap-2 mt-0.5">
                  <div class="flex-1 h-1.5 bg-gray-100 dark:bg-gray-700 rounded-full overflow-hidden">
                    <div
                      class="h-full rounded-full transition-all duration-300"
                      :style="{
                        width: cat.totalBytes > 0 ? `${(item.sizeBytes / cat.totalBytes) * 100}%` : '0%',
                        backgroundColor: categoryColor(cat.category),
                      }"
                    ></div>
                  </div>
                  <span class="text-xs text-gray-400 dark:text-gray-500 font-mono flex-shrink-0">
                    {{ formatSize(item.sizeBytes) }}
                  </span>
                </div>
              </div>
              <button
                class="p-1 rounded text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                :title="$t('diskUsage.openFolder')"
                @click="openFolder(item.itemType, item.folderName)"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </button>
            </div>
          </div>
        </details>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="!store.isScanning"
      class="flex-1 flex flex-col items-center justify-center text-gray-400 dark:text-gray-500"
    >
      <svg class="w-12 h-12 mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
      </svg>
      <p class="text-sm">{{ $t('diskUsage.empty') }}</p>
      <p class="text-xs mt-1">{{ $t('diskUsage.emptyHint') }}</p>
    </div>

    <!-- Scanning overlay -->
    <div
      v-if="store.isScanning && !store.report"
      class="flex-1 flex flex-col items-center justify-center"
    >
      <div class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mb-3"></div>
      <p class="text-sm text-gray-500 dark:text-gray-400">{{ $t('diskUsage.scanning') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useDiskUsageStore } from '@/stores/diskUsage'
import { useAppStore } from '@/stores/app'
import { useThemeStore } from '@/stores/theme'
import DiskUsageChart from '@/components/DiskUsageChart.vue'

useI18n()

const store = useDiskUsageStore()
const appStore = useAppStore()
const themeStore = useThemeStore()

const isDark = computed(() => themeStore.isDark)

const COLORS: Record<string, string> = {
  Aircraft: '#3b82f6',
  Plugins: '#8b5cf6',
  Scenery: '#10b981',
  Navdata: '#f59e0b',
  Screenshots: '#ef4444',
}

function categoryColor(name: string): string {
  return COLORS[name] || '#6b7280'
}

const chartData = computed(() => {
  if (!store.report) return []
  return store.report.categories.map((c) => ({
    name: c.category,
    bytes: c.totalBytes,
    color: categoryColor(c.category),
  }))
})

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(2)} GB`
}

async function openFolder(itemType: string, folderName: string) {
  try {
    await invoke('open_management_folder', {
      xplanePath: appStore.xplanePath,
      itemType,
      folderName,
    })
  } catch {
    // silently ignore
  }
}
</script>
