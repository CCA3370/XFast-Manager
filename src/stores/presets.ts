import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
import { useAppStore } from './app'

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
}

export const usePresetsStore = defineStore('presets', () => {
  const presets = ref<PresetSummary[]>([])
  const isLoading = ref(false)
  const isSaving = ref(false)
  const isApplying = ref(false)

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
      await invoke('save_preset', { xplanePath: appStore.xplanePath, name, description })
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
    await invoke('update_preset', { presetId, name, description, updateSnapshot })
    await loadPresets()
  }

  async function deletePreset(presetId: number) {
    await invoke('delete_preset', { presetId })
    await loadPresets()
  }

  async function applyPreset(presetId: number): Promise<PresetApplyResult> {
    isApplying.value = true
    try {
      const appStore = useAppStore()
      return await invoke<PresetApplyResult>('apply_preset', {
        xplanePath: appStore.xplanePath,
        presetId,
      })
    } finally {
      isApplying.value = false
    }
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
    loadPresets,
    savePreset,
    updatePreset,
    deletePreset,
    applyPreset,
    exportPreset,
    importPreset,
  }
})
