<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AircraftInfo, PluginInfo, NavdataManagerInfo, ManagementItemType } from '@/types'

type EntryType = AircraftInfo | PluginInfo | NavdataManagerInfo

const props = withDefaults(defineProps<{
  entry: EntryType
  itemType: ManagementItemType
  isToggling?: boolean
}>(), {
  isToggling: false
})

const emit = defineEmits<{
  (e: 'toggle-enabled', folderName: string): void
  (e: 'delete', folderName: string): void
  (e: 'open-folder', folderName: string): void
}>()

const { t } = useI18n()

const showDeleteConfirmModal = ref(false)
const isDeleting = ref(false)

// Type guards
function isAircraft(entry: EntryType): entry is AircraftInfo {
  return 'acfFile' in entry
}

function isPlugin(entry: EntryType): entry is PluginInfo {
  return 'xplFiles' in entry
}

function isNavdata(entry: EntryType): entry is NavdataManagerInfo {
  return 'providerName' in entry
}

// Display name
const displayName = computed(() => {
  if (isAircraft(props.entry)) {
    return props.entry.displayName
  } else if (isPlugin(props.entry)) {
    return props.entry.displayName
  } else if (isNavdata(props.entry)) {
    return props.entry.providerName
  }
  return props.entry.folderName
})

// Badge info
const badgeInfo = computed(() => {
  if (isAircraft(props.entry) && props.entry.hasLiveries) {
    return {
      text: `${props.entry.liveryCount} ${t('management.liveries')}`,
      color: 'text-blue-700 dark:text-blue-300',
      bgColor: 'bg-blue-100 dark:bg-blue-900/30'
    }
  } else if (isPlugin(props.entry)) {
    const platformColors: Record<string, { color: string; bgColor: string }> = {
      win: { color: 'text-blue-700 dark:text-blue-300', bgColor: 'bg-blue-100 dark:bg-blue-900/30' },
      mac: { color: 'text-gray-700 dark:text-gray-300', bgColor: 'bg-gray-100 dark:bg-gray-800/50' },
      lin: { color: 'text-orange-700 dark:text-orange-300', bgColor: 'bg-orange-100 dark:bg-orange-900/30' },
      multi: { color: 'text-green-700 dark:text-green-300', bgColor: 'bg-green-100 dark:bg-green-900/30' },
      unknown: { color: 'text-gray-600 dark:text-gray-400', bgColor: 'bg-gray-100 dark:bg-gray-800/50' }
    }
    const colors = platformColors[props.entry.platform] || platformColors.unknown
    return {
      text: props.entry.platform.toUpperCase(),
      ...colors
    }
  } else if (isNavdata(props.entry)) {
    const cycleText = props.entry.cycle || props.entry.airac || ''
    if (cycleText) {
      return {
        text: cycleText,
        color: 'text-purple-700 dark:text-purple-300',
        bgColor: 'bg-purple-100 dark:bg-purple-900/30'
      }
    }
  }
  return null
})

// Version info (for aircraft and plugins)
const versionInfo = computed(() => {
  if (isAircraft(props.entry) || isPlugin(props.entry)) {
    return props.entry.version || null
  }
  return null
})

function handleDoubleClick() {
  emit('open-folder', props.entry.folderName)
}

function handleDeleteConfirm() {
  isDeleting.value = true
  emit('delete', props.entry.folderName)
  // Parent will handle the actual deletion and close modal on success
  setTimeout(() => {
    isDeleting.value = false
    showDeleteConfirmModal.value = false
  }, 500)
}
</script>

<template>
  <div
    class="flex items-center gap-2 p-2 rounded-lg border transition-all hover:bg-gray-50 dark:hover:bg-gray-700/30"
    :class="[
      entry.enabled
        ? 'bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700'
        : 'bg-gray-50 dark:bg-gray-900/50 border-gray-200/50 dark:border-gray-700/50 opacity-60'
    ]"
    @dblclick="handleDoubleClick"
  >
    <!-- Enable/Disable toggle -->
    <button
      @click="emit('toggle-enabled', entry.folderName)"
      :disabled="isToggling"
      class="flex-shrink-0 w-9 h-5 rounded-full relative transition-colors disabled:opacity-70"
      :class="entry.enabled ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'"
    >
      <span
        v-if="isToggling"
        class="absolute inset-0 flex items-center justify-center"
      >
        <svg class="w-3 h-3 animate-spin text-white" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
      </span>
      <span
        v-else
        class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
        :class="entry.enabled ? 'left-4.5' : 'left-0.5'"
      />
    </button>

    <!-- Display name -->
    <div class="flex-1 min-w-0">
      <div class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate" :title="entry.folderName">
        {{ displayName }}
      </div>
    </div>

    <!-- Version info (if available) -->
    <span
      v-if="versionInfo"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700"
      :title="versionInfo"
    >
      v{{ versionInfo }}
    </span>

    <!-- Badge (liveries count / platform / cycle) -->
    <span
      v-if="badgeInfo"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium"
      :class="[badgeInfo.color, badgeInfo.bgColor]"
    >
      {{ badgeInfo.text }}
    </span>

    <!-- Delete button -->
    <button
      @click.stop="showDeleteConfirmModal = true"
      class="flex-shrink-0 p-0.5 rounded hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
      :title="t('common.delete')"
    >
      <svg class="w-3.5 h-3.5 text-gray-400 hover:text-red-500 dark:text-gray-500 dark:hover:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
      </svg>
    </button>
  </div>

  <!-- Delete Confirmation Modal -->
  <Teleport to="body">
    <div
      v-if="showDeleteConfirmModal"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      @click="showDeleteConfirmModal = false"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full mx-4"
        style="max-width: 400px;"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between p-5 pb-3">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
            {{ t('management.deleteConfirmTitle') }}
          </h3>
          <button
            @click="showDeleteConfirmModal = false"
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Content -->
        <div class="px-5 pb-3">
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">
            {{ t('management.deleteConfirmMessage') }}
          </p>
          <p class="text-sm font-medium text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 rounded px-3 py-2 break-all">
            {{ entry.folderName }}
          </p>
        </div>

        <!-- Action Buttons -->
        <div class="flex gap-2 p-5 pt-3 border-t border-gray-200 dark:border-gray-700">
          <button
            @click="showDeleteConfirmModal = false"
            class="flex-1 px-4 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 rounded-lg transition-colors"
          >
            {{ t('common.cancel') }}
          </button>
          <button
            @click="handleDeleteConfirm"
            :disabled="isDeleting"
            class="flex-1 px-4 py-2 bg-red-500 hover:bg-red-600 disabled:bg-red-400 disabled:cursor-not-allowed text-white rounded-lg transition-colors flex items-center justify-center gap-2"
          >
            <svg v-if="isDeleting" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {{ isDeleting ? t('common.deleting') : t('common.delete') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
