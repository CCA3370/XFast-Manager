<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useManagementStore } from '@/stores/management'
import { useToastStore } from '@/stores/toast'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { getNavdataCycleStatus } from '@/utils/airac'
import ManagementEntryCard from '@/components/ManagementEntryCard.vue'
import SceneryTab from '@/views/SceneryTab.vue'
import type { ManagementTab, ManagementItemType, NavdataBackupInfo } from '@/types'

const { t, locale } = useI18n()
const route = useRoute()
const router = useRouter()
const managementStore = useManagementStore()
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

// Filter state for non-scenery tabs
const showOnlyUpdates = ref(false)
const showOnlyOutdated = ref(false)
const suppressLoading = ref(false)

// Initialize tab from route query
onMounted(() => {
  const tabParam = route.query.tab as ManagementTab | undefined
  if (tabParam && availableTabs.value.includes(tabParam)) {
    activeTab.value = tabParam
  }

  // Load initial data (non-blocking to avoid route transition delay)
  loadTabData(activeTab.value)
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

  // Start loading data (non-blocking)
  const loadPromise = loadTabData(newTab)

  // Wait for transition animation to complete before showing loading state
  setTimeout(() => {
    suppressLoading.value = false
  }, 350) // Slightly longer than transition duration (300ms)

  await loadPromise
})

// Auto-reset filter when no updates available
watch(
  () => managementStore.aircraftUpdateCount + managementStore.pluginsUpdateCount,
  (newCount) => {
    if (newCount === 0 && showOnlyUpdates.value) {
      showOnlyUpdates.value = false
    }
  },
)

// Auto-reset filter when no outdated navdata
watch(
  () => managementStore.navdataOutdatedCount,
  (newCount) => {
    if (newCount === 0 && showOnlyOutdated.value) {
      showOnlyOutdated.value = false
    }
  },
)

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
        // SceneryTab handles its own data loading
        break
    }
  } catch (e) {
    modalStore.showError(t('management.scanFailed') + ': ' + String(e))
  }
}

// Filtered entries for non-scenery tabs
const filteredAircraft = computed(() => {
  let items = managementStore.sortedAircraft
  if (showOnlyUpdates.value) {
    items = items.filter((a) => a.hasUpdate)
  }
  if (!searchQuery.value.trim()) return items
  const query = searchQuery.value.toLowerCase()
  return items.filter(
    (a) =>
      a.displayName.toLowerCase().includes(query) || a.folderName.toLowerCase().includes(query),
  )
})

const filteredPlugins = computed(() => {
  let items = managementStore.sortedPlugins
  if (showOnlyUpdates.value) {
    items = items.filter((p) => p.hasUpdate)
  }
  if (!searchQuery.value.trim()) return items
  const query = searchQuery.value.toLowerCase()
  return items.filter(
    (p) =>
      p.displayName.toLowerCase().includes(query) || p.folderName.toLowerCase().includes(query),
  )
})

const filteredNavdata = computed(() => {
  let items = managementStore.sortedNavdata
  if (showOnlyOutdated.value) {
    items = items.filter((n) => {
      const cycleText = n.cycle || n.airac
      return getNavdataCycleStatus(cycleText) === 'outdated'
    })
  }
  if (!searchQuery.value.trim()) return items
  const query = searchQuery.value.toLowerCase()
  return items.filter(
    (n) =>
      n.providerName.toLowerCase().includes(query) || n.folderName.toLowerCase().includes(query),
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
  return (
    managementStore.navdataBackups.find((b) => b.verification.providerName === providerName) || null
  )
}

// Handle restore navdata backup
function handleRestoreBackup(backupInfo: NavdataBackupInfo) {
  const cycle = backupInfo.verification.cycle || backupInfo.verification.airac || ''
  const backupTime = new Date(backupInfo.verification.backupTime).toLocaleString()
  // Truncate long provider names
  const providerName =
    backupInfo.verification.providerName.length > 30
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
    onCancel: () => {},
  })
}

// ========== Scenery-specific functions (migrated from SceneryManager.vue) ==========

const groupCounts = computed(() => {
  const counts: Record<string, { enabled: number; disabled: number }> = {}
  for (const category of categoryOrder) {
    const entries = localGroupedEntries.value[category] || []
    const enabled = entries.filter((entry) => entry.enabled).length
    counts[category] = { enabled, disabled: entries.length - enabled }
  }
  return counts
})

// Base computed for all entries flattened - used by multiple computeds below
const allSceneryEntries = computed(() => {
  return categoryOrder.flatMap((category) => localGroupedEntries.value[category] || [])
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
const knownContinents = [
  'Asia',
  'Europe',
  'North America',
  'South America',
  'Africa',
  'Oceania',
  'Antarctica',
]

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
  const continentsWithEntries = Object.keys(continentGroupedEntries.value).filter((continent) => {
    const data = continentGroupedEntries.value[continent]
    return categoryOrder.some((cat) => (data[cat]?.length || 0) > 0)
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
    enabled += entries.filter((e) => e.enabled).length
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
    enabled += entries.filter((e) => e.enabled).length
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

  const entries = categoryOrder.flatMap((cat) => continentData[cat] || [])
  const allEnabled = entries.every((e) => e.enabled)
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

  const allEnabled = entries.every((e) => e.enabled)
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

  const entries = categoryOrder.flatMap((cat) => continentData[cat] || [])
  return entries.length > 0 && entries.every((e) => e.enabled)
}

// Check if all entries in a continent's category are enabled
function isContinentCategoryAllEnabled(continent: string, category: string): boolean {
  const entries = continentGroupedEntries.value[continent]?.[category] || []
  return entries.length > 0 && entries.every((e) => e.enabled)
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
    entries = entries.filter((entry) => entry.missingLibraries && entry.missingLibraries.length > 0)
  }

  // Filter by duplicate tiles
  if (showOnlyDuplicateTiles.value) {
    entries = entries.filter((entry) => entry.duplicateTiles && entry.duplicateTiles.length > 0)
  }

  // Filter by continent
  if (selectedContinent.value) {
    entries = entries.filter((entry) => entry.continent === selectedContinent.value)
  }

  // Filter by enabled/disabled state
  if (enabledFilter.value === 'enabled') {
    entries = entries.filter((entry) => entry.enabled)
  } else if (enabledFilter.value === 'disabled') {
    entries = entries.filter((entry) => !entry.enabled)
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
  const continentsWithEntries = Object.keys(filteredContinentGroupedEntries.value).filter(
    (continent) => {
      const data = filteredContinentGroupedEntries.value[continent]
      return categoryOrder.some((cat) => (data[cat]?.length || 0) > 0)
    },
  )
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
      index: getGlobalIndex(entry.folderName),
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
  const edgeZone = 60 // 容器内边缘触发区域 (px)
  const maxSpeed = 18 // 到达边缘时的最大速度 (px/frame)
  const outsideAccel = 0.4 // 超出边界后每像素额外加速 (px/frame/px)

  let scrollDelta = 0

  if (dragPointerY < rect.top + edgeZone) {
    // 光标在顶部边缘或上方 → 向上滚动
    if (dragPointerY < rect.top) {
      const dist = rect.top - dragPointerY
      scrollDelta = -(maxSpeed + dist * outsideAccel)
    } else {
      const ratio = (rect.top + edgeZone - dragPointerY) / edgeZone
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
  const index = entries.findIndex((e) => e.folderName === folderName)

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
  const index = entries.findIndex((e) => e.folderName === folderName)
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
  const allEntries = categoryOrder.flatMap((category) => localGroupedEntries.value[category] || [])
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
  })
    .then((remoteLinks) => {
      if (!showMissingLibsModal.value) return
      if (!selectedModalEntry.value) return
      if (selectedModalEntry.value.folderName !== entry.folderName) return
      if (libraryLinksRequestSeq.value !== requestSeq) return
      libraryLinksMap.value = remoteLinks
    })
    .catch(() => {
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
  navigator.clipboard
    .writeText(libNames)
    .then(() => {
      toastStore.success(t('sceneryManager.missingLibsCopied'))
    })
    .catch(() => {
      modalStore.showError(t('copy.copyFailed'))
    })
}

function handleCopySingleLib(libName: string) {
  navigator.clipboard
    .writeText(libName)
    .then(() => {
      toastStore.success(t('sceneryManager.libNameCopied'))
    })
    .catch(() => {
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
    'Please review this link. If valid, add the `approved-link` label to trigger auto-update for `data/library_links.json` on `dev`.',
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
      }),
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
      const localizedMessage = t(errorKey) !== errorKey ? t(errorKey) : apiError.message
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
    onCancel: () => {},
  })
}

async function performAutoSort() {
  if (!sceneryStore.indexExists) return
  isSortingScenery.value = true
  try {
    const hasChanges = await invoke<boolean>('sort_scenery_packs', {
      xplanePath: appStore.xplanePath,
    })
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
    onCancel: () => {},
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
    onCancel: () => {},
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

  const transitioning = container.querySelectorAll(
    '.collapse-enter-active, .collapse-leave-active',
  ) as NodeListOf<HTMLElement>
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

function isSafelyInsideContainerViewport(
  container: HTMLElement,
  element: HTMLElement,
  padding: number,
): boolean {
  const containerRect = container.getBoundingClientRect()
  const elementRect = element.getBoundingClientRect()
  return (
    elementRect.top >= containerRect.top + padding &&
    elementRect.bottom <= containerRect.bottom - padding
  )
}

function scrollToMatch(index: number) {
  ensureGroupExpandedForIndex(index)
  highlightedIndex.value = index
  const requestId = ++activeScrollRequestId

  const attemptScroll = (
    attempt: number,
    behavior: ScrollBehavior,
    predictPostCollapse = false,
  ) => {
    if (highlightedIndex.value !== index || requestId !== activeScrollRequestId) return
    const container = scrollContainerRef.value
    if (!container) return

    const element = container.querySelector(`[data-scenery-index="${index}"]`) as HTMLElement | null
    if (element && element.getClientRects().length > 0) {
      const containerRect = container.getBoundingClientRect()
      const elementRect = element.getBoundingClientRect()
      const currentScrollTop = container.scrollTop
      const targetScrollTop =
        currentScrollTop +
        (elementRect.top - containerRect.top) -
        container.clientHeight / 2 +
        elementRect.height / 2

      const predictedDelta = predictPostCollapse ? estimatePostCollapseDelta(container, element) : 0

      const predictedTargetScrollTop = targetScrollTop + predictedDelta

      const clampedScrollTop = Math.max(
        0,
        Math.min(predictedTargetScrollTop, container.scrollHeight - container.clientHeight),
      )

      container.scrollTo({
        top: clampedScrollTop,
        behavior,
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
    const element = container?.querySelector(
      `[data-scenery-index="${index}"]`,
    ) as HTMLElement | null
    if (
      container &&
      element &&
      isSafelyInsideContainerViewport(container, element, FINAL_CALIBRATION_VIEWPORT_PADDING_PX)
    ) {
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
  currentMatchIndex.value =
    (currentMatchIndex.value - 1 + matchedIndices.value.length) % matchedIndices.value.length
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
  return managementStore.isLoading
})
</script>

<template>
  <div class="management-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Tab Bar -->
    <div
      class="mb-3 flex-shrink-0 relative flex items-center gap-1 p-1 bg-gray-100 dark:bg-gray-800 rounded-lg"
    >
      <!-- Sliding indicator background -->
      <div
        class="tab-indicator absolute top-1 bottom-1 rounded-md bg-white dark:bg-gray-700 shadow-sm transition-all duration-300 ease-out"
        :style="{
          width: `calc((100% - 0.5rem - ${(availableTabs.length - 1) * 0.25}rem) / ${availableTabs.length})`,
          left: `calc(0.25rem + ${activeTabIndex} * (100% - 0.5rem) / ${availableTabs.length})`,
        }"
      />
      <button
        v-for="tab in availableTabs"
        :key="tab"
        class="relative z-10 flex-1 px-3 py-1.5 rounded-md text-sm font-medium transition-colors duration-200"
        :class="
          activeTab === tab
            ? 'text-blue-600 dark:text-blue-400'
            : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
        "
        @click="activeTab = tab"
      >
        <Transition name="text-fade" mode="out-in">
          <span :key="locale">{{ t(`management.${tab}`) }}</span>
        </Transition>
      </button>
    </div>

    <!-- SceneryTab (self-contained with own header, stats, content, modals) -->
    <SceneryTab v-if="activeTab === 'scenery'" class="flex-1 min-h-0 flex flex-col" />

    <!-- Non-scenery content -->
    <template v-else>
      <!-- Header with search and action buttons -->
      <div class="mb-3 flex-shrink-0 flex items-center gap-3">
        <!-- Search box -->
        <div class="flex-1 relative">
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="t('management.searchPlaceholder')"
            class="w-full px-3 py-1.5 pl-9 pr-8 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <svg
            class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>

          <!-- Clear button -->
          <button
            v-if="searchQuery"
            class="absolute right-2 top-1/2 -translate-y-1/2 p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            title="Clear search"
            @click="searchQuery = ''"
          >
            <svg
              class="w-3 h-3 text-gray-600 dark:text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <!-- Check updates button for aircraft/plugin tabs -->
        <button
          v-if="activeTab === 'aircraft' || activeTab === 'plugin'"
          :disabled="managementStore.isCheckingUpdates"
          class="px-3 py-1.5 rounded-lg bg-emerald-500 text-white hover:bg-emerald-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 text-sm"
          @click="handleCheckUpdates"
        >
          <svg
            v-if="!managementStore.isCheckingUpdates"
            class="w-3.5 h-3.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            ></path>
          </svg>
          <svg
            v-else
            class="w-3.5 h-3.5 animate-spin [animation-direction:reverse]"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            ></path>
          </svg>
          <Transition name="text-fade" mode="out-in">
            <span :key="locale">{{ t('management.checkUpdates') }}</span>
          </Transition>
        </button>
      </div>

      <!-- Statistics bar -->
      <div
        class="flex items-center gap-4 px-3 py-2 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 mb-3 text-sm"
      >
        <div class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400"
              >{{ t('management.total') }}:</span
            >
          </Transition>
          <span class="font-semibold text-gray-900 dark:text-gray-100">
            {{
              activeTab === 'aircraft'
                ? managementStore.aircraftTotalCount
                : activeTab === 'plugin'
                  ? managementStore.pluginsTotalCount
                  : managementStore.navdataTotalCount
            }}
          </span>
        </div>
        <div v-if="activeTab !== 'navdata'" class="flex items-center gap-2">
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400"
              >{{ t('management.enabled') }}:</span
            >
          </Transition>
          <span class="font-semibold text-green-600 dark:text-green-400">
            {{
              activeTab === 'aircraft'
                ? managementStore.aircraftEnabledCount
                : managementStore.pluginsEnabledCount
            }}
          </span>
        </div>
        <!-- Update available count for aircraft -->
        <div
          v-if="activeTab === 'aircraft' && managementStore.aircraftUpdateCount > 0"
          class="flex items-center gap-2"
        >
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400"
              >{{ t('management.hasUpdate') }}:</span
            >
          </Transition>
          <span class="font-semibold text-emerald-600 dark:text-emerald-400">
            {{ managementStore.aircraftUpdateCount }}
          </span>
          <button
            class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
            :class="
              showOnlyUpdates
                ? 'bg-emerald-500 text-white hover:bg-emerald-600'
                : 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400 hover:bg-emerald-200 dark:hover:bg-emerald-900/50'
            "
            :title="t('management.filterUpdatesOnly')"
            @click="showOnlyUpdates = !showOnlyUpdates"
          >
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{
                showOnlyUpdates ? t('management.showAll') : t('management.filterUpdatesOnly')
              }}</span>
            </Transition>
          </button>
        </div>
        <!-- Update available count for plugins -->
        <div
          v-if="activeTab === 'plugin' && managementStore.pluginsUpdateCount > 0"
          class="flex items-center gap-2"
        >
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400"
              >{{ t('management.hasUpdate') }}:</span
            >
          </Transition>
          <span class="font-semibold text-emerald-600 dark:text-emerald-400">
            {{ managementStore.pluginsUpdateCount }}
          </span>
          <button
            class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
            :class="
              showOnlyUpdates
                ? 'bg-emerald-500 text-white hover:bg-emerald-600'
                : 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400 hover:bg-emerald-200 dark:hover:bg-emerald-900/50'
            "
            :title="t('management.filterUpdatesOnly')"
            @click="showOnlyUpdates = !showOnlyUpdates"
          >
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{
                showOnlyUpdates ? t('management.showAll') : t('management.filterUpdatesOnly')
              }}</span>
            </Transition>
          </button>
        </div>
        <!-- Checking updates indicator -->
        <div
          v-if="
            (activeTab === 'aircraft' || activeTab === 'plugin') &&
            managementStore.isCheckingUpdates
          "
          class="flex items-center gap-2 text-gray-500 dark:text-gray-400"
        >
          <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle
              class="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              stroke-width="4"
            ></circle>
            <path
              class="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
          <span class="text-xs">{{ t('management.checkingUpdates') }}</span>
        </div>
        <!-- Outdated count for navdata -->
        <div
          v-if="activeTab === 'navdata' && managementStore.navdataOutdatedCount > 0"
          class="flex items-center gap-2"
        >
          <Transition name="text-fade" mode="out-in">
            <span :key="locale" class="text-xs text-gray-600 dark:text-gray-400"
              >{{ t('management.outdated') }}:</span
            >
          </Transition>
          <span class="font-semibold text-red-600 dark:text-red-400">
            {{ managementStore.navdataOutdatedCount }}
          </span>
          <button
            class="ml-1 px-2 py-0.5 rounded text-xs transition-colors"
            :class="
              showOnlyOutdated
                ? 'bg-red-500 text-white hover:bg-red-600'
                : 'bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50'
            "
            :title="t('management.filterOutdatedOnly')"
            @click="showOnlyOutdated = !showOnlyOutdated"
          >
            <Transition name="text-fade" mode="out-in">
              <span :key="locale">{{
                showOnlyOutdated ? t('management.showAll') : t('management.filterOutdatedOnly')
              }}</span>
            </Transition>
          </button>
        </div>
        <!-- Restoring backup indicator -->
        <div
          v-if="activeTab === 'navdata' && managementStore.isRestoringBackup"
          class="flex items-center gap-2 text-gray-500 dark:text-gray-400"
        >
          <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle
              class="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              stroke-width="4"
            ></circle>
            <path
              class="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
          <span class="text-xs">{{ t('management.restoringBackup') }}</span>
        </div>
      </div>

      <!-- Content -->
      <div ref="scrollContainerRef" class="flex-1 overflow-y-auto tab-content-container">
        <!-- No X-Plane path set -->
        <div v-if="!appStore.xplanePath" class="flex items-center justify-center h-full">
          <div class="text-center">
            <svg
              class="w-16 h-16 mx-auto text-gray-400 dark:text-gray-600 mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <Transition name="text-fade" mode="out-in">
              <p :key="locale" class="text-gray-600 dark:text-gray-400">
                {{ t('settings.sceneryAutoSortNeedPath') }}
              </p>
            </Transition>
          </div>
        </div>

        <!-- Loading state -->
        <div v-else-if="isLoading" class="flex items-center justify-center py-12">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
        </div>

        <!-- Tab Content with Transition -->
        <Transition v-else :name="tabTransitionName" mode="out-in">
          <div :key="activeTab" class="tab-content-wrapper">
            <!-- Aircraft Tab Content -->
            <template v-if="activeTab === 'aircraft'">
              <div class="space-y-1.5 px-1">
                <div v-if="filteredAircraft.length === 0" class="text-center py-12">
                  <Transition name="text-fade" mode="out-in">
                    <p :key="locale" class="text-gray-600 dark:text-gray-400">
                      {{ t('management.noItems') }}
                    </p>
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
                    <p :key="locale" class="text-gray-600 dark:text-gray-400">
                      {{ t('management.noItems') }}
                    </p>
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
                    <p :key="locale" class="text-gray-600 dark:text-gray-400">
                      {{ t('management.noItems') }}
                    </p>
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
          </div>
        </Transition>
      </div>
    </template>
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
</style>
