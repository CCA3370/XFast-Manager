import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SceneryManagerData, SceneryManagerEntry } from '@/types'
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

  // Computed properties
  const entries = computed(() => data.value?.entries ?? [])
  const totalCount = computed(() => data.value?.totalCount ?? 0)
  const enabledCount = computed(() => data.value?.enabledCount ?? 0)
  const missingDepsCount = computed(() => data.value?.missingDepsCount ?? 0)

  // Sort entries by sortOrder
  const sortedEntries = computed(() => {
    return [...entries.value].sort((a, b) => a.sortOrder - b.sortOrder)
  })

  // Check if there are unsaved changes
  const hasChanges = computed(() => {
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
        sortOrder: null
      })
    } catch (e) {
      // Revert on error
      entry.enabled = !entry.enabled
      data.value.enabledCount = data.value.entries.filter(e => e.enabled).length
      error.value = String(e)
      console.error('Failed to toggle enabled:', e)
    }
  }

  // Move an entry to a new position
  async function moveEntry(folderName: string, newSortOrder: number) {
    if (!data.value || !appStore.xplanePath) return

    try {
      await invoke('move_scenery_entry', {
        xplanePath: appStore.xplanePath,
        folderName,
        newSortOrder
      })

      // Reload data to get updated sort orders
      await loadData()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to move entry:', e)
    }
  }

  // Reorder entries after drag-and-drop (batch update)
  async function reorderEntries(newOrder: SceneryManagerEntry[]) {
    if (!data.value || !appStore.xplanePath) return

    try {
      // Update sort_order for each entry based on new position
      for (let i = 0; i < newOrder.length; i++) {
        const entry = newOrder[i]
        if (entry.sortOrder !== i) {
          await invoke('update_scenery_entry', {
            xplanePath: appStore.xplanePath,
            folderName: entry.folderName,
            enabled: null,
            sortOrder: i
          })
        }
      }

      // Reload data to get updated state
      await loadData()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to reorder entries:', e)
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
      await invoke('apply_scenery_changes', {
        xplanePath: appStore.xplanePath
      })

      // Update original state after successful save
      if (data.value) {
        originalEntries.value = JSON.parse(JSON.stringify(data.value.entries))
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

    // Computed
    entries,
    sortedEntries,
    totalCount,
    enabledCount,
    missingDepsCount,
    hasChanges,

    // Actions
    loadData,
    toggleEnabled,
    moveEntry,
    reorderEntries,
    applyChanges,
    resetChanges,
    clear
  }
})
