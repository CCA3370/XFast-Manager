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

export const useDiskUsageStore = defineStore('diskUsage', () => {
  const report = ref<DiskUsageReport | null>(null)
  const isScanning = ref(false)
  const error = ref<string | null>(null)
  const selectedItem = ref<FolderDiskUsage | null>(null)

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

  function reset() {
    report.value = null
    error.value = null
    selectedItem.value = null
  }

  return { report, isScanning, error, selectedItem, scan, scanFolder, reset }
})
