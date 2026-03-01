<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useManagementStore, isProtectedAircraft } from '@/stores/management'
import { useToastStore } from '@/stores/toast'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { getNavdataCycleStatus } from '@/utils/airac'
import ManagementEntryCard from '@/components/ManagementEntryCard.vue'
import AddonUpdateDrawer from '@/components/AddonUpdateDrawer.vue'
import SceneryTab from '@/views/SceneryTab.vue'
import type {
  ManagementTab,
  ManagementItemType,
  NavdataBackupInfo,
  AddonUpdatableItemType,
} from '@/types'

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

// Selection mode & state (transient, not persisted)
const selectionMode = ref(false)
const selectedAircraft = ref<Set<string>>(new Set())
const selectedPlugins = ref<Set<string>>(new Set())

// Addon update drawer state
const showUpdateDrawer = ref(false)
type UpdateDrawerTask = {
  itemType: AddonUpdatableItemType
  folderName: string
  displayName: string
  initialLocalVersion?: string
  initialTargetVersion?: string
}
const updateDrawerTasks = ref<UpdateDrawerTask[]>([])
const updateDrawerActiveKey = ref('')

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
  // Clear selection when switching tabs
  selectionMode.value = false
  selectedAircraft.value = new Set()
  selectedPlugins.value = new Set()

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

watch(showUpdateDrawer, (open) => {
  if (open) return
  updateDrawerTasks.value = []
  updateDrawerActiveKey.value = ''
})

async function loadTabData(tab: ManagementTab) {
  if (!appStore.xplanePath) return

  try {
    switch (tab) {
      case 'aircraft':
        await managementStore.loadAircraft()
        if (managementStore.error) {
          toastStore.warning(t('management.scanFailed') + ': ' + managementStore.error)
        }
        break
      case 'plugin':
        await managementStore.loadPlugins()
        if (managementStore.error) {
          toastStore.warning(t('management.scanFailed') + ': ' + managementStore.error)
        }
        break
      case 'navdata':
        await managementStore.loadNavdata()
        if (managementStore.error) {
          toastStore.warning(t('management.scanFailed') + ': ' + managementStore.error)
        }
        break
      case 'scenery':
        // SceneryTab handles its own data loading
        break
    }
  } catch (e) {
    toastStore.warning(t('management.scanFailed') + ': ' + String(e))
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

// Selection computed properties
const currentSelected = computed(() => {
  return activeTab.value === 'aircraft' ? selectedAircraft.value : selectedPlugins.value
})

const currentFilteredItems = computed(() => {
  return activeTab.value === 'aircraft' ? filteredAircraft.value : filteredPlugins.value
})

const isAllSelected = computed(() => {
  const items = currentFilteredItems.value
  const selected = currentSelected.value
  return items.length > 0 && items.every((item) => selected.has(item.folderName))
})

const isIndeterminate = computed(() => {
  const items = currentFilteredItems.value
  const selected = currentSelected.value
  const selectedCount = items.filter((item) => selected.has(item.folderName)).length
  return selectedCount > 0 && selectedCount < items.length
})

const selectedCount = computed(() => {
  return currentSelected.value.size
})

// Selection methods
function toggleSelectionMode() {
  selectionMode.value = !selectionMode.value
  if (!selectionMode.value) {
    selectedAircraft.value = new Set()
    selectedPlugins.value = new Set()
  }
}

function toggleSelectAll() {
  const items = currentFilteredItems.value
  const selected = activeTab.value === 'aircraft' ? selectedAircraft : selectedPlugins

  if (isAllSelected.value) {
    // Deselect all filtered items
    const newSet = new Set(selected.value)
    for (const item of items) {
      newSet.delete(item.folderName)
    }
    selected.value = newSet
  } else {
    // Select all filtered items
    const newSet = new Set(selected.value)
    for (const item of items) {
      newSet.add(item.folderName)
    }
    selected.value = newSet
  }
}

function toggleSelect(folderName: string) {
  const selected = activeTab.value === 'aircraft' ? selectedAircraft : selectedPlugins
  const newSet = new Set(selected.value)
  if (newSet.has(folderName)) {
    newSet.delete(folderName)
  } else {
    newSet.add(folderName)
  }
  selected.value = newSet
}

const isBatchProcessing = ref(false)
const isUpdatingAllAircraft = ref(false)

function isSkunkUpdateUrl(url?: string): boolean {
  const value = (url || '').trim().toLowerCase()
  return !!value && !value.startsWith('x-updater:')
}

const skunkUpdatableAircraftCount = computed(() => {
  return managementStore.sortedAircraft.filter((item) => isSkunkUpdateUrl(item.updateUrl)).length
})

async function batchSetEnabled(enabled: boolean) {
  if (isBatchProcessing.value) return
  const itemType = activeTab.value as ManagementItemType
  const folderNames = [...currentSelected.value]

  isBatchProcessing.value = true
  try {
    await managementStore.batchSetEnabled(itemType, folderNames, enabled)
  } catch (e) {
    await loadTabData(activeTab.value)
    modalStore.showError(t('management.toggleFailed') + ': ' + String(e))
  } finally {
    isBatchProcessing.value = false
    // Clear selection after batch operation
    if (activeTab.value === 'aircraft') {
      selectedAircraft.value = new Set()
    } else {
      selectedPlugins.value = new Set()
    }
  }
}

async function runUpdateAllAircraft() {
  isUpdatingAllAircraft.value = true
  try {
    await managementStore.checkAircraftUpdates(true)

    const targets = managementStore.sortedAircraft.filter(
      (item) => isSkunkUpdateUrl(item.updateUrl) && item.hasUpdate,
    )

    if (targets.length === 0) {
      toastStore.info(t('management.allUpToDate'))
      return
    }

    let success = 0
    const failed: string[] = []

    for (const item of targets) {
      try {
        await managementStore.executeAddonUpdate('aircraft', item.folderName)
        success += 1
      } catch (e) {
        failed.push(`${item.displayName}: ${String(e)}`)
      }
    }

    if (success > 0) {
      toastStore.success(
        t('management.updateAllSuccess', {
          updated: success,
          total: targets.length,
        }),
      )
    }

    if (failed.length > 0) {
      modalStore.showError(
        `${t('management.updateAllPartialFailed', {
          failed: failed.length,
          total: targets.length,
        })}\n\n${failed.join('\n')}`,
      )
    }
  } finally {
    isUpdatingAllAircraft.value = false
  }
}

function handleUpdateAllAircraft() {
  if (isUpdatingAllAircraft.value || managementStore.isExecutingUpdate) return
  if (activeTab.value !== 'aircraft') return
  if (skunkUpdatableAircraftCount.value === 0) return

  modalStore.showConfirm({
    title: t('management.updateAll'),
    message: t('management.updateAllConfirm', { count: skunkUpdatableAircraftCount.value }),
    confirmText: t('management.updateAll'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: async () => {
      await runUpdateAllAircraft()
    },
    onCancel: () => {},
  })
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

function handleOpenUpdate(
  itemType: AddonUpdatableItemType,
  folderName: string,
  displayName: string,
  currentVersion?: string,
  latestVersion?: string,
) {
  const key = `${itemType}:${folderName}`
  const task: UpdateDrawerTask = {
    itemType,
    folderName,
    displayName,
    initialLocalVersion: currentVersion || '',
    initialTargetVersion: latestVersion || '',
  }

  const index = updateDrawerTasks.value.findIndex(
    (entry) => `${entry.itemType}:${entry.folderName}` === key,
  )
  if (index >= 0) {
    updateDrawerTasks.value[index] = task
  } else {
    updateDrawerTasks.value.push(task)
  }
  updateDrawerActiveKey.value = key
  showUpdateDrawer.value = true
}

function handleSelectUpdateTask(key: string) {
  if (!key) return
  updateDrawerActiveKey.value = key
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

        <!-- Selection mode toggle (aircraft/plugin tabs only) -->
        <button
          v-if="activeTab === 'aircraft' || activeTab === 'plugin'"
          class="px-3 py-2 rounded-lg transition-colors flex items-center justify-center"
          :class="
            selectionMode
              ? 'bg-blue-500 text-white hover:bg-blue-600'
              : 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-300 dark:hover:bg-gray-600'
          "
          :title="t('management.batchMode')"
          @click="toggleSelectionMode"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
            />
          </svg>
        </button>
      </div>

      <!-- Batch action bar (replaces statistics bar when in selection mode) -->
      <Transition name="bar-swap" mode="out-in">
        <div
          v-if="selectionMode"
          key="batch-bar"
          class="flex items-center gap-3 min-h-11 px-3 py-2 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800 mb-3 text-sm cursor-pointer"
          :title="t('management.selectAll')"
          @click="toggleSelectAll"
        >
          <!-- Master checkbox -->
          <button
            class="flex-shrink-0 w-4 h-4 rounded border-2 transition-all duration-150 flex items-center justify-center"
            :class="
              isAllSelected
                ? 'bg-blue-500 border-blue-500'
                : isIndeterminate
                  ? 'bg-blue-500 border-blue-500'
                  : 'border-blue-300 dark:border-blue-500 hover:border-blue-400'
            "
            :title="t('management.selectAll')"
            @click.stop="toggleSelectAll"
          >
            <svg
              v-if="isAllSelected"
              class="w-3 h-3 text-white"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M5 13l4 4L19 7"
              />
            </svg>
            <svg
              v-else-if="isIndeterminate"
              class="w-3 h-3 text-white"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M5 12h14"
              />
            </svg>
          </button>

          <span class="text-xs text-blue-600 dark:text-blue-300">
            {{ selectedCount > 0 ? t('management.selectedCount', { count: selectedCount }) : t('management.selectAll') }}
          </span>

          <div class="flex-1"></div>

          <!-- Batch buttons (only when items selected) -->
          <template v-if="selectedCount > 0">
            <button
              :disabled="isBatchProcessing"
              class="px-2.5 py-1 rounded text-xs font-medium transition-colors bg-green-500 text-white hover:bg-green-600 disabled:opacity-50"
              @click.stop="batchSetEnabled(true)"
            >
              <Transition name="text-fade" mode="out-in">
                <span :key="locale">{{ t('management.enableSelected') }}</span>
              </Transition>
            </button>
            <button
              :disabled="isBatchProcessing"
              class="px-2.5 py-1 rounded text-xs font-medium transition-colors bg-gray-400 dark:bg-gray-500 text-white hover:bg-gray-500 dark:hover:bg-gray-400 disabled:opacity-50"
              @click.stop="batchSetEnabled(false)"
            >
              <Transition name="text-fade" mode="out-in">
                <span :key="locale">{{ t('management.disableSelected') }}</span>
              </Transition>
            </button>
          </template>
        </div>

        <!-- Statistics bar (normal mode) -->
        <div
          v-else
          key="stats-bar"
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
        <div
          v-if="activeTab === 'aircraft' && skunkUpdatableAircraftCount > 0"
          class="flex items-center gap-2"
        >
          <button
            class="px-2.5 py-1 rounded text-xs font-medium transition-colors bg-sky-500 text-white hover:bg-sky-600 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1.5"
            :disabled="
              isUpdatingAllAircraft || managementStore.isCheckingUpdates || managementStore.isExecutingUpdate
            "
            @click="handleUpdateAllAircraft"
          >
            <svg
              v-if="isUpdatingAllAircraft"
              class="w-3 h-3 animate-spin"
              fill="none"
              viewBox="0 0 24 24"
            >
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
            <Transition name="text-fade" mode="out-in">
              <span :key="`${locale}-${isUpdatingAllAircraft}`">{{
                isUpdatingAllAircraft ? t('management.updateAllRunning') : t('management.updateAll')
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
      </Transition>

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
                  :selected="selectedAircraft.has(item.folderName)"
                  :show-checkbox="selectionMode"
                  :is-protected="isProtectedAircraft(item.displayName)"
                  @toggle-enabled="(fn) => handleToggleEnabled('aircraft', fn)"
                  @delete="(fn) => handleDelete('aircraft', fn)"
                  @open-folder="(fn) => handleOpenFolder('aircraft', fn)"
                  @view-liveries="handleViewLiveries"
                  @toggle-select="toggleSelect"
                  @update="(fn) => handleOpenUpdate('aircraft', fn, item.displayName, item.version, item.latestVersion)"
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
                  :selected="selectedPlugins.has(item.folderName)"
                  :show-checkbox="selectionMode"
                  @toggle-enabled="(fn) => handleToggleEnabled('plugin', fn)"
                  @delete="(fn) => handleDelete('plugin', fn)"
                  @open-folder="(fn) => handleOpenFolder('plugin', fn)"
                  @view-scripts="handleViewScripts"
                  @toggle-select="toggleSelect"
                  @update="(fn) => handleOpenUpdate('plugin', fn, item.displayName, item.version, item.latestVersion)"
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

    <AddonUpdateDrawer
      v-model:show="showUpdateDrawer"
      :tasks="updateDrawerTasks"
      :active-task-key="updateDrawerActiveKey"
      @select-task="handleSelectUpdateTask"
      @updated="loadTabData(activeTab)"
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

/* Bar swap transition between stats bar and batch action bar */
.bar-swap-enter-active,
.bar-swap-leave-active {
  transition: all 0.2s ease;
}

.bar-swap-enter-from {
  opacity: 0;
  transform: translateY(-4px);
}

.bar-swap-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
