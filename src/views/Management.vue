<script setup lang="ts">
import { ref, onMounted, computed, watch, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { useManagementStore } from '@/stores/management'
import { useSceneryStore } from '@/stores/scenery'
import { useToastStore } from '@/stores/toast'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { invoke } from '@tauri-apps/api/core'
import ManagementEntryCard from '@/components/ManagementEntryCard.vue'
import SceneryEntryCard from '@/components/SceneryEntryCard.vue'
import draggable from 'vuedraggable'
import type { SceneryManagerEntry, ManagementTab, ManagementItemType } from '@/types'

const { t, locale } = useI18n()
const route = useRoute()
const managementStore = useManagementStore()
const sceneryStore = useSceneryStore()
const toastStore = useToastStore()
const appStore = useAppStore()
const modalStore = useModalStore()

// Check if current language is Chinese
const isChineseLocale = computed(() => locale.value === 'zh')

// Tab state
const activeTab = ref<ManagementTab>('aircraft')

// Available tabs based on settings
const availableTabs = computed(() => {
  const tabs: ManagementTab[] = ['aircraft', 'plugin', 'navdata']
  if (appStore.autoSortScenery) {
    tabs.push('scenery')
  }
  return tabs
})

// Transition direction for tab switching
const tabTransitionName = ref('tab-slide-left')

// Search state
const searchQuery = ref('')

// Toggling state to prevent rapid clicks
const togglingItems = ref<Set<string>>(new Set())

// Scenery-specific state (migrated from SceneryManager.vue)
const drag = ref(false)
const isSortingScenery = ref(false)
const isCreatingIndex = ref(false)
const highlightedIndex = ref(-1)
const currentMatchIndex = ref(0)
const searchExpandedGroups = ref<Record<string, boolean>>({})
const showOnlyMissingLibs = ref(false)
const showMoreMenu = ref(false)
const suppressLoading = ref(false)
const moreMenuRef = ref<HTMLElement | null>(null)

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

// Initialize tab from route query
onMounted(async () => {
  const tabParam = route.query.tab as ManagementTab | undefined
  if (tabParam && availableTabs.value.includes(tabParam)) {
    activeTab.value = tabParam
  }

  document.addEventListener('click', handleClickOutside)

  // Load initial data
  await loadTabData(activeTab.value)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside)
})

// Watch for tab changes
watch(activeTab, async (newTab, oldTab) => {
  // Determine transition direction
  const oldIndex = availableTabs.value.indexOf(oldTab)
  const newIndex = availableTabs.value.indexOf(newTab)
  tabTransitionName.value = newIndex > oldIndex ? 'tab-slide-left' : 'tab-slide-right'

  searchQuery.value = ''
  await loadTabData(newTab)
})

async function loadTabData(tab: ManagementTab) {
  if (!appStore.xplanePath) return

  try {
    switch (tab) {
      case 'aircraft':
        await managementStore.loadAircraft()
        if (managementStore.error) {
          modalStore.showError(t('management.scanFailed') + ': ' + managementStore.error)
        }
        break
      case 'plugin':
        await managementStore.loadPlugins()
        if (managementStore.error) {
          modalStore.showError(t('management.scanFailed') + ': ' + managementStore.error)
        }
        break
      case 'navdata':
        await managementStore.loadNavdata()
        if (managementStore.error) {
          modalStore.showError(t('management.scanFailed') + ': ' + managementStore.error)
        }
        break
      case 'scenery':
        await sceneryStore.loadData()
        if (sceneryStore.error) {
          modalStore.showError(t('management.scanFailed') + ': ' + sceneryStore.error)
        }
        syncLocalEntries()
        break
    }
  } catch (e) {
    modalStore.showError(t('management.scanFailed') + ': ' + String(e))
  }
}

// Filtered entries for non-scenery tabs
const filteredAircraft = computed(() => {
  if (!searchQuery.value.trim()) return managementStore.sortedAircraft
  const query = searchQuery.value.toLowerCase()
  return managementStore.sortedAircraft.filter(a =>
    a.displayName.toLowerCase().includes(query) ||
    a.folderName.toLowerCase().includes(query)
  )
})

const filteredPlugins = computed(() => {
  if (!searchQuery.value.trim()) return managementStore.sortedPlugins
  const query = searchQuery.value.toLowerCase()
  return managementStore.sortedPlugins.filter(p =>
    p.displayName.toLowerCase().includes(query) ||
    p.folderName.toLowerCase().includes(query)
  )
})

const filteredNavdata = computed(() => {
  if (!searchQuery.value.trim()) return managementStore.sortedNavdata
  const query = searchQuery.value.toLowerCase()
  return managementStore.sortedNavdata.filter(n =>
    n.providerName.toLowerCase().includes(query) ||
    n.folderName.toLowerCase().includes(query)
  )
})

// Handle toggle for non-scenery items
async function handleToggleEnabled(itemType: ManagementItemType, folderName: string) {
  // Prevent rapid clicks
  const key = `${itemType}:${folderName}`
  if (togglingItems.value.has(key)) {
    return
  }

  togglingItems.value.add(key)
  try {
    await managementStore.toggleEnabled(itemType, folderName)
  } catch (e) {
    // Reload to get the actual state
    await loadTabData(activeTab.value)
    modalStore.showError(t('management.toggleFailed') + ': ' + String(e))
  } finally {
    togglingItems.value.delete(key)
  }
}

// Handle delete for non-scenery items
async function handleDelete(itemType: ManagementItemType, folderName: string) {
  try {
    await managementStore.deleteItem(itemType, folderName)
    toastStore.success(t('management.deleteSuccess'))
  } catch (e) {
    modalStore.showError(t('management.deleteFailed') + ': ' + String(e))
  }
}

// Handle open folder for non-scenery items
async function handleOpenFolder(itemType: ManagementItemType, folderName: string) {
  try {
    await managementStore.openFolder(itemType, folderName)
  } catch (e) {
    modalStore.showError(t('management.openFolderFailed') + ': ' + String(e))
  }
}

// ========== Scenery-specific functions (migrated from SceneryManager.vue) ==========

const groupCounts = computed(() => {
  const counts: Record<string, { enabled: number; disabled: number }> = {}
  for (const category of categoryOrder) {
    const entries = localGroupedEntries.value[category] || []
    const enabled = entries.filter(entry => entry.enabled).length
    counts[category] = { enabled, disabled: entries.length - enabled }
  }
  return counts
})

const filteredSceneryEntries = computed(() => {
  const allEntries = categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])
  if (!showOnlyMissingLibs.value) return allEntries
  return allEntries.filter(entry => entry.missingLibraries && entry.missingLibraries.length > 0)
})

const globalIndexMap = computed(() => {
  const map = new Map<string, number>()
  const allEntries = categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])
  allEntries.forEach((entry, index) => {
    map.set(entry.folderName, index)
  })
  return map
})

function isGroupExpanded(category: string): boolean {
  return !sceneryStore.collapsedGroups[category] || !!searchExpandedGroups.value[category]
}

const matchedIndices = computed(() => {
  if (!searchQuery.value.trim() || activeTab.value !== 'scenery') return []
  const query = searchQuery.value.toLowerCase()
  return filteredSceneryEntries.value
    .map((entry, index) => ({ entry, index }))
    .filter(({ entry }) => entry.folderName.toLowerCase().includes(query))
    .map(({ index }) => index)
})

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

function toggleGroupCollapse(category: string) {
  const expanded = isGroupExpanded(category)
  if (expanded) {
    sceneryStore.collapsedGroups[category] = true
    if (searchExpandedGroups.value[category]) {
      delete searchExpandedGroups.value[category]
    }
  } else {
    sceneryStore.collapsedGroups[category] = false
  }
}

function getCategoryTranslationKey(category: string): string {
  return `sceneryManager.category${category}`
}

function handleDragStart() {
  drag.value = true
}

async function handleSceneryToggleEnabled(folderName: string) {
  await sceneryStore.toggleEnabled(folderName)
  syncLocalEntries()
}

async function handleMoveUp(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)

  if (index > 0) {
    const currentEntry = entries[index]
    const targetEntry = entries[index - 1]

    if (currentEntry.category !== targetEntry.category) {
      await sceneryStore.updateCategory(folderName, targetEntry.category)
    } else {
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

    if (currentEntry.category !== targetEntry.category) {
      await sceneryStore.updateCategory(folderName, targetEntry.category)
    } else {
      await sceneryStore.moveEntry(folderName, index + 1)
    }
    syncLocalEntries()
  }
}

async function handleDragEnd() {
  drag.value = false
  const allEntries = categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])
  await sceneryStore.reorderEntries(allEntries)
  syncLocalEntries()
}

function getGlobalIndex(folderName: string): number {
  return globalIndexMap.value.get(folderName) ?? -1
}

async function handleGroupChange(category: string, evt: any) {
  if (evt.added) {
    const entry = evt.added.element
    const newCategory = category as any

    try {
      await sceneryStore.updateCategory(entry.folderName, newCategory)
    } catch (e) {
      console.error('Failed to update category:', e)
      suppressLoading.value = true
      try {
        await sceneryStore.loadData()
        syncLocalEntries()
      } catch (reloadError) {
        console.error('Failed to reload scenery data:', reloadError)
      } finally {
        suppressLoading.value = false
      }
    }
  }
}

async function handleApplyChanges() {
  try {
    await sceneryStore.applyChanges()
    toastStore.success(t('sceneryManager.changesApplied'))
    syncLocalEntries()
  } catch (e) {
    modalStore.showError(t('sceneryManager.applyFailed'))
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
  if (!sceneryStore.indexExists) return
  isSortingScenery.value = true
  try {
    const hasChanges = await invoke<boolean>('sort_scenery_packs', { xplanePath: appStore.xplanePath })
    await sceneryStore.loadData()
    syncLocalEntries()

    if (sceneryStore.hasChanges) {
      toastStore.success(t('sceneryManager.autoSortDone'))
    } else if (hasChanges) {
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
  if (isSortingScenery.value || !appStore.xplanePath || !sceneryStore.indexExists) return

  showMoreMenu.value = false

  modalStore.showConfirm({
    title: t('sceneryManager.autoSort'),
    message: t('sceneryManager.autoSortConfirm'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => {
      setTimeout(() => {
        performAutoSort()
      }, 0)
    },
    onCancel: () => {}
  })
}

async function handleCreateIndex() {
  if (isCreatingIndex.value || !appStore.xplanePath) return

  isCreatingIndex.value = true
  try {
    await invoke('rebuild_scenery_index', { xplanePath: appStore.xplanePath })
    await sceneryStore.loadData()
    syncLocalEntries()
    toastStore.success(t('settings.indexRebuilt'))
  } catch (e) {
    modalStore.showError(t('settings.indexRebuildFailed') + ': ' + String(e))
  } finally {
    isCreatingIndex.value = false
  }
}

function handleClickOutside(event: MouseEvent) {
  if (moreMenuRef.value && !moreMenuRef.value.contains(event.target as Node)) {
    showMoreMenu.value = false
  }
}

// Search navigation functions
function ensureGroupExpandedForIndex(index: number) {
  if (showOnlyMissingLibs.value) return
  const entry = filteredSceneryEntries.value[index]
  if (!entry) return
  if (sceneryStore.collapsedGroups[entry.category]) {
    searchExpandedGroups.value[entry.category] = true
  }
}

function scrollToMatch(index: number) {
  ensureGroupExpandedForIndex(index)
  highlightedIndex.value = index

  const attemptScroll = (attempt: number) => {
    if (highlightedIndex.value !== index) return
    const element = document.querySelector(`[data-scenery-index="${index}"]`) as HTMLElement | null
    if (element && element.getClientRects().length > 0) {
      element.scrollIntoView({ behavior: 'smooth', block: 'center' })
      return
    }
    if (attempt < 6) {
      setTimeout(() => attemptScroll(attempt + 1), 60)
    }
  }

  setTimeout(() => attemptScroll(0), 0)
}

function handleSearchInput() {
  if (activeTab.value !== 'scenery') return

  if (!searchQuery.value.trim()) {
    highlightedIndex.value = -1
    currentMatchIndex.value = 0
    searchExpandedGroups.value = {}
    return
  }

  if (matchedIndices.value.length > 0) {
    currentMatchIndex.value = 0
    scrollToMatch(matchedIndices.value[0])
  } else {
    highlightedIndex.value = -1
    searchExpandedGroups.value = {}
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
  searchExpandedGroups.value = {}
}

// Current loading state
const isLoading = computed(() => {
  if (activeTab.value === 'scenery') {
    return sceneryStore.isLoading && !suppressLoading.value
  }
  return managementStore.isLoading
})
</script>

<template>
  <div class="management-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Tab Bar -->
    <div class="mb-3 flex-shrink-0 flex items-center gap-1 p-1 bg-gray-100 dark:bg-gray-800 rounded-lg">
      <button
        v-for="tab in availableTabs"
        :key="tab"
        @click="activeTab = tab"
        class="flex-1 px-3 py-1.5 rounded-md text-sm font-medium transition-all"
        :class="activeTab === tab
          ? 'bg-white dark:bg-gray-700 text-blue-600 dark:text-blue-400 shadow-sm'
          : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'"
      >
        <Transition name="text-fade" mode="out-in">
          <span :key="locale">{{ t(`management.${tab}`) }}</span>
        </Transition>
      </button>
    </div>

    <!-- Header with search and action buttons -->
    <div class="mb-3 flex-shrink-0 flex items-center gap-3">
      <!-- Search box -->
      <div class="flex-1 relative">
        <input
          v-model="searchQuery"
          @input="handleSearchInput"
          type="text"
          :placeholder="t('management.searchPlaceholder')"
          class="w-full px-3 py-1.5 pl-9 pr-20 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>

        <!-- Search navigation buttons (scenery only) -->
        <div v-if="activeTab === 'scenery' && searchQuery && matchedIndices.length > 0" class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
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
          :class="{ 'right-20': activeTab === 'scenery' && matchedIndices.length > 0 }"
          title="Clear search"
        >
          <svg class="w-3 h-3 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Scenery-specific action buttons -->
      <template v-if="activeTab === 'scenery'">
        <!-- Auto-sort button (Chinese locale only - shown directly) -->
        <Transition name="button-fade" mode="out-in">
          <button
            v-if="isChineseLocale"
            key="auto-sort-button"
            @click="handleSortSceneryNow"
            :disabled="isSortingScenery || !appStore.xplanePath || !sceneryStore.indexExists"
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
                :disabled="isSortingScenery || !appStore.xplanePath || !sceneryStore.indexExists"
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
          :disabled="!sceneryStore.indexExists || !sceneryStore.hasChanges || sceneryStore.isSaving"
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
      </template>
    </div>

    <!-- Statistics bar -->
    <div class="flex items-center gap-4 px-3 py-2 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 mb-3 text-sm">
      <!-- Non-scenery stats -->
      <template v-if="activeTab !== 'scenery'">
        <div class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('management.total') }}:</span>
          </Transition>
          <span class="font-semibold text-gray-900 dark:text-gray-100">
            {{ activeTab === 'aircraft' ? managementStore.aircraftTotalCount :
               activeTab === 'plugin' ? managementStore.pluginsTotalCount :
               managementStore.navdataTotalCount }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('management.enabled') }}:</span>
          </Transition>
          <span class="font-semibold text-green-600 dark:text-green-400">
            {{ activeTab === 'aircraft' ? managementStore.aircraftEnabledCount :
               activeTab === 'plugin' ? managementStore.pluginsEnabledCount :
               managementStore.navdataEnabledCount }}
          </span>
        </div>
      </template>

      <!-- Scenery stats -->
      <template v-else>
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
      </template>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto tab-content-container">
      <!-- No X-Plane path set -->
      <div v-if="!appStore.xplanePath" class="flex items-center justify-center h-full">
        <div class="text-center">
          <svg class="w-16 h-16 mx-auto text-gray-400 dark:text-gray-600 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <Transition name="text-fade" mode="out-in">
            <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('settings.sceneryAutoSortNeedPath') }}</p>
          </Transition>
        </div>
      </div>

      <!-- Loading state -->
      <div v-else-if="isLoading" class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>

      <!-- Tab Content with Transition -->
      <Transition :name="tabTransitionName" mode="out-in" v-else>
        <div :key="activeTab" class="tab-content-wrapper">
          <!-- Aircraft Tab Content -->
          <template v-if="activeTab === 'aircraft'">
            <div class="space-y-1.5 px-1">
              <div v-if="filteredAircraft.length === 0" class="text-center py-12">
                <Transition name="text-fade" mode="out-in">
                  <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('management.noItems') }}</p>
                </Transition>
              </div>
              <ManagementEntryCard
                v-for="item in filteredAircraft"
                :key="item.folderName"
                :entry="item"
                item-type="aircraft"
                :is-toggling="togglingItems.has(`aircraft:${item.folderName}`)"
                @toggle-enabled="(fn) => handleToggleEnabled('aircraft', fn)"
                @delete="(fn) => handleDelete('aircraft', fn)"
                @open-folder="(fn) => handleOpenFolder('aircraft', fn)"
              />
            </div>
          </template>

          <!-- Plugin Tab Content -->
          <template v-else-if="activeTab === 'plugin'">
            <div class="space-y-1.5 px-1">
              <div v-if="filteredPlugins.length === 0" class="text-center py-12">
                <Transition name="text-fade" mode="out-in">
                  <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('management.noItems') }}</p>
                </Transition>
              </div>
              <ManagementEntryCard
                v-for="item in filteredPlugins"
                :key="item.folderName"
                :entry="item"
                item-type="plugin"
                :is-toggling="togglingItems.has(`plugin:${item.folderName}`)"
                @toggle-enabled="(fn) => handleToggleEnabled('plugin', fn)"
                @delete="(fn) => handleDelete('plugin', fn)"
                @open-folder="(fn) => handleOpenFolder('plugin', fn)"
              />
            </div>
          </template>

          <!-- Navdata Tab Content -->
          <template v-else-if="activeTab === 'navdata'">
            <div class="space-y-1.5 px-1">
              <div v-if="filteredNavdata.length === 0" class="text-center py-12">
                <Transition name="text-fade" mode="out-in">
                  <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('management.noItems') }}</p>
                </Transition>
              </div>
              <ManagementEntryCard
                v-for="item in filteredNavdata"
                :key="item.folderName"
                :entry="item"
                item-type="navdata"
                :is-toggling="togglingItems.has(`navdata:${item.folderName}`)"
                @toggle-enabled="(fn) => handleToggleEnabled('navdata', fn)"
                @delete="(fn) => handleDelete('navdata', fn)"
                @open-folder="(fn) => handleOpenFolder('navdata', fn)"
              />
            </div>
          </template>

          <!-- Scenery Tab Content (migrated from SceneryManager.vue) -->
          <template v-else-if="activeTab === 'scenery'">
        <!-- No index created -->
        <div v-if="!sceneryStore.indexExists" class="text-center py-12">
          <Transition name="text-fade" mode="out-in">
            <p :key="locale" class="text-gray-600 dark:text-gray-400 mb-4">{{ t('sceneryManager.noIndex') }}</p>
          </Transition>
          <div class="flex justify-center">
            <button
              @click="handleCreateIndex"
              :disabled="isCreatingIndex"
              class="px-4 py-2 rounded-lg bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm flex items-center justify-center space-x-2"
            >
              <svg v-if="!isCreatingIndex" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
              </svg>
              <svg v-else class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
              </svg>
              <Transition name="text-fade" mode="out-in">
                <span :key="locale">{{ isCreatingIndex ? t('settings.creatingIndex') : t('settings.createIndex') }}</span>
              </Transition>
            </button>
          </div>
        </div>

        <!-- No scenery found -->
        <div v-else-if="sceneryStore.totalCount === 0" class="text-center py-12">
          <Transition name="text-fade" mode="out-in">
            <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noScenery') }}</p>
          </Transition>
        </div>

        <!-- Filtered view (no drag-and-drop) -->
        <div v-else-if="showOnlyMissingLibs" class="space-y-1.5 px-1">
          <div
            v-for="(element, index) in filteredSceneryEntries"
            :key="element.folderName"
            :data-scenery-index="index"
            class="relative"
            style="scroll-margin-top: 100px"
          >
            <div
              v-if="highlightedIndex === index"
              class="absolute inset-0 ring-4 ring-blue-500 rounded-lg pointer-events-none"
            ></div>
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
                @toggle-enabled="handleSceneryToggleEnabled"
                @move-up="handleMoveUp"
                @move-down="handleMoveDown"
              />
            </div>
          </div>
          <div v-if="filteredSceneryEntries.length === 0" class="text-center py-12">
            <Transition name="text-fade" mode="out-in">
              <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noMissingLibs') }}</p>
            </Transition>
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
                <svg
                  class="w-4 h-4 text-gray-700 dark:text-gray-200 transition-transform duration-200"
                  :class="{ 'rotate-90': isGroupExpanded(category) }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
                </svg>
                <span class="font-semibold text-sm text-gray-900 dark:text-gray-50">
                  <Transition name="text-fade" mode="out-in">
                    <span :key="locale">{{ t(getCategoryTranslationKey(category)) }}</span>
                  </Transition>
                </span>
                <span class="text-xs font-medium text-gray-600 dark:text-gray-300 bg-white dark:bg-gray-800 px-2 py-0.5 rounded-full">
                  <span class="text-green-700 dark:text-green-300">{{ groupCounts[category]?.enabled ?? 0 }}</span>
                  <span class="mx-1 text-gray-400">/</span>
                  <span class="text-gray-600 dark:text-gray-400">{{ localGroupedEntries[category]?.length || 0 }}</span>
                </span>
              </div>

              <!-- Group Content (Collapsible) -->
              <Transition name="collapse">
                <div v-if="isGroupExpanded(category)" style="overflow: visible;">
                  <draggable
                    v-model="localGroupedEntries[category]"
                    :group="{ name: 'scenery', pull: true, put: true }"
                    item-key="folderName"
                    handle=".drag-handle"
                    :disabled="!sceneryStore.indexExists"
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
                    <template #item="{ element }">
                      <div
                        :data-scenery-index="getGlobalIndex(element.folderName)"
                        class="relative"
                        style="scroll-margin-top: 100px"
                      >
                        <div
                          v-if="highlightedIndex === getGlobalIndex(element.folderName)"
                          class="absolute inset-0 ring-4 ring-blue-500 rounded-lg pointer-events-none"
                        ></div>
                        <div
                          :class="{
                            'opacity-30 transition-opacity': searchQuery && !element.folderName.toLowerCase().includes(searchQuery.toLowerCase())
                          }"
                        >
                          <SceneryEntryCard
                            :entry="element"
                            :index="getGlobalIndex(element.folderName)"
                            :total-count="sceneryStore.totalCount"
                            :disable-reorder="!sceneryStore.indexExists"
                            @toggle-enabled="handleSceneryToggleEnabled"
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
          </template>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.management-view {
  background: linear-gradient(to bottom, rgba(248, 250, 252, 0.5), rgba(241, 245, 249, 0.5));
}

.dark .management-view {
  background: linear-gradient(to bottom, rgba(17, 24, 39, 0.5), rgba(31, 41, 55, 0.5));
}

/* Tab content container - hide overflow except during transition */
.tab-content-container {
  overflow-x: hidden;
}

/* Tab content wrapper for transitions */
.tab-content-wrapper {
  width: 100%;
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

/* Tab slide transitions */
.tab-slide-left-enter-active,
.tab-slide-left-leave-active,
.tab-slide-right-enter-active,
.tab-slide-right-leave-active {
  transition: all 0.25s ease;
}

.tab-slide-left-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.tab-slide-left-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

.tab-slide-right-enter-from {
  opacity: 0;
  transform: translateX(-20px);
}

.tab-slide-right-leave-to {
  opacity: 0;
  transform: translateX(20px);
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
  opacity: 0.35 !important;
}

:global(.sortable-drag) {
  opacity: 1 !important;
}
</style>
