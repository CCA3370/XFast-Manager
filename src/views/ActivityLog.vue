<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ $t('activityLog.title') }}
        </h1>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
          {{ $t('activityLog.subtitle', { count: store.totalCount }) }}
        </p>
      </div>
      <div class="flex items-center gap-3">
        <!-- Filter -->
        <select
          v-model="store.filterItemType"
          class="text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 px-3 py-1.5 outline-none focus:ring-2 focus:ring-blue-500/30"
          @change="store.loadRecent()"
        >
          <option :value="null">{{ $t('activityLog.allTypes') }}</option>
          <option value="aircraft">{{ $t('activityLog.typeAircraft') }}</option>
          <option value="plugin">{{ $t('activityLog.typePlugin') }}</option>
          <option value="scenery">{{ $t('activityLog.typeScenery') }}</option>
          <option value="navdata">{{ $t('activityLog.typeNavdata') }}</option>
          <option value="livery">{{ $t('activityLog.typeLivery') }}</option>
          <option value="lua_script">{{ $t('activityLog.typeLuaScript') }}</option>
          <option value="preset">{{ $t('activityLog.typePreset') }}</option>
        </select>
        <!-- Clear -->
        <button
          class="text-sm px-3 py-1.5 rounded-lg border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
          :disabled="store.entries.length === 0"
          @click="handleClear"
        >
          {{ $t('activityLog.clear') }}
        </button>
      </div>
    </div>

    <!-- Timeline -->
    <div class="flex-1 overflow-y-auto space-y-1">
      <template v-if="store.entries.length > 0">
        <div
          v-for="entry in store.entries"
          :key="entry.id"
          class="flex items-start gap-3 px-4 py-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors group"
        >
          <!-- Color dot -->
          <div
            class="mt-1.5 w-2.5 h-2.5 rounded-full flex-shrink-0"
            :class="dotColor(entry.operation)"
          ></div>

          <!-- Content -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-gray-900 dark:text-white">
                {{ operationLabel(entry.operation) }}
              </span>
              <span
                v-if="!entry.success"
                class="text-[10px] font-semibold uppercase px-1.5 py-0.5 rounded bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
              >
                {{ $t('activityLog.failed') }}
              </span>
            </div>
            <p class="text-sm text-gray-600 dark:text-gray-400 truncate">
              {{ entry.itemName }}
              <span class="text-gray-400 dark:text-gray-500">&middot; {{ typeLabel(entry.itemType) }}</span>
            </p>
            <!-- Expandable details -->
            <details v-if="entry.details" class="mt-1">
              <summary
                class="text-xs text-gray-400 dark:text-gray-500 cursor-pointer hover:text-gray-600 dark:hover:text-gray-300"
              >
                {{ $t('activityLog.showDetails') }}
              </summary>
              <pre
                class="mt-1 text-xs text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-800 rounded p-2 overflow-x-auto whitespace-pre-wrap"
              >{{ formatDetails(entry.details) }}</pre>
            </details>
          </div>

          <!-- Timestamp -->
          <span class="text-xs text-gray-400 dark:text-gray-500 flex-shrink-0 whitespace-nowrap">
            {{ relativeTime(entry.timestamp) }}
          </span>
        </div>

        <!-- Load more -->
        <div v-if="store.hasMore" class="flex justify-center py-4">
          <button
            class="text-sm text-blue-600 dark:text-blue-400 hover:underline"
            :disabled="store.isLoading"
            @click="store.loadMore()"
          >
            {{ store.isLoading ? $t('activityLog.loading') : $t('activityLog.loadMore') }}
          </button>
        </div>
      </template>

      <!-- Empty state -->
      <div
        v-else-if="!store.isLoading"
        class="flex flex-col items-center justify-center h-full text-gray-400 dark:text-gray-500"
      >
        <svg class="w-12 h-12 mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <p class="text-sm">{{ $t('activityLog.empty') }}</p>
      </div>

      <!-- Loading -->
      <div v-if="store.isLoading && store.entries.length === 0" class="flex justify-center py-12">
        <div class="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useActivityLogStore } from '@/stores/activityLog'
import { useModalStore } from '@/stores/modal'

const { t } = useI18n()
const store = useActivityLogStore()
const modalStore = useModalStore()

onMounted(() => {
  store.loadRecent()
})

function dotColor(op: string): string {
  switch (op) {
    case 'install':
      return 'bg-green-500'
    case 'update':
      return 'bg-blue-500'
    case 'delete':
      return 'bg-red-500'
    case 'enable':
    case 'disable':
      return 'bg-purple-500'
    case 'config_change':
    case 'scenery_sort':
      return 'bg-yellow-500'
    case 'preset_apply':
      return 'bg-cyan-500'
    default:
      return 'bg-gray-400'
  }
}

function operationLabel(op: string): string {
  return t(`activityLog.op_${op}`, op)
}

function typeLabel(type: string): string {
  return t(`activityLog.type_${type}`, type)
}

function formatDetails(details: string): string {
  try {
    return JSON.stringify(JSON.parse(details), null, 2)
  } catch {
    return details
  }
}

function relativeTime(epoch: number): string {
  const diff = Date.now() / 1000 - epoch
  if (diff < 60) return t('activityLog.justNow')
  if (diff < 3600) return t('activityLog.minutesAgo', { n: Math.floor(diff / 60) })
  if (diff < 86400) return t('activityLog.hoursAgo', { n: Math.floor(diff / 3600) })
  return t('activityLog.daysAgo', { n: Math.floor(diff / 86400) })
}

function handleClear() {
  modalStore.showConfirm({
    title: t('activityLog.clearTitle'),
    message: t('activityLog.clearMessage'),
    confirmText: t('activityLog.clearConfirm'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: async () => {
      await store.clearLog()
    },
    onCancel: () => {},
  })
}
</script>
