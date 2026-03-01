<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useManagementStore } from '@/stores/management'
import { useToastStore } from '@/stores/toast'
import ToggleSwitch from '@/components/ToggleSwitch.vue'
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

const minSheetHeight = 320
const maxSheetHeight = ref(560)
const sheetHeight = ref(460)
const isResizing = ref(false)
let dragStartY = 0
let dragStartHeight = 0

const progressPercent = ref(0)
const progressPhase = ref<'preparing' | 'downloading' | 'applying' | 'finishing' | 'completed'>(
  'preparing',
)
let progressTimer: number | null = null
let progressDoneTimer: number | null = null

const phaseLabelMap = {
  preparing: 'management.progressPreparing',
  downloading: 'management.progressDownloading',
  applying: 'management.progressApplying',
  finishing: 'management.progressFinishing',
  completed: 'management.progressCompleted',
} as const

const showProgress = computed(() => executingUpdate.value || progressPercent.value > 0)
const progressLabel = computed(() => t(phaseLabelMap[progressPhase.value]))

const hasPendingFileChanges = computed(() => {
  if (!plan.value) return false
  return (
    plan.value.addFiles.length > 0 ||
    plan.value.replaceFiles.length > 0 ||
    plan.value.deleteFiles.length > 0
  )
})

const canRunUpdate = computed(() => {
  if (!plan.value) return false
  if (plan.value.remoteLocked) return false
  if (executingUpdate.value) return false
  return hasPendingFileChanges.value
})

const isUpToDate = computed(() => {
  if (!plan.value) return false
  return !plan.value.remoteLocked && !hasPendingFileChanges.value
})

const currentVersion = computed(() => plan.value?.localVersion || '-')
const targetVersion = computed(() => plan.value?.remoteVersion || '-')

function clampSheetHeight(next: number): number {
  return Math.max(minSheetHeight, Math.min(next, maxSheetHeight.value))
}

function updateMaxSheetHeight() {
  maxSheetHeight.value = Math.max(minSheetHeight, Math.floor(window.innerHeight * 0.92))
  sheetHeight.value = clampSheetHeight(sheetHeight.value)
}

function resetSheetHeight() {
  updateMaxSheetHeight()
  sheetHeight.value = clampSheetHeight(Math.round(window.innerHeight * 0.58))
}

function onResizeMove(event: PointerEvent) {
  if (!isResizing.value) return
  const delta = dragStartY - event.clientY
  sheetHeight.value = clampSheetHeight(dragStartHeight + delta)
}

function stopResize() {
  if (!isResizing.value) return
  isResizing.value = false
  window.removeEventListener('pointermove', onResizeMove)
  window.removeEventListener('pointerup', stopResize)
}

function onResizeStart(event: PointerEvent) {
  if (!props.show) return
  isResizing.value = true
  dragStartY = event.clientY
  dragStartHeight = sheetHeight.value
  window.addEventListener('pointermove', onResizeMove)
  window.addEventListener('pointerup', stopResize)
}

function clearProgressTimers() {
  if (progressTimer !== null) {
    window.clearInterval(progressTimer)
    progressTimer = null
  }
  if (progressDoneTimer !== null) {
    window.clearTimeout(progressDoneTimer)
    progressDoneTimer = null
  }
}

function resetProgress() {
  clearProgressTimers()
  progressPercent.value = 0
  progressPhase.value = 'preparing'
}

function startProgress() {
  clearProgressTimers()
  progressPhase.value = 'preparing'
  progressPercent.value = 6

  progressTimer = window.setInterval(() => {
    if (progressPercent.value < 18) {
      progressPhase.value = 'preparing'
      progressPercent.value = Math.min(progressPercent.value + 2, 18)
      return
    }
    if (progressPercent.value < 80) {
      progressPhase.value = 'downloading'
      progressPercent.value = Math.min(progressPercent.value + 1.6, 80)
      return
    }
    progressPhase.value = 'applying'
    progressPercent.value = Math.min(progressPercent.value + 0.7, 94)
  }, 180)
}

function completeProgress() {
  clearProgressTimers()
  progressPhase.value = 'completed'
  progressPercent.value = 100
  progressDoneTimer = window.setTimeout(() => {
    progressDoneTimer = null
    if (!executingUpdate.value) {
      resetProgress()
    }
  }, 1300)
}

function failProgress() {
  clearProgressTimers()
  progressPercent.value = 0
  progressPhase.value = 'preparing'
}

function closeDrawer() {
  if (executingUpdate.value) return
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

async function setParallelDownloads(event: Event) {
  const value = Number((event.target as HTMLSelectElement).value)
  if (!Number.isFinite(value)) return
  await setOption({ parallelDownloads: Math.min(Math.max(Math.round(value), 1), 8) })
}

async function runUpdate() {
  if (!canRunUpdate.value) return

  startProgress()
  try {
    const result = await managementStore.executeSkunkUpdate(props.itemType, props.folderName)
    progressPhase.value = 'finishing'
    completeProgress()
    toast.success(
      t('management.updateSuccessSummary', {
        updated: result.updatedFiles,
        deleted: result.deletedFiles,
      }),
    )
    emit('updated')
    await reloadPlan()
  } catch (e) {
    failProgress()
    toast.error(t('management.updateFailed') + ': ' + String(e))
  }
}

onMounted(() => {
  updateMaxSheetHeight()
  window.addEventListener('resize', updateMaxSheetHeight)
})

onBeforeUnmount(() => {
  stopResize()
  resetProgress()
  window.removeEventListener('resize', updateMaxSheetHeight)
})

watch(
  () => props.show,
  async (open) => {
    if (!open) {
      stopResize()
      resetProgress()
      return
    }
    resetSheetHeight()
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
        <div class="absolute inset-0 bg-black/35 backdrop-blur-[1px]" @click="closeDrawer" />

        <div class="absolute inset-x-0 bottom-0 px-2 pb-2 sm:px-4 sm:pb-4 pointer-events-none">
          <div
            class="mx-auto w-full max-w-4xl pointer-events-auto rounded-2xl border border-slate-200/80 dark:border-slate-700/80 bg-white/95 dark:bg-slate-900/95 shadow-2xl overflow-hidden flex flex-col"
            :style="{ height: `${sheetHeight}px` }"
          >
            <div class="shrink-0">
              <div class="h-5 flex items-center justify-center">
                <button
                  class="h-2.5 w-16 rounded-full bg-slate-300/80 hover:bg-slate-400/80 dark:bg-slate-600/80 dark:hover:bg-slate-500/80 cursor-ns-resize transition-colors"
                  :title="t('management.updateDrawerTitle')"
                  @pointerdown.prevent="onResizeStart"
                />
              </div>

              <div
                class="px-4 pb-3 border-b border-slate-200/70 dark:border-slate-700/70 flex items-start justify-between gap-3"
              >
                <div class="min-w-0">
                  <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100 truncate">
                    {{ t('management.updateDrawerTitle') }}
                  </h3>
                  <p class="text-xs text-slate-500 dark:text-slate-400 truncate">
                    {{ displayName || folderName }}
                  </p>
                </div>
                <button
                  class="shrink-0 px-2.5 py-1 text-xs rounded-lg bg-slate-200 hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300 transition-colors"
                  :disabled="executingUpdate"
                  @click="closeDrawer"
                >
                  {{ t('common.close') }}
                </button>
              </div>
            </div>

            <div class="flex-1 overflow-y-auto px-4 py-4 space-y-3">
              <div
                v-if="showProgress"
                class="rounded-xl border border-sky-200/80 dark:border-sky-800/70 bg-sky-50/70 dark:bg-sky-900/20 p-3"
              >
                <div class="flex items-center justify-between text-xs mb-2">
                  <span class="text-slate-700 dark:text-slate-200 font-medium">
                    {{ t('management.progress') }} · {{ progressLabel }}
                  </span>
                  <span class="text-slate-500 dark:text-slate-400">
                    {{ Math.round(progressPercent) }}%
                  </span>
                </div>
                <div class="h-2 rounded-full bg-slate-200 dark:bg-slate-700 overflow-hidden">
                  <div
                    class="h-full rounded-full bg-gradient-to-r from-sky-500 to-cyan-500 transition-all duration-200"
                    :style="{ width: `${Math.max(0, Math.min(progressPercent, 100))}%` }"
                  />
                </div>
              </div>

              <div class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <p class="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
                      {{ t('management.versionInfo') }}
                    </p>
                    <p class="text-sm font-medium text-slate-900 dark:text-slate-100 mt-1">
                      {{ currentVersion }} → {{ targetVersion }}
                    </p>
                  </div>
                  <div>
                    <p class="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
                      {{ t('management.estimatedDownload') }}
                    </p>
                    <p class="text-sm font-medium text-slate-900 dark:text-slate-100 mt-1">
                      {{ formatBytes(plan?.estimatedDownloadBytes || 0) }}
                    </p>
                  </div>
                </div>
                <div v-if="plan?.remoteLocked" class="mt-3 text-xs text-amber-700 dark:text-amber-300">
                  {{ t('management.remoteLocked') }}
                </div>
                <div
                  v-else-if="isUpToDate && !loadingPlan"
                  class="mt-3 text-xs text-emerald-700 dark:text-emerald-300"
                >
                  {{ t('management.updateUpToDate') }}
                </div>
              </div>

              <div class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                <p class="text-xs font-semibold text-slate-900 dark:text-slate-100 mb-3">
                  {{ t('management.updateOptions') }}
                </p>
                <div class="space-y-2.5">
                  <div class="flex items-center justify-between">
                    <span class="text-xs text-slate-600 dark:text-slate-300">{{ t('management.useBeta') }}</span>
                    <ToggleSwitch
                      :model-value="options.useBeta"
                      size="lg"
                      active-class="bg-sky-500"
                      inactive-class="bg-slate-300 dark:bg-slate-600"
                      :disabled="executingUpdate"
                      :aria-label="t('management.useBeta')"
                      @update:model-value="setOption({ useBeta: $event })"
                    />
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-xs text-slate-600 dark:text-slate-300">{{ t('management.includeLiveries') }}</span>
                    <ToggleSwitch
                      :model-value="options.includeLiveries"
                      size="lg"
                      active-class="bg-sky-500"
                      inactive-class="bg-slate-300 dark:bg-slate-600"
                      :disabled="executingUpdate"
                      :aria-label="t('management.includeLiveries')"
                      @update:model-value="setOption({ includeLiveries: $event })"
                    />
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-xs text-slate-600 dark:text-slate-300">{{ t('management.applyBlacklist') }}</span>
                    <ToggleSwitch
                      :model-value="options.applyBlacklist"
                      size="lg"
                      active-class="bg-sky-500"
                      inactive-class="bg-slate-300 dark:bg-slate-600"
                      :disabled="executingUpdate"
                      :aria-label="t('management.applyBlacklist')"
                      @update:model-value="setOption({ applyBlacklist: $event })"
                    />
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-xs text-slate-600 dark:text-slate-300">{{ t('management.rollbackOnFailure') }}</span>
                    <ToggleSwitch
                      :model-value="options.rollbackOnFailure"
                      size="lg"
                      active-class="bg-sky-500"
                      inactive-class="bg-slate-300 dark:bg-slate-600"
                      :disabled="executingUpdate"
                      :aria-label="t('management.rollbackOnFailure')"
                      @update:model-value="setOption({ rollbackOnFailure: $event })"
                    />
                  </div>

                  <div class="flex items-center justify-between">
                    <span class="text-xs text-slate-600 dark:text-slate-300">{{ t('management.parallelDownloads') }}</span>
                    <select
                      class="text-xs rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-200 px-2 py-1"
                      :value="options.parallelDownloads ?? 4"
                      @change="setParallelDownloads"
                    >
                      <option v-for="count in [1, 2, 3, 4, 5, 6, 7, 8]" :key="count" :value="count">
                        {{ count }}
                      </option>
                    </select>
                  </div>
                </div>
              </div>

              <div v-if="loadingPlan" class="text-xs text-slate-500 dark:text-slate-400">
                {{ t('management.updatePlanLoading') }}
              </div>

              <div
                v-else-if="planError"
                class="text-xs text-rose-600 dark:text-rose-300 bg-rose-50 dark:bg-rose-900/20 rounded-xl p-3"
              >
                {{ t('management.updatePlanFailed') }}: {{ planError }}
              </div>
            </div>

            <div
              class="shrink-0 px-4 py-3 border-t border-slate-200/70 dark:border-slate-700/70 bg-white/90 dark:bg-slate-900/85 flex items-center justify-end gap-2"
            >
              <button
                class="px-3 py-1.5 rounded-lg text-xs bg-slate-200 hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300 disabled:opacity-50 transition-colors"
                :disabled="loadingPlan || executingUpdate"
                @click="reloadPlan"
              >
                {{ t('management.refreshPlan') }}
              </button>
              <button
                class="px-3 py-1.5 rounded-lg text-xs text-white bg-sky-600 hover:bg-sky-700 disabled:opacity-50 transition-colors"
                :disabled="!canRunUpdate"
                @click="runUpdate"
              >
                {{ executingUpdate ? t('management.updating') : t('management.startUpdate') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
