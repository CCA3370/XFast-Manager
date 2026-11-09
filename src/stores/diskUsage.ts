import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from './app'

export interface DiskUsageReport {
  totalBytes: number
  categories: CategoryDiskUsage[]
  scanDurationMs: number
}

export interface CategoryDiskUsage {
  category: string
  totalBytes: number
  itemCount: number
  items: ItemDiskUsage[]
}

export interface ItemDiskUsage {
  folderName: string
  displayName: string
  sizeBytes: number
  fileCount: number
  itemType: string
}

export interface FolderDiskUsage {
  folderName: string
  totalBytes: number
  fileCount: number
  largestFiles: { path: string; sizeBytes: number }[]
}

export type OutputCleanupLevel = 'recommended' | 'cleanable' | 'cautious' | 'unknown'
export type OutputCleanupSubmissionLevel = Exclude<OutputCleanupLevel, 'unknown'>

export interface OutputCleanupReport {
  version: number
  updated: string
  source: string
  totalBytes: number
  totalFiles: number
  items: OutputCleanupItem[]
  disabledItems: DisabledOutputItem[]
}

export interface DisabledOutputItem {
  id: string
  relativePath: string
  savedPath: string
  savedAt: number
  sizeBytes: number
  fileCount: number
}

export interface OutputCleanupItem {
  id: string
  relativePath: string
  folderName: string
  displayName: string
  description: string
  level: OutputCleanupLevel
  defaultSelected: boolean
  warningKey: string | null
  sizeBytes: number
  fileCount: number
  exists: boolean
  recognized: boolean
}

export interface OutputCleanupTarget {
  id: string
  relativePath: string
}

export type OutputCleanupStateChange =
  | { action: 'disable'; target: OutputCleanupTarget }
  | { action: 'restore'; id: string }

export interface OutputCleanupResult {
  totalDeletedBytes: number
  totalDeletedFiles: number
  items: OutputCleanupItemResult[]
}

export interface OutputCleanupItemResult {
  id: string
  relativePath: string
  deletedBytes: number
  deletedFiles: number
  failedPaths: string[]
}

export interface OutputCleanupSubmissionResult {
  issue_url: string
  issue_number: number
  issue_title: string
}

export const useDiskUsageStore = defineStore('diskUsage', () => {
  const report = ref<DiskUsageReport | null>(null)
  const isScanning = ref(false)
  const error = ref<string | null>(null)
  const selectedItem = ref<FolderDiskUsage | null>(null)
  const outputCleanupReport = ref<OutputCleanupReport | null>(null)
  const isCleanupScanning = ref(false)
  const isCleanupRefreshing = ref(false)
  const isCleaning = ref(false)
  const isChangingCleanupState = ref(false)
  const cleanupError = ref<string | null>(null)
  let cleanupScanToken = 0
  let cleanupPath = ''

  async function scan() {
    isScanning.value = true
    error.value = null
    try {
      const appStore = useAppStore()
      report.value = await invoke<DiskUsageReport>('scan_disk_usage', {
        xplanePath: appStore.xplanePath,
      })
    } catch (e) {
      error.value = String(e)
    } finally {
      isScanning.value = false
    }
  }

  async function scanFolder(itemType: string, folderName: string) {
    try {
      const appStore = useAppStore()
      selectedItem.value = await invoke<FolderDiskUsage>('scan_folder_disk_usage', {
        xplanePath: appStore.xplanePath,
        itemType,
        folderName,
      })
    } catch (e) {
      error.value = String(e)
    }
  }

  async function scanOutputCleanup(resetToEmbedded = false) {
    const { xplanePath } = useAppStore()
    const token = ++cleanupScanToken
    if (xplanePath !== cleanupPath) {
      cleanupPath = xplanePath
      outputCleanupReport.value = null
    }
    if (!xplanePath) {
      outputCleanupReport.value = null
      isCleanupScanning.value = false
      return
    }

    isCleanupScanning.value = true
    cleanupError.value = null
    try {
      if (resetToEmbedded) {
        await invoke('reset_output_cleanup_items_to_embedded')
      }
      const result = await invoke<OutputCleanupReport>('scan_output_cleanup_items', {
        xplanePath,
      })
      if (token === cleanupScanToken && xplanePath === useAppStore().xplanePath) {
        outputCleanupReport.value = result
      }
    } catch (e) {
      if (token === cleanupScanToken) cleanupError.value = String(e)
    } finally {
      if (token === cleanupScanToken) isCleanupScanning.value = false
    }
  }

  async function refreshOutputCleanupItems() {
    isCleanupRefreshing.value = true
    try {
      await invoke('refresh_output_cleanup_items')
      await scanOutputCleanup()
    } catch {
      // Remote data is optional; keep the embedded/current catalog on failure.
    } finally {
      isCleanupRefreshing.value = false
    }
  }

  async function cleanOutputItems(
    xplanePath: string,
    targets: OutputCleanupTarget[],
  ): Promise<OutputCleanupResult> {
    isCleaning.value = true
    cleanupError.value = null
    try {
      const result = await invoke<OutputCleanupResult>('clean_output_items', {
        xplanePath,
        targets,
      })
      return result
    } catch (e) {
      cleanupError.value = String(e)
      throw e
    } finally {
      isCleaning.value = false
    }
  }

  async function changeOutputCleanupItemState(
    xplanePath: string,
    change: OutputCleanupStateChange,
  ) {
    isChangingCleanupState.value = true
    try {
      await invoke('change_output_cleanup_item_state', { xplanePath, change })
    } finally {
      isChangingCleanupState.value = false
    }
  }

  async function submitUnknownOutputCleanupItem(payload: {
    folderName: string
    relativePath: string
    description: string
    expectedLevel: OutputCleanupSubmissionLevel
    sizeBytes: number
    fileCount: number
  }): Promise<OutputCleanupSubmissionResult> {
    return await invoke<OutputCleanupSubmissionResult>(
      'submit_unknown_output_cleanup_item',
      payload,
    )
  }

  function reset() {
    cleanupScanToken++
    report.value = null
    error.value = null
    selectedItem.value = null
    outputCleanupReport.value = null
    isCleanupScanning.value = false
    cleanupError.value = null
  }

  return {
    report,
    isScanning,
    error,
    selectedItem,
    outputCleanupReport,
    isCleanupScanning,
    isCleanupRefreshing,
    isCleaning,
    isChangingCleanupState,
    cleanupError,
    scan,
    scanFolder,
    scanOutputCleanup,
    refreshOutputCleanupItems,
    cleanOutputItems,
    changeOutputCleanupItemState,
    submitUnknownOutputCleanupItem,
    reset,
  }
})
