import { defineStore } from 'pinia'
import { ref } from 'vue'
import { createXplaneScope } from '@/utils/xplaneContext'
import { save, open } from '@tauri-apps/plugin-dialog'
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
  luaTotal: number
  luaEnabled: number
}

export interface PresetApplyResult {
  changesMade: number
  errors: string[]
  missingItems: string[]
  lockState?: Partial<LockedItemsData> | null
}

function buildPresetLockState(lockState: LockedItemsData) {
  return {
    aircraft: lockState.aircraft,
    plugin: lockState.plugin,
    scenery: lockState.scenery,
    lua: lockState.lua,
  }
}

export const usePresetsStore = defineStore('presets', () => {
  const presets = ref<PresetSummary[]>([])
  const isLoading = ref(false)
  const isSaving = ref(false)
  const isApplying = ref(false)
  const updatingSnapshotId = ref<number | null>(null)

  const capture = createXplaneScope(() => {
    presets.value = []
    isLoading.value = isSaving.value = isApplying.value = false
    updatingSnapshotId.value = null
  })

  async function loadPresets() {
    const { path, invoke, isCurrent } = capture()
    if (!path) return
    isLoading.value = true
    try {
      presets.value = await invoke<PresetSummary[]>('list_presets')
    } catch (error) {
      if (isCurrent()) throw error
    } finally {
      if (isCurrent()) isLoading.value = false
    }
  }

  async function savePreset(name: string, description?: string) {
    const { path, invoke, isCurrent } = capture()
    isSaving.value = true
    try {
      const lockStore = useLockStore()
      await invoke('save_preset', {
        xplanePath: path || null,
        name,
        description,
        lockState: buildPresetLockState(lockStore.exportLockState(path)),
      })
      if (isCurrent()) await loadPresets()
    } finally {
      if (isCurrent()) isSaving.value = false
    }
  }

  async function updatePreset(
    presetId: number,
    name?: string,
    description?: string,
    updateSnapshot?: boolean,
  ) {
    const { path, invoke, isCurrent } = capture()
    const lockStore = useLockStore()
    const shouldUpdateSnapshot = updateSnapshot === true

    if (shouldUpdateSnapshot) {
      updatingSnapshotId.value = presetId
    }

    try {
      await invoke('update_preset', {
        presetId,
        xplanePath: path || null,
        name,
        description,
        updateSnapshot,
        lockState: shouldUpdateSnapshot
          ? buildPresetLockState(lockStore.exportLockState(path))
          : null,
      })
      if (isCurrent()) await loadPresets()
    } finally {
      if (isCurrent() && updatingSnapshotId.value === presetId) {
        updatingSnapshotId.value = null
      }
    }
  }

  async function deletePreset(presetId: number) {
    const { invoke, isCurrent } = capture()
    await invoke('delete_preset', { presetId })
    if (isCurrent()) await loadPresets()
  }

  async function applyPreset(presetId: number): Promise<PresetApplyResult> {
    const { path, invoke, isCurrent } = capture()
    isApplying.value = true
    try {
      const lockStore = useLockStore()
      const result = await invoke<PresetApplyResult>('apply_preset', {
        xplanePath: path,
        presetId,
      })
      if (result.lockState) {
        try {
          await lockStore.applyLockState(
            {
              ...result.lockState,
              navdata: lockStore.exportLockState(path).navdata,
            },
            path || undefined,
          )
        } catch (e) {
          result.errors.push(`Failed to apply lock state: ${String(e)}`)
        }
      }
      return result
    } finally {
      if (isCurrent()) isApplying.value = false
    }
  }

  function isUpdatingSnapshot(presetId: number): boolean {
    return updatingSnapshotId.value === presetId
  }

  async function exportPreset(presetId: number) {
    const { invoke } = capture()
    const filePath = await save({
      defaultPath: 'preset.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!filePath) return
    await invoke('export_preset', { presetId, exportPath: filePath })
  }

  async function importPreset() {
    const { invoke, isCurrent } = capture()
    const filePath = await open({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      multiple: false,
    })
    if (!filePath) return
    await invoke('import_preset', { importPath: filePath })
    if (isCurrent()) await loadPresets()
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
