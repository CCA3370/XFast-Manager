import { defineStore } from 'pinia'
import { ref } from 'vue'
import { createXplaneScope } from '@/utils/xplaneContext'

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

  const capture = createXplaneScope(() => {
    entries.value = []
    totalCount.value = 0
    hasMore.value = true
    isLoading.value = false
  })

  async function loadRecent() {
    const { path, invoke, isCurrent } = capture()
    if (!path) return
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
    } catch (error) {
      if (isCurrent()) throw error
    } finally {
      if (isCurrent()) isLoading.value = false
    }
  }

  async function loadMore() {
    const { path, invoke, isCurrent } = capture()
    if (!path) return
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
    } catch (error) {
      if (isCurrent()) throw error
    } finally {
      if (isCurrent()) isLoading.value = false
    }
  }

  async function clearLog(beforeDays?: number) {
    const { invoke, isCurrent } = capture()
    const deleted = await invoke<number>('clear_activity_log', { beforeDays })
    if (isCurrent() && deleted > 0) {
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
