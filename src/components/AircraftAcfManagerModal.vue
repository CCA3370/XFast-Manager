<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div v-if="aircraft" class="fixed inset-0 z-[90] flex items-center justify-center px-4">
        <div
          class="absolute inset-0 bg-black/30 dark:bg-black/50 backdrop-blur-sm"
          @click="$emit('close')"
        ></div>
        <div
          class="relative w-full max-w-2xl bg-white dark:bg-gray-800 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 p-6"
        >
          <div class="flex items-start justify-between gap-4 mb-4">
            <div class="min-w-0">
              <h2 class="text-lg font-semibold text-gray-900 dark:text-white truncate">
                {{ $t('management.manageAcfFilesTitle', { name: aircraft.displayName }) }}
              </h2>
              <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
                {{ $t('management.manageAcfFilesHint') }}
              </p>
            </div>
            <button
              class="flex-shrink-0 p-2 rounded-lg text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              :title="$t('common.close')"
              @click="$emit('close')"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          <div
            v-if="aircraft.hasMixedAcfStates"
            class="mb-4 rounded-lg border border-amber-200 dark:border-amber-500/30 bg-amber-50 dark:bg-amber-500/10 px-3 py-2 text-sm text-amber-700 dark:text-amber-300"
          >
            {{ $t('management.partialAcfState') }}
          </div>

          <div class="space-y-2 max-h-[60vh] overflow-y-auto pr-1">
            <div
              v-for="file in aircraft.acfFiles"
              :key="file.fileName"
              class="flex items-center justify-between gap-3 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50/80 dark:bg-gray-900/40 px-3 py-3"
            >
              <div class="min-w-0">
                <div class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                  {{ getDisplayName(file.fileName) }}
                </div>
                <div class="mt-1">
                  <span
                    class="inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium"
                    :class="
                      file.enabled
                        ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300'
                        : 'bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-300'
                    "
                  >
                    {{ file.enabled ? $t('management.acfEnabled') : $t('management.acfDisabled') }}
                  </span>
                </div>
              </div>

              <div class="flex-shrink-0 relative">
                <ToggleSwitch
                  :model-value="file.enabled"
                  size="lg"
                  active-class="bg-blue-500"
                  inactive-class="bg-gray-300 dark:bg-gray-600"
                  :disabled="folderBusy"
                  :aria-label="file.enabled ? $t('contextMenu.disable') : $t('contextMenu.enable')"
                  @update:model-value="$emit('toggle', file.fileName)"
                />
                <span
                  v-if="busyFileSet.has(file.fileName)"
                  class="absolute inset-0 flex items-center justify-center pointer-events-none"
                >
                  <svg class="w-3 h-3 animate-spin text-white" fill="none" viewBox="0 0 24 24">
                    <circle
                      class="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      stroke-width="4"
                    ></circle>
                    <path
                      class="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    ></path>
                  </svg>
                </span>
              </div>
            </div>
          </div>

          <div class="flex justify-end mt-6">
            <button
              class="px-4 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              @click="$emit('close')"
            >
              {{ $t('common.close') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { AircraftInfo } from '@/types'
import ToggleSwitch from '@/components/ToggleSwitch.vue'

const props = withDefaults(
  defineProps<{
    aircraft: AircraftInfo | null
    busyFiles?: string[]
    folderBusy?: boolean
  }>(),
  {
    busyFiles: () => [],
    folderBusy: false,
  },
)

defineEmits<{
  close: []
  toggle: [fileName: string]
}>()

const busyFileSet = computed(() => new Set(props.busyFiles))

function getDisplayName(fileName: string) {
  return fileName.replace(/\.(acf|xfma)$/i, '')
}
</script>

<style scoped>
.modal-fade-enter-active {
  transition: opacity 0.15s ease;
}

.modal-fade-leave-active {
  transition: opacity 0.1s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}
</style>
