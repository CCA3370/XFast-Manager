<script setup lang="ts">
import { ref, onMounted, computed, watch, onBeforeUnmount, defineAsyncComponent, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useManagementStore } from '@/stores/management'
import { useSceneryStore } from '@/stores/scenery'
import { useToastStore } from '@/stores/toast'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { invoke } from '@tauri-apps/api/core'
import { logError } from '@/services/logger'
import { getNavdataCycleStatus } from '@/utils/airac'
import ManagementEntryCard from '@/components/ManagementEntryCard.vue'
import ConfirmModal from '@/components/ConfirmModal.vue'
import type { SceneryManagerEntry, ManagementTab, ManagementItemType, SceneryCategory, SceneryIndexScanResult, NavdataBackupInfo } from '@/types'
import { parseApiError, getErrorMessage } from '@/types'

// Lazy load heavy components to reduce initial render time
const SceneryEntryCard = defineAsyncComponent(() => import('@/components/SceneryEntryCard.vue'))
const draggable = defineAsyncComponent(() => import('vuedraggable'))

const { t, locale } = useI18n()
const route = useRoute()
const router = useRouter()
const managementStore = useManagementStore()
const sceneryStore = useSceneryStore()
const toastStore = useToastStore()
const appStore = useAppStore()
const modalStore = useModalStore()

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

// Active tab index for indicator position
const activeTabIndex = computed(() => {
  return availableTabs.value.indexOf(activeTab.value)
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
const searchExpandedContinents = ref<Record<string, boolean>>({})
const searchExpandedContinentCategories = ref<Record<string, boolean>>({})
const showOnlyMissingLibs = ref(false)
const showOnlyDuplicateTiles = ref(false)
const showOnlyUpdates = ref(false)
const showOnlyOutdated = ref(false)
const showMoreMenu = ref(false)
const suppressLoading = ref(false)
const moreMenuRef = ref<HTMLElement | null>(null)
const scrollContainerRef = ref<HTMLElement | null>(null)
const syncWarningDismissed = ref(false)
const showFilterDropdown = ref(false)
const filterDropdownRef = ref<HTMLElement | null>(null)
const enabledFilter = ref<'all' | 'enabled' | 'disabled'>('all')
const isFilterTransitioning = ref(false)
let activeScrollRequestId = 0
const COLLAPSE_TRANSITION_WAIT_MS = 380
const FINAL_CALIBRATION_VIEWPORT_PADDING_PX = 28
const CONTRIBUTION_SUBMIT_TIMEOUT_MS = 10000

// Shared modal state for scenery entry actions
const selectedModalEntry = ref<SceneryManagerEntry | null>(null)
const showMissingLibsModal = ref(false)
const showDuplicateTilesModal = ref(false)
const showDeleteConfirmModal = ref(false)
const libraryLinksMap = ref<Record<string, string | null>>({})
const isLoadingLinks = ref(false)
const libraryLinksRequestSeq = ref(0)
const showContributeLinkModal = ref(false)
const contributingLibName = ref('')
const contributingLibUrl = ref('')
const isSubmittingContributeLink = ref(false)
const isDeletingEntry = ref(false)

// 拖拽自动滚动状态 (非响应式，无需触发渲染)
let dragAutoScrollRafId: number | null = null
let dragPointerY: number = 0

// Geo filtering state
const selectedContinent = ref<string>('')

// View mode state for scenery tab
const viewMode = ref<'category' | 'continent'>('category')
const isViewModeTransitioning = ref(false)

// Collapsed state for continent groups
const collapsedContinents = ref<Record<string, boolean>>({})

// Collapsed state for categories within continent groups (key: "continent:category")
const collapsedContinentCategories = ref<Record<string, boolean>>({})

// Index update state
const isUpdatingIndex = ref(false)

// Whether any filter is active (for button highlight)
const hasActiveFilters = computed(() => {
  return showOnlyMissingLibs.value || showOnlyDuplicateTiles.value || enabledFilter.value !== 'all' || viewMode.value === 'continent'
})

// Whether any data-level filter is active (excluding view mode)
const hasDataFilters = computed(() => {
  return showOnlyMissingLibs.value || showOnlyDuplicateTiles.value || enabledFilter.value !== 'all'
})

// Local copy of grouped entries for drag-and-drop
const localGroupedEntries = ref<Record<string, SceneryManagerEntry[]>>({
  FixedHighPriority: [],
  Airport: [],
  DefaultAirport: [],
  Library: [],
  Other: [],
  Overlay: [],
  AirportMesh: [],
  Mesh: [],
  Unrecognized: []
})

// Category order for display
const categoryOrder = ['FixedHighPriority', 'Airport', 'DefaultAirport', 'Library', 'Other', 'Overlay', 'AirportMesh', 'Mesh', 'Unrecognized']

// Initialize tab from route query
onMounted(() => {
  const tabParam = route.query.tab as ManagementTab | undefined
  if (tabParam && availableTabs.value.includes(tabParam)) {
    activeTab.value = tabParam
  }

  document.addEventListener('click', handleClickOutside)

  // Load initial data (non-blocking to avoid route transition delay)
  loadTabData(activeTab.value)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside)
  stopDragAutoScroll()
})

// Watch for tab changes
watch(activeTab, async (newTab, oldTab) => {
  // Determine transition direction
  const oldIndex = availableTabs.value.indexOf(oldTab)
  const newIndex = availableTabs.value.indexOf(newTab)
  tabTransitionName.value = newIndex > oldIndex ? 'tab-slide-left' : 'tab-slide-right'

  // Suppress loading during transition to prevent animation blocking
  suppressLoading.value = true

  searchQuery.value = ''
  // Reset filter states when switching tabs
  showOnlyUpdates.value = false
  showOnlyOutdated.value = false
  enabledFilter.value = 'all'

  // Start loading data (non-blocking)
  const loadPromise = loadTabData(newTab)

  // Wait for transition animation to complete before showing loading state
  setTimeout(() => {
    suppressLoading.value = false
  }, 350) // Slightly longer than transition duration (300ms)

  await loadPromise
})

// Watch for scenery store data changes (e.g., after delete operation)
// Use a computed trigger instead of deep watch for better performance
// This triggers on: data reference change, entries count change, or needsSync change
const sceneryDataTrigger = computed(() => ({
  hasData: !!sceneryStore.data,
  entriesCount: sceneryStore.entries.length,
  needsSync: sceneryStore.data?.needsSync ?? false
}))

watch(sceneryDataTrigger, () => {
  if (activeTab.value === 'scenery') {
    syncLocalEntries()
  }
})

// Auto-reset filter when no missing dependencies remain
watch(() => sceneryStore.missingDepsCount, (newCount) => {
  if (newCount === 0 && showOnlyMissingLibs.value) {
    showOnlyMissingLibs.value = false
  }
})

// Auto-reset filter when no duplicate tiles remain
watch(() => sceneryStore.duplicateTilesCount, (newCount) => {
  if (newCount === 0 && showOnlyDuplicateTiles.value) {
    showOnlyDuplicateTiles.value = false
  }
})

// Auto-reset filter when no updates available
watch(() => managementStore.aircraftUpdateCount + managementStore.pluginsUpdateCount, (newCount) => {
  if (newCount === 0 && showOnlyUpdates.value) {
    showOnlyUpdates.value = false
  }
})

// Auto-reset filter when no outdated navdata
watch(() => managementStore.navdataOutdatedCount, (newCount) => {
  if (newCount === 0 && showOnlyOutdated.value) {
    showOnlyOutdated.value = false
  }
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
        // Don't reload if user has made local modifications - preserve their work
        // Use hasLocalChanges (not hasChanges) to allow reload even when needsSync is true
        if (!sceneryStore.hasLocalChanges) {
          syncWarningDismissed.value = false
          await sceneryStore.loadData()
          if (sceneryStore.error) {
            modalStore.showError(t('management.scanFailed') + ': ' + sceneryStore.error)
          }
        }
        syncLocalEntries()
        // Start async index scan (non-blocking)
        runSceneryIndexScan()
        break
    }
  } catch (e) {
    modalStore.showError(t('management.scanFailed') + ': ' + String(e))
  }
}

// Run scenery index scan asynchronously without blocking UI
async function runSceneryIndexScan() {
  if (!appStore.xplanePath || isUpdatingIndex.value) return

  isUpdatingIndex.value = true
  try {
    const result = await invoke<SceneryIndexScanResult>('quick_scan_scenery_index', {
      xplanePath: appStore.xplanePath
    })

    if (!result.indexExists) return

    const hasChanges = result.added + result.removed + result.updated > 0
    if (hasChanges && !sceneryStore.hasLocalChanges) {
      // Reload scenery data to reflect changes
      await sceneryStore.loadData()
      syncLocalEntries()
    }
  } catch (error) {
    logError(`Failed to quick scan scenery index: ${error}`, 'management')
  } finally {
    isUpdatingIndex.value = false
  }
}

// Filtered entries for non-scenery tabs
const filteredAircraft = computed(() => {
  let items = managementStore.sortedAircraft
  if (showOnlyUpdates.value) {
    items = items.filter(a => a.hasUpdate)
  }
  if (!searchQuery.value.trim()) return items
  const query = searchQuery.value.toLowerCase()
  return items.filter(a =>
    a.displayName.toLowerCase().includes(query) ||
    a.folderName.toLowerCase().includes(query)
  )
})

const filteredPlugins = computed(() => {
  let items = managementStore.sortedPlugins
  if (showOnlyUpdates.value) {
    items = items.filter(p => p.hasUpdate)
  }
  if (!searchQuery.value.trim()) return items
  const query = searchQuery.value.toLowerCase()
  return items.filter(p =>
    p.displayName.toLowerCase().includes(query) ||
    p.folderName.toLowerCase().includes(query)
  )
})

const filteredNavdata = computed(() => {
  let items = managementStore.sortedNavdata
  if (showOnlyOutdated.value) {
    items = items.filter(n => {
      const cycleText = n.cycle || n.airac
      return getNavdataCycleStatus(cycleText) === 'outdated'
    })
  }
  if (!searchQuery.value.trim()) return items
  const query = searchQuery.value.toLowerCase()
  return items.filter(n =>
    n.providerName.toLowerCase().includes(query) ||
    n.folderName.toLowerCase().includes(query)
  )
})

// Computed property to determine if sync warning should be shown
const showSyncWarning = computed(() => {
  return sceneryStore.data?.needsSync && !syncWarningDismissed.value
})

// Dismiss the sync warning
function dismissSyncWarning() {
  syncWarningDismissed.value = true
}

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

// Handle view liveries for aircraft
function handleViewLiveries(folderName: string) {
  router.push('/management/liveries?aircraft=' + encodeURIComponent(folderName))
}

// Handle view scripts for FlyWithLua
function handleViewScripts(_folderName: string) {
  router.push('/management/scripts')
}

// Handle manual check updates for aircraft/plugin tabs
async function handleCheckUpdates() {
  if (managementStore.isCheckingUpdates) return

  if (activeTab.value === 'aircraft') {
    await managementStore.checkAircraftUpdates(true)
  } else if (activeTab.value === 'plugin') {
    await managementStore.checkPluginsUpdates(true)
  }
}

// Find backup for a navdata entry by matching provider name
function getNavdataBackup(providerName: string): NavdataBackupInfo | null {
  return managementStore.navdataBackups.find(
    b => b.verification.providerName === providerName
  ) || null
}

// Handle restore navdata backup
function handleRestoreBackup(backupInfo: NavdataBackupInfo) {
  const cycle = backupInfo.verification.cycle || backupInfo.verification.airac || ''
  const backupTime = new Date(backupInfo.verification.backupTime).toLocaleString()
  // Truncate long provider names
  const providerName = backupInfo.verification.providerName.length > 30
    ? backupInfo.verification.providerName.substring(0, 30) + '...'
    : backupInfo.verification.providerName

  modalStore.showConfirm({
    title: t('management.restoreBackup'),
    message: `${t('management.restoreBackupConfirm')}\n\n${t('management.backupVersion')}: ${providerName} ${cycle}\n${t('management.backupTime')}: ${backupTime}`,
    confirmText: t('management.restoreBackup'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: async () => {
      try {
        await managementStore.restoreNavdataBackup(backupInfo.folderName)
      } catch (e) {
        modalStore.showError(String(e))
      }
    },
    onCancel: () => {}
  })
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

// Base computed for all entries flattened - used by multiple computeds below
const allSceneryEntries = computed(() => {
  return categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])
})

// Unique continents from all entries
const uniqueContinents = computed(() => {
  const continents = new Set<string>()
  for (const entry of allSceneryEntries.value) {
    if (entry.continent) {
      continents.add(entry.continent)
    }
  }
  return Array.from(continents).sort()
})

// Known continents list (for validation)
const knownContinents = ['Asia', 'Europe', 'North America', 'South America', 'Africa', 'Oceania', 'Antarctica']

// Entries grouped by continent, then by category within each continent
const continentGroupedEntries = computed(() => {
  const result: Record<string, Record<string, SceneryManagerEntry[]>> = {}

  // Group entries by continent and category
  for (const entry of allSceneryEntries.value) {
    const continent = entry.continent || 'Other'
    const targetContinent = knownContinents.includes(continent) ? continent : 'Other'
    if (!result[targetContinent]) {
      result[targetContinent] = {}
      for (const cat of categoryOrder) {
        result[targetContinent][cat] = []
      }
    }
    result[targetContinent][entry.category].push(entry)
  }

  return result
})

// Sorted continent order for display (alphabetically, with 'Other' always at the end)
const sortedContinentOrder = computed(() => {
  const continentsWithEntries = Object.keys(continentGroupedEntries.value)
    .filter(continent => {
      const data = continentGroupedEntries.value[continent]
      return categoryOrder.some(cat => (data[cat]?.length || 0) > 0)
    })

  // Sort alphabetically, but keep 'Other' at the end
  return continentsWithEntries.sort((a, b) => {
    if (a === 'Other') return 1
    if (b === 'Other') return -1
    return a.localeCompare(b)
  })
})

// Get stats for a continent (enabled/total)
function getContinentStats(continent: string): { enabled: number; total: number } {
  const continentData = continentGroupedEntries.value[continent]
  if (!continentData) return { enabled: 0, total: 0 }

  let enabled = 0
  let total = 0
  for (const cat of categoryOrder) {
    const entries = continentData[cat] || []
    total += entries.length
    enabled += entries.filter(e => e.enabled).length
  }
  return { enabled, total }
}

function getFilteredContinentStats(continent: string): { enabled: number; total: number } {
  const continentData = filteredContinentGroupedEntries.value[continent]
  if (!continentData) return { enabled: 0, total: 0 }

  let enabled = 0
  let total = 0
  for (const cat of categoryOrder) {
    const entries = continentData[cat] || []
    total += entries.length
    enabled += entries.filter(e => e.enabled).length
  }
  return { enabled, total }
}

// Check if continent is expanded (default: collapsed)
function isContinentExpanded(continent: string): boolean {
  return collapsedContinents.value[continent] === false
}

// Toggle continent collapse state
function toggleContinentCollapse(continent: string) {
  // If undefined (default collapsed), set to false (expanded)
  // If false (expanded), set to true (collapsed)
  // If true (collapsed), set to false (expanded)
  if (collapsedContinents.value[continent] === undefined) {
    collapsedContinents.value[continent] = false // expand
  } else {
    collapsedContinents.value[continent] = !collapsedContinents.value[continent]
  }
}

// Toggle view mode with loading animation
function toggleViewMode() {
  isViewModeTransitioning.value = true
  selectedContinent.value = ''

  // Use setTimeout to allow loading spinner to render before heavy computation
  setTimeout(() => {
    viewMode.value = viewMode.value === 'category' ? 'continent' : 'category'
    // Small delay to let Vue finish rendering before hiding spinner
    setTimeout(() => {
      isViewModeTransitioning.value = false
    }, 50)
  }, 10)
}

// Apply a filter change with loading animation to avoid UI freeze
function applyFilterWithTransition(fn: () => void) {
  isFilterTransitioning.value = true
  // Let spinner render first, then apply the actual state change
  setTimeout(() => {
    fn()
    nextTick(() => {
      isFilterTransitioning.value = false
    })
  }, 10)
}

// Check if category within continent is expanded (default: collapsed)
function isContinentCategoryExpanded(continent: string, category: string): boolean {
  const key = `${continent}:${category}`
  return collapsedContinentCategories.value[key] === false
}

// Toggle category collapse state within continent
function toggleContinentCategoryCollapse(continent: string, category: string) {
  const key = `${continent}:${category}`
  // If undefined (default collapsed), set to false (expanded)
  // If false (expanded), set to true (collapsed)
  // If true (collapsed), set to false (expanded)
  if (collapsedContinentCategories.value[key] === undefined) {
    collapsedContinentCategories.value[key] = false // expand
  } else {
    collapsedContinentCategories.value[key] = !collapsedContinentCategories.value[key]
  }
}

// Toggle all entries in a continent
function toggleContinentEnabled(continent: string) {
  const continentData = continentGroupedEntries.value[continent]
  if (!continentData) return

  const entries = categoryOrder.flatMap(cat => continentData[cat] || [])
  const allEnabled = entries.every(e => e.enabled)
  const newState = !allEnabled

  for (const entry of entries) {
    if (entry.enabled !== newState) {
      sceneryStore.toggleEnabled(entry.folderName)
    }
  }
  syncLocalEntries()
}

// Toggle all entries in a continent's category
function toggleContinentCategoryEnabled(continent: string, category: string) {
  const entries = continentGroupedEntries.value[continent]?.[category] || []
  if (entries.length === 0) return

  const allEnabled = entries.every(e => e.enabled)
  const newState = !allEnabled

  for (const entry of entries) {
    if (entry.enabled !== newState) {
      sceneryStore.toggleEnabled(entry.folderName)
    }
  }
  syncLocalEntries()
}

// Check if all entries in a continent are enabled
function isContinentAllEnabled(continent: string): boolean {
  const continentData = continentGroupedEntries.value[continent]
  if (!continentData) return false

  const entries = categoryOrder.flatMap(cat => continentData[cat] || [])
  return entries.length > 0 && entries.every(e => e.enabled)
}

// Check if all entries in a continent's category are enabled
function isContinentCategoryAllEnabled(continent: string, category: string): boolean {
  const entries = continentGroupedEntries.value[continent]?.[category] || []
  return entries.length > 0 && entries.every(e => e.enabled)
}

// The last entry before Unrecognized category should have move-down disabled
const lastEntryBeforeUnrecognized = computed(() => {
  const unrecognizedEntries = localGroupedEntries.value['Unrecognized'] || []
  if (unrecognizedEntries.length === 0) return ''
  // Find the last non-Unrecognized category that has entries
  for (let i = categoryOrder.length - 1; i >= 0; i--) {
    const cat = categoryOrder[i]
    if (cat === 'Unrecognized') continue
    const entries = localGroupedEntries.value[cat] || []
    if (entries.length > 0) return entries[entries.length - 1].folderName
  }
  return ''
})

const filteredSceneryEntries = computed(() => {
  let entries = allSceneryEntries.value

  // Filter by missing libraries
  if (showOnlyMissingLibs.value) {
    entries = entries.filter(entry => entry.missingLibraries && entry.missingLibraries.length > 0)
  }

  // Filter by duplicate tiles
  if (showOnlyDuplicateTiles.value) {
    entries = entries.filter(entry => entry.duplicateTiles && entry.duplicateTiles.length > 0)
  }

  // Filter by continent
  if (selectedContinent.value) {
    entries = entries.filter(entry => entry.continent === selectedContinent.value)
  }

  // Filter by enabled/disabled state
  if (enabledFilter.value === 'enabled') {
    entries = entries.filter(entry => entry.enabled)
  } else if (enabledFilter.value === 'disabled') {
    entries = entries.filter(entry => !entry.enabled)
  }

  return entries
})

// Filtered entries grouped by category (for grouped filtered view)
const filteredGroupedEntries = computed(() => {
  const result: Record<string, SceneryManagerEntry[]> = {}
  for (const category of categoryOrder) {
    result[category] = []
  }
  for (const entry of filteredSceneryEntries.value) {
    const cat = entry.category || 'Unrecognized'
    if (result[cat]) {
      result[cat].push(entry)
    } else {
      result['Unrecognized'].push(entry)
    }
  }
  return result
})

// Filtered entries grouped by continent then category (for continent filtered view)
const filteredContinentGroupedEntries = computed(() => {
  const result: Record<string, Record<string, SceneryManagerEntry[]>> = {}
  for (const entry of filteredSceneryEntries.value) {
    const continent = entry.continent || 'Other'
    const targetContinent = knownContinents.includes(continent) ? continent : 'Other'
    if (!result[targetContinent]) {
      result[targetContinent] = {}
      for (const cat of categoryOrder) {
        result[targetContinent][cat] = []
      }
    }
    result[targetContinent][entry.category].push(entry)
  }
  return result
})

// Sorted continent order based on filtered entries
const filteredSortedContinentOrder = computed(() => {
  const continentsWithEntries = Object.keys(filteredContinentGroupedEntries.value)
    .filter(continent => {
      const data = filteredContinentGroupedEntries.value[continent]
      return categoryOrder.some(cat => (data[cat]?.length || 0) > 0)
    })
  return continentsWithEntries.sort((a, b) => {
    if (a === 'Other') return 1
    if (b === 'Other') return -1
    return a.localeCompare(b)
  })
})

// Cached map for O(1) lookup of entry index by folderName
const globalIndexMap = computed(() => {
  const map = new Map<string, number>()
  allSceneryEntries.value.forEach((entry, index) => {
    map.set(entry.folderName, index)
  })
  return map
})

function isGroupExpanded(category: string): boolean {
  // Default: collapsed (undefined or true = collapsed, false = expanded)
  // searchExpandedGroups can override to expand for search
  if (searchExpandedGroups.value[category]) return true
  return sceneryStore.collapsedGroups[category as SceneryCategory] === false
}

const searchQueryLower = computed(() => searchQuery.value?.toLowerCase() ?? '')

const matchedIndices = computed(() => {
  if (!searchQuery.value.trim() || activeTab.value !== 'scenery') return []
  const query = searchQueryLower.value
  return filteredSceneryEntries.value
    .map((entry) => ({
      entry,
      index: getGlobalIndex(entry.folderName)
    }))
    .filter(({ index }) => index >= 0)
    .filter(({ entry }) => entry.folderName.toLowerCase().includes(query))
    .map(({ index }) => index)
})

function syncLocalEntries() {
  const grouped = sceneryStore.groupedEntries
  for (const cat of categoryOrder) {
    const newArr = grouped[cat as SceneryCategory] || []
    const oldArr = localGroupedEntries.value[cat] || []
    if (newArr.length !== oldArr.length || newArr.some((e, i) => e !== oldArr[i])) {
      localGroupedEntries.value[cat] = [...newArr]
    }
  }
}

function toggleGroupCollapse(category: string) {
  const expanded = isGroupExpanded(category)
  if (expanded) {
    sceneryStore.collapsedGroups[category as SceneryCategory] = true
    if (searchExpandedGroups.value[category]) {
      delete searchExpandedGroups.value[category]
    }
  } else {
    sceneryStore.collapsedGroups[category as SceneryCategory] = false
  }
}

function getCategoryTranslationKey(category: string): string {
  return `sceneryManager.category${category}`
}

function handleDragPointerMove(e: PointerEvent) {
  dragPointerY = e.clientY
}

function dragAutoScrollLoop() {
  const container = scrollContainerRef.value
  if (!container) {
    dragAutoScrollRafId = requestAnimationFrame(dragAutoScrollLoop)
    return
  }

  const rect = container.getBoundingClientRect()
  const edgeZone = 60        // 容器内边缘触发区域 (px)
  const maxSpeed = 18        // 到达边缘时的最大速度 (px/frame)
  const outsideAccel = 0.4   // 超出边界后每像素额外加速 (px/frame/px)

  let scrollDelta = 0

  if (dragPointerY < rect.top + edgeZone) {
    // 光标在顶部边缘或上方 → 向上滚动
    if (dragPointerY < rect.top) {
      const dist = rect.top - dragPointerY
      scrollDelta = -(maxSpeed + dist * outsideAccel)
    } else {
      const ratio = ((rect.top + edgeZone) - dragPointerY) / edgeZone
      scrollDelta = -ratio * maxSpeed
    }
  } else if (dragPointerY > rect.bottom - edgeZone) {
    // 光标在底部边缘或下方 → 向下滚动
    if (dragPointerY > rect.bottom) {
      const dist = dragPointerY - rect.bottom
      scrollDelta = maxSpeed + dist * outsideAccel
    } else {
      const ratio = (dragPointerY - (rect.bottom - edgeZone)) / edgeZone
      scrollDelta = ratio * maxSpeed
    }
  }

  if (scrollDelta !== 0) {
    container.scrollTop += scrollDelta
  }

  dragAutoScrollRafId = requestAnimationFrame(dragAutoScrollLoop)
}

function startDragAutoScroll() {
  document.addEventListener('pointermove', handleDragPointerMove)
  dragAutoScrollRafId = requestAnimationFrame(dragAutoScrollLoop)
}

function stopDragAutoScroll() {
  document.removeEventListener('pointermove', handleDragPointerMove)
  if (dragAutoScrollRafId !== null) {
    cancelAnimationFrame(dragAutoScrollRafId)
    dragAutoScrollRafId = null
  }
}

function handleDragStart() {
  drag.value = true
  startDragAutoScroll()
}

async function handleSceneryToggleEnabled(folderName: string) {
  syncWarningDismissed.value = true
  await sceneryStore.toggleEnabled(folderName)
  syncLocalEntries()
}

async function handleMoveUp(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)

  if (index > 0) {
    const currentEntry = entries[index]
    const targetEntry = entries[index - 1]

    // Prevent moving into or out of Unrecognized
    if (currentEntry.category === 'Unrecognized' || targetEntry.category === 'Unrecognized') return

    syncWarningDismissed.value = true
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

    // Prevent moving into or out of Unrecognized
    if (currentEntry.category === 'Unrecognized' || targetEntry.category === 'Unrecognized') return

    syncWarningDismissed.value = true
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
  stopDragAutoScroll()
  syncWarningDismissed.value = true
  const allEntries = categoryOrder.flatMap(category => localGroupedEntries.value[category] || [])
  await sceneryStore.reorderEntries(allEntries)
  syncLocalEntries()
}

function getGlobalIndex(folderName: string): number {
  return globalIndexMap.value.get(folderName) ?? -1
}

// Shared modal handlers for scenery entry actions
async function handleShowMissingLibs(entry: SceneryManagerEntry) {
  selectedModalEntry.value = entry
  showMissingLibsModal.value = true
  libraryLinksMap.value = {}
  libraryLinksRequestSeq.value += 1
  const requestSeq = libraryLinksRequestSeq.value

  // Phase 1: immediate local links (embedded JSON)
  isLoadingLinks.value = true
  try {
    const links: Record<string, string | null> = await invoke('lookup_library_links', {
      libraryNames: entry.missingLibraries,
    })
    libraryLinksMap.value = links
  } catch {
    // Silently fail -- per-library download buttons simply won't show
    libraryLinksMap.value = {}
  } finally {
    isLoadingLinks.value = false
  }

  // Phase 2: remote refresh, then replace current displayed links (add/remove)
  void invoke<Record<string, string | null>>('lookup_library_links_remote', {
    libraryNames: entry.missingLibraries,
  }).then((remoteLinks) => {
    if (!showMissingLibsModal.value) return
    if (!selectedModalEntry.value) return
    if (selectedModalEntry.value.folderName !== entry.folderName) return
    if (libraryLinksRequestSeq.value !== requestSeq) return
    libraryLinksMap.value = remoteLinks
  }).catch(() => {
    // Keep local links if remote refresh fails
  })
}

function handleShowDuplicateTiles(entry: SceneryManagerEntry) {
  selectedModalEntry.value = entry
  showDuplicateTilesModal.value = true
}

function handleShowDeleteConfirm(entry: SceneryManagerEntry) {
  selectedModalEntry.value = entry
  showDeleteConfirmModal.value = true
}

function handleCopyMissingLibs() {
  if (!selectedModalEntry.value) return
  const libNames = selectedModalEntry.value.missingLibraries.join('\n')
  navigator.clipboard.writeText(libNames).then(() => {
    toastStore.success(t('sceneryManager.missingLibsCopied'))
  }).catch(() => {
    modalStore.showError(t('copy.copyFailed'))
  })
}

function handleCopySingleLib(libName: string) {
  navigator.clipboard.writeText(libName).then(() => {
    toastStore.success(t('sceneryManager.libNameCopied'))
  }).catch(() => {
    modalStore.showError(t('copy.copyFailed'))
  })
}

async function handleDirectDownload(url: string) {
  try {
    await invoke('open_url', { url })
  } catch (error) {
    modalStore.showError(t('sceneryManager.openUrlFailed') + ': ' + getErrorMessage(error))
  }
}

async function handleSearchSingleLib(libName: string) {
  const bingUrl = `https://www.bing.com/search?q=${encodeURIComponent(libName + ' X-Plane library')}`
  try {
    await invoke('open_url', { url: bingUrl })
  } catch (error) {
    modalStore.showError(t('sceneryManager.openUrlFailed') + ': ' + getErrorMessage(error))
  }
}

function handleOpenContributeLink(libName: string) {
  contributingLibName.value = libName
  contributingLibUrl.value = ''
  showContributeLinkModal.value = true
}

function closeContributeLinkModal() {
  isSubmittingContributeLink.value = false
  showContributeLinkModal.value = false
  contributingLibName.value = ''
  contributingLibUrl.value = ''
}

function isValidHttpUrl(value: string): boolean {
  try {
    const parsed = new URL(value)
    return parsed.protocol === 'http:' || parsed.protocol === 'https:'
  } catch {
    return false
  }
}

async function handleSubmitContributeLink() {
  if (isSubmittingContributeLink.value) return

  const libName = contributingLibName.value.trim()
  const inputUrl = contributingLibUrl.value.trim()

  if (!libName || !selectedModalEntry.value) return

  if (!isValidHttpUrl(inputUrl)) {
    modalStore.showError(t('sceneryManager.invalidContributionUrl'))
    return
  }

  const title = `[Library Link] ${libName}`
  const body = [
    '### Library Link Submission',
    '',
    `- Library Name: \`${libName}\``,
    `- Download URL: ${inputUrl}`,
    `- Referenced By Scenery: \`${selectedModalEntry.value.folderName}\``,
    '',
    'Please review this link. If valid, add the `approved-link` label to trigger auto-update for `data/library_links.json` on `dev`.'
  ].join('\n')

  const issueUrl = `https://github.com/CCA3370/XFast-Manager/issues/new?template=library_link_submission.yml&labels=${encodeURIComponent('library-link')}&title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`

  isSubmittingContributeLink.value = true
  appStore.setLibraryLinkSubmitting(true)
  let submitTimeoutId: ReturnType<typeof setTimeout> | null = null
  try {
    const createdIssueUrl = await Promise.race<string>([
      invoke<string>('create_library_link_issue', {
        libraryName: libName,
        downloadUrl: inputUrl,
        referencedBy: selectedModalEntry.value.folderName,
      }),
      new Promise<string>((_, reject) => {
        submitTimeoutId = setTimeout(() => {
          reject(new Error('CONTRIBUTION_SUBMIT_TIMEOUT'))
        }, CONTRIBUTION_SUBMIT_TIMEOUT_MS)
      })
    ])

    toastStore.success(t('sceneryManager.contributionCreated'))
    closeContributeLinkModal()

    // Open created issue for user visibility
    await invoke('open_url', { url: createdIssueUrl })
  } catch (error) {
    // Fallback: open prefilled GitHub issue page if direct API creation is unavailable
    try {
      await invoke('open_url', { url: issueUrl })
      toastStore.success(t('sceneryManager.contributionOpened'))
      closeContributeLinkModal()
    } catch {
      modalStore.showError(t('sceneryManager.openUrlFailed') + ': ' + getErrorMessage(error))
    }
  } finally {
    if (submitTimeoutId !== null) {
      clearTimeout(submitTimeoutId)
    }
    isSubmittingContributeLink.value = false
    appStore.setLibraryLinkSubmitting(false)
  }
}

async function handleDeleteEntryConfirm() {
  if (!selectedModalEntry.value || isDeletingEntry.value) return

  isDeletingEntry.value = true
  try {
    await sceneryStore.deleteEntry(selectedModalEntry.value.folderName)
    toastStore.success(t('sceneryManager.deleteSuccess'))
    showDeleteConfirmModal.value = false
  } catch (error) {
    const apiError = parseApiError(error)
    if (apiError) {
      const errorKey = `errors.${apiError.code}`
      const localizedMessage = t(errorKey) !== errorKey
        ? t(errorKey)
        : apiError.message
      modalStore.showError(t('sceneryManager.deleteFailed') + ': ' + localizedMessage)
    } else {
      modalStore.showError(t('sceneryManager.deleteFailed') + ': ' + getErrorMessage(error))
    }
  } finally {
    isDeletingEntry.value = false
  }
}

// Type for vuedraggable change event
interface DraggableChangeEvent<T> {
  added?: { element: T; newIndex: number }
  removed?: { element: T; oldIndex: number }
  moved?: { element: T; newIndex: number; oldIndex: number }
}

async function handleGroupChange(category: string, evt: DraggableChangeEvent<SceneryManagerEntry>) {
  if (evt.added) {
    const entry = evt.added.element
    const newCategory = category as SceneryCategory

    try {
      await sceneryStore.updateCategory(entry.folderName, newCategory)
    } catch (e) {
      logError(`Failed to update category: ${e}`, 'management')
      suppressLoading.value = true
      try {
        await sceneryStore.loadData()
        syncLocalEntries()
      } catch (reloadError) {
        logError(`Failed to reload scenery data: ${reloadError}`, 'management')
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
      // Restore sync warning if data still needs sync
      syncWarningDismissed.value = false
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

// State for database reset
const isResettingDatabase = ref(false)

async function handleResetDatabase() {
  modalStore.showConfirm({
    title: t('sceneryManager.resetDatabase'),
    message: t('sceneryManager.resetDatabaseConfirm'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: async () => {
      isResettingDatabase.value = true
      try {
        const success = await sceneryStore.resetDatabase()
        if (success) {
          toastStore.success(t('sceneryManager.resetDatabaseSuccess'))
        } else {
          modalStore.showError(t('sceneryManager.resetDatabaseFailed'))
        }
      } catch (e) {
        modalStore.showError(t('sceneryManager.resetDatabaseFailed') + ': ' + String(e))
      } finally {
        isResettingDatabase.value = false
      }
    },
    onCancel: () => {}
  })
}

function handleClickOutside(event: MouseEvent) {
  if (moreMenuRef.value && !moreMenuRef.value.contains(event.target as Node)) {
    showMoreMenu.value = false
  }
  if (filterDropdownRef.value && !filterDropdownRef.value.contains(event.target as Node)) {
    showFilterDropdown.value = false
  }
}

// Search navigation functions
// Collapse groups that were expanded for search
function collapseSearchExpandedGroups() {
  if (viewMode.value === 'continent') {
    // Collapse continents that were expanded for search
    for (const continent of Object.keys(searchExpandedContinents.value)) {
      if (searchExpandedContinents.value[continent]) {
        collapsedContinents.value[continent] = true
      }
    }
    searchExpandedContinents.value = {}

    // Collapse continent categories that were expanded for search
    for (const key of Object.keys(searchExpandedContinentCategories.value)) {
      if (searchExpandedContinentCategories.value[key]) {
        collapsedContinentCategories.value[key] = true
      }
    }
    searchExpandedContinentCategories.value = {}
  } else {
    // Collapse category groups that were expanded for search
    searchExpandedGroups.value = {}
  }
}

function ensureGroupExpandedForIndex(index: number) {
  if (showOnlyMissingLibs.value || showOnlyDuplicateTiles.value) return
  const entry = allSceneryEntries.value[index]
  if (!entry) return

  // First collapse previously expanded groups
  collapseSearchExpandedGroups()

  if (viewMode.value === 'continent') {
    // In continent view, expand both continent and category within continent
    const continent = entry.continent || 'Other'
    const targetContinent = knownContinents.includes(continent) ? continent : 'Other'

    // Expand continent if collapsed (undefined or true means collapsed)
    if (collapsedContinents.value[targetContinent] !== false) {
      collapsedContinents.value[targetContinent] = false
      searchExpandedContinents.value[targetContinent] = true
    }

    // Expand category within continent if collapsed (undefined or true means collapsed)
    const categoryKey = `${targetContinent}:${entry.category}`
    if (collapsedContinentCategories.value[categoryKey] !== false) {
      collapsedContinentCategories.value[categoryKey] = false
      searchExpandedContinentCategories.value[categoryKey] = true
    }
  } else {
    // In category view, expand category group
    if (sceneryStore.collapsedGroups[entry.category]) {
      searchExpandedGroups.value[entry.category] = true
    }
  }
}

function waitForCollapseTransitions(): Promise<void> {
  const container = scrollContainerRef.value
  if (!container) return Promise.resolve()

  const hasActiveCollapseTransition = () => {
    return !!container.querySelector('.collapse-enter-active, .collapse-leave-active')
  }

  if (!hasActiveCollapseTransition()) {
    return Promise.resolve()
  }

  return new Promise((resolve) => {
    let settled = false

    const finalize = () => {
      if (settled) return
      settled = true
      container.removeEventListener('transitionend', onTransitionEvent, true)
      container.removeEventListener('transitioncancel', onTransitionEvent, true)
      clearTimeout(timeoutId)
      resolve()
    }

    const onTransitionEvent = () => {
      if (!hasActiveCollapseTransition()) {
        finalize()
      }
    }

    const timeoutId = setTimeout(finalize, COLLAPSE_TRANSITION_WAIT_MS)
    container.addEventListener('transitionend', onTransitionEvent, true)
    container.addEventListener('transitioncancel', onTransitionEvent, true)
  })
}

function estimatePostCollapseDelta(container: HTMLElement, targetElement: HTMLElement): number {
  const targetRect = targetElement.getBoundingClientRect()
  let delta = 0

  const transitioning = container.querySelectorAll('.collapse-enter-active, .collapse-leave-active') as NodeListOf<HTMLElement>
  transitioning.forEach((element) => {
    const rect = element.getBoundingClientRect()
    if (rect.bottom <= targetRect.top) {
      const currentHeight = rect.height
      if (element.classList.contains('collapse-enter-active')) {
        const finalHeight = Math.max(element.scrollHeight, currentHeight)
        delta += finalHeight - currentHeight
      } else if (element.classList.contains('collapse-leave-active')) {
        delta -= currentHeight
      }
    }
  })

  return delta
}

function isSafelyInsideContainerViewport(container: HTMLElement, element: HTMLElement, padding: number): boolean {
  const containerRect = container.getBoundingClientRect()
  const elementRect = element.getBoundingClientRect()
  return elementRect.top >= containerRect.top + padding
    && elementRect.bottom <= containerRect.bottom - padding
}

function scrollToMatch(index: number) {
  ensureGroupExpandedForIndex(index)
  highlightedIndex.value = index
  const requestId = ++activeScrollRequestId

  const attemptScroll = (attempt: number, behavior: ScrollBehavior, predictPostCollapse = false) => {
    if (highlightedIndex.value !== index || requestId !== activeScrollRequestId) return
    const container = scrollContainerRef.value
    if (!container) return

    const element = container.querySelector(`[data-scenery-index="${index}"]`) as HTMLElement | null
    if (element && element.getClientRects().length > 0) {
      const containerRect = container.getBoundingClientRect()
      const elementRect = element.getBoundingClientRect()
      const currentScrollTop = container.scrollTop
      const targetScrollTop = currentScrollTop
        + (elementRect.top - containerRect.top)
        - (container.clientHeight / 2)
        + (elementRect.height / 2)

      const predictedDelta = predictPostCollapse
        ? estimatePostCollapseDelta(container, element)
        : 0

      const predictedTargetScrollTop = targetScrollTop + predictedDelta

      const clampedScrollTop = Math.max(0, Math.min(
        predictedTargetScrollTop,
        container.scrollHeight - container.clientHeight
      ))

      container.scrollTo({
        top: clampedScrollTop,
        behavior
      })
      return
    }
    if (attempt < 6) {
      setTimeout(() => attemptScroll(attempt + 1, behavior, predictPostCollapse), 60)
    }
  }

  void nextTick(async () => {
    if (requestId !== activeScrollRequestId || highlightedIndex.value !== index) return

    // Start scroll immediately so it animates in parallel with collapse/expand animation
    attemptScroll(0, 'smooth', true)

    // After collapse animation settles, do one precise correction pass
    await waitForCollapseTransitions()
    if (requestId !== activeScrollRequestId || highlightedIndex.value !== index) return
    await nextTick()
    if (requestId !== activeScrollRequestId || highlightedIndex.value !== index) return

    const container = scrollContainerRef.value
    const element = container?.querySelector(`[data-scenery-index="${index}"]`) as HTMLElement | null
    if (container && element && isSafelyInsideContainerViewport(container, element, FINAL_CALIBRATION_VIEWPORT_PADDING_PX)) {
      return
    }

    attemptScroll(0, 'auto')
  })
}

function handleSearchInput() {
  if (activeTab.value !== 'scenery') return

  if (!searchQuery.value.trim()) {
    highlightedIndex.value = -1
    currentMatchIndex.value = 0
    collapseSearchExpandedGroups()
    return
  }

  if (matchedIndices.value.length > 0) {
    currentMatchIndex.value = 0
    scrollToMatch(matchedIndices.value[0])
  } else {
    highlightedIndex.value = -1
    collapseSearchExpandedGroups()
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
  collapseSearchExpandedGroups()
}

// Current loading state (suppressed during tab transitions)
const isLoading = computed(() => {
  if (suppressLoading.value) return false
  if (activeTab.value === 'scenery') {
    return sceneryStore.isLoading
  }
  return managementStore.isLoading
})
</script>

<template>
  <div class="management-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Tab Bar -->
    <div class="mb-3 flex-shrink-0 relative flex items-center gap-1 p-1 bg-gray-100 dark:bg-gray-800 rounded-lg">
      <!-- Sliding indicator background -->
      <div
        class="tab-indicator absolute top-1 bottom-1 rounded-md bg-white dark:bg-gray-700 shadow-sm transition-all duration-300 ease-out"
        :style="{
          width: `calc((100% - 0.5rem - ${(availableTabs.length - 1) * 0.25}rem) / ${availableTabs.length})`,
          left: `calc(0.25rem + ${activeTabIndex} * (100% - 0.5rem) / ${availableTabs.length})`
        }"
      />
      <button
        v-for="tab in availableTabs"
        :key="tab"
        @click="activeTab = tab"
        class="relative z-10 flex-1 px-3 py-1.5 rounded-md text-sm font-medium transition-colors duration-200"
        :class="activeTab === tab
          ? 'text-blue-600 dark:text-blue-400'
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

      <!-- Check updates button for aircraft/plugin tabs -->
      <button
        v-if="activeTab === 'aircraft' || activeTab === 'plugin'"
        @click="handleCheckUpdates"
        :disabled="managementStore.isCheckingUpdates"
        class="px-3 py-1.5 rounded-lg bg-emerald-500 text-white hover:bg-emerald-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 text-sm"
      >
        <svg v-if="!managementStore.isCheckingUpdates" class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
        </svg>
        <svg v-else class="w-3.5 h-3.5 animate-spin [animation-direction:reverse]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
        </svg>
        <Transition name="text-fade" mode="out-in">
          <span :key="locale">{{ t('management.checkUpdates') }}</span>
        </Transition>
      </button>

      <!-- Scenery-specific action buttons -->
      <template v-if="activeTab === 'scenery'">
        <!-- Auto-sort button (shown for all locales, only when index exists) -->
        <Transition v-if="sceneryStore.indexExists" name="button-fade" mode="out-in">
          <button
            key="auto-sort-button"
            @click="handleSortSceneryNow"
            :disabled="isSortingScenery || !appStore.xplanePath || !sceneryStore.indexExists"
            class="px-3 py-1.5 rounded-lg bg-cyan-500 text-white hover:bg-cyan-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 text-sm"
          >
            <svg v-if="!isSortingScenery" class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"></path>
            </svg>
            <svg v-else class="w-3.5 h-3.5 animate-spin [animation-direction:reverse]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
            </svg>
            <span class="transition-opacity">{{ isSortingScenery ? t('settings.sorting') : t('sceneryManager.autoSort') }}</span>
          </button>
        </Transition>

        <button
          v-if="sceneryStore.hasLocalChanges && sceneryStore.indexExists"
          @click="handleReset"
          class="px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-sm"
        >
          <Transition name="text-fade" mode="out-in">
            <span :key="locale">{{ t('sceneryManager.reset') }}</span>
          </Transition>
        </button>
        <!-- Apply button with tooltip popover (only when index exists) -->
        <div v-if="sceneryStore.hasChanges && sceneryStore.indexExists" class="relative">
          <button
            @click="handleApplyChanges"
            :disabled="!sceneryStore.indexExists || sceneryStore.isSaving"
            class="px-3 py-1.5 rounded-lg bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 text-sm"
            :class="{ 'ring-2 ring-amber-400 ring-offset-1': showSyncWarning }"
          >
            <svg v-if="sceneryStore.isSaving" class="animate-spin h-3.5 w-3.5" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <!-- Warning icon when ini out of sync -->
            <svg v-else-if="showSyncWarning" class="h-3.5 w-3.5 text-amber-200" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
            </svg>
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{ t('sceneryManager.applyChanges') }}</span>
            </Transition>
          </button>
          <!-- Tooltip popover pointing to button -->
          <Transition name="fade">
            <div
              v-if="showSyncWarning"
              class="absolute right-0 top-full mt-2 w-64 p-2.5 bg-amber-50 dark:bg-amber-900/90 border border-amber-300 dark:border-amber-600 rounded-lg shadow-lg z-50"
            >
              <!-- Arrow pointing up -->
              <div class="absolute -top-2 right-4 w-0 h-0 border-l-8 border-r-8 border-b-8 border-l-transparent border-r-transparent border-b-amber-300 dark:border-b-amber-600"></div>
              <div class="absolute -top-1.5 right-4 w-0 h-0 border-l-8 border-r-8 border-b-8 border-l-transparent border-r-transparent border-b-amber-50 dark:border-b-amber-900/90"></div>
              <div class="flex items-start gap-2">
                <svg class="h-4 w-4 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
                </svg>
                <span class="text-xs text-amber-800 dark:text-amber-200 flex-1">{{ t('sceneryManager.iniOutOfSync') }}</span>
                <!-- Close button -->
                <button
                  @click.stop="dismissSyncWarning"
                  class="p-0.5 rounded hover:bg-amber-200 dark:hover:bg-amber-800 transition-colors flex-shrink-0"
                  :title="t('common.close')"
                >
                  <svg class="h-3.5 w-3.5 text-amber-600 dark:text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>
          </Transition>
        </div>
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
        <div v-if="activeTab !== 'navdata'" class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('management.enabled') }}:</span>
          </Transition>
          <span class="font-semibold text-green-600 dark:text-green-400">
            {{ activeTab === 'aircraft' ? managementStore.aircraftEnabledCount :
               managementStore.pluginsEnabledCount }}
          </span>
        </div>
        <!-- Update available count for aircraft -->
        <div v-if="activeTab === 'aircraft' && managementStore.aircraftUpdateCount > 0" class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('management.hasUpdate') }}:</span>
          </Transition>
          <span class="font-semibold text-emerald-600 dark:text-emerald-400">
            {{ managementStore.aircraftUpdateCount }}
          </span>
          <button
            @click="showOnlyUpdates = !showOnlyUpdates"
            class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
            :class="showOnlyUpdates
              ? 'bg-emerald-500 text-white hover:bg-emerald-600'
              : 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400 hover:bg-emerald-200 dark:hover:bg-emerald-900/50'"
            :title="t('management.filterUpdatesOnly')"
          >
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{ showOnlyUpdates ? t('management.showAll') : t('management.filterUpdatesOnly') }}</span>
            </Transition>
          </button>
        </div>
        <!-- Update available count for plugins -->
        <div v-if="activeTab === 'plugin' && managementStore.pluginsUpdateCount > 0" class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('management.hasUpdate') }}:</span>
          </Transition>
          <span class="font-semibold text-emerald-600 dark:text-emerald-400">
            {{ managementStore.pluginsUpdateCount }}
          </span>
          <button
            @click="showOnlyUpdates = !showOnlyUpdates"
            class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
            :class="showOnlyUpdates
              ? 'bg-emerald-500 text-white hover:bg-emerald-600'
              : 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400 hover:bg-emerald-200 dark:hover:bg-emerald-900/50'"
            :title="t('management.filterUpdatesOnly')"
          >
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{ showOnlyUpdates ? t('management.showAll') : t('management.filterUpdatesOnly') }}</span>
            </Transition>
          </button>
        </div>
        <!-- Checking updates indicator -->
        <div v-if="(activeTab === 'aircraft' || activeTab === 'plugin') && managementStore.isCheckingUpdates" class="flex items-center gap-2 text-gray-500 dark:text-gray-400">
          <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span class="text-xs">{{ t('management.checkingUpdates') }}</span>
        </div>
        <!-- Outdated count for navdata -->
        <div v-if="activeTab === 'navdata' && managementStore.navdataOutdatedCount > 0" class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('management.outdated') }}:</span>
          </Transition>
          <span class="font-semibold text-red-600 dark:text-red-400">
            {{ managementStore.navdataOutdatedCount }}
          </span>
          <button
            @click="showOnlyOutdated = !showOnlyOutdated"
            class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
            :class="showOnlyOutdated
              ? 'bg-red-500 text-white hover:bg-red-600'
              : 'bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50'"
            :title="t('management.filterOutdatedOnly')"
          >
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{ showOnlyOutdated ? t('management.showAll') : t('management.filterOutdatedOnly') }}</span>
            </Transition>
          </button>
        </div>
        <!-- Restoring backup indicator -->
        <div v-if="activeTab === 'navdata' && managementStore.isRestoringBackup" class="flex items-center gap-2 text-gray-500 dark:text-gray-400">
          <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span class="text-xs">{{ t('management.restoringBackup') }}</span>
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
        </div>
        <div v-if="sceneryStore.duplicateTilesCount > 0" class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400">{{ t('sceneryManager.duplicateTiles') }}:</span>
          </Transition>
          <span class="font-semibold text-orange-600 dark:text-orange-400">{{ sceneryStore.duplicateTilesCount }}</span>
        </div>
        <!-- Filter dropdown menu -->
        <div ref="filterDropdownRef" class="relative">
          <button
            @click="showFilterDropdown = !showFilterDropdown"
            class="text-xs px-2.5 py-1 rounded-md transition-all duration-200 flex items-center gap-1.5 border"
            :class="hasActiveFilters
              ? 'bg-blue-500 text-white border-blue-500 hover:bg-blue-600 hover:border-blue-600 shadow-sm shadow-blue-500/25'
              : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500 hover:bg-gray-50 dark:hover:bg-gray-700'"
          >
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 3c2.755 0 5.455.232 8.083.678.533.09.917.556.917 1.096v1.044a2.25 2.25 0 01-.659 1.591l-5.432 5.432a2.25 2.25 0 00-.659 1.591v2.927a2.25 2.25 0 01-1.244 2.013L9.75 21v-6.568a2.25 2.25 0 00-.659-1.591L3.659 7.409A2.25 2.25 0 013 5.818V4.774c0-.54.384-1.006.917-1.096A48.32 48.32 0 0112 3z" />
            </svg>
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{ t('sceneryManager.filters') }}</span>
            </Transition>
            <svg class="w-3 h-3 transition-transform duration-200" :class="showFilterDropdown ? 'rotate-180' : ''" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
            </svg>
          </button>
          <!-- Dropdown panel -->
          <Transition name="dropdown">
            <div
              v-if="showFilterDropdown"
              class="absolute right-0 top-full mt-1.5 w-60 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-xl shadow-black/8 dark:shadow-black/25 z-50 py-1.5 ring-1 ring-black/5 dark:ring-white/5"
            >
              <!-- Issues section -->
              <template v-if="sceneryStore.missingDepsCount > 0 || sceneryStore.duplicateTilesCount > 0">
                <!-- Missing deps -->
                <div
                  v-if="sceneryStore.missingDepsCount > 0"
                  @click="applyFilterWithTransition(() => showOnlyMissingLibs = !showOnlyMissingLibs)"
                  class="flex items-center gap-2.5 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer text-xs transition-colors mx-1 rounded-lg group"
                >
                  <span class="filter-check border-gray-300 dark:border-gray-500 group-hover:border-amber-400 dark:group-hover:border-amber-500" :class="showOnlyMissingLibs && 'filter-check-active bg-amber-500 !border-amber-500'">
                    <svg class="filter-check-icon" viewBox="0 0 12 12" fill="none"><path d="M3.5 6L5.5 8L8.5 4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </span>
                  <span class="flex-1 text-gray-700 dark:text-gray-200">{{ t('sceneryManager.missingDeps') }}</span>
                  <span class="tabular-nums text-[11px] text-gray-400 dark:text-gray-500 font-medium">{{ sceneryStore.missingDepsCount }}</span>
                </div>
                <!-- Duplicate tiles -->
                <div
                  v-if="sceneryStore.duplicateTilesCount > 0"
                  @click="applyFilterWithTransition(() => showOnlyDuplicateTiles = !showOnlyDuplicateTiles)"
                  class="flex items-center gap-2.5 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer text-xs transition-colors mx-1 rounded-lg group"
                >
                  <span class="filter-check border-gray-300 dark:border-gray-500 group-hover:border-orange-400 dark:group-hover:border-orange-500" :class="showOnlyDuplicateTiles && 'filter-check-active bg-orange-500 !border-orange-500'">
                    <svg class="filter-check-icon" viewBox="0 0 12 12" fill="none"><path d="M3.5 6L5.5 8L8.5 4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </span>
                  <span class="flex-1 text-gray-700 dark:text-gray-200">{{ t('sceneryManager.duplicateTiles') }}</span>
                  <span class="tabular-nums text-[11px] text-gray-400 dark:text-gray-500 font-medium">{{ sceneryStore.duplicateTilesCount }}</span>
                </div>
                <!-- Separator -->
                <div class="border-t border-gray-100 dark:border-gray-700 my-1.5 mx-3"></div>
              </template>
              <!-- Enabled only -->
              <div
                @click="applyFilterWithTransition(() => enabledFilter = enabledFilter === 'enabled' ? 'all' : 'enabled')"
                class="flex items-center gap-2.5 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer text-xs transition-colors mx-1 rounded-lg group"
              >
                <span class="filter-check border-gray-300 dark:border-gray-500 group-hover:border-green-400 dark:group-hover:border-green-500" :class="enabledFilter === 'enabled' && 'filter-check-active bg-green-500 !border-green-500'">
                  <svg class="filter-check-icon" viewBox="0 0 12 12" fill="none"><path d="M3.5 6L5.5 8L8.5 4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </span>
                <span class="flex-1 text-gray-700 dark:text-gray-200">{{ t('sceneryManager.showOnlyEnabled') }}</span>
              </div>
              <!-- Disabled only -->
              <div
                @click="applyFilterWithTransition(() => enabledFilter = enabledFilter === 'disabled' ? 'all' : 'disabled')"
                class="flex items-center gap-2.5 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer text-xs transition-colors mx-1 rounded-lg group"
              >
                <span class="filter-check border-gray-300 dark:border-gray-500 group-hover:border-red-400 dark:group-hover:border-red-500" :class="enabledFilter === 'disabled' && 'filter-check-active bg-red-500 !border-red-500'">
                  <svg class="filter-check-icon" viewBox="0 0 12 12" fill="none"><path d="M3.5 6L5.5 8L8.5 4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </span>
                <span class="flex-1 text-gray-700 dark:text-gray-200">{{ t('sceneryManager.showOnlyDisabled') }}</span>
              </div>
              <!-- Separator -->
              <div v-if="uniqueContinents.length > 0" class="border-t border-gray-100 dark:border-gray-700 my-1.5 mx-3"></div>
              <!-- Group by continent -->
              <div
                v-if="uniqueContinents.length > 0"
                @click="toggleViewMode"
                class="flex items-center gap-2.5 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer text-xs transition-colors mx-1 rounded-lg group"
              >
                <span class="filter-check border-gray-300 dark:border-gray-500 group-hover:border-blue-400 dark:group-hover:border-blue-500" :class="viewMode === 'continent' && 'filter-check-active bg-blue-500 !border-blue-500'">
                  <svg class="filter-check-icon" viewBox="0 0 12 12" fill="none"><path d="M3.5 6L5.5 8L8.5 4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </span>
                <svg class="w-3.5 h-3.5 text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" />
                </svg>
                <span class="flex-1 text-gray-700 dark:text-gray-200">{{ t('sceneryManager.groupByContinent') }}</span>
              </div>
            </div>
          </Transition>
        </div>
        <!-- Updating index indicator -->
        <div v-if="isUpdatingIndex" class="flex items-center gap-2 text-gray-500 dark:text-gray-400">
          <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span class="text-xs">{{ t('sceneryManager.updatingIndex') }}</span>
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
    <div ref="scrollContainerRef" class="flex-1 overflow-y-auto tab-content-container">
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
                @view-liveries="handleViewLiveries"
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
                @view-scripts="handleViewScripts"
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
                :backup-info="getNavdataBackup(item.providerName)"
                @toggle-enabled="(fn) => handleToggleEnabled('navdata', fn)"
                @delete="(fn) => handleDelete('navdata', fn)"
                @open-folder="(fn) => handleOpenFolder('navdata', fn)"
                @restore-backup="handleRestoreBackup"
              />
            </div>
          </template>

          <!-- Scenery Tab Content (migrated from SceneryManager.vue) -->
          <template v-else-if="activeTab === 'scenery'">
        <!-- Database version error - needs reset -->
        <div v-if="sceneryStore.needsDatabaseReset" class="text-center py-12">
          <div class="flex flex-col items-center gap-4">
            <svg class="w-12 h-12 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            <Transition name="text-fade" mode="out-in">
              <p :key="locale" class="text-lg font-medium text-gray-900 dark:text-gray-100">{{ t('sceneryManager.databaseVersionError') }}</p>
            </Transition>
            <Transition name="text-fade" mode="out-in">
              <p :key="locale" class="text-gray-600 dark:text-gray-400 max-w-md">{{ t('sceneryManager.databaseVersionErrorDesc') }}</p>
            </Transition>
            <button
              @click="handleResetDatabase"
              :disabled="isResettingDatabase"
              class="px-4 py-2 rounded-lg bg-amber-500 text-white hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm flex items-center justify-center space-x-2"
            >
              <svg v-if="!isResettingDatabase" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
              </svg>
              <svg v-else class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <Transition name="text-fade" mode="out-in">
                <span :key="locale">{{ t('sceneryManager.resetDatabase') }}</span>
              </Transition>
            </button>
          </div>
        </div>

        <!-- No index created -->
        <div v-else-if="!sceneryStore.indexExists" class="text-center py-12">
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
              <svg v-else class="w-4 h-4 animate-spin [animation-direction:reverse]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

        <!-- View mode transitioning loading -->
        <div v-else-if="isViewModeTransitioning || isFilterTransitioning" class="flex items-center justify-center py-12">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
        </div>

        <!-- Continent grouped view (no drag-and-drop, filter-aware) -->
        <div v-else-if="viewMode === 'continent'" class="space-y-3 pb-2" style="overflow: visible;">
          <template v-for="continent in (hasDataFilters ? filteredSortedContinentOrder : sortedContinentOrder)" :key="continent">
            <div class="continent-group" style="overflow: visible;">
              <!-- Continent Header -->
              <div
                @click="toggleContinentCollapse(continent)"
                class="continent-header flex items-center gap-2 px-3 py-2 bg-gradient-to-r from-blue-100 to-blue-200 dark:from-blue-900/50 dark:to-blue-800/50 rounded-lg cursor-pointer hover:from-blue-200 hover:to-blue-300 dark:hover:from-blue-800/50 dark:hover:to-blue-700/50 transition-all duration-200 mb-2 border border-blue-300 dark:border-blue-600 shadow-md"
              >
                <div class="flex-1 flex items-center gap-2">
                  <svg
                    class="w-4 h-4 text-blue-700 dark:text-blue-300 transition-transform duration-200"
                    :class="{ 'rotate-90': isContinentExpanded(continent) }"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
                  </svg>
                  <!-- Globe icon -->
                  <svg class="w-4 h-4 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span class="font-semibold text-sm text-blue-900 dark:text-blue-100">
                    {{ t(`geo.continents.${continent}`, continent) }}
                  </span>
                  <span class="text-xs font-medium text-blue-700 dark:text-blue-300 bg-white dark:bg-gray-800 px-2 py-0.5 rounded-full">
                    <span class="text-green-700 dark:text-green-300">{{ (hasDataFilters ? getFilteredContinentStats(continent) : getContinentStats(continent)).enabled }}</span>
                    <span class="mx-1 text-gray-400">/</span>
                    <span class="text-gray-600 dark:text-gray-400">{{ (hasDataFilters ? getFilteredContinentStats(continent) : getContinentStats(continent)).total }}</span>
                  </span>
                </div>
                <!-- Continent toggle switch -->
                <button
                  v-if="!hasDataFilters"
                  @click.stop="toggleContinentEnabled(continent)"
                  class="flex-shrink-0 px-2 py-0.5 rounded text-xs font-medium transition-colors"
                  :class="isContinentAllEnabled(continent)
                    ? 'bg-green-500 text-white hover:bg-green-600'
                    : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'"
                  :title="isContinentAllEnabled(continent) ? t('sceneryManager.disableAll') : t('sceneryManager.enableAll')"
                >
                  {{ isContinentAllEnabled(continent) ? t('sceneryManager.disableAll') : t('sceneryManager.enableAll') }}
                </button>
              </div>

              <!-- Continent Content (Collapsible) -->
              <Transition name="collapse">
                <div v-if="isContinentExpanded(continent)" class="pl-4 space-y-2" style="overflow: visible;">
                  <template v-for="category in categoryOrder" :key="category">
                    <div v-if="(hasDataFilters ? filteredContinentGroupedEntries : continentGroupedEntries)[continent][category]?.length > 0" class="category-in-continent">
                      <!-- Category Header within Continent -->
                      <div
                        class="flex items-center gap-2 px-2 py-1 bg-gray-100 dark:bg-gray-800 rounded mb-1 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                        @click="toggleContinentCategoryCollapse(continent, category)"
                      >
                        <svg
                          class="w-3 h-3 text-gray-500 dark:text-gray-400 transition-transform duration-200"
                          :class="{ 'rotate-90': isContinentCategoryExpanded(continent, category) }"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                        </svg>
                        <span class="flex-1 text-xs font-medium text-gray-700 dark:text-gray-300">
                          {{ t(getCategoryTranslationKey(category)) }}
                          <span class="ml-1 text-gray-500 dark:text-gray-400">
                            ({{ (hasDataFilters ? filteredContinentGroupedEntries : continentGroupedEntries)[continent][category].filter(e => e.enabled).length }}/{{ (hasDataFilters ? filteredContinentGroupedEntries : continentGroupedEntries)[continent][category].length }})
                          </span>
                        </span>
                        <!-- Category toggle switch -->
                        <button
                          v-if="!hasDataFilters"
                          @click.stop="toggleContinentCategoryEnabled(continent, category)"
                          class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors"
                          :class="isContinentCategoryAllEnabled(continent, category)
                            ? 'bg-green-500 text-white hover:bg-green-600'
                            : 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-300 dark:hover:bg-gray-600'"
                        >
                          {{ isContinentCategoryAllEnabled(continent, category) ? t('sceneryManager.disableAll') : t('sceneryManager.enableAll') }}
                        </button>
                      </div>
                      <!-- Entries in this category (Collapsible) -->
                      <Transition name="collapse">
                        <div v-if="isContinentCategoryExpanded(continent, category)" class="space-y-1.5 px-1">
                          <div
                            v-for="element in (hasDataFilters ? filteredContinentGroupedEntries : continentGroupedEntries)[continent][category]"
                            :key="element.folderName"
                            v-memo="[
                              element.enabled,
                              element.category,
                              element.missingLibraries?.length ?? 0,
                              element.duplicateTiles?.length ?? 0,
                              searchQueryLower,
                              highlightedIndex === getGlobalIndex(element.folderName)
                            ]"
                            :data-scenery-index="getGlobalIndex(element.folderName)"
                            class="relative scenery-entry-item"
                            style="scroll-margin-top: 100px"
                          >
                            <div
                              v-if="highlightedIndex === getGlobalIndex(element.folderName)"
                              class="absolute inset-0 border-2 border-blue-500 rounded-lg pointer-events-none z-10"
                            ></div>
                            <div
                              :class="{
                                'opacity-30 transition-opacity': searchQueryLower && !element.folderName.toLowerCase().includes(searchQueryLower)
                              }"
                            >
                              <SceneryEntryCard
                                :entry="element"
                                :index="getGlobalIndex(element.folderName)"
                                :total-count="sceneryStore.totalCount"
                                :disable-reorder="true"
                                @toggle-enabled="handleSceneryToggleEnabled"
                                @move-up="handleMoveUp"
                                @move-down="handleMoveDown"
                                @show-missing-libs="handleShowMissingLibs"
                                @show-duplicate-tiles="handleShowDuplicateTiles"
                                @show-delete-confirm="handleShowDeleteConfirm"
                              />
                            </div>
                          </div>
                        </div>
                      </Transition>
                    </div>
                  </template>
                </div>
              </Transition>
            </div>
          </template>
          <div v-if="hasDataFilters && filteredSceneryEntries.length === 0" class="text-center py-12">
            <Transition name="text-fade" mode="out-in">
              <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noMissingLibs') }}</p>
            </Transition>
          </div>
        </div>

        <!-- Filtered view with groups (no drag-and-drop) -->
        <div v-else-if="hasDataFilters" class="space-y-3 pb-2" style="overflow: visible;">
          <template
            v-for="category in categoryOrder"
            :key="category"
          >
            <div
              v-if="filteredGroupedEntries[category] && filteredGroupedEntries[category].length > 0"
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
                  <span class="text-green-700 dark:text-green-300">{{ filteredGroupedEntries[category].filter(e => e.enabled).length }}</span>
                  <span class="mx-1 text-gray-400">/</span>
                  <span class="text-gray-600 dark:text-gray-400">{{ filteredGroupedEntries[category].length }}</span>
                </span>
              </div>

              <!-- Group Content (Collapsible) -->
              <Transition name="collapse">
                <div v-if="isGroupExpanded(category)" class="space-y-1.5 px-1">
                  <div
                    v-for="element in filteredGroupedEntries[category]"
                    :key="element.folderName"
                    v-memo="[
                      element.enabled,
                      element.category,
                      element.missingLibraries?.length ?? 0,
                      element.duplicateTiles?.length ?? 0,
                      searchQueryLower,
                      highlightedIndex === getGlobalIndex(element.folderName)
                    ]"
                    :data-scenery-index="getGlobalIndex(element.folderName)"
                    class="relative scenery-entry-item"
                    style="scroll-margin-top: 100px"
                  >
                    <div
                      v-if="highlightedIndex === getGlobalIndex(element.folderName)"
                      class="absolute inset-0 border-2 border-blue-500 rounded-lg pointer-events-none z-10"
                    ></div>
                    <div
                      :class="{
                        'opacity-30 transition-opacity': searchQueryLower && !element.folderName.toLowerCase().includes(searchQueryLower)
                      }"
                    >
                      <SceneryEntryCard
                        :entry="element"
                        :index="getGlobalIndex(element.folderName)"
                        :total-count="sceneryStore.totalCount"
                        :disable-reorder="true"
                        @toggle-enabled="handleSceneryToggleEnabled"
                        @move-up="handleMoveUp"
                        @move-down="handleMoveDown"
                        @show-missing-libs="handleShowMissingLibs"
                        @show-duplicate-tiles="handleShowDuplicateTiles"
                        @show-delete-confirm="handleShowDeleteConfirm"
                      />
                    </div>
                  </div>
                </div>
              </Transition>
            </div>
          </template>
          <div v-if="filteredSceneryEntries.length === 0" class="text-center py-12">
            <Transition name="text-fade" mode="out-in">
              <p :key="locale" class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noMissingLibs') }}</p>
            </Transition>
          </div>
        </div>

        <!-- Normal view with drag-and-drop groups -->
        <div v-else class="space-y-3 pb-2" style="overflow: visible;">
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
                    :group="category === 'Unrecognized'
                      ? { name: 'unrecognized', pull: false, put: false }
                      : { name: 'scenery', pull: true, put: true }"
                    item-key="folderName"
                    handle=".drag-handle"
                    :disabled="!sceneryStore.indexExists || category === 'Unrecognized'"
                    :animation="180"
                    :easing="'cubic-bezier(0.25, 0.8, 0.25, 1)'"
                    :force-fallback="true"
                    :fallback-on-body="true"
                    :fallback-tolerance="5"
                    :direction="'vertical'"
                    ghost-class="drag-ghost"
                    drag-class="sortable-drag"
                    @start="handleDragStart"
                    @end="handleDragEnd"
                    @change="(evt: DraggableChangeEvent<SceneryManagerEntry>) => handleGroupChange(category, evt)"
                    class="space-y-1.5"
                    style="overflow: visible; padding: 0 0.5rem;"
                  >
                    <template #item="{ element }">
                      <div
                        :data-scenery-index="getGlobalIndex(element.folderName)"
                        class="relative scenery-entry-item"
                        style="scroll-margin-top: 100px"
                      >
                        <div
                          v-if="highlightedIndex === getGlobalIndex(element.folderName)"
                          class="absolute inset-0 border-2 border-blue-500 rounded-lg pointer-events-none z-10"
                        ></div>
                        <div
                          :class="{
                            'opacity-30 transition-opacity': searchQueryLower && !element.folderName.toLowerCase().includes(searchQueryLower)
                          }"
                        >
                          <SceneryEntryCard
                            :entry="element"
                            :index="getGlobalIndex(element.folderName)"
                            :total-count="sceneryStore.totalCount"
                            :disable-reorder="!sceneryStore.indexExists || category === 'Unrecognized'"
                            :disable-move-down="element.folderName === lastEntryBeforeUnrecognized"
                            @toggle-enabled="handleSceneryToggleEnabled"
                            @move-up="handleMoveUp"
                            @move-down="handleMoveDown"
                            @show-missing-libs="handleShowMissingLibs"
                            @show-duplicate-tiles="handleShowDuplicateTiles"
                            @show-delete-confirm="handleShowDeleteConfirm"
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

    <!-- Shared Missing Libraries Modal -->
    <Teleport to="body">
      <div
        v-if="showMissingLibsModal && selectedModalEntry"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        @click="showMissingLibsModal = false"
      >
        <div
          class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full mx-4 flex flex-col"
          style="max-width: 520px; max-height: 80vh;"
          @click.stop
        >
          <!-- Modal Header -->
          <div class="flex items-center justify-between px-5 pt-4 pb-3 flex-shrink-0">
            <div class="flex items-center gap-2.5">
              <div class="w-8 h-8 rounded-lg bg-amber-100 dark:bg-amber-900/40 flex items-center justify-center flex-shrink-0">
                <svg class="w-4 h-4 text-amber-600 dark:text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.5c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z" />
                </svg>
              </div>
              <div>
                <h3 class="text-base font-semibold text-gray-900 dark:text-white leading-tight">
                  {{ t('sceneryManager.missingLibrariesTitle') }}
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate max-w-[320px]">
                  {{ selectedModalEntry.folderName }}
                </p>
              </div>
            </div>
            <button
              @click="showMissingLibsModal = false"
              class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors rounded-md"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Scrollable Content Area -->
          <div class="flex-1 overflow-y-auto px-5 pb-3 min-h-0">
            <!-- Missing Libraries List -->
            <div class="rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div
                v-for="(lib, index) in selectedModalEntry.missingLibraries"
                :key="lib"
                class="flex items-center justify-between gap-2 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                :class="{ 'border-t border-gray-200 dark:border-gray-700': index > 0 }"
              >
                <!-- Library Name with status indicator -->
                <div class="flex items-center gap-2 min-w-0 flex-1">
                  <div
                    class="w-1.5 h-1.5 rounded-full flex-shrink-0"
                    :class="libraryLinksMap[lib] ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'"
                  ></div>
                  <span class="text-[13px] text-gray-700 dark:text-gray-200 font-mono truncate">
                    {{ lib }}
                  </span>
                </div>

                <!-- Action Buttons -->
                <div class="flex items-center gap-0.5 flex-shrink-0">
                  <!-- Copy -->
                  <button
                    @click="handleCopySingleLib(lib)"
                    class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    :title="t('sceneryManager.copyAllLibNames')"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                  </button>

                  <!-- Direct Download -->
                  <button
                    v-if="libraryLinksMap[lib]"
                    @click="handleDirectDownload(libraryLinksMap[lib]!)"
                    class="px-2 py-1 text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 rounded bg-blue-50 dark:bg-blue-900/30 hover:bg-blue-100 dark:hover:bg-blue-800/40 transition-colors flex items-center gap-1.5 text-xs font-medium"
                    :title="t('sceneryManager.directDownload')"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                    </svg>
                    {{ t('sceneryManager.directDownload') }}
                  </button>

                  <!-- Bing Search (only when no direct link) -->
                  <button
                    v-else
                    @click="handleSearchSingleLib(lib)"
                    class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    :title="t('sceneryManager.searchOnBing')"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                  </button>

                  <button
                    v-if="!libraryLinksMap[lib]"
                    @click="handleOpenContributeLink(lib)"
                    class="px-2 py-1 text-emerald-600 hover:text-emerald-700 dark:text-emerald-400 dark:hover:text-emerald-300 rounded bg-emerald-50 dark:bg-emerald-900/30 hover:bg-emerald-100 dark:hover:bg-emerald-800/40 transition-colors text-xs font-medium"
                    :title="t('sceneryManager.contributeLink')"
                  >
                    {{ t('sceneryManager.contributeLink') }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex gap-2 px-5 py-3 flex-shrink-0 border-t border-gray-200 dark:border-gray-700">
            <button
              @click="handleCopyMissingLibs"
              class="flex-1 px-3 py-1.5 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-sm rounded-lg transition-colors flex items-center justify-center gap-1.5"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
              {{ t('sceneryManager.copyAllLibNames') }}
            </button>
            <button
              @click="showMissingLibsModal = false"
              class="px-4 py-1.5 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-sm rounded-lg transition-colors"
            >
              {{ t('common.close') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Contribution Link Modal -->
    <Teleport to="body">
      <div
        v-if="showContributeLinkModal"
        class="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 p-4"
        @click="!isSubmittingContributeLink && closeContributeLinkModal()"
      >
        <div
          class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md p-5"
          @click.stop
        >
          <h3 class="text-base font-semibold text-gray-900 dark:text-white">
            {{ t('sceneryManager.contributeLinkTitle') }}
          </h3>
          <p class="mt-2 text-sm text-gray-600 dark:text-gray-300">
            {{ t('sceneryManager.contributeLinkDesc') }}
          </p>

          <div class="mt-4 space-y-3">
            <div>
              <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">{{ t('sceneryManager.libraryNameLabel') }}</label>
              <div class="relative">
                <input
                  :value="contributingLibName"
                  disabled
                  class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-700/50 pl-3 pr-20 py-2 text-sm text-gray-700 dark:text-gray-200"
                />
                <div class="absolute inset-y-0 right-2 flex items-center gap-1">
                  <button
                    @click="handleCopySingleLib(contributingLibName)"
                    class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    :title="t('sceneryManager.copyAllLibNames')"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                  </button>
                  <button
                    @click="handleSearchSingleLib(contributingLibName)"
                    class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    :title="t('sceneryManager.searchOnBing')"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>
            <div>
              <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">{{ t('sceneryManager.downloadUrlLabel') }}</label>
              <input
                v-model="contributingLibUrl"
                type="url"
                :placeholder="t('sceneryManager.downloadUrlPlaceholder')"
                class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-700 px-3 py-2 text-sm text-gray-900 dark:text-gray-100"
              />
            </div>
          </div>

          <div class="mt-5 flex justify-end gap-2">
            <button
              @click="closeContributeLinkModal"
              :disabled="isSubmittingContributeLink"
              class="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-sm rounded-lg text-gray-700 dark:text-gray-200"
            >
              {{ t('common.cancel') }}
            </button>
            <button
              @click="handleSubmitContributeLink"
              :disabled="isSubmittingContributeLink"
              class="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-70 disabled:cursor-not-allowed text-sm rounded-lg text-white inline-flex items-center gap-1.5"
            >
              <svg
                v-if="isSubmittingContributeLink"
                class="w-4 h-4 animate-spin"
                fill="none"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              {{ isSubmittingContributeLink ? t('sceneryManager.submittingContribution') : t('sceneryManager.submitContribution') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Shared Duplicate Tiles Modal -->
    <Teleport to="body">
      <div
        v-if="showDuplicateTilesModal && selectedModalEntry"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        @click="showDuplicateTilesModal = false"
      >
        <div
          class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full mx-4 flex flex-col"
          style="max-width: 500px; max-height: 80vh;"
          @click.stop
        >
          <!-- Modal Header -->
          <div class="flex items-center justify-between p-5 pb-3 flex-shrink-0">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
              {{ t('sceneryManager.duplicateTilesTitle') }}
            </h3>
            <button
              @click="showDuplicateTilesModal = false"
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
              {{ selectedModalEntry.folderName }}
            </div>

            <!-- Description -->
            <div class="mb-3 text-sm text-gray-600 dark:text-gray-400">
              {{ t('sceneryManager.duplicateTilesDesc') }}
            </div>

            <!-- Conflicting Packages List -->
            <div class="bg-gray-50 dark:bg-gray-900 rounded p-3">
              <ul class="space-y-1">
                <li
                  v-for="pkg in selectedModalEntry.duplicateTiles"
                  :key="pkg"
                  class="text-sm text-gray-800 dark:text-gray-200 font-mono"
                >
                  • {{ pkg }}
                </li>
              </ul>
            </div>
          </div>

          <!-- Close Button -->
          <div class="flex flex-col gap-2 p-5 pt-3 flex-shrink-0 border-t border-gray-200 dark:border-gray-700">
            <button
              @click="showDuplicateTilesModal = false"
              class="w-full px-4 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 rounded-lg transition-colors"
            >
              {{ t('common.close') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Shared Delete Confirmation Modal -->
    <ConfirmModal
      v-model:show="showDeleteConfirmModal"
      :title="t('sceneryManager.deleteConfirmTitle')"
      :message="t('sceneryManager.deleteConfirmMessage')"
      :item-name="selectedModalEntry?.folderName ?? ''"
      :confirm-text="t('common.delete')"
      :loading-text="t('common.deleting')"
      :is-loading="isDeletingEntry"
      variant="danger"
      @confirm="handleDeleteEntryConfirm"
    />
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

/* Dropdown enter/leave transition */
.dropdown-enter-active {
  transition: opacity 0.15s ease-out, transform 0.15s ease-out;
}
.dropdown-leave-active {
  transition: opacity 0.1s ease-in, transform 0.1s ease-in;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.97);
}

/* Custom filter checkbox */
.filter-check {
  width: 16px;
  height: 16px;
  border-radius: 4px;
  border-width: 1.5px;
  border-style: solid;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  transition: background-color 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}
.filter-check-icon {
  width: 12px;
  height: 12px;
  opacity: 0;
  transform: scale(0.5);
  transition: opacity 0.15s ease, transform 0.15s cubic-bezier(0.2, 0, 0.13, 2);
}
.filter-check-active .filter-check-icon {
  opacity: 1;
  transform: scale(1);
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
/* Tab slide animations */
.tab-slide-left-enter-active,
.tab-slide-left-leave-active,
.tab-slide-right-enter-active,
.tab-slide-right-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.tab-slide-left-enter-from {
  opacity: 0;
  transform: translateX(30px);
}

.tab-slide-left-leave-to {
  opacity: 0;
  transform: translateX(-30px);
}

.tab-slide-right-enter-from {
  opacity: 0;
  transform: translateX(-30px);
}

.tab-slide-right-leave-to {
  opacity: 0;
  transform: translateX(30px);
}

/* Tab indicator animation enhancement */
.tab-indicator {
  will-change: transform, width, left;
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

/* Performance: Use content-visibility for offscreen items in large lists */
/* This allows the browser to skip rendering of offscreen items */
.scenery-entry-item {
  content-visibility: auto;
  contain-intrinsic-size: auto 44px; /* Approximate height of entry card */
}

/* Performance: Optimize list rendering */
.scenery-group {
  contain: layout style;
}
</style>
