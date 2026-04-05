import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ActivityLogEntry {
  id: number
  timestamp: number
  operation: string
  itemType: string
  itemName: string
  details: string | null
  success: boolean
}

export interface ActivityLogPage {
  entries: ActivityLogEntry[]
  totalCount: number
}

export const useActivityLogStore = defineStore('activityLog', () => {
  const entries = ref<ActivityLogEntry[]>([])
  const totalCount = ref(0)
  const isLoading = ref(false)
  const hasMore = ref(true)
  const filterItemType = ref<string | null>(null)

  async function loadRecent() {
    isLoading.value = true
    try {
      const page = await invoke<ActivityLogPage>('get_activity_log', {
        limit: 50,
        offset: 0,
        itemType: filterItemType.value,
      })
      entries.value = page.entries
      totalCount.value = page.totalCount
      hasMore.value = entries.value.length < totalCount.value
    } finally {
      isLoading.value = false
    }
  }

  async function loadMore() {
    if (!hasMore.value || isLoading.value) return
    isLoading.value = true
    try {
      const page = await invoke<ActivityLogPage>('get_activity_log', {
        limit: 50,
        offset: entries.value.length,
        itemType: filterItemType.value,
      })
      entries.value.push(...page.entries)
      totalCount.value = page.totalCount
      hasMore.value = entries.value.length < totalCount.value
    } finally {
      isLoading.value = false
    }
  }

  async function clearLog(beforeDays?: number) {
    const deleted = await invoke<number>('clear_activity_log', { beforeDays })
    if (deleted > 0) {
      await loadRecent()
    }
    return deleted
  }

  return {
    entries,
    totalCount,
    isLoading,
    hasMore,
    filterItemType,
    loadRecent,
    loadMore,
    clearLog,
  }
})
