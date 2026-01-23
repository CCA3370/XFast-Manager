<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useSceneryStore } from '@/stores/scenery'
import type { SceneryManagerEntry } from '@/types'
import { SceneryCategory } from '@/types'

const props = withDefaults(defineProps<{
  entry: SceneryManagerEntry
  index: number
  totalCount: number
  disableReorder?: boolean
}>(), {
  disableReorder: false
})

const emit = defineEmits<{
  (e: 'toggle-enabled', folderName: string): void
  (e: 'move-up', folderName: string): void
  (e: 'move-down', folderName: string): void
}>()

const { t } = useI18n()
const appStore = useAppStore()
const toastStore = useToastStore()
const modalStore = useModalStore()
const sceneryStore = useSceneryStore()

const showMissingLibsModal = ref(false)
const showDeleteConfirmModal = ref(false)
const isSearching = ref(false)
const isDeleting = ref(false)

// Category display config
const categoryConfig = computed(() => {
  const configs: Record<SceneryCategory, { label: string; color: string; bgColor: string }> = {
    [SceneryCategory.FixedHighPriority]: { label: 'SAM', color: 'text-purple-700 dark:text-purple-300', bgColor: 'bg-purple-100 dark:bg-purple-900/30' },
    [SceneryCategory.Airport]: { label: t('sceneryManager.categoryAirport'), color: 'text-blue-700 dark:text-blue-300', bgColor: 'bg-blue-100 dark:bg-blue-900/30' },
    [SceneryCategory.DefaultAirport]: { label: t('sceneryManager.categoryDefaultAirport'), color: 'text-gray-600 dark:text-gray-400', bgColor: 'bg-gray-100 dark:bg-gray-800/50' },
    [SceneryCategory.Library]: { label: t('sceneryManager.categoryLibrary'), color: 'text-green-700 dark:text-green-300', bgColor: 'bg-green-100 dark:bg-green-900/30' },
    [SceneryCategory.Overlay]: { label: t('sceneryManager.categoryOverlay'), color: 'text-yellow-700 dark:text-yellow-300', bgColor: 'bg-yellow-100 dark:bg-yellow-900/30' },
    [SceneryCategory.Orthophotos]: { label: t('sceneryManager.categoryOrthophotos'), color: 'text-orange-700 dark:text-orange-300', bgColor: 'bg-orange-100 dark:bg-orange-900/30' },
    [SceneryCategory.Mesh]: { label: t('sceneryManager.categoryMesh'), color: 'text-red-700 dark:text-red-300', bgColor: 'bg-red-100 dark:bg-red-900/30' },
    [SceneryCategory.Other]: { label: t('sceneryManager.categoryOther'), color: 'text-gray-600 dark:text-gray-400', bgColor: 'bg-gray-100 dark:bg-gray-800/50' },
  }
  return configs[props.entry.category] || configs[SceneryCategory.Other]
})

const hasMissingDeps = computed(() => props.entry.missingLibraries.length > 0)
const isFirst = computed(() => props.index === 0)
const isLast = computed(() => props.index === props.totalCount - 1)

async function handleDoubleClick() {
  if (!appStore.xplanePath) {
    modalStore.showError(t('sceneryManager.noXplanePath'))
    return
  }

  try {
    await invoke('open_scenery_folder', {
      xplanePath: appStore.xplanePath,
      folderName: props.entry.folderName
    })
  } catch (error) {
    modalStore.showError(t('sceneryManager.openFolderFailed') + ': ' + String(error))
  }
}

function handleClick(event: Event) {
  // If has missing libraries, show modal on single click
  if (hasMissingDeps.value) {
    // Don't trigger if clicking on interactive elements
    const target = event.target as HTMLElement
    if (target.closest('button') || target.closest('.drag-handle')) {
      return
    }
    event.stopPropagation()
    showMissingLibsModal.value = true
  }
}

function handleCopyMissingLibs() {
  const libNames = props.entry.missingLibraries.join('\n')
  navigator.clipboard.writeText(libNames).then(() => {
    toastStore.success(t('sceneryManager.missingLibsCopied'))
  }).catch(() => {
    modalStore.showError(t('copy.copyFailed'))
  })
}

async function handleSearchMissingLibs() {
  if (isSearching.value) return // Prevent duplicate clicks

  isSearching.value = true
  try {
    // Open a separate search tab for each missing library
    for (const libName of props.entry.missingLibraries) {
      const bingUrl = `https://www.bing.com/search?q=${encodeURIComponent(libName + ' X-Plane library')}`

      try {
        await invoke('open_url', { url: bingUrl })
        // Add a small delay between opening tabs to avoid overwhelming the browser
        await new Promise(resolve => setTimeout(resolve, 300))
      } catch (error) {
        modalStore.showError(t('sceneryManager.openUrlFailed') + ': ' + String(error))
        break // Stop if there's an error
      }
    }
  } finally {
    isSearching.value = false
  }
}

async function handleDeleteConfirm() {
  if (isDeleting.value) return

  isDeleting.value = true
  try {
    await sceneryStore.deleteEntry(props.entry.folderName)
    toastStore.success(t('sceneryManager.deleteSuccess'))
    showDeleteConfirmModal.value = false
  } catch (error) {
    modalStore.showError(t('sceneryManager.deleteFailed') + ': ' + String(error))
  } finally {
    isDeleting.value = false
  }
}
</script>

<template>
  <div
    class="flex items-center gap-2 p-2 rounded-lg border transition-all hover:bg-gray-50 dark:hover:bg-gray-700/30"
    :class="[
      entry.enabled
        ? 'bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700'
        : 'bg-gray-50 dark:bg-gray-900/50 border-gray-200/50 dark:border-gray-700/50 opacity-60',
      hasMissingDeps ? 'cursor-pointer' : ''
    ]"
    @click="handleClick"
    @dblclick="handleDoubleClick"
  >
    <!-- Drag handle -->
    <div v-if="!props.disableReorder" class="cursor-grab active:cursor-grabbing text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 drag-handle select-none">
      <svg class="w-4 h-4 pointer-events-none" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8h16M4 16h16" />
      </svg>
    </div>

    <!-- Enable/Disable toggle -->
    <button
      @click="emit('toggle-enabled', entry.folderName)"
      class="flex-shrink-0 w-9 h-5 rounded-full relative transition-colors"
      :class="entry.enabled ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'"
    >
      <span
        class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
        :class="entry.enabled ? 'left-4.5' : 'left-0.5'"
      />
    </button>

    <!-- Folder name -->
    <div class="flex-1 min-w-0">
      <div class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate" :title="entry.folderName">
        {{ entry.folderName }}
      </div>
    </div>

    <!-- Missing dependencies warning (before category badge) -->
    <div
      v-if="hasMissingDeps"
      class="flex-shrink-0 flex items-center gap-0.5 px-1.5 py-0.5 rounded text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20"
      :title="t('sceneryManager.clickToViewMissingLibs')"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
      <span class="text-[10px] font-medium">{{ entry.missingLibraries.length }}</span>
    </div>

    <!-- Category badge -->
    <span
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium"
      :class="[categoryConfig.color, categoryConfig.bgColor]"
    >
      {{ categoryConfig.label }}
    </span>

    <!-- Move buttons -->
    <div v-if="!props.disableReorder" class="flex-shrink-0 flex gap-0.5">
      <button
        @click="emit('move-up', entry.folderName)"
        :disabled="isFirst"
        class="p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        :title="t('sceneryManager.moveUp')"
      >
        <svg class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
        </svg>
      </button>
      <button
        @click="emit('move-down', entry.folderName)"
        :disabled="isLast"
        class="p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        :title="t('sceneryManager.moveDown')"
      >
        <svg class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
    </div>

    <!-- Delete button -->
    <button
      @click.stop="showDeleteConfirmModal = true"
      class="flex-shrink-0 p-0.5 rounded hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
      :title="t('sceneryManager.delete')"
    >
      <svg class="w-3.5 h-3.5 text-gray-400 hover:text-red-500 dark:text-gray-500 dark:hover:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
      </svg>
    </button>
  </div>

  <!-- Missing Libraries Modal -->
  <Teleport to="body">
    <div
      v-if="showMissingLibsModal"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      @click="showMissingLibsModal = false"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full mx-4 flex flex-col"
        style="max-width: 500px; max-height: 80vh;"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between p-5 pb-3 flex-shrink-0">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
            {{ t('sceneryManager.missingLibrariesTitle') }}
          </h3>
          <button
            @click="showMissingLibsModal = false"
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Scrollable Content Area -->
        <div class="flex-1 overflow-y-auto px-5 pb-3 min-h-0">
          <!-- Scenery Name -->
          <div class="mb-3 text-sm text-gray-600 dark:text-gray-400">
            {{ entry.folderName }}
          </div>

          <!-- Missing Libraries List -->
          <div class="bg-gray-50 dark:bg-gray-900 rounded p-3">
            <ul class="space-y-1">
              <li
                v-for="lib in entry.missingLibraries"
                :key="lib"
                class="text-sm text-gray-800 dark:text-gray-200 font-mono"
              >
                • {{ lib }}
              </li>
            </ul>
          </div>
        </div>

        <!-- Action Buttons (Fixed at bottom) -->
        <div class="flex flex-col gap-2 p-5 pt-3 flex-shrink-0 border-t border-gray-200 dark:border-gray-700">
          <div class="flex gap-2">
            <button
              @click="handleCopyMissingLibs"
              class="flex-1 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors flex items-center justify-center gap-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
              {{ t('sceneryManager.copyAllLibNames') }}
            </button>
            <button
              @click="handleSearchMissingLibs"
              :disabled="isSearching"
              class="flex-1 px-4 py-2 bg-green-500 hover:bg-green-600 disabled:bg-green-400 disabled:cursor-not-allowed text-white rounded-lg transition-colors flex items-center justify-center gap-2"
            >
              <svg v-if="!isSearching" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <svg v-else class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ isSearching ? t('sceneryManager.searching') : t('sceneryManager.searchOnBing') }}
            </button>
          </div>
          <button
            @click="showMissingLibsModal = false"
            class="w-full px-4 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 rounded-lg transition-colors"
          >
            {{ t('common.close') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>

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
            {{ t('sceneryManager.deleteConfirmTitle') }}
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
            {{ t('sceneryManager.deleteConfirmMessage') }}
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
