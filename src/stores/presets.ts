import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
import { useAppStore } from './app'
import { useLockStore, type LockedItemsData } from './lock'

export interface PresetSummary {
  id: number
  name: string
  description: string | null
  createdAt: number
  updatedAt: number
  addonCounts: PresetAddonCounts
}

export interface PresetAddonCounts {
  aircraftTotal: number
  aircraftEnabled: number
  pluginsTotal: number
  pluginsEnabled: number
  sceneryTotal: number
  sceneryEnabled: number
  navdataTotal: number
  navdataEnabled: number
}

export interface PresetApplyResult {
  changesMade: number
  errors: string[]
  missingItems: string[]
  lockState?: LockedItemsData | null
}

export const usePresetsStore = defineStore('presets', () => {
  const presets = ref<PresetSummary[]>([])
  const isLoading = ref(false)
  const isSaving = ref(false)
  const isApplying = ref(false)
  const updatingSnapshotId = ref<number | null>(null)

  async function loadPresets() {
    isLoading.value = true
    try {
      presets.value = await invoke<PresetSummary[]>('list_presets')
    } finally {
      isLoading.value = false
    }
  }

  async function savePreset(name: string, description?: string) {
    isSaving.value = true
    try {
      const appStore = useAppStore()
      const lockStore = useLockStore()
      await invoke('save_preset', {
        xplanePath: appStore.xplanePath || null,
        name,
        description,
        lockState: lockStore.exportLockState(),
      })
      await loadPresets()
    } finally {
      isSaving.value = false
    }
  }

  async function updatePreset(
    presetId: number,
    name?: string,
    description?: string,
    updateSnapshot?: boolean,
  ) {
    const appStore = useAppStore()
    const lockStore = useLockStore()
    const shouldUpdateSnapshot = updateSnapshot === true

    if (shouldUpdateSnapshot) {
      updatingSnapshotId.value = presetId
    }

    try {
      await invoke('update_preset', {
        presetId,
        xplanePath: appStore.xplanePath || null,
        name,
        description,
        updateSnapshot,
        lockState: shouldUpdateSnapshot ? lockStore.exportLockState() : null,
      })
      await loadPresets()
    } finally {
      if (updatingSnapshotId.value === presetId) {
        updatingSnapshotId.value = null
      }
    }
  }

  async function deletePreset(presetId: number) {
    await invoke('delete_preset', { presetId })
    await loadPresets()
  }

  async function applyPreset(presetId: number): Promise<PresetApplyResult> {
    isApplying.value = true
    try {
      const appStore = useAppStore()
      const lockStore = useLockStore()
      const result = await invoke<PresetApplyResult>('apply_preset', {
        xplanePath: appStore.xplanePath,
        presetId,
      })
      if (result.lockState) {
        try {
          await lockStore.applyLockState(result.lockState, appStore.xplanePath || undefined)
        } catch (e) {
          result.errors.push(`Failed to apply lock state: ${String(e)}`)
        }
      }
      return result
    } finally {
      isApplying.value = false
    }
  }

  function isUpdatingSnapshot(presetId: number): boolean {
    return updatingSnapshotId.value === presetId
  }

  async function exportPreset(presetId: number) {
    const filePath = await save({
      defaultPath: 'preset.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!filePath) return
    await invoke('export_preset', { presetId, exportPath: filePath })
  }

  async function importPreset() {
    const filePath = await open({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      multiple: false,
    })
    if (!filePath) return
    await invoke('import_preset', { importPath: filePath })
    await loadPresets()
  }

  return {
    presets,
    isLoading,
    isSaving,
    isApplying,
    updatingSnapshotId,
    loadPresets,
    savePreset,
    updatePreset,
    deletePreset,
    applyPreset,
    exportPreset,
    importPreset,
    isUpdatingSnapshot,
  }
})
