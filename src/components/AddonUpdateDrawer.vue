<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import { useManagementStore } from '@/stores/management'
import { useToastStore } from '@/stores/toast'
import type { AddonUpdatableItemType, AddonUpdatePlan } from '@/types'

interface AddonUpdateDrawerTask {
  itemType: AddonUpdatableItemType
  folderName: string
  displayName: string
  initialLocalVersion?: string
  initialTargetVersion?: string
}

interface AddonUpdateProgressEvent {
  itemType: string
  folderName: string
  stage: string
  status: string
  percentage: number
  processedBytes: number
  totalBytes: number
  speedBytesPerSec: number
  message?: string | null
}

interface TaskUiState {
  plan: AddonUpdatePlan | null
  loadingPlan: boolean
  planError: string
  progress: number
  speedBytes: number
  status: 'idle' | 'planning' | 'installing' | 'completed' | 'failed' | 'cancelled'
  installing: boolean
  message: string
}

const props = defineProps<{
  show: boolean
  tasks: AddonUpdateDrawerTask[]
  activeTaskKey?: string
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'updated'): void
  (e: 'select-task', key: string): void
}>()

const { t } = useI18n()
const managementStore = useManagementStore()
const toast = useToastStore()

const minSheetHeight = 340
const collapsedSheetHeight = 78
const collapseSnapThreshold = 160
const maxSheetHeight = ref(560)
const sheetHeight = ref(500)
const isResizing = ref(false)
const isCollapsed = ref(false)
let dragStartY = 0
let dragStartHeight = 0
let dragStartedCollapsed = false
let preDragExpandedHeight = 500

const expandedTaskKey = ref('')
const taskStateMap = ref<Record<string, TaskUiState>>({})
let unlistenAddonUpdateProgress: UnlistenFn | null = null

function taskKeyOf(task: Pick<AddonUpdateDrawerTask, 'itemType' | 'folderName'>): string {
  return `${task.itemType}:${task.folderName}`
}

const taskCards = computed(() => props.tasks || [])
const taskKeySet = computed(() => new Set(taskCards.value.map((task) => taskKeyOf(task))))

const effectiveSheetHeight = computed(() => {
  if (isResizing.value) return Math.round(sheetHeight.value)
  return isCollapsed.value ? collapsedSheetHeight : sheetHeight.value
})

function createTaskState(): TaskUiState {
  return {
    plan: null,
    loadingPlan: false,
    planError: '',
    progress: 0,
    speedBytes: 0,
    status: 'idle',
    installing: false,
    message: '',
  }
}

function ensureTaskState(key: string): TaskUiState {
  if (!taskStateMap.value[key]) {
    taskStateMap.value[key] = createTaskState()
  }
  return taskStateMap.value[key]
}

function stateFor(task: AddonUpdateDrawerTask): TaskUiState {
  return ensureTaskState(taskKeyOf(task))
}

function syncTaskStates() {
  const next: Record<string, TaskUiState> = {}
  for (const task of taskCards.value) {
    const key = taskKeyOf(task)
    next[key] = taskStateMap.value[key] || createTaskState()
  }
  taskStateMap.value = next

  if (!taskCards.value.length) {
    expandedTaskKey.value = ''
    return
  }

  if (props.activeTaskKey && next[props.activeTaskKey]) {
    expandedTaskKey.value = props.activeTaskKey
    return
  }

  if (!expandedTaskKey.value || !next[expandedTaskKey.value]) {
    expandedTaskKey.value = taskKeyOf(taskCards.value[0])
  }
}

function primePlansForVisibleTasks() {
  if (!props.show) return
  for (const task of taskCards.value) {
    const state = stateFor(task)
    if (state.plan || state.loadingPlan || state.installing) continue
    void loadPlanForTask(task, false)
  }
}

function updateMaxSheetHeight() {
  maxSheetHeight.value = Math.max(minSheetHeight, Math.floor(window.innerHeight * 0.92))
  sheetHeight.value = Math.max(minSheetHeight, Math.min(sheetHeight.value, maxSheetHeight.value))
}

function resetSheetHeight() {
  updateMaxSheetHeight()
  sheetHeight.value = Math.max(
    minSheetHeight,
    Math.min(Math.round(window.innerHeight * 0.62), maxSheetHeight.value),
  )
}

function onResizeMove(event: PointerEvent) {
  if (!isResizing.value) return
  const delta = dragStartY - event.clientY
  const next = dragStartHeight + delta
  sheetHeight.value = Math.max(collapsedSheetHeight, Math.min(next, maxSheetHeight.value))
}

function stopResize() {
  if (!isResizing.value) return
  isResizing.value = false
  window.removeEventListener('pointermove', onResizeMove)
  window.removeEventListener('pointerup', stopResize)

  const finalHeight = sheetHeight.value
  if (finalHeight <= collapseSnapThreshold) {
    isCollapsed.value = true
    if (!dragStartedCollapsed) {
      sheetHeight.value = Math.max(
        minSheetHeight,
        Math.min(preDragExpandedHeight, maxSheetHeight.value),
      )
    }
    return
  }

  isCollapsed.value = false
  sheetHeight.value = Math.max(minSheetHeight, Math.min(finalHeight, maxSheetHeight.value))
}

function onResizeStart(event: PointerEvent) {
  if (!props.show) return
  isResizing.value = true
  dragStartedCollapsed = isCollapsed.value
  preDragExpandedHeight = sheetHeight.value
  dragStartY = event.clientY
  dragStartHeight = isCollapsed.value ? collapsedSheetHeight : sheetHeight.value
  sheetHeight.value = dragStartHeight
  window.addEventListener('pointermove', onResizeMove)
  window.addEventListener('pointerup', stopResize)
}

function closeDrawer() {
  emit('update:show', false)
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let idx = 0
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024
    idx++
  }
  return `${value.toFixed(value >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`
}

function prettyVersion(value?: string | null): string {
  const raw = String(value || '').trim()
  if (!raw) return '-'
  if (/^\d{6}$/.test(raw)) {
    const major = String(Number(raw.slice(0, 2)))
    const minor = String(Number(raw.slice(2, 4)))
    const patch = String(Number(raw.slice(4, 6)))
    return `${major}.${minor}.${patch}`
  }
  return raw
}

function localVersion(task: AddonUpdateDrawerTask, state: TaskUiState): string {
  return prettyVersion(state.plan?.localVersion || task.initialLocalVersion)
}

function remoteVersion(task: AddonUpdateDrawerTask, state: TaskUiState): string {
  return prettyVersion(state.plan?.remoteVersion || task.initialTargetVersion)
}

function planNeedsAction(plan: AddonUpdatePlan | null | undefined): boolean {
  if (!plan) return false
  if (plan.hasUpdate) return true
  if ((plan.estimatedDownloadBytes || 0) > 0) return true
  if ((plan.addFiles?.length || 0) > 0) return true
  if ((plan.replaceFiles?.length || 0) > 0) return true
  if ((plan.deleteFiles?.length || 0) > 0) return true
  return false
}

function taskHasUpdate(task: AddonUpdateDrawerTask, state: TaskUiState): boolean {
  if (state.plan) return planNeedsAction(state.plan)
  const local = String(task.initialLocalVersion || '').trim()
  const remote = String(task.initialTargetVersion || '').trim()
  if (!remote) return false
  if (!local) return true
  return local !== remote
}

function localVersionTextClass(task: AddonUpdateDrawerTask, state: TaskUiState): string {
  if (taskHasUpdate(task, state)) return 'text-amber-600 dark:text-amber-400 font-medium'
  return 'text-emerald-600 dark:text-emerald-400 font-medium'
}

function targetVersionTextClass(_task: AddonUpdateDrawerTask, _state: TaskUiState): string {
  return 'text-emerald-600 dark:text-emerald-400 font-semibold'
}

function isCancelledError(error: unknown): boolean {
  return String(error || '')
    .toLowerCase()
    .includes('cancelled')
}

async function loadPlanForTask(task: AddonUpdateDrawerTask, force = false) {
  const key = taskKeyOf(task)
  const state = ensureTaskState(key)
  if (state.loadingPlan) return
  if (state.plan && !force) return

  state.loadingPlan = true
  state.planError = ''
  state.status = state.status === 'installing' ? 'installing' : 'planning'

  try {
    const plan = await managementStore.buildAddonUpdatePlan(task.itemType, task.folderName)
    state.plan = plan
    if (!planNeedsAction(plan) && !plan.remoteLocked) {
      state.progress = 100
      state.status = 'completed'
    } else if (state.status !== 'installing') {
      state.status = 'idle'
    }
  } catch (e) {
    state.planError = String(e)
    state.status = 'failed'
  } finally {
    state.loadingPlan = false
  }
}

function toggleTaskDetails(task: AddonUpdateDrawerTask) {
  const key = taskKeyOf(task)
  if (expandedTaskKey.value === key) {
    expandedTaskKey.value = ''
    return
  }
  expandedTaskKey.value = key
  emit('select-task', key)
  const state = ensureTaskState(key)
  if (!state.plan && !state.loadingPlan && !state.planError) {
    void loadPlanForTask(task, false)
  }
}

async function startUpdate(task: AddonUpdateDrawerTask) {
  const key = taskKeyOf(task)
  const state = ensureTaskState(key)
  if (state.loadingPlan || state.installing || managementStore.isExecutingUpdate) return

  if (!state.plan) {
    await loadPlanForTask(task, false)
  }

  if (!state.plan) return
  if (state.plan.remoteLocked) {
    toast.warning(t('management.remoteLocked'))
    return
  }

  if (!planNeedsAction(state.plan)) {
    state.progress = 100
    state.status = 'completed'
    toast.info(t('management.updateUpToDate'))
    return
  }

  state.installing = true
  state.status = 'installing'
  state.planError = ''
  state.progress = 0
  state.speedBytes = 0
  state.message = ''

  try {
    const result = await managementStore.executeAddonUpdate(task.itemType, task.folderName)
    state.progress = 100
    state.installing = false
    state.status = 'completed'
    state.speedBytes = 0
    toast.success(
      t('management.updateSuccessSummary', {
        updated: result.updatedFiles,
        deleted: result.deletedFiles,
      }),
    )
    emit('updated')
    await loadPlanForTask(task, true)
  } catch (e) {
    state.installing = false
    state.speedBytes = 0
    if (isCancelledError(e)) {
      state.status = 'cancelled'
      return
    }
    state.status = 'failed'
    state.planError = String(e)
    toast.error(t('management.updateFailed') + ': ' + String(e))
  }
}

async function cancelTask(task: AddonUpdateDrawerTask) {
  const key = taskKeyOf(task)
  const state = ensureTaskState(key)
  try {
    await invoke('cancel_installation')
  } catch {
    // ignore
  }
  state.installing = false
  state.status = 'cancelled'
  state.speedBytes = 0
}

function applyAddonProgressEvent(event: AddonUpdateProgressEvent) {
  const key = `${event.itemType}:${event.folderName}`
  if (!taskKeySet.value.has(key)) return
  const state = ensureTaskState(key)
  const stage = String(event.stage || '').toLowerCase()
  const status = String(event.status || '').toLowerCase()
  const percent = Math.max(0, Math.min(100, Number(event.percentage || 0)))

  state.message = String(event.message || '')

  if (stage === 'scan') {
    if (status === 'started' || status === 'in_progress') {
      state.status = 'planning'
    } else if (status === 'completed') {
      state.status = state.installing ? 'installing' : 'idle'
    } else if (status === 'failed') {
      state.status = 'failed'
    } else if (status === 'cancelled') {
      state.status = 'cancelled'
    }
    return
  }

  if (stage === 'install') {
    const processedBytes = Math.max(0, Number(event.processedBytes || 0))
    const totalBytes = Math.max(0, Number(event.totalBytes || 0))
    if (status === 'started' || status === 'in_progress') {
      state.installing = true
      state.status = 'installing'
      state.progress = totalBytes > 0 && processedBytes <= 0 ? 0 : percent
      state.speedBytes = Math.max(0, Number(event.speedBytesPerSec || 0))
    } else if (status === 'completed') {
      state.installing = false
      state.status = 'completed'
      state.progress = 100
      state.speedBytes = 0
    } else if (status === 'failed') {
      state.installing = false
      state.status = 'failed'
      state.speedBytes = 0
    } else if (status === 'cancelled') {
      state.installing = false
      state.status = 'cancelled'
      state.speedBytes = 0
    }
  }
}

onMounted(async () => {
  updateMaxSheetHeight()
  window.addEventListener('resize', updateMaxSheetHeight)
  unlistenAddonUpdateProgress = await listen<AddonUpdateProgressEvent>(
    'addon-update-progress',
    (event) => {
      if (!props.show) return
      if (!event.payload) return
      applyAddonProgressEvent(event.payload)
    },
  )
})

onBeforeUnmount(() => {
  stopResize()
  if (unlistenAddonUpdateProgress) {
    unlistenAddonUpdateProgress()
    unlistenAddonUpdateProgress = null
  }
  window.removeEventListener('resize', updateMaxSheetHeight)
})

watch(taskCards, () => {
  syncTaskStates()
  primePlansForVisibleTasks()
}, { immediate: true })

watch(
  () => props.activeTaskKey,
  (key) => {
    if (!key) return
    if (taskStateMap.value[key]) {
      expandedTaskKey.value = key
    }
  },
)

watch(
  () => props.show,
  (open) => {
    if (!open) {
      stopResize()
      return
    }
    resetSheetHeight()
    syncTaskStates()
    primePlansForVisibleTasks()
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
      <div v-if="show" class="fixed inset-0 z-[120] pointer-events-none">
        <div class="absolute inset-x-0 bottom-0 px-2 pb-2 sm:px-4 sm:pb-4 pointer-events-none">
          <div
            class="mx-auto w-full max-w-5xl pointer-events-auto rounded-2xl border border-slate-200/80 dark:border-slate-700/80 bg-white/95 dark:bg-slate-900/95 shadow-2xl overflow-hidden flex flex-col transition-[height] duration-200 ease-out"
            :style="{ height: `${effectiveSheetHeight}px` }"
          >
            <div class="shrink-0">
              <div class="h-5 flex items-center justify-center">
                <button
                  class="h-2.5 w-16 rounded-full bg-slate-300/80 hover:bg-slate-400/80 dark:bg-slate-600/80 dark:hover:bg-slate-500/80 cursor-ns-resize transition-colors"
                  @pointerdown.prevent="onResizeStart"
                />
              </div>
              <div class="px-4 pb-3 border-b border-slate-200/70 dark:border-slate-700/70 flex items-center justify-between gap-3">
                <div class="min-w-0">
                  <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100 truncate">
                    {{ t('management.updateDrawerTitle') }}
                  </h3>
                  <p class="text-xs text-slate-500 dark:text-slate-400 truncate">
                    {{ t('management.taskCountCollapsed', { count: taskCards.length }) }}
                  </p>
                </div>
                <button
                  class="px-2.5 py-1 text-xs rounded-lg bg-slate-200 hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300 transition-colors"
                  @click="closeDrawer"
                >
                  {{ t('common.close') }}
                </button>
              </div>
            </div>

            <div v-if="isCollapsed" class="px-4 py-3 text-xs text-slate-500 dark:text-slate-400">
              {{ t('management.updateCollapsedHint') }}
            </div>

            <div v-else class="flex-1 overflow-y-auto px-4 py-4">
              <div v-if="!taskCards.length" class="rounded-xl border border-slate-200 dark:border-slate-700 p-4 bg-white/70 dark:bg-slate-800/40 text-xs text-slate-600 dark:text-slate-300">
                {{ t('management.noTasks') }}
              </div>

              <div v-else class="space-y-3">
                <div
                  v-for="task in taskCards"
                  :key="taskKeyOf(task)"
                  class="rounded-xl border border-slate-200 dark:border-slate-700 bg-white/70 dark:bg-slate-800/40 w-full"
                >
                  <button
                    class="w-full text-left px-3 py-3"
                    @click="toggleTaskDetails(task)"
                  >
                    <div class="flex items-start justify-between gap-3">
                      <div class="min-w-0">
                        <p class="text-sm font-semibold text-slate-900 dark:text-slate-100 truncate">
                          {{ task.displayName || task.folderName }}
                        </p>
                        <p class="mt-1 text-xs text-slate-500 dark:text-slate-400 truncate">
                          {{ t('management.currentVersionLabel') }}
                          <span :class="localVersionTextClass(task, stateFor(task))">
                            {{ localVersion(task, stateFor(task)) }}
                          </span>
                          <span
                            :class="
                              taskHasUpdate(task, stateFor(task))
                                ? 'text-slate-500 dark:text-slate-400'
                                : 'text-emerald-600 dark:text-emerald-400'
                            "
                          >
                            ->
                          </span>
                          {{ t('management.targetVersionLabel') }}
                          <span :class="targetVersionTextClass(task, stateFor(task))">
                            {{ remoteVersion(task, stateFor(task)) }}
                          </span>
                        </p>
                      </div>
                      <div class="flex items-center gap-2 shrink-0">
                        <button
                          v-if="stateFor(task).installing"
                          class="px-3 py-1.5 rounded-lg text-xs text-white bg-rose-600 hover:bg-rose-700"
                          @click.stop="cancelTask(task)"
                        >
                          {{ t('common.cancel') }}
                        </button>
                        <button
                          v-if="
                            !stateFor(task).loadingPlan &&
                            (!stateFor(task).plan || planNeedsAction(stateFor(task).plan))
                          "
                          class="px-3 py-1.5 rounded-lg text-xs text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50"
                          :disabled="stateFor(task).loadingPlan || stateFor(task).installing || managementStore.isExecutingUpdate"
                          @click.stop="startUpdate(task)"
                        >
                          {{ stateFor(task).installing ? t('management.updating') : t('management.startUpdate') }}
                        </button>
                      </div>
                    </div>

                    <div class="mt-2 h-2 rounded-full bg-slate-200 dark:bg-slate-700 overflow-hidden">
                      <div
                        class="h-full bg-gradient-to-r from-sky-500 to-cyan-500 transition-all duration-200"
                        :style="{ width: `${Math.max(0, Math.min(100, stateFor(task).progress))}%` }"
                      />
                    </div>
                    <div class="mt-1 flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400">
                      <span>{{ Math.round(stateFor(task).progress) }}%</span>
                      <span v-if="stateFor(task).speedBytes > 0">
                        {{ t('management.downloadSpeed', { speed: `${formatBytes(stateFor(task).speedBytes)}/s` }) }}
                      </span>
                    </div>
                  </button>

                  <Transition name="task-detail">
                    <div
                      v-if="expandedTaskKey === taskKeyOf(task)"
                      class="px-3 pb-3 border-t border-slate-200/70 dark:border-slate-700/70"
                    >
                    <div class="pt-3 flex items-center justify-between">
                      <p class="text-xs font-semibold text-slate-900 dark:text-slate-100">
                        {{ t('management.planDetails') }}
                      </p>
                    </div>

                    <div v-if="stateFor(task).loadingPlan" class="mt-2 flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                      <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                      </svg>
                      <span>{{ t('management.updatePlanLoading') }}</span>
                    </div>

                    <div
                      v-else-if="stateFor(task).planError"
                      class="mt-2 rounded-lg border border-rose-200 dark:border-rose-700 bg-rose-50/70 dark:bg-rose-900/20 p-2"
                    >
                      <p class="text-xs text-rose-700 dark:text-rose-300">{{ stateFor(task).planError }}</p>
                    </div>

                    <div v-else-if="stateFor(task).plan" class="mt-2 space-y-3">
                      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
                        <div class="rounded-lg border border-slate-200 dark:border-slate-700 p-2">
                          <p class="text-slate-500 dark:text-slate-400">{{ t('management.installInfo') }}</p>
                          <p
                            class="mt-1"
                            :class="
                              planNeedsAction(stateFor(task).plan)
                                ? stateFor(task).plan?.hasUpdate
                                  ? 'text-slate-800 dark:text-slate-100'
                                  : 'text-amber-600 dark:text-amber-300 font-semibold'
                                : 'text-emerald-600 dark:text-emerald-400 font-semibold'
                            "
                          >
                            {{
                              planNeedsAction(stateFor(task).plan)
                                ? stateFor(task).plan?.hasUpdate
                                  ? t('management.updateAvailablePanel')
                                  : t('management.repairRequired')
                                : t('management.updateUpToDate')
                            }}
                          </p>
                        </div>
                        <div class="rounded-lg border border-slate-200 dark:border-slate-700 p-2">
                          <p class="text-slate-500 dark:text-slate-400">{{ t('management.estimatedDownload') }}</p>
                          <p class="mt-1 text-slate-800 dark:text-slate-100">
                            {{ formatBytes(stateFor(task).plan?.estimatedDownloadBytes || 0) }}
                          </p>
                        </div>
                      </div>

                      <p
                        v-if="stateFor(task).plan?.remoteLocked"
                        class="text-xs text-amber-700 dark:text-amber-300"
                      >
                        {{ t('management.remoteLocked') }}
                      </p>

                      <div class="grid grid-cols-1 lg:grid-cols-2 gap-2">
                        <div
                          v-if="(stateFor(task).plan?.addFiles?.length || 0) > 0"
                          class="rounded-lg border border-slate-200 dark:border-slate-700 p-2"
                        >
                          <p class="text-xs font-semibold text-slate-800 dark:text-slate-100">{{ t('management.filesToAdd') }}</p>
                          <ul class="mt-1 max-h-32 overflow-auto text-[11px] text-slate-600 dark:text-slate-300 space-y-1">
                            <li v-for="file in stateFor(task).plan?.addFiles || []" :key="`add-${file}`">{{ file }}</li>
                          </ul>
                        </div>

                        <div
                          v-if="(stateFor(task).plan?.replaceFiles?.length || 0) > 0"
                          class="rounded-lg border border-slate-200 dark:border-slate-700 p-2"
                        >
                          <p class="text-xs font-semibold text-slate-800 dark:text-slate-100">{{ t('management.filesToReplace') }}</p>
                          <ul class="mt-1 max-h-32 overflow-auto text-[11px] text-slate-600 dark:text-slate-300 space-y-1">
                            <li v-for="file in stateFor(task).plan?.replaceFiles || []" :key="`replace-${file}`">{{ file }}</li>
                          </ul>
                        </div>

                        <div
                          v-if="(stateFor(task).plan?.deleteFiles?.length || 0) > 0"
                          class="rounded-lg border border-slate-200 dark:border-slate-700 p-2"
                        >
                          <p class="text-xs font-semibold text-slate-800 dark:text-slate-100">{{ t('management.filesToDelete') }}</p>
                          <ul class="mt-1 max-h-32 overflow-auto text-[11px] text-slate-600 dark:text-slate-300 space-y-1">
                            <li v-for="file in stateFor(task).plan?.deleteFiles || []" :key="`delete-${file}`">{{ file }}</li>
                          </ul>
                        </div>

                      </div>

                      <div
                        v-if="(stateFor(task).plan?.warnings?.length || 0) > 0"
                        class="rounded-lg border border-amber-200 dark:border-amber-700 p-2"
                      >
                        <p class="text-xs font-semibold text-amber-800 dark:text-amber-200">{{ t('management.warnings') }}</p>
                        <ul class="mt-1 max-h-28 overflow-auto text-[11px] text-amber-700 dark:text-amber-300 space-y-1">
                          <li v-for="warn in stateFor(task).plan?.warnings || []" :key="warn">{{ warn }}</li>
                        </ul>
                      </div>

                      <p
                        v-if="
                          (stateFor(task).plan?.addFiles?.length || 0) === 0 &&
                          (stateFor(task).plan?.replaceFiles?.length || 0) === 0 &&
                          (stateFor(task).plan?.deleteFiles?.length || 0) === 0
                        "
                        class="text-xs text-slate-500 dark:text-slate-400"
                      >
                        {{ t('management.noFileChanges') }}
                      </p>
                    </div>

                    <p v-else class="mt-2 text-xs text-slate-500 dark:text-slate-400">
                      {{ t('management.noPlanYet') }}
                    </p>
                    </div>
                  </Transition>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
<style scoped>
.task-detail-enter-active,
.task-detail-leave-active {
  transition:
    max-height 0.24s ease,
    opacity 0.2s ease,
    transform 0.2s ease;
  overflow: hidden;
}

.task-detail-enter-from,
.task-detail-leave-to {
  max-height: 0;
  opacity: 0;
  transform: translateY(-4px);
}

.task-detail-enter-to,
.task-detail-leave-from {
  max-height: 1400px;
  opacity: 1;
  transform: translateY(0);
}
</style>
