<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSceneryStore } from '@/stores/scenery'
import { useToastStore } from '@/stores/toast'
import SceneryEntryCard from './SceneryEntryCard.vue'
import draggable from 'vuedraggable'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()
const sceneryStore = useSceneryStore()
const toastStore = useToastStore()

const drag = ref(false)

// Local copy of entries for drag-and-drop
const localEntries = computed({
  get: () => sceneryStore.sortedEntries,
  set: (value) => {
    // Update will be handled by drag end
  }
})

onMounted(async () => {
  if (props.show) {
    await sceneryStore.loadData()
  }
})

async function handleToggleEnabled(folderName: string) {
  await sceneryStore.toggleEnabled(folderName)
}

async function handleMoveUp(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)
  if (index > 0) {
    await sceneryStore.moveEntry(folderName, index - 1)
  }
}

async function handleMoveDown(folderName: string) {
  const entries = sceneryStore.sortedEntries
  const index = entries.findIndex(e => e.folderName === folderName)
  if (index < entries.length - 1) {
    await sceneryStore.moveEntry(folderName, index + 1)
  }
}

async function handleDragEnd() {
  drag.value = false
  // Reorder entries based on new positions
  await sceneryStore.reorderEntries(localEntries.value)
}

async function handleApplyChanges() {
  try {
    await sceneryStore.applyChanges()
    toastStore.addToast({
      type: 'success',
      message: t('sceneryManager.changesApplied')
    })
  } catch (e) {
    toastStore.addToast({
      type: 'error',
      message: t('sceneryManager.applyFailed')
    })
  }
}

function handleClose() {
  if (sceneryStore.hasChanges) {
    if (confirm(t('sceneryManager.unsavedChangesWarning'))) {
      sceneryStore.resetChanges()
      emit('close')
    }
  } else {
    emit('close')
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50"
        @click.self="handleClose"
      >
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-4xl max-h-[90vh] flex flex-col">
          <!-- Header -->
          <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
            <div>
              <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
                {{ t('sceneryManager.title') }}
              </h2>
              <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                {{ t('sceneryManager.subtitle') }}
              </p>
            </div>
            <button
              @click="handleClose"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              <svg class="w-6 h-6 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Statistics bar -->
          <div class="flex items-center gap-6 px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
            <div class="flex items-center gap-2">
              <span class="text-sm text-gray-600 dark:text-gray-400">{{ t('sceneryManager.total') }}:</span>
              <span class="font-semibold text-gray-900 dark:text-gray-100">{{ sceneryStore.totalCount }}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-sm text-gray-600 dark:text-gray-400">{{ t('sceneryManager.enabled') }}:</span>
              <span class="font-semibold text-green-600 dark:text-green-400">{{ sceneryStore.enabledCount }}</span>
            </div>
            <div v-if="sceneryStore.missingDepsCount > 0" class="flex items-center gap-2">
              <span class="text-sm text-gray-600 dark:text-gray-400">{{ t('sceneryManager.missingDeps') }}:</span>
              <span class="font-semibold text-amber-600 dark:text-amber-400">{{ sceneryStore.missingDepsCount }}</span>
            </div>
            <div v-if="sceneryStore.hasChanges" class="ml-auto flex items-center gap-2 text-blue-600 dark:text-blue-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              <span class="text-sm font-medium">{{ t('sceneryManager.unsavedChanges') }}</span>
            </div>
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-y-auto p-6">
            <div v-if="sceneryStore.isLoading" class="flex items-center justify-center py-12">
              <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
            </div>

            <div v-else-if="sceneryStore.error" class="text-center py-12">
              <p class="text-red-600 dark:text-red-400">{{ sceneryStore.error }}</p>
            </div>

            <div v-else-if="sceneryStore.totalCount === 0" class="text-center py-12">
              <p class="text-gray-600 dark:text-gray-400">{{ t('sceneryManager.noScenery') }}</p>
            </div>

            <draggable
              v-else
              v-model="localEntries"
              item-key="folderName"
              handle=".drag-handle"
              @start="drag = true"
              @end="handleDragEnd"
              class="space-y-2"
            >
              <template #item="{ element, index }">
                <SceneryEntryCard
                  :entry="element"
                  :index="index"
                  :total-count="sceneryStore.totalCount"
                  @toggle-enabled="handleToggleEnabled"
                  @move-up="handleMoveUp"
                  @move-down="handleMoveDown"
                />
              </template>
            </draggable>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-end gap-3 p-6 border-t border-gray-200 dark:border-gray-700">
            <button
              @click="handleClose"
              class="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            >
              {{ t('common.close') }}
            </button>
            <button
              @click="handleApplyChanges"
              :disabled="!sceneryStore.hasChanges || sceneryStore.isSaving"
              class="px-4 py-2 rounded-lg bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
            >
              <svg v-if="sceneryStore.isSaving" class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ t('sceneryManager.applyChanges') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .bg-white,
.modal-leave-active .bg-white {
  transition: transform 0.2s ease;
}

.modal-enter-from .bg-white,
.modal-leave-to .bg-white {
  transform: scale(0.95);
}
</style>
