import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SceneryManagerData, SceneryManagerEntry, SceneryCategory } from '@/types'
import { useAppStore } from './app'

export const useSceneryStore = defineStore('scenery', () => {
  const appStore = useAppStore()

  // State
  const data = ref<SceneryManagerData | null>(null)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)

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
      Orthophotos: [],
      Mesh: []
    }

    for (const entry of sortedEntries.value) {
      groups[entry.category].push(entry)
    }

    return groups
  })

  // Check if there are unsaved changes (either local changes or index differs from ini)
  const hasChanges = computed(() => {
    // If index differs from ini, we have changes to apply
    if (data.value?.needsSync) return true

    if (!data.value || originalEntries.value.length === 0) return false

    const current = entries.value
    if (current.length !== originalEntries.value.length) return true

    for (let i = 0; i < current.length; i++) {
      const curr = current[i]
      const orig = originalEntries.value.find(e => e.folderName === curr.folderName)
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
      error.value = 'X-Plane path not set'
      return
    }

    isLoading.value = true
    error.value = null

    try {
      const result = await invoke<SceneryManagerData>('get_scenery_manager_data', {
        xplanePath: appStore.xplanePath
      })
      data.value = result
      // Store original state for change detection
      originalEntries.value = JSON.parse(JSON.stringify(result.entries))
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load scenery data:', e)
    } finally {
      isLoading.value = false
    }
  }

  // Toggle enabled state for an entry
  async function toggleEnabled(folderName: string) {
    if (!data.value) return

    const entry = data.value.entries.find(e => e.folderName === folderName)
    if (!entry) return

    try {
      // Update locally first for immediate UI feedback
      entry.enabled = !entry.enabled

      // Update enabled count
      data.value.enabledCount = data.value.entries.filter(e => e.enabled).length

      // Update in backend
      await invoke('update_scenery_entry', {
        xplanePath: appStore.xplanePath,
        folderName,
        enabled: entry.enabled,
        sortOrder: null,
        category: null
      })
    } catch (e) {
      // Revert on error
      entry.enabled = !entry.enabled
      data.value.enabledCount = data.value.entries.filter(e => e.enabled).length
      error.value = String(e)
      console.error('Failed to toggle enabled:', e)
    }
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
      console.error('Failed to update category:', e)
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

  async function persistPendingSortOrder() {
    if (!data.value || !appStore.xplanePath) return

    // Ensure sortOrder fields are aligned with current order
    data.value.entries = data.value.entries
      .sort((a, b) => a.sortOrder - b.sortOrder)
      .map((entry, index) => ({
        ...entry,
        sortOrder: index
      }))

    for (const entry of data.value.entries) {
      const original = originalEntries.value.find(e => e.folderName === entry.folderName)
      if (!original || original.sortOrder !== entry.sortOrder) {
        await invoke('update_scenery_entry', {
          xplanePath: appStore.xplanePath,
          folderName: entry.folderName,
          enabled: null,
          sortOrder: entry.sortOrder
        })
      }
    }
  }

  // Apply changes to scenery_packs.ini
  async function applyChanges() {
    if (!appStore.xplanePath) {
      error.value = 'X-Plane path not set'
      return
    }

    isSaving.value = true
    error.value = null

    try {
      // Persist any staged sort order changes before applying
      await persistPendingSortOrder()

      await invoke('apply_scenery_changes', {
        xplanePath: appStore.xplanePath
      })

      // Update original state after successful save
      if (data.value) {
        originalEntries.value = JSON.parse(JSON.stringify(data.value.entries))
        // Mark as synced since we just wrote to ini
        data.value.needsSync = false
      }
    } catch (e) {
      error.value = String(e)
      console.error('Failed to apply changes:', e)
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

    // Actions
    loadData,
    toggleEnabled,
    updateCategory,
    moveEntry,
    reorderEntries,
    applyChanges,
    resetChanges,
    clear
  }
})
