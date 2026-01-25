import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SceneryIndexStatus, SceneryManagerData, SceneryManagerEntry, SceneryCategory } from '@/types'
import { parseApiError, getErrorMessage } from '@/types'
import { useAppStore } from './app'
import { logError } from '@/services/logger'

export const useSceneryStore = defineStore('scenery', () => {
  const appStore = useAppStore()

  // State
  const data = ref<SceneryManagerData | null>(null)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)
  const indexExists = ref(false)

  // Track original state for change detection
  const originalEntries = ref<SceneryManagerEntry[]>([])

  // Collapsed groups state (persisted to localStorage)
  // Default: all groups are expanded (false = expanded, true = collapsed)
  const collapsedGroups = ref<Record<SceneryCategory, boolean>>(
    JSON.parse(localStorage.getItem('sceneryGroupsCollapsed') || '{}')
  )

  // Watch for changes and persist to localStorage
  watch(collapsedGroups, (newVal) => {
    localStorage.setItem('sceneryGroupsCollapsed', JSON.stringify(newVal))
  }, { deep: true })

  // Computed properties
  const entries = computed(() => data.value?.entries ?? [])
  const totalCount = computed(() => data.value?.totalCount ?? 0)
  const enabledCount = computed(() => data.value?.enabledCount ?? 0)
  const missingDepsCount = computed(() => data.value?.missingDepsCount ?? 0)

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
      Mesh: []
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

    if (!data.value || originalEntries.value.length === 0) return false

    const current = entries.value
    if (current.length !== originalEntries.value.length) return true

    // Build a Map from original entries for O(1) lookup
    const originalMap = new Map(
      originalEntries.value.map(e => [e.folderName, e])
    )

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
        xplanePath: appStore.xplanePath
      })
      data.value = result
      // Store original state for change detection
      originalEntries.value = JSON.parse(JSON.stringify(result.entries))
    } catch (e) {
      error.value = String(e)
      logError(`Failed to load scenery data: ${e}`, 'scenery')
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
        xplanePath: appStore.xplanePath
      })
      indexExists.value = status.indexExists
    } catch (e) {
      indexExists.value = false
      logError(`Failed to load scenery index status: ${e}`, 'scenery')
    }
  }

  // Toggle enabled state for an entry (local only, no backend write)
  function toggleEnabled(folderName: string) {
    if (!data.value) return

    const entry = data.value.entries.find(e => e.folderName === folderName)
    if (!entry) return

    // Update locally only - will be persisted when user clicks Apply
    entry.enabled = !entry.enabled

    // Update enabled count
    data.value.enabledCount = data.value.entries.filter(e => e.enabled).length
  }

  // Update category for an entry
  async function updateCategory(folderName: string, newCategory: SceneryCategory) {
    if (!data.value) return

    const entry = data.value.entries.find(e => e.folderName === folderName)
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
        category: newCategory
      })
    } catch (e) {
      // Revert on error
      entry.category = oldCategory
      error.value = String(e)
      logError(`Failed to update category: ${e}`, 'scenery')
      throw e
    }
  }

  // Apply a local sort order without persisting immediately
  function applyLocalOrder(newOrder: SceneryManagerEntry[]) {
    if (!data.value) return
    data.value.entries = newOrder.map((entry, index) => ({
      ...entry,
      sortOrder: index
    }))
  }

  // Move an entry locally to a new position (no persistence until apply)
  async function moveEntry(folderName: string, newSortOrder: number) {
    if (!data.value) return

    const ordered = [...sortedEntries.value]
    const currentIndex = ordered.findIndex(e => e.folderName === folderName)
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
          sortOrder: index
        }))

      // Update local data with normalized sortOrder
      data.value.entries = normalizedEntries

      // Send only necessary fields to backend for batch update
      const updates = normalizedEntries.map(entry => ({
        folderName: entry.folderName,
        enabled: entry.enabled,
        sortOrder: entry.sortOrder
      }))

      await invoke('apply_scenery_changes', {
        xplanePath: appStore.xplanePath,
        entries: updates
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
      data.value.enabledCount = data.value.entries.filter(e => e.enabled).length
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
        folderName
      })

      // Remove from local data
      if (data.value) {
        data.value.entries = data.value.entries.filter(e => e.folderName !== folderName)

        // Recalculate sortOrder to eliminate gaps
        // Sort by current sortOrder first, then reassign consecutive values
        data.value.entries = data.value.entries
          .sort((a, b) => a.sortOrder - b.sortOrder)
          .map((entry, index) => ({
            ...entry,
            sortOrder: index
          }))

        data.value.totalCount = data.value.entries.length
        data.value.enabledCount = data.value.entries.filter(e => e.enabled).length
        data.value.missingDepsCount = data.value.entries.filter(e => e.missingLibraries.length > 0).length
      }

      // Also remove from original entries and recalculate their sortOrder
      originalEntries.value = originalEntries.value
        .filter(e => e.folderName !== folderName)
        .sort((a, b) => a.sortOrder - b.sortOrder)
        .map((entry, index) => ({
          ...entry,
          sortOrder: index
        }))
    } catch (e) {
      // Parse structured error if available
      const apiError = parseApiError(e)
      if (apiError) {
        error.value = apiError.message
        logError(`Failed to delete scenery entry [${apiError.code}]: ${apiError.message}`, 'scenery')

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
  }

  return {
    // State
    data,
    isLoading,
    isSaving,
    error,
    collapsedGroups,

    // Computed
    entries,
    sortedEntries,
    groupedEntries,
    totalCount,
    enabledCount,
    missingDepsCount,
    hasChanges,
    indexExists,

    // Actions
    loadData,
    loadIndexStatus,
    toggleEnabled,
    updateCategory,
    moveEntry,
    reorderEntries,
    applyChanges,
    resetChanges,
    deleteEntry,
    clear
  }
})
