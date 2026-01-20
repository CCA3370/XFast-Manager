<script setup lang="ts">
import { ref, onMounted, computed, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSceneryStore } from '@/stores/scenery'
import { useToastStore } from '@/stores/toast'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { invoke } from '@tauri-apps/api/core'
import SceneryEntryCard from '@/components/SceneryEntryCard.vue'
import draggable from 'vuedraggable'
import type { SceneryManagerEntry } from '@/types'

const { t, locale } = useI18n()
const sceneryStore = useSceneryStore()
const toastStore = useToastStore()
const appStore = useAppStore()
const modalStore = useModalStore()

// Check if current language is Chinese
const isChineseLocale = computed(() => locale.value === 'zh')

const drag = ref(false)
const dragStartX = ref(0)
const dragStartY = ref(0)
const draggedItemHeight = ref(0)
const isSortingScenery = ref(false)
const searchQuery = ref('')
const highlightedIndex = ref(-1)
const currentMatchIndex = ref(0)
const showOnlyMissingLibs = ref(false)
const showMoreMenu = ref(false)
const draggableRef = ref<InstanceType<typeof draggable> | null>(null)
const listContainerRef = ref<HTMLElement | null>(null)
const moreMenuRef = ref<HTMLElement | null>(null)
let dragGhostEl: HTMLElement | null = null
let dragBounds: DOMRect | null = null

type DragPointerEvent = MouseEvent | TouchEvent | PointerEvent | Event | undefined

// Local copy of grouped entries for drag-and-drop
const localGroupedEntries = ref<Record<string, SceneryManagerEntry[]>>({
  FixedHighPriority: [],
  Airport: [],
  DefaultAirport: [],
  Library: [],
  Other: [],
  Overlay: [],
  Orthophotos: [],
  Mesh: []
})

// Category order for display
const categoryOrder = ['FixedHighPriority', 'Airport', 'DefaultAirport', 'Library', 'Other', 'Overlay', 'Orthophotos', 'Mesh']

// Filtered entries based on missing libraries filter
const filteredEntries = computed(() => {
  const allEntries = Object.values(localGroupedEntries.value).flat()
  if (!showOnlyMissingLibs.value) return allEntries
  return allEntries.filter(entry => entry.missingLibraries && entry.missingLibraries.length > 0)
})

// Matched indices in the filtered list
const matchedIndices = computed(() => {
  if (!searchQuery.value.trim()) return []
  const query = searchQuery.value.toLowerCase()
  return filteredEntries.value
    .map((entry, index) => ({ entry, index }))
    .filter(({ entry }) => entry.folderName.toLowerCase().includes(query))
    .map(({ index }) => index)
})

// Sync local entries with store
function syncLocalEntries() {
  const grouped = sceneryStore.groupedEntries
  localGroupedEntries.value = {
    FixedHighPriority: [...(grouped.FixedHighPriority || [])],
    Airport: [...(grouped.Airport || [])],
    DefaultAirport: [...(grouped.DefaultAirport || [])],
    Library: [...(grouped.Library || [])],
    Other: [...(grouped.Other || [])],
    Overlay: [...(grouped.Overlay || [])],
    Orthophotos: [...(grouped.Orthophotos || [])],
    Mesh: [...(grouped.Mesh || [])]
  }
}

// Toggle group collapse state
function toggleGroupCollapse(category: string) {
  sceneryStore.collapsedGroups[category] = !sceneryStore.collapsedGroups[category]
}

// Get category translation key
function getCategoryTranslationKey(category: string): string {
  return `sceneryManager.category${category}`
}

function getClientPoint(evt: DragPointerEvent) {
  if (evt && 'touches' in evt && evt.touches && evt.touches.length > 0) {
    const touch = evt.touches[0]
    return { x: touch.clientX, y: touch.clientY }
  }

  if (evt && 'changedTouches' in evt && evt.changedTouches && evt.changedTouches.length > 0) {
    const touch = evt.changedTouches[0]
    return { x: touch.clientX, y: touch.clientY }
  }

  if (evt && 'clientX' in evt && 'clientY' in evt) {
    const mouseEvent = evt as MouseEvent
    return { x: mouseEvent.clientX, y: mouseEvent.clientY }
  }

  return { x: dragStartX.value, y: dragStartY.value }
}

// Keep drag interactions constrained to the vertical axis by canceling horizontal offset
function updateVerticalDrag(evt?: DragPointerEvent) {
  const { x, y } = getClientPoint(evt)
  const sortable = (draggableRef.value as any)?._sortable
  const offsetX = dragStartX.value - x

  const containerRect = listContainerRef.value?.getBoundingClientRect() || dragBounds || null
  const halfHeight = draggedItemHeight.value > 0 ? draggedItemHeight.value / 2 : 0
  const minY = containerRect ? containerRect.top + halfHeight : -Infinity
  const maxY = containerRect ? containerRect.bottom - halfHeight : Infinity
  const clampedY = Math.min(Math.max(y, minY), maxY)

  if (sortable) {
    sortable.option('fallbackOffset', { x: offsetX, y: dragStartY.value - clampedY })
  }

  if (dragGhostEl) {
    const deltaY = clampedY - dragStartY.value
    dragGhostEl.style.transform = `translate3d(0px, ${deltaY}px, 0px)`
  }
}

function cleanupDragLock() {
  window.removeEventListener('pointermove', updateVerticalDrag as EventListener)
  window.removeEventListener('touchmove', updateVerticalDrag as EventListener)
  dragBounds = null
  draggedItemHeight.value = 0
  if (dragGhostEl) {
    dragGhostEl.style.transform = ''
    dragGhostEl = null
  }
}

function handleDragStart(evt: any) {
  drag.value = true
}

onMounted(async () => {
  if (appStore.xplanePath) {
    await sceneryStore.loadData()
    syncLocalEntries()
  }
  document.addEventListener('click', handleClickOutside)
})

onBeforeUnmount(() => {
  cleanupDragLock()
  document.removeEventListener('click', handleClickOutside)
})

async function handleToggleEnabled(folderName: string) {
  await sceneryStore.toggleEnabled(folderName)
  syncLocalEntries()
}

async function handleMoveUp(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)

  // Allow moving up unless it's the very first item in the entire list
  if (index > 0) {
    const currentEntry = entries[index]
    const targetEntry = entries[index - 1]

    // Check if moving to a different category
    if (currentEntry.category !== targetEntry.category) {
      // Cross-category move: update category
      await sceneryStore.updateCategory(folderName, targetEntry.category)
    } else {
      // Same category: just reorder
      await sceneryStore.moveEntry(folderName, index - 1)
    }
    syncLocalEntries()
  }
}

async function handleMoveDown(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)
  if (index < entries.length - 1) {
    const currentEntry = entries[index]
    const targetEntry = entries[index + 1]

    // Check if moving to a different category
    if (currentEntry.category !== targetEntry.category) {
      // Cross-category move: update category
      await sceneryStore.updateCategory(folderName, targetEntry.category)
    } else {
      // Same category: just reorder
      await sceneryStore.moveEntry(folderName, index + 1)
    }
    syncLocalEntries()
  }
}

async function handleDragEnd() {
  drag.value = false
  cleanupDragLock()

  // Flatten all groups back into a single array with updated sortOrder
  const allEntries = categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])

  // Reorder entries based on new positions (staged locally)
  await sceneryStore.reorderEntries(allEntries)

  // Sync back after reorder to reflect staged sortOrder
  syncLocalEntries()
}

// Get global index for an entry (used for move up/down button state)
function getGlobalIndex(folderName: string): number {
  // Flatten all groups to get the current global order
  const allEntries = categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])
  const index = allEntries.findIndex(e => e.folderName === folderName)
  return index
}

// Handle cross-group drag (category change)
async function handleGroupChange(category: string, evt: any) {
  if (evt.added) {
    const entry = evt.added.element
    const newCategory = category as any

    try {
      // Update category in backend
      await sceneryStore.updateCategory(entry.folderName, newCategory)
      // Reload data to get updated state
      await sceneryStore.loadData()
      syncLocalEntries()
    } catch (e) {
      console.error('Failed to update category:', e)
      // Reload to revert UI
      await sceneryStore.loadData()
      syncLocalEntries()
    }
  }
}

async function handleApplyChanges() {
  try {
    await sceneryStore.applyChanges()
    toastStore.success(t('sceneryManager.changesApplied'))
    syncLocalEntries()
  } catch (e) {
    toastStore.error(t('sceneryManager.applyFailed'))
  }
}

function handleReset() {
  modalStore.showConfirm({
    title: t('sceneryManager.reset'),
    message: t('sceneryManager.resetConfirm'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => {
      sceneryStore.resetChanges()
      syncLocalEntries()
    },
    onCancel: () => {}
  })
}

async function performAutoSort() {
  isSortingScenery.value = true
  try {
    const hasChanges = await invoke<boolean>('sort_scenery_packs', { xplanePath: appStore.xplanePath })
    // Reload data after sorting
    await sceneryStore.loadData()
    syncLocalEntries()

    // Check if there are unsaved changes (either from auto-sort or previous manual changes)
    if (sceneryStore.hasChanges) {
      toastStore.success(t('sceneryManager.autoSortDone'))
    } else if (hasChanges) {
      // Sort order was reset but happens to match what's in ini
      toastStore.success(t('sceneryManager.autoSortDone'))
    } else {
      toastStore.info(t('sceneryManager.autoSortNoChange'))
    }
  } catch (e) {
    modalStore.showError(t('sceneryManager.autoSortFailed') + ': ' + String(e))
  } finally {
    isSortingScenery.value = false
  }
}

function handleSortSceneryNow() {
  if (isSortingScenery.value || !appStore.xplanePath) return

  showMoreMenu.value = false

  // Confirm before auto-sorting
  modalStore.showConfirm({
    title: t('sceneryManager.autoSort'),
    message: t('sceneryManager.autoSortConfirm'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => {
      // Use setTimeout to ensure modal closes before starting the sort operation
      setTimeout(() => {
        performAutoSort()
      }, 0)
    },
    onCancel: () => {}
  })
}

function handleClickOutside(event: MouseEvent) {
  if (moreMenuRef.value && !moreMenuRef.value.contains(event.target as Node)) {
    showMoreMenu.value = false
  }
}

// Search navigation functions
function scrollToMatch(index: number) {
  setTimeout(() => {
    const element = document.querySelector(`[data-scenery-index="${index}"]`)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'center' })
      highlightedIndex.value = index
    }
  }, 100)
}

function handleSearchInput() {
  if (matchedIndices.value.length > 0) {
    currentMatchIndex.value = 0
    scrollToMatch(matchedIndices.value[0])
  } else {
    highlightedIndex.value = -1
  }
}

function goToNextMatch() {
  if (matchedIndices.value.length === 0) return
  currentMatchIndex.value = (currentMatchIndex.value + 1) % matchedIndices.value.length
  scrollToMatch(matchedIndices.value[currentMatchIndex.value])
}

function goToPrevMatch() {
  if (matchedIndices.value.length === 0) return
  currentMatchIndex.value = (currentMatchIndex.value - 1 + matchedIndices.value.length) % matchedIndices.value.length
  scrollToMatch(matchedIndices.value[currentMatchIndex.value])
}

function clearSearch() {
  searchQuery.value = ''
  highlightedIndex.value = -1
  currentMatchIndex.value = 0
}
</script>

<template>
  <div class="scenery-manager-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Header with search and action buttons -->
    <div class="mb-3 flex-shrink-0 flex items-center gap-3">
      <!-- Search box -->
      <div class="flex-1 relative">
        <input
          v-model="searchQuery"
          @input="handleSearchInput"
          type="text"
          :placeholder="t('sceneryManager.searchPlaceholder')"
          class="w-full px-3 py-1.5 pl-9 pr-20 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>

        <!-- Search navigation buttons -->
        <div v-if="searchQuery && matchedIndices.length > 0" class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
          <span class="text-xs text-gray-500 dark:text-gray-400 mr-1">
            {{ currentMatchIndex + 1 }}/{{ matchedIndices.length }}
          </span>
          <button
            @click="goToPrevMatch"
            class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            title="Previous match"
          >
            <svg class="w-3 h-3 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
            </svg>
          </button>
          <button
            @click="goToNextMatch"
            class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            title="Next match"
          >
            <svg class="w-3 h-3 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </button>
        </div>

        <!-- Clear button -->
        <button
          v-if="searchQuery"
          @click="clearSearch"
          class="absolute right-2 top-1/2 -translate-y-1/2 p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
          :class="{ 'right-20': matchedIndices.length > 0 }"
          title="Clear search"
        >
          <svg class="w-3 h-3 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Action buttons -->
      <div class="flex items-center gap-2">
        <!-- Auto-sort button (Chinese locale only - shown directly) -->
        <Transition name="button-fade" mode="out-in">
          <button
            v-if="isChineseLocale"
            key="auto-sort-button"
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
            <span class="transition-opacity">{{ isSortingScenery ? t('settings.sorting') : t('sceneryManager.autoSort') }}</span>
          </button>
        </Transition>

        <!-- More actions dropdown (English locale only) -->
        <Transition name="button-fade" mode="out-in">
          <div v-if="!isChineseLocale" key="more-menu" ref="moreMenuRef" class="relative">
            <button
              @click.stop="showMoreMenu = !showMoreMenu"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              :title="t('sceneryManager.moreActions')"
            >
              <svg class="w-4 h-4 text-gray-600 dark:text-gray-400" fill="currentColor" viewBox="0 0 24 24">
                <circle cx="12" cy="5" r="2" />
                <circle cx="12" cy="12" r="2" />
                <circle cx="12" cy="19" r="2" />
              </svg>
            </button>

            <!-- Dropdown menu -->
            <div
              v-if="showMoreMenu"
              class="absolute right-0 mt-1 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-50"
            >
              <button
                @click="handleSortSceneryNow"
                :disabled="isSortingScenery || !appStore.xplanePath"
                class="w-full px-3 py-2 text-left text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
              >
                <svg v-if="!isSortingScenery" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"></path>
                </svg>
                <svg v-else class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                </svg>
                <span class="transition-opacity">{{ isSortingScenery ? t('settings.sorting') : t('sceneryManager.autoSort') }}</span>
              </button>
            </div>
          </div>
        </Transition>

        <button
          v-if="sceneryStore.hasChanges"
          @click="handleReset"
          class="px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-sm"
        >
          <Transition name="text-fade" mode="out-in">
            <span :key="locale">{{ t('sceneryManager.reset') }}</span>
          </Transition>
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
          <Transition name="text-fade" mode="out-in">
            <span :key="locale">{{ t('sceneryManager.applyChanges') }}</span>
          </Transition>
        </button>
      </div>
    </div>

    <!-- Statistics bar -->
    <div class="flex items-center gap-4 px-3 py-2 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 mb-3 text-sm">
      <div class="flex items-center gap-2">
        <Transition name="text-fade" mode="out-in">
          <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.total') }}:</span>
        </Transition>
        <span class="font-semibold text-gray-900 dark:text-gray-100">{{ sceneryStore.totalCount }}</span>
      </div>
      <div class="flex items-center gap-2">
        <Transition name="text-fade" mode="out-in">
          <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.enabled') }}:</span>
        </Transition>
        <span class="font-semibold text-green-600 dark:text-green-400">{{ sceneryStore.enabledCount }}</span>
      </div>
      <div v-if="sceneryStore.missingDepsCount > 0" class="flex items-center gap-2">
        <Transition name="text-fade" mode="out-in">
          <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.missingDeps') }}:</span>
        </Transition>
        <span class="font-semibold text-amber-600 dark:text-amber-400">{{ sceneryStore.missingDepsCount }}</span>
        <button
          @click="showOnlyMissingLibs = !showOnlyMissingLibs"
          class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
          :class="showOnlyMissingLibs
            ? 'bg-amber-500 text-white hover:bg-amber-600'
            : 'bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400 hover:bg-amber-200 dark:hover:bg-amber-900/50'"
          :title="t('sceneryManager.filterMissingLibs')"
        >
          <Transition name="text-fade" mode="out-in">
            <span :key="locale">{{ showOnlyMissingLibs ? t('sceneryManager.showAll') : t('sceneryManager.filterOnly') }}</span>
          </Transition>
        </button>
      </div>
      <div v-if="sceneryStore.hasChanges" class="ml-auto flex items-center gap-2 text-blue-600 dark:text-blue-400">
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <Transition name="text-fade" mode="out-in">
          <span :key="locale" class="text-xs font-medium">{{ t('sceneryManager.unsavedChanges') }}</span>
        </Transition>
      </div>
    </div>

    <!-- Content -->
    <div ref="listContainerRef" class="flex-1 overflow-y-auto" style="overflow-x: hidden;">
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

      <!-- Filtered view (no drag-and-drop) -->
      <div v-else-if="showOnlyMissingLibs" class="space-y-1.5 px-1">
        <div
          v-for="(element, index) in filteredEntries"
          :key="element.folderName"
          :data-scenery-index="index"
          class="relative"
          style="scroll-margin-top: 100px"
        >
          <!-- Highlight ring overlay -->
          <div
            v-if="highlightedIndex === index"
            class="absolute inset-0 ring-4 ring-blue-500 rounded-lg pointer-events-none"
          ></div>

          <!-- Content with opacity effect for non-matches -->
          <div
            :class="{
              'opacity-30 transition-opacity': searchQuery && !element.folderName.toLowerCase().includes(searchQuery.toLowerCase())
            }"
          >
            <SceneryEntryCard
              :entry="element"
              :index="index"
              :total-count="sceneryStore.totalCount"
              :disable-reorder="true"
              @toggle-enabled="handleToggleEnabled"
              @move-up="handleMoveUp"
              @move-down="handleMoveDown"
            />
          </div>
        </div>
        <div v-if="filteredEntries.length === 0" class="text-center py-12">
          <p class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noMissingLibs') }}</p>
        </div>
      </div>

      <!-- Normal view with drag-and-drop groups -->
      <div v-else class="space-y-3" style="overflow: visible;">
        <template
          v-for="category in categoryOrder"
          :key="category"
        >
          <div
            v-if="localGroupedEntries[category] && localGroupedEntries[category].length > 0"
            class="scenery-group"
            style="overflow: visible;"
          >
          <!-- Group Header -->
          <div
            @click="toggleGroupCollapse(category)"
            class="group-header flex items-center gap-2 px-3 py-1.5 bg-gradient-to-r from-gray-100 to-gray-200 dark:from-gray-700 dark:to-gray-600 rounded-lg cursor-pointer hover:from-gray-200 hover:to-gray-300 dark:hover:from-gray-600 dark:hover:to-gray-500 transition-all duration-200 mb-2 border border-gray-300 dark:border-gray-500 shadow-md"
          >
            <!-- Collapse/Expand Icon -->
            <svg
              class="w-4 h-4 text-gray-700 dark:text-gray-200 transition-transform duration-200"
              :class="{ 'rotate-90': !sceneryStore.collapsedGroups[category] }"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
            </svg>

            <!-- Category Name and Count -->
            <span class="font-semibold text-sm text-gray-900 dark:text-gray-50">
              {{ t(getCategoryTranslationKey(category)) }}
            </span>
            <span class="text-xs font-medium text-gray-600 dark:text-gray-300 bg-white dark:bg-gray-800 px-2 py-0.5 rounded-full">
              {{ localGroupedEntries[category]?.length || 0 }}
            </span>
          </div>

          <!-- Group Content (Collapsible) -->
          <Transition name="collapse">
            <div v-if="!sceneryStore.collapsedGroups[category]" style="overflow: visible;">
              <draggable
                v-model="localGroupedEntries[category]"
                :group="{ name: 'scenery', pull: true, put: true }"
                item-key="folderName"
                handle=".drag-handle"
                :animation="180"
                :easing="'cubic-bezier(0.25, 0.8, 0.25, 1)'"
                :force-fallback="true"
                :fallback-tolerance="5"
                :direction="'vertical'"
                ghost-class="drag-ghost"
                drag-class="sortable-drag"
                @start="handleDragStart"
                @end="handleDragEnd"
                @change="(evt) => handleGroupChange(category, evt)"
                class="space-y-1.5"
                style="overflow: visible; padding: 0 0.5rem;"
              >
                <template #item="{ element, index }">
                  <div
                    :data-scenery-index="index"
                    class="relative"
                    style="scroll-margin-top: 100px"
                  >
                    <!-- Highlight ring overlay -->
                    <div
                      v-if="highlightedIndex === index"
                      class="absolute inset-0 ring-4 ring-blue-500 rounded-lg pointer-events-none"
                    ></div>

                    <!-- Content with opacity effect for non-matches -->
                    <div
                      :class="{
                        'opacity-30 transition-opacity': searchQuery && !element.folderName.toLowerCase().includes(searchQuery.toLowerCase())
                      }"
                    >
                      <SceneryEntryCard
                        :entry="element"
                        :index="getGlobalIndex(element.folderName)"
                        :total-count="sceneryStore.totalCount"
                        @toggle-enabled="handleToggleEnabled"
                        @move-up="handleMoveUp"
                        @move-down="handleMoveDown"
                      />
                    </div>
                  </div>
                </template>
              </draggable>
            </div>
          </Transition>
        </div>
        </template>
      </div>
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

/* Collapse/Expand transition */
.collapse-enter-active,
.collapse-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.collapse-enter-from,
.collapse-leave-to {
  max-height: 0;
  opacity: 0;
}

.collapse-enter-to,
.collapse-leave-from {
  max-height: 10000px;
  opacity: 1;
}

/* Button fade transition for language switching */
.button-fade-leave-active {
  transition: none;
}

.button-fade-enter-active {
  transition: opacity 0.25s ease-in;
}

.button-fade-enter-from,
.button-fade-leave-to {
  opacity: 0;
}

/* Text fade transition for language switching */
.text-fade-leave-active {
  transition: none;
}

.text-fade-enter-active {
  transition: opacity 0.2s ease-in;
}

.text-fade-enter-from,
.text-fade-leave-to {
  opacity: 0;
}

:global(.hidden-ghost) {
  opacity: 0 !important;
  pointer-events: none !important;
}

:global(.drag-ghost) {
  opacity: 0.35;
  transition: transform 0.22s cubic-bezier(0.25, 0.8, 0.25, 1), opacity 0.22s ease;
}

:global(.dragging-scale) {
  opacity: 0 !important;
}

:global(.sortable-fallback) {
  opacity: 1 !important;
  transform: scale(1.02) !important;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.2), 0 0 0 2px rgb(59, 130, 246) !important;
  border-radius: 0.5rem !important;
  transition: none !important;
  position: fixed !important;
  z-index: 100000 !important;
  pointer-events: none !important;
  background-color: white !important;
}

:global(.dark .sortable-fallback) {
  background-color: rgb(31, 41, 55) !important;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.4), 0 0 0 2px rgb(96, 165, 250) !important;
}

:global(.sortable-chosen) {
  opacity: 0 !important;
}

:global(.sortable-drag) {
  opacity: 0 !important;
}
</style>
