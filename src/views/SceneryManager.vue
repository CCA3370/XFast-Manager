<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSceneryStore } from '@/stores/scenery'
import { useToastStore } from '@/stores/toast'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { invoke } from '@tauri-apps/api/core'
import SceneryEntryCard from '@/components/SceneryEntryCard.vue'
import draggable from 'vuedraggable'

const { t } = useI18n()
const sceneryStore = useSceneryStore()
const toastStore = useToastStore()
const appStore = useAppStore()
const modalStore = useModalStore()

const drag = ref(false)
const isSortingScenery = ref(false)

// Local copy of entries for drag-and-drop
const localEntries = ref<typeof sceneryStore.sortedEntries>([])

// Sync local entries with store
function syncLocalEntries() {
  localEntries.value = [...sceneryStore.sortedEntries]
}

onMounted(async () => {
  if (appStore.xplanePath) {
    await sceneryStore.loadData()
    syncLocalEntries()
  }
})

async function handleToggleEnabled(folderName: string) {
  await sceneryStore.toggleEnabled(folderName)
  syncLocalEntries()
}

async function handleMoveUp(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)
  if (index > 0) {
    await sceneryStore.moveEntry(folderName, index - 1)
    syncLocalEntries()
  }
}

async function handleMoveDown(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)
  if (index < entries.length - 1) {
    await sceneryStore.moveEntry(folderName, index + 1)
    syncLocalEntries()
  }
}

async function handleDragEnd() {
  drag.value = false
  // Reorder entries based on new positions
  await sceneryStore.reorderEntries(localEntries.value)
  // Sync back after reorder
  syncLocalEntries()
}

async function handleApplyChanges() {
  try {
    await sceneryStore.applyChanges()
    toastStore.success(t('sceneryManager.changesApplied'))
  } catch (e) {
    toastStore.error(t('sceneryManager.applyFailed'))
  }
}

function handleReset() {
  if (confirm(t('sceneryManager.resetConfirm'))) {
    sceneryStore.resetChanges()
    syncLocalEntries()
  }
}

async function handleSortSceneryNow() {
  if (isSortingScenery.value || !appStore.xplanePath) return

  isSortingScenery.value = true
  try {
    await invoke('sort_scenery_packs', { xplanePath: appStore.xplanePath })
    toastStore.success(t('settings.scenerySorted'))
    // Reload data after sorting
    await sceneryStore.loadData()
    syncLocalEntries()
  } catch (e) {
    modalStore.showError(t('settings.scenerySortFailed') + ': ' + String(e))
  } finally {
    isSortingScenery.value = false
  }
}
</script>

<template>
  <div class="scenery-manager-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Header -->
    <div class="mb-3 flex-shrink-0 flex items-center justify-between">
      <div>
        <h2 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ t('sceneryManager.title') }}
        </h2>
        <p class="text-xs text-gray-600 dark:text-gray-400 mt-0.5">
          {{ t('sceneryManager.subtitle') }}
        </p>
      </div>

      <!-- Action buttons -->
      <div class="flex items-center gap-2">
        <button
          @click="handleSortSceneryNow"
          :disabled="isSortingScenery || !appStore.xplanePath"
          class="px-3 py-1.5 rounded-lg bg-cyan-500 text-white hover:bg-cyan-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 text-sm"
        >
          <svg v-if="!isSortingScenery" class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"></path>
          </svg>
          <svg v-else class="w-3.5 h-3.5 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ isSortingScenery ? t('settings.sorting') : t('settings.sortNow') }}
        </button>
        <button
          v-if="sceneryStore.hasChanges"
          @click="handleReset"
          class="px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-sm"
        >
          {{ t('sceneryManager.reset') }}
        </button>
        <button
          @click="handleApplyChanges"
          :disabled="!sceneryStore.hasChanges || sceneryStore.isSaving"
          class="px-3 py-1.5 rounded-lg bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 text-sm"
        >
          <svg v-if="sceneryStore.isSaving" class="animate-spin h-3.5 w-3.5" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          {{ t('sceneryManager.applyChanges') }}
        </button>
      </div>
    </div>

    <!-- Statistics bar -->
    <div class="flex items-center gap-4 px-3 py-2 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 mb-3 text-sm">
      <div class="flex items-center gap-2">
        <span class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.total') }}:</span>
        <span class="font-semibold text-gray-900 dark:text-gray-100">{{ sceneryStore.totalCount }}</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.enabled') }}:</span>
        <span class="font-semibold text-green-600 dark:text-green-400">{{ sceneryStore.enabledCount }}</span>
      </div>
      <div v-if="sceneryStore.missingDepsCount > 0" class="flex items-center gap-2">
        <span class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.missingDeps') }}:</span>
        <span class="font-semibold text-amber-600 dark:text-amber-400">{{ sceneryStore.missingDepsCount }}</span>
      </div>
      <div v-if="sceneryStore.hasChanges" class="ml-auto flex items-center gap-2 text-blue-600 dark:text-blue-400">
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span class="text-xs font-medium">{{ t('sceneryManager.unsavedChanges') }}</span>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      <div v-if="!appStore.xplanePath" class="flex items-center justify-center h-full">
        <div class="text-center">
          <svg class="w-16 h-16 mx-auto text-gray-400 dark:text-gray-600 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <p class="text-gray-600 dark:text-gray-400">{{ t('settings.sceneryAutoSortNeedPath') }}</p>
        </div>
      </div>

      <div v-else-if="sceneryStore.isLoading" class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>

      <div v-else-if="sceneryStore.error" class="text-center py-12">
        <p class="text-red-600 dark:text-red-400">{{ sceneryStore.error }}</p>
      </div>

      <div v-else-if="sceneryStore.totalCount === 0" class="text-center py-12">
        <p class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noScenery') }}</p>
      </div>

      <draggable
        v-else
        v-model="localEntries"
        item-key="folderName"
        handle=".drag-handle"
        @start="drag = true"
        @end="handleDragEnd"
        class="space-y-1.5"
      >
        <template #item="{ element, index }">
          <SceneryEntryCard
            :entry="element"
            :index="index"
            :total-count="sceneryStore.totalCount"
            @toggle-enabled="handleToggleEnabled"
            @move-up="handleMoveUp"
            @move-down="handleMoveDown"
          />
        </template>
      </draggable>
    </div>
  </div>
</template>

<style scoped>
.scenery-manager-view {
  background: linear-gradient(to bottom, rgba(248, 250, 252, 0.5), rgba(241, 245, 249, 0.5));
}

.dark .scenery-manager-view {
  background: linear-gradient(to bottom, rgba(17, 24, 39, 0.5), rgba(31, 41, 55, 0.5));
}
</style>
