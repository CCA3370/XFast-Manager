import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  SceneryIndexStatus,
  SceneryManagerData,
  SceneryManagerEntry,
  SceneryCategory,
} from '@/types'
import { parseApiError, getErrorMessage } from '@/types'
import { useAppStore } from './app'
import { logError } from '@/services/logger'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'

export const useSceneryStore = defineStore('scenery', () => {
  const appStore = useAppStore()

  // State
  const data = ref<SceneryManagerData | null>(null)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)
  const indexExists = ref(false)
  const needsDatabaseReset = ref(false)

  // Track original state for change detection
  const originalEntries = ref<SceneryManagerEntry[]>([])

  // Collapsed groups state (persisted to Tauri Store)
  // Default: all groups are expanded (false = expanded, true = collapsed)
  const collapsedGroups = ref<Record<SceneryCategory, boolean>>(
    {} as Record<SceneryCategory, boolean>,
  )

  // Load collapsed groups from storage
  async function initStore(): Promise<void> {
    const saved = await getItem<Record<SceneryCategory, boolean>>(
      STORAGE_KEYS.SCENERY_GROUPS_COLLAPSED,
    )
    if (saved && typeof saved === 'object') {
      collapsedGroups.value = saved
    }
  }

  // Watch for changes and persist to Tauri Store
  watch(
    collapsedGroups,
    (newVal) => {
      setItem(STORAGE_KEYS.SCENERY_GROUPS_COLLAPSED, newVal)
    },
    { deep: true },
  )

  // Computed properties
  const entries = computed(() => data.value?.entries ?? [])
  const totalCount = computed(() => data.value?.totalCount ?? 0)
  const enabledCount = computed(() => data.value?.enabledCount ?? 0)
  const missingDepsCount = computed(() => data.value?.missingDepsCount ?? 0)
  const duplicateTilesCount = computed(() => data.value?.duplicateTilesCount ?? 0)
  const duplicateAirportsCount = computed(() => data.value?.duplicateAirportsCount ?? 0)
  const duplicatesCount = computed(() => {
    if (!data.value) return 0
    return data.value.entries.filter(
      (e) => e.duplicateTiles?.length > 0 || e.duplicateAirports?.length > 0,
    ).length
  })

  // Sort entries by sortOrder
  const sortedEntries = computed(() => {
    return [...entries.value].sort((a, b) => a.sortOrder - b.sortOrder)
  })

  // Group entries by category
  const groupedEntries = computed(() => {
    const groups: Record<SceneryCategory, SceneryManagerEntry[]> = {
      FixedHighPriority: [],
      Airport: [],
      DefaultAirport: [],
      Library: [],
      Other: [],
      Overlay: [],
      AirportMesh: [],
      Mesh: [],
      Unrecognized: [],
    }

    for (const entry of sortedEntries.value) {
      groups[entry.category].push(entry)
    }

    return groups
  })

  // Check if there are unsaved changes (either local changes or index differs from ini)
  // Optimized with Map for O(1) lookups instead of O(n) .find() in loop
  const hasChanges = computed(() => {
    // If index differs from ini, we have changes to apply
    if (data.value?.needsSync) return true

    return hasLocalChanges.value
  })

  // Check if user has made local modifications (separate from needsSync)
  const hasLocalChanges = computed(() => {
    if (!data.value || originalEntries.value.length === 0) return false

    const current = entries.value
    if (current.length !== originalEntries.value.length) return true

    // Build a Map from original entries for O(1) lookup
    const originalMap = new Map(originalEntries.value.map((e) => [e.folderName, e]))

    for (const curr of current) {
      const orig = originalMap.get(curr.folderName)
      if (!orig) return true
      if (curr.enabled !== orig.enabled || curr.sortOrder !== orig.sortOrder) {
        return true
      }
    }

    return false
  })

  // Load scenery data from backend
  async function loadData() {
    if (!appStore.xplanePath) {
      indexExists.value = false
      error.value = 'X-Plane path not set'
      return
    }

    // Set isLoading before any async operations
    isLoading.value = true
    error.value = null

    try {
      // Load index status first
      await loadIndexStatus()

      const result = await invoke<SceneryManagerData>('get_scenery_manager_data', {
        xplanePath: appStore.xplanePath,
      })
      data.value = result
      // Store original state for change detection
      originalEntries.value = JSON.parse(JSON.stringify(result.entries))
      // Clear any previous database reset flag on successful load
      needsDatabaseReset.value = false
    } catch (e) {
      const errorStr = String(e)
      error.value = errorStr
      logError(`Failed to load scenery data: ${e}`, 'scenery')

      // Check if this is a migration error (newer database version)
      if (errorStr.includes('migration_failed') && errorStr.includes('newer than supported')) {
        // Mark that we need a database reset
        needsDatabaseReset.value = true
      }
    } finally {
      isLoading.value = false
    }
  }

  async function loadIndexStatus() {
    if (!appStore.xplanePath) {
      indexExists.value = false
      return
    }

    try {
      const status = await invoke<SceneryIndexStatus>('get_scenery_index_status', {
        xplanePath: appStore.xplanePath,
      })
      indexExists.value = status.indexExists
    } catch (e) {
      indexExists.value = false
      logError(`Failed to load scenery index status: ${e}`, 'scenery')
    }
  }

  // Reset the database (delete it) and reload
  async function resetDatabase() {
    try {
      await invoke<boolean>('reset_scenery_database')
      needsDatabaseReset.value = false
      error.value = null
      // After reset, the user needs to rebuild the index
      indexExists.value = false
      data.value = null
      originalEntries.value = []
      return true
    } catch (e) {
      logError(`Failed to reset database: ${e}`, 'scenery')
      error.value = String(e)
      return false
    }
  }

  // Toggle enabled state for an entry (local only, no backend write)
  function toggleEnabled(folderName: string) {
    if (!data.value) return

    const entry = data.value.entries.find((e) => e.folderName === folderName)
    if (!entry) return

    // Update locally only - will be persisted when user clicks Apply
    entry.enabled = !entry.enabled

    // Update enabled count
    data.value.enabledCount = data.value.entries.filter((e) => e.enabled).length
  }

  // Update category for an entry
  async function updateCategory(folderName: string, newCategory: SceneryCategory) {
    if (!data.value) return

    const entry = data.value.entries.find((e) => e.folderName === folderName)
    if (!entry) return

    const oldCategory = entry.category

    try {
      // Update locally first for immediate UI feedback
      entry.category = newCategory

      // Update in backend
      await invoke('update_scenery_entry', {
        xplanePath: appStore.xplanePath,
        folderName,
        enabled: null,
        sortOrder: null,
        category: newCategory,
      })
    } catch (e) {
      // Revert on error
      entry.category = oldCategory
      error.value = String(e)
      logError(`Failed to update category: ${e}`, 'scenery')
      throw e
    }
  }

  // Recalculate duplicate tiles based on raw tile overlaps and current sort order.
  // This enables real-time conflict display when entries are reordered.
  // Creates new entry objects for changed entries to force Vue reactivity propagation
  // through computed properties (groupedEntries) into the template.
  let recalcTimer: ReturnType<typeof setTimeout> | null = null

  function recalcDuplicateTiles() {
    if (!data.value) return
    const overlaps = data.value.tileOverlaps
    if (!overlaps || Object.keys(overlaps).length === 0) return

    const entryMap = new Map(data.value.entries.map((e) => [e.folderName, e]))
    let anyChanged = false

    const newEntries = data.value.entries.map((entry) => {
      let newTiles: string[]

      if (entry.folderName.startsWith('XPME_')) {
        newTiles = []
      } else {
        const rawOverlaps = overlaps[entry.folderName]
        if (!rawOverlaps || rawOverlaps.length === 0) {
          newTiles = []
        } else {
          newTiles = rawOverlaps.filter((other) => {
            const otherEntry = entryMap.get(other)
            if (!otherEntry) return false
            if (other.startsWith('XPME_')) {
              return entry.sortOrder > otherEntry.sortOrder
            }
            return true
          })
        }
      }

      // Only create a new object when duplicateTiles actually differs
      // Note: duplicateTiles may be undefined — backend omits empty arrays via skip_serializing_if
      const prev = entry.duplicateTiles ?? []
      if (prev.length !== newTiles.length || prev.some((v, i) => v !== newTiles[i])) {
        anyChanged = true
        return { ...entry, duplicateTiles: newTiles }
      }
      return entry
    })

    if (anyChanged) {
      data.value.entries = newEntries
      data.value.duplicateTilesCount = newEntries.filter(
        (e) => (e.duplicateTiles ?? []).length > 0,
      ).length
    }
  }

  // Debounced, deferred recalculation — yields to the UI thread first
  function scheduleRecalcDuplicateTiles() {
    if (recalcTimer !== null) {
      clearTimeout(recalcTimer)
    }
    recalcTimer = setTimeout(() => {
      recalcTimer = null
      recalcDuplicateTiles()
    }, 0)
  }

  // Apply a local sort order without persisting immediately.
  // Mutates sortOrder in-place to avoid creating new objects and triggering
  // a full re-render of the entire list.
  // Runs recalcDuplicateTiles synchronously so that callers (e.g. handleDragEnd)
  // get entries with up-to-date duplicateTiles before syncLocalEntries copies them.
  function applyLocalOrder(newOrder: SceneryManagerEntry[]) {
    if (!data.value) return
    for (let i = 0; i < newOrder.length; i++) {
      newOrder[i].sortOrder = i
    }
    data.value.entries = newOrder
    recalcDuplicateTiles()
  }

  // Move an entry locally to a new position (no persistence until apply)
  async function moveEntry(folderName: string, newSortOrder: number) {
    if (!data.value) return

    const ordered = [...sortedEntries.value]
    const currentIndex = ordered.findIndex((e) => e.folderName === folderName)
    if (currentIndex === -1) return

    const targetIndex = Math.min(Math.max(newSortOrder, 0), ordered.length - 1)
    const [moved] = ordered.splice(currentIndex, 1)
    ordered.splice(targetIndex, 0, moved)
    applyLocalOrder(ordered)
  }

  // Reorder entries after drag-and-drop (staged locally)
  async function reorderEntries(newOrder: SceneryManagerEntry[]) {
    applyLocalOrder(newOrder)
  }

  // Apply changes to scenery_packs.ini
  async function applyChanges() {
    // Prevent concurrent calls (race condition protection)
    if (isSaving.value) return

    if (!appStore.xplanePath || !data.value) {
      error.value = 'X-Plane path not set'
      return
    }

    isSaving.value = true
    error.value = null

    try {
      // Ensure sortOrder fields are aligned with current order
      const normalizedEntries = data.value.entries
        .sort((a, b) => a.sortOrder - b.sortOrder)
        .map((entry, index) => ({
          ...entry,
          sortOrder: index,
        }))

      // Update local data with normalized sortOrder
      data.value.entries = normalizedEntries

      // Send only necessary fields to backend for batch update
      const updates = normalizedEntries.map((entry) => ({
        folderName: entry.folderName,
        enabled: entry.enabled,
        sortOrder: entry.sortOrder,
      }))

      await invoke('apply_scenery_changes', {
        xplanePath: appStore.xplanePath,
        entries: updates,
      })

      // Update original state after successful save
      originalEntries.value = JSON.parse(JSON.stringify(normalizedEntries))
      // Mark as synced since we just wrote to ini
      data.value.needsSync = false
    } catch (e) {
      error.value = String(e)
      logError(`Failed to apply changes: ${e}`, 'scenery')
      throw e
    } finally {
      isSaving.value = false
    }
  }

  // Reset to original state
  function resetChanges() {
    if (originalEntries.value.length > 0 && data.value) {
      data.value.entries = JSON.parse(JSON.stringify(originalEntries.value))
      data.value.enabledCount = data.value.entries.filter((e) => e.enabled).length
    }
  }

  // Delete a scenery entry (folder)
  async function deleteEntry(folderName: string) {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      throw new Error(error.value)
    }

    try {
      await invoke('delete_scenery_folder', {
        xplanePath: appStore.xplanePath,
        folderName,
      })

      // Remove from local data
      if (data.value) {
        data.value.entries = data.value.entries.filter((e) => e.folderName !== folderName)

        // Recalculate sortOrder to eliminate gaps
        // Sort by current sortOrder first, then reassign consecutive values
        data.value.entries = data.value.entries
          .sort((a, b) => a.sortOrder - b.sortOrder)
          .map((entry, index) => ({
            ...entry,
            sortOrder: index,
          }))

        data.value.totalCount = data.value.entries.length
        data.value.enabledCount = data.value.entries.filter((e) => e.enabled).length
        data.value.missingDepsCount = data.value.entries.filter(
          (e) => e.missingLibraries.length > 0,
        ).length
        scheduleRecalcDuplicateTiles()
      }

      // Also remove from original entries and recalculate their sortOrder
      originalEntries.value = originalEntries.value
        .filter((e) => e.folderName !== folderName)
        .sort((a, b) => a.sortOrder - b.sortOrder)
        .map((entry, index) => ({
          ...entry,
          sortOrder: index,
        }))
    } catch (e) {
      // Parse structured error if available
      const apiError = parseApiError(e)
      if (apiError) {
        error.value = apiError.message
        logError(
          `Failed to delete scenery entry [${apiError.code}]: ${apiError.message}`,
          'scenery',
        )

        // Rethrow with structured error info for UI handling
        throw { ...apiError, isApiError: true }
      } else {
        error.value = getErrorMessage(e)
        logError(`Failed to delete scenery entry: ${error.value}`, 'scenery')
        throw e
      }
    }
  }

  // Clear store state
  function clear() {
    data.value = null
    originalEntries.value = []
    error.value = null
    needsDatabaseReset.value = false
  }

  return {
    // State
    data,
    isLoading,
    isSaving,
    error,
    collapsedGroups,
    needsDatabaseReset,

    // Computed
    entries,
    sortedEntries,
    groupedEntries,
    totalCount,
    enabledCount,
    missingDepsCount,
    duplicateTilesCount,
    duplicateAirportsCount,
    duplicatesCount,
    hasChanges,
    hasLocalChanges,
    indexExists,

    // Actions
    initStore,
    loadData,
    loadIndexStatus,
    resetDatabase,
    toggleEnabled,
    updateCategory,
    moveEntry,
    reorderEntries,
    applyChanges,
    resetChanges,
    deleteEntry,
    clear,
  }
})
