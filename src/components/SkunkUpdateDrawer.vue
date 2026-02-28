<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useManagementStore } from '@/stores/management'
import { useToastStore } from '@/stores/toast'
import type { SkunkUpdatableItemType, SkunkUpdatePlan } from '@/types'

const props = defineProps<{
  show: boolean
  itemType: SkunkUpdatableItemType
  folderName: string
  displayName: string
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'updated'): void
}>()

const { t } = useI18n()
const managementStore = useManagementStore()
const toast = useToastStore()

const plan = ref<SkunkUpdatePlan | null>(null)
const planError = ref('')

const loadingPlan = computed(() => managementStore.isBuildingUpdatePlan)
const executingUpdate = computed(() => managementStore.isExecutingUpdate)
const options = computed(() => managementStore.skunkUpdateOptions)

const addPreview = computed(() => (plan.value?.addFiles ?? []).slice(0, 6))
const replacePreview = computed(() => (plan.value?.replaceFiles ?? []).slice(0, 6))
const deletePreview = computed(() => (plan.value?.deleteFiles ?? []).slice(0, 6))
const warningPreview = computed(() => plan.value?.warnings ?? [])

const canRunUpdate = computed(() => {
  if (!plan.value) return false
  if (plan.value.remoteLocked) return false
  if (executingUpdate.value) return false
  return (
    plan.value.addFiles.length > 0 ||
    plan.value.replaceFiles.length > 0 ||
    plan.value.deleteFiles.length > 0
  )
})

function closeDrawer() {
  emit('update:show', false)
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let idx = 0
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024
    idx++
  }
  return `${value.toFixed(value >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`
}

async function reloadPlan() {
  if (!props.show || !props.folderName) return
  planError.value = ''
  try {
    plan.value = await managementStore.buildSkunkUpdatePlan(props.itemType, props.folderName)
  } catch (e) {
    plan.value = null
    planError.value = String(e)
  }
}

async function setOption(next: Parameters<typeof managementStore.setSkunkUpdateOptions>[0]) {
  try {
    await managementStore.setSkunkUpdateOptions(next)
    await reloadPlan()
  } catch (e) {
    toast.error(String(e))
  }
}

async function runUpdate() {
  if (!canRunUpdate.value) return

  try {
    const result = await managementStore.executeSkunkUpdate(props.itemType, props.folderName)
    toast.success(
      t('management.updateSuccessSummary', {
        updated: result.updatedFiles,
        deleted: result.deletedFiles,
      }),
    )
    emit('updated')
    await reloadPlan()
  } catch (e) {
    toast.error(t('management.updateFailed') + ': ' + String(e))
  }
}

watch(
  () => props.show,
  async (open) => {
    if (!open) return
    await managementStore.loadSkunkUpdateOptions()
    await reloadPlan()
  },
)

watch(
  () => [props.itemType, props.folderName],
  async () => {
    if (props.show) {
      await reloadPlan()
    }
  },
)
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div v-if="show" class="fixed inset-0 z-[120]">
        <div class="absolute inset-0 bg-black/40" @click="closeDrawer" />

        <div
          class="absolute right-0 top-0 h-full w-full max-w-xl bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700 shadow-2xl overflow-hidden flex flex-col"
        >
          <div
            class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between"
          >
            <div class="min-w-0">
              <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">
                {{ t('management.updateDrawerTitle') }}
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 truncate">
                {{ displayName || folderName }}
              </p>
            </div>
            <button
              class="px-2 py-1 text-xs rounded bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600"
              @click="closeDrawer"
            >
              {{ t('common.close') }}
            </button>
          </div>

          <div class="flex-1 overflow-y-auto p-4 space-y-4">
            <div class="rounded-lg border border-gray-200 dark:border-gray-700 p-3 space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('management.useBeta') }}</span>
                <input
                  type="checkbox"
                  class="h-4 w-4"
                  :checked="options.useBeta"
                  @change="setOption({ useBeta: ($event.target as HTMLInputElement).checked })"
                />
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('management.includeLiveries') }}</span>
                <input
                  type="checkbox"
                  class="h-4 w-4"
                  :checked="options.includeLiveries"
                  @change="
                    setOption({
                      includeLiveries: ($event.target as HTMLInputElement).checked,
                    })
                  "
                />
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('management.applyBlacklist') }}</span>
                <input
                  type="checkbox"
                  class="h-4 w-4"
                  :checked="options.applyBlacklist"
                  @change="
                    setOption({
                      applyBlacklist: ($event.target as HTMLInputElement).checked,
                    })
                  "
                />
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('management.rollbackOnFailure') }}</span>
                <input
                  type="checkbox"
                  class="h-4 w-4"
                  :checked="options.rollbackOnFailure"
                  @change="
                    setOption({
                      rollbackOnFailure: ($event.target as HTMLInputElement).checked,
                    })
                  "
                />
              </div>
            </div>

            <div v-if="loadingPlan" class="text-xs text-gray-500 dark:text-gray-400">
              {{ t('management.updatePlanLoading') }}
            </div>

            <div
              v-else-if="planError"
              class="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 rounded p-2"
            >
              {{ t('management.updatePlanFailed') }}: {{ planError }}
            </div>

            <template v-else-if="plan">
              <div class="rounded-lg border border-gray-200 dark:border-gray-700 p-3 space-y-1">
                <div class="text-xs text-gray-500 dark:text-gray-400">
                  {{ t('management.versionInfo') }}:
                  {{ plan.localVersion || '-' }} -> {{ plan.remoteVersion || '-' }}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">
                  {{ t('management.estimatedDownload') }}: {{ formatBytes(plan.estimatedDownloadBytes) }}
                </div>
                <div
                  v-if="plan.remoteLocked"
                  class="text-xs text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-900/20 rounded p-2 mt-2"
                >
                  {{ t('management.remoteLocked') }}
                </div>
              </div>

              <div class="grid grid-cols-2 gap-2 text-xs">
                <div class="rounded bg-emerald-50 dark:bg-emerald-900/20 p-2 text-emerald-700 dark:text-emerald-300">
                  {{ t('management.filesToAdd') }}: {{ plan.addFiles.length }}
                </div>
                <div class="rounded bg-blue-50 dark:bg-blue-900/20 p-2 text-blue-700 dark:text-blue-300">
                  {{ t('management.filesToReplace') }}: {{ plan.replaceFiles.length }}
                </div>
                <div class="rounded bg-red-50 dark:bg-red-900/20 p-2 text-red-700 dark:text-red-300">
                  {{ t('management.filesToDelete') }}: {{ plan.deleteFiles.length }}
                </div>
                <div class="rounded bg-gray-100 dark:bg-gray-800 p-2 text-gray-600 dark:text-gray-300">
                  {{ t('management.filesToSkip') }}: {{ plan.skipFiles.length }}
                </div>
              </div>

              <div v-if="warningPreview.length > 0" class="space-y-1">
                <div class="text-xs font-medium text-amber-700 dark:text-amber-300">
                  {{ t('management.warnings') }}
                </div>
                <div
                  v-for="(w, idx) in warningPreview"
                  :key="idx"
                  class="text-xs text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-900/20 rounded p-2"
                >
                  {{ w }}
                </div>
              </div>

              <div v-if="addPreview.length > 0" class="space-y-1">
                <div class="text-xs font-medium text-emerald-700 dark:text-emerald-300">
                  {{ t('management.filesToAdd') }}
                </div>
                <div
                  v-for="path in addPreview"
                  :key="path"
                  class="text-[11px] font-mono text-emerald-700 dark:text-emerald-300 truncate"
                  :title="path"
                >
                  {{ path }}
                </div>
              </div>

              <div v-if="replacePreview.length > 0" class="space-y-1">
                <div class="text-xs font-medium text-blue-700 dark:text-blue-300">
                  {{ t('management.filesToReplace') }}
                </div>
                <div
                  v-for="path in replacePreview"
                  :key="path"
                  class="text-[11px] font-mono text-blue-700 dark:text-blue-300 truncate"
                  :title="path"
                >
                  {{ path }}
                </div>
              </div>

              <div v-if="deletePreview.length > 0" class="space-y-1">
                <div class="text-xs font-medium text-red-700 dark:text-red-300">
                  {{ t('management.filesToDelete') }}
                </div>
                <div
                  v-for="path in deletePreview"
                  :key="path"
                  class="text-[11px] font-mono text-red-700 dark:text-red-300 truncate"
                  :title="path"
                >
                  {{ path }}
                </div>
              </div>
            </template>
          </div>

          <div
            class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex items-center gap-2 justify-end"
          >
            <button
              class="px-3 py-1.5 rounded text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 disabled:opacity-60"
              :disabled="loadingPlan || executingUpdate"
              @click="reloadPlan"
            >
              {{ t('management.refreshPlan') }}
            </button>
            <button
              class="px-3 py-1.5 rounded text-xs text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-60"
              :disabled="!canRunUpdate"
              @click="runUpdate"
            >
              {{ executingUpdate ? t('management.updating') : t('management.startUpdate') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
