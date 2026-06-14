import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * Coordinates the aircraft-patch install modal. The modal itself (hosted in the
 * Home view, which owns the install machinery) reacts to `isModalOpen`.
 *
 * Patch install is a user-directed intent that bypasses signature detection, so
 * it can be opened from anywhere: the Home drop flow, the confirmation-modal
 * "this is actually a patch" escape hatch, or a management aircraft card.
 */
export const usePatchStore = defineStore('patch', () => {
  const isModalOpen = ref(false)
  /** Absolute path of the patch archive to install. */
  const archivePath = ref<string>('')
  /** When opened from a specific aircraft, the target is pre-selected. */
  const presetAircraftFolder = ref<string | null>(null)

  function open(archive: string, presetFolder?: string | null) {
    archivePath.value = archive
    presetAircraftFolder.value = presetFolder ?? null
    isModalOpen.value = true
  }

  function close() {
    isModalOpen.value = false
    archivePath.value = ''
    presetAircraftFolder.value = null
  }

  return { isModalOpen, archivePath, presetAircraftFolder, open, close }
})
