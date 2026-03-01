<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import { useManagementStore } from '@/stores/management'
import { useToastStore } from '@/stores/toast'
import ToggleSwitch from '@/components/ToggleSwitch.vue'
import type {
  AddonDiskSpaceInfo,
  AddonUpdatableItemType,
  AddonUpdateOptions,
  AddonUpdatePlan,
  AddonUpdatePreview,
} from '@/types'

type XChannel = 'stable' | 'beta' | 'alpha'
type XUpdaterStep = 'credentials' | 'options' | 'scan' | 'install' | 'done'

interface AddonUpdateProgressEvent {
  itemType: string
  folderName: string
  stage: string
  status: string
  percentage: number
  processedUnits: number
  totalUnits: number
  processedBytes: number
  totalBytes: number
  speedBytesPerSec: number
  currentFile?: string | null
  message?: string | null
}

const props = defineProps<{
  show: boolean
  itemType: AddonUpdatableItemType
  folderName: string
  displayName: string
  isXUpdaterTarget?: boolean
  initialLocalVersion?: string
  initialTargetVersion?: string
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'updated'): void
}>()

const { t } = useI18n()
const managementStore = useManagementStore()
const toast = useToastStore()

const plan = ref<AddonUpdatePlan | null>(null)
const preview = ref<AddonUpdatePreview | null>(null)
const diskInfo = ref<AddonDiskSpaceInfo | null>(null)
const planError = ref('')
const previewError = ref('')

const isPanelInitializing = ref(false)
const checkingPreview = ref(false)
const scanning = ref(false)
const installing = ref(false)
const isCollapsed = ref(false)
const xStep = ref<XUpdaterStep>('credentials')

const credentialsLogin = ref('')
const credentialsKey = ref('')
const selectedChannel = ref<XChannel>('stable')
const rollbackOnFailure = ref(true)
const freshInstall = ref(false)

const scanProgress = ref(0)
const installProgress = ref(0)
const installSpeedBytes = ref(0)

let panelInitToken = 0
let checkToken = 0
let scanToken = 0
let unlistenAddonUpdateProgress: UnlistenFn | null = null

const minSheetHeight = 340
const maxSheetHeight = ref(560)
const sheetHeight = ref(500)
const isResizing = ref(false)
let dragStartY = 0
let dragStartHeight = 0
const MIN_INIT_SPINNER_MS = 420

const isBusy = computed(
  () =>
    checkingPreview.value ||
    scanning.value ||
    installing.value ||
    managementStore.isBuildingUpdatePlan ||
    managementStore.isExecutingUpdate,
)

const detectedXUpdater = computed(() => {
  if (props.isXUpdaterTarget) return true
  const provider = (plan.value?.provider || preview.value?.provider || '').toLowerCase()
  return provider === 'x-updater'
})

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

function versionFromChangelog(changelog?: string | null): string {
  const raw = String(changelog || '')
  if (!raw.trim()) return ''

  const semverMatch = raw.match(/\b[vV]?(\d+(?:\.\d+){1,3})\b/)
  if (semverMatch?.[1]) {
    return semverMatch[1]
  }

  const sixDigitMatch = raw.match(/\b(\d{6})\b/)
  if (sixDigitMatch?.[1]) {
    const digits = sixDigitMatch[1]
    const major = String(Number(digits.slice(0, 2)))
    const minor = String(Number(digits.slice(2, 4)))
    const patch = String(Number(digits.slice(4, 6)))
    return `${major}.${minor}.${patch}`
  }

  return ''
}

const currentVersion = computed(() =>
  prettyVersion(preview.value?.localVersion || plan.value?.localVersion || props.initialLocalVersion),
)
const targetVersion = computed(() =>
  prettyVersion(
    versionFromChangelog(preview.value?.changelog) ||
      preview.value?.targetVersion ||
      plan.value?.remoteVersion ||
      props.initialTargetVersion,
  ),
)

const hasPendingFileChanges = computed(() => {
  if (!plan.value) return false
  return (
    plan.value.addFiles.length > 0 ||
    plan.value.replaceFiles.length > 0 ||
    plan.value.deleteFiles.length > 0
  )
})

const noUpdateAfterScan = computed(
  () => !!plan.value && !hasPendingFileChanges.value && !plan.value.remoteLocked,
)

const canCheckUpdates = computed(() => {
  if (!detectedXUpdater.value) return false
  if (isBusy.value) return false
  return credentialsLogin.value.trim().length > 0 && credentialsKey.value.trim().length > 0
})

const canStartScan = computed(() => detectedXUpdater.value && !!preview.value && !isBusy.value)
const canInstall = computed(
  () => !!plan.value && hasPendingFileChanges.value && !plan.value.remoteLocked && !isBusy.value,
)
const canGoBack = computed(
  () => detectedXUpdater.value && !isBusy.value && xStep.value !== 'credentials',
)
const xStepLabel = computed(() => {
  switch (xStep.value) {
    case 'credentials':
      return '步骤 1/4：账户凭据'
    case 'options':
      return '步骤 2/4：更新信息与选项'
    case 'scan':
      return '步骤 3/4：扫描本地文件'
    case 'install':
      return '步骤 4/4：下载安装'
    case 'done':
      return '步骤 4/4：完成'
    default:
      return 'Addon Update'
  }
})

const availableChannels = computed<XChannel[]>(() => {
  if (!preview.value?.availableChannels?.length) {
    return ['stable', 'beta', 'alpha']
  }

  const out: XChannel[] = []
  for (const raw of preview.value.availableChannels) {
    const normalized = normalizeChannel(raw)
    if (!out.includes(normalized)) {
      out.push(normalized)
    }
  }
  if (!out.length) {
    out.push('stable', 'beta', 'alpha')
  }
  return out
})

function normalizeChannel(value?: string | null): XChannel {
  const raw = String(value || '').trim().toLowerCase()
  if (raw === 'alpha') return 'alpha'
  if (raw === 'beta') return 'beta'
  return 'stable'
}

function optionsOverride(): Partial<AddonUpdateOptions> {
  return {
    channel: selectedChannel.value,
    useBeta: selectedChannel.value !== 'stable',
    rollbackOnFailure: rollbackOnFailure.value,
    freshInstall: freshInstall.value,
  }
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

function isCancelledError(error: unknown): boolean {
  return String(error || '')
    .toLowerCase()
    .includes('cancelled')
}

function applyAddonProgressEvent(event: AddonUpdateProgressEvent) {
  const stage = String(event.stage || '').toLowerCase()
  const status = String(event.status || '').toLowerCase()

  if (stage === 'check') {
    if (status === 'started' || status === 'in_progress') {
      checkingPreview.value = true
    } else if (status === 'completed' || status === 'failed' || status === 'cancelled') {
      checkingPreview.value = false
    }
    return
  }

  if (stage === 'scan') {
    scanProgress.value = Math.max(0, Math.min(100, Number(event.percentage || 0)))
    if (status === 'started' || status === 'in_progress') {
      scanning.value = true
    } else if (status === 'completed') {
      scanning.value = false
      scanProgress.value = 100
    } else if (status === 'failed' || status === 'cancelled') {
      scanning.value = false
      if (status === 'cancelled') {
        scanProgress.value = 0
      }
    }
    return
  }

  if (stage === 'install') {
    installProgress.value = Math.max(0, Math.min(100, Number(event.percentage || 0)))
    installSpeedBytes.value = Math.max(0, Number(event.speedBytesPerSec || 0))
    if (status === 'started' || status === 'in_progress') {
      installing.value = true
    } else if (status === 'completed') {
      installing.value = false
      installProgress.value = 100
      installSpeedBytes.value = 0
    } else if (status === 'failed' || status === 'cancelled') {
      installing.value = false
      installSpeedBytes.value = 0
      if (status === 'cancelled') {
        installProgress.value = 0
      }
    }
  }
}

function resetState() {
  xStep.value = 'credentials'
  preview.value = null
  previewError.value = ''
  plan.value = null
  planError.value = ''
  diskInfo.value = null
  checkingPreview.value = false
  scanning.value = false
  installing.value = false
  scanProgress.value = 0
  installProgress.value = 0
  installSpeedBytes.value = 0
}

function updateMaxSheetHeight() {
  maxSheetHeight.value = Math.max(minSheetHeight, Math.floor(window.innerHeight * 0.92))
  sheetHeight.value = Math.max(minSheetHeight, Math.min(sheetHeight.value, maxSheetHeight.value))
}

function resetSheetHeight() {
  updateMaxSheetHeight()
  sheetHeight.value = Math.max(minSheetHeight, Math.min(Math.round(window.innerHeight * 0.62), maxSheetHeight.value))
}

function onResizeMove(event: PointerEvent) {
  if (!isResizing.value || isCollapsed.value) return
  const delta = dragStartY - event.clientY
  const next = dragStartHeight + delta
  sheetHeight.value = Math.max(minSheetHeight, Math.min(next, maxSheetHeight.value))
}

function stopResize() {
  if (!isResizing.value) return
  isResizing.value = false
  window.removeEventListener('pointermove', onResizeMove)
  window.removeEventListener('pointerup', stopResize)
}

function onResizeStart(event: PointerEvent) {
  if (!props.show || isCollapsed.value) return
  isResizing.value = true
  dragStartY = event.clientY
  dragStartHeight = sheetHeight.value
  window.addEventListener('pointermove', onResizeMove)
  window.addEventListener('pointerup', stopResize)
}

function closeDrawer() {
  if (isBusy.value) {
    isCollapsed.value = true
    return
  }
  emit('update:show', false)
}

function goToPreviousStep() {
  if (!canGoBack.value) return

  if (xStep.value === 'options') {
    xStep.value = 'credentials'
    return
  }

  if (xStep.value === 'scan' || xStep.value === 'install' || xStep.value === 'done') {
    xStep.value = 'options'
  }
}

async function initializePanel() {
  if (!props.show) return

  const token = ++panelInitToken
  const startedAt = performance.now()
  isPanelInitializing.value = true
  isCollapsed.value = false
  resetState()

  try {
    await managementStore.loadAddonUpdateOptions()
    selectedChannel.value = normalizeChannel(managementStore.addonUpdateOptions.channel)
    rollbackOnFailure.value = !!managementStore.addonUpdateOptions.rollbackOnFailure
    freshInstall.value = !!managementStore.addonUpdateOptions.freshInstall

    try {
      const credentials = await managementStore.getAddonUpdaterCredentials(props.itemType, props.folderName)
      if (credentials) {
        credentialsLogin.value = credentials.login
        credentialsKey.value = credentials.licenseKey
      } else {
        credentialsLogin.value = ''
        credentialsKey.value = ''
      }
    } catch {
      credentialsLogin.value = ''
      credentialsKey.value = ''
    }

    if (!detectedXUpdater.value) {
      plan.value = await managementStore.buildAddonUpdatePlan(props.itemType, props.folderName)
    }
  } catch (e) {
    if (!detectedXUpdater.value) {
      planError.value = String(e)
    }
  } finally {
    const elapsed = performance.now() - startedAt
    const waitMs = Math.max(0, MIN_INIT_SPINNER_MS - elapsed)
    if (waitMs > 0) {
      await new Promise((resolve) => window.setTimeout(resolve, waitMs))
    }
    if (token === panelInitToken) {
      isPanelInitializing.value = false
    }
  }
}

async function checkUpdates() {
  if (!canCheckUpdates.value) return
  const token = ++checkToken
  checkingPreview.value = true
  previewError.value = ''
  preview.value = null
  plan.value = null
  planError.value = ''
  diskInfo.value = null

  try {
    const data = await managementStore.fetchAddonUpdatePreview(
      props.itemType,
      props.folderName,
      credentialsLogin.value,
      credentialsKey.value,
      optionsOverride(),
    )
    if (token !== checkToken) return

    preview.value = data
    selectedChannel.value = normalizeChannel(data.selectedChannel)
    xStep.value = 'options'
    await managementStore.setAddonUpdateOptions(optionsOverride())
    await managementStore.setAddonUpdaterCredentials(
      props.itemType,
      props.folderName,
      credentialsLogin.value,
      credentialsKey.value,
    )
    toast.success('检查成功，凭据已自动保存')
  } catch (e) {
    if (token === checkToken) {
      if (isCancelledError(e)) {
        previewError.value = ''
      } else {
        previewError.value = String(e)
        toast.error('检查失败: ' + String(e))
      }
    }
  } finally {
    if (token === checkToken) {
      checkingPreview.value = false
    }
  }
}

function cancelCheck() {
  checkToken++
  checkingPreview.value = false
  invoke('cancel_installation').catch(() => {
    // ignore
  })
}

async function scanLocal() {
  if (!canStartScan.value) return
  const token = ++scanToken
  xStep.value = 'scan'
  scanning.value = true
  plan.value = null
  planError.value = ''
  diskInfo.value = null
  scanProgress.value = 0

  try {
    await managementStore.setAddonUpdateOptions(optionsOverride())
    const nextPlan = await managementStore.buildAddonUpdatePlan(
      props.itemType,
      props.folderName,
      optionsOverride(),
    )
    if (token !== scanToken) return

    plan.value = nextPlan
    diskInfo.value = await managementStore.getAddonUpdateDiskSpace(props.itemType, props.folderName)
    xStep.value = noUpdateAfterScan.value ? 'done' : 'install'
    scanProgress.value = 100
  } catch (e) {
    if (token === scanToken) {
      if (isCancelledError(e)) {
        planError.value = ''
        xStep.value = 'options'
      } else {
        planError.value = String(e)
        toast.error('扫描失败: ' + String(e))
        xStep.value = 'scan'
      }
      scanProgress.value = 0
    }
  } finally {
    if (token === scanToken) {
      scanning.value = false
    }
  }
}

async function cancelScan() {
  scanToken++
  try {
    await invoke('cancel_installation')
  } catch {
    // ignore
  }
  scanProgress.value = 0
  scanning.value = false
  xStep.value = 'options'
}

async function installUpdate() {
  if (!canInstall.value || !plan.value) return
  xStep.value = 'install'
  installing.value = true
  installProgress.value = 0
  installSpeedBytes.value = 0

  try {
    await managementStore.setAddonUpdateOptions(optionsOverride())
    const result = await managementStore.executeAddonUpdate(
      props.itemType,
      props.folderName,
      optionsOverride(),
    )
    installProgress.value = 100
    installSpeedBytes.value = 0
    toast.success(
      t('management.updateSuccessSummary', {
        updated: result.updatedFiles,
        deleted: result.deletedFiles,
      }),
    )
    emit('updated')
    await scanLocal()
  } catch (e) {
    if (!isCancelledError(e)) {
      toast.error(t('management.updateFailed') + ': ' + String(e))
    }
    installProgress.value = 0
    installSpeedBytes.value = 0
  } finally {
    installing.value = false
  }
}

async function cancelInstall() {
  try {
    await invoke('cancel_installation')
  } catch {
    // ignore
  }
  installing.value = false
  installProgress.value = 0
  installSpeedBytes.value = 0
}

onMounted(async () => {
  updateMaxSheetHeight()
  window.addEventListener('resize', updateMaxSheetHeight)
  unlistenAddonUpdateProgress = await listen<AddonUpdateProgressEvent>(
    'addon-update-progress',
    (event) => {
      const payload = event.payload
      if (!payload) return
      if (!props.show) return
      if (payload.itemType !== props.itemType || payload.folderName !== props.folderName) return
      applyAddonProgressEvent(payload)
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

watch(
  () => [props.show, props.itemType, props.folderName, props.isXUpdaterTarget] as const,
  async ([open, itemType, folderName, isX], [oldOpen, oldItemType, oldFolderName, oldIsX]) => {
    if (!open) {
      panelInitToken++
      stopResize()
      resetState()
      return
    }

    if (!oldOpen) {
      resetSheetHeight()
      await initializePanel()
      return
    }

    const changed =
      itemType !== oldItemType || folderName !== oldFolderName || isX !== oldIsX
    if (changed) {
      await initializePanel()
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
            :style="{ height: `${isCollapsed ? 78 : sheetHeight}px` }"
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
                  <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100 truncate">Addon Update</h3>
                  <p class="text-xs text-slate-500 dark:text-slate-400 truncate">{{ displayName || folderName }}</p>
                </div>
                <div class="flex items-center gap-2">
                  <button class="px-2.5 py-1 text-xs rounded-lg bg-slate-200 hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300 transition-colors" @click="isCollapsed = !isCollapsed">
                    {{ isCollapsed ? '展开' : '收起' }}
                  </button>
                  <button class="px-2.5 py-1 text-xs rounded-lg bg-slate-200 hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300 transition-colors" @click="closeDrawer">
                    {{ isBusy ? '收起' : t('common.close') }}
                  </button>
                </div>
              </div>
            </div>

            <div v-if="isCollapsed" class="px-4 py-3">
              <div v-if="checkingPreview" class="text-xs text-slate-600 dark:text-slate-300">检查更新中...</div>
              <div v-else-if="scanning" class="text-xs text-slate-600 dark:text-slate-300">扫描中 {{ Math.round(scanProgress) }}%</div>
              <div v-else-if="installing" class="text-xs text-slate-600 dark:text-slate-300">安装中 {{ Math.round(installProgress) }}%</div>
              <div v-else class="text-xs text-slate-500 dark:text-slate-400">点击“展开”查看详情</div>
            </div>

            <div v-else-if="isPanelInitializing" class="flex-1 flex items-center justify-center">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-500" />
            </div>

            <div v-else class="flex-1 overflow-y-auto px-4 py-4 space-y-3">
              <div
                v-if="
                  (!detectedXUpdater || preview || plan) &&
                  !(detectedXUpdater && (xStep === 'credentials' || (xStep === 'done' && noUpdateAfterScan)))
                "
                class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40"
              >
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <p class="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">当前版本</p>
                    <p class="text-sm font-medium text-slate-900 dark:text-slate-100 mt-1">{{ currentVersion }}</p>
                  </div>
                  <div>
                    <p class="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">目标版本</p>
                    <p class="text-sm font-medium text-slate-900 dark:text-slate-100 mt-1">{{ targetVersion }}</p>
                  </div>
                </div>
              </div>

              <template v-if="detectedXUpdater">
                <div
                  v-if="xStep !== 'credentials'"
                  class="rounded-xl border border-slate-200 dark:border-slate-700 px-3 py-2 bg-white/70 dark:bg-slate-800/40 flex items-center justify-between gap-3"
                >
                  <p class="text-xs text-slate-600 dark:text-slate-300">{{ xStepLabel }}</p>
                  <button
                    class="px-3 py-1.5 rounded-lg text-xs text-slate-700 dark:text-slate-200 bg-slate-200 hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600 disabled:opacity-50"
                    :disabled="!canGoBack"
                    @click="goToPreviousStep"
                  >
                    上一步
                  </button>
                </div>

                <div
                  v-if="xStep === 'credentials'"
                  class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40"
                >
                  <p class="text-xs font-semibold text-slate-900 dark:text-slate-100 mb-3">账户凭据</p>
                  <form class="space-y-2.5" @submit.prevent="checkUpdates">
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                      <input v-model="credentialsLogin" type="text" autocomplete="username" class="w-full rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-100 px-3 py-2 text-xs" placeholder="账号" :disabled="isBusy" />
                      <input v-model="credentialsKey" type="password" autocomplete="current-password" class="w-full rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-100 px-3 py-2 text-xs" placeholder="激活码" :disabled="isBusy" />
                    </div>
                    <div class="flex justify-end gap-2">
                      <button v-if="checkingPreview" type="button" class="px-4 py-2 rounded-lg text-xs text-white bg-rose-600 hover:bg-rose-700" @click="cancelCheck">取消</button>
                      <button type="submit" class="px-4 py-2 rounded-lg text-xs text-white bg-sky-600 hover:bg-sky-700 disabled:opacity-50" :disabled="!canCheckUpdates">
                        {{ checkingPreview ? '检查中...' : '检查更新' }}
                      </button>
                    </div>
                  </form>
                  <p v-if="previewError" class="mt-2 text-xs text-rose-600 dark:text-rose-300">{{ previewError }}</p>
                </div>

                <template v-else-if="xStep === 'options'">
                  <div v-if="preview" class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                    <p class="text-xs font-semibold text-slate-900 dark:text-slate-100">版本变更</p>
                    <pre class="mt-2 max-h-44 overflow-auto rounded-lg bg-slate-50 dark:bg-slate-900/60 border border-slate-200 dark:border-slate-700 p-2 text-[11px] whitespace-pre-wrap leading-5 text-slate-700 dark:text-slate-200">{{ preview.changelog || '暂无更新说明' }}</pre>
                  </div>

                  <div class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                    <p class="text-xs font-semibold text-slate-900 dark:text-slate-100 mb-2.5">更新选项</p>
                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                      <select v-model="selectedChannel" class="w-full rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-100 px-2.5 py-2 text-xs" :disabled="isBusy">
                        <option v-for="channel in availableChannels" :key="channel" :value="channel">
                          {{ channel === 'stable' ? 'Stable' : channel === 'beta' ? 'Beta' : 'Alpha' }}
                        </option>
                      </select>
                      <label class="flex items-center justify-between gap-2 rounded-lg border border-slate-200 dark:border-slate-700 px-2.5 py-2">
                        <span class="text-xs text-slate-600 dark:text-slate-300">失败回滚</span>
                        <ToggleSwitch :model-value="rollbackOnFailure" size="lg" active-class="bg-sky-500" inactive-class="bg-slate-300 dark:bg-slate-600" :disabled="isBusy" @update:model-value="rollbackOnFailure = $event" />
                      </label>
                      <label class="flex items-center justify-between gap-2 rounded-lg border border-slate-200 dark:border-slate-700 px-2.5 py-2">
                        <span class="text-xs text-slate-600 dark:text-slate-300">Fresh 安装</span>
                        <ToggleSwitch :model-value="freshInstall" size="lg" active-class="bg-sky-500" inactive-class="bg-slate-300 dark:bg-slate-600" :disabled="isBusy" @update:model-value="freshInstall = $event" />
                      </label>
                    </div>
                    <div class="mt-3 flex justify-end">
                      <button class="px-4 py-2 rounded-lg text-xs text-white bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50" :disabled="!canStartScan" @click="scanLocal">
                        开始更新
                      </button>
                    </div>
                  </div>
                </template>

                <template v-else-if="xStep === 'scan'">
                  <div class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                    <div class="flex items-center justify-between gap-2">
                      <p class="text-xs font-semibold text-slate-900 dark:text-slate-100">扫描本地文件</p>
                      <button v-if="scanning" class="px-3 py-1.5 rounded-lg text-xs text-white bg-rose-600 hover:bg-rose-700" @click="cancelScan">取消</button>
                    </div>
                    <div class="mt-2 h-2 rounded-full bg-slate-200 dark:bg-slate-700 overflow-hidden">
                      <div class="h-full bg-gradient-to-r from-sky-500 to-cyan-500 transition-all duration-200" :style="{ width: `${scanProgress}%` }" />
                    </div>
                    <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">{{ Math.round(scanProgress) }}%</p>
                    <p v-if="planError" class="mt-1 text-xs text-rose-600 dark:text-rose-300">{{ planError }}</p>
                  </div>
                </template>

                <template v-else-if="xStep === 'install'">
                  <div v-if="plan && hasPendingFileChanges" class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                    <p class="text-xs font-semibold text-slate-900 dark:text-slate-100">安装准备</p>
                    <div class="mt-2 grid grid-cols-1 sm:grid-cols-2 gap-2">
                      <div>
                        <p class="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">需要下载</p>
                        <p class="text-sm font-medium text-slate-900 dark:text-slate-100">{{ formatBytes(plan.estimatedDownloadBytes || 0) }}</p>
                      </div>
                      <div>
                        <p class="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">磁盘剩余</p>
                        <p class="text-sm font-medium text-slate-900 dark:text-slate-100">{{ formatBytes(diskInfo?.freeBytes || 0) }}</p>
                      </div>
                    </div>
                    <div v-if="installing || installProgress > 0" class="mt-3">
                      <div class="flex items-center justify-between text-xs mb-1">
                        <span class="text-slate-700 dark:text-slate-200">下载安装中</span>
                        <span class="text-slate-500 dark:text-slate-400">{{ Math.round(installProgress) }}%</span>
                      </div>
                      <div class="h-2 rounded-full bg-slate-200 dark:bg-slate-700 overflow-hidden">
                        <div class="h-full bg-gradient-to-r from-emerald-500 to-teal-500 transition-all duration-200" :style="{ width: `${installProgress}%` }" />
                      </div>
                      <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">下载速度: {{ formatBytes(installSpeedBytes) }}/s</p>
                    </div>
                    <div class="mt-3 flex justify-end gap-2">
                      <button v-if="installing" class="px-4 py-2 rounded-lg text-xs text-white bg-rose-600 hover:bg-rose-700" @click="cancelInstall">取消</button>
                      <button class="px-4 py-2 rounded-lg text-xs text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50" :disabled="!canInstall" @click="installUpdate">
                        {{ installing ? '安装中...' : '安装' }}
                      </button>
                    </div>
                  </div>
                  <div v-else class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                    <p class="text-xs text-slate-600 dark:text-slate-300">未检测到可安装内容，请返回上一步重新扫描。</p>
                  </div>
                </template>

                <template v-else-if="xStep === 'done'">
                  <div
                    class="rounded-xl border border-emerald-200 dark:border-emerald-700 p-6 bg-emerald-50/80 dark:bg-emerald-900/20 text-center"
                  >
                    <p class="text-base font-semibold text-emerald-700 dark:text-emerald-300">无需更新</p>
                  </div>
                </template>
              </template>

              <template v-else>
                <p v-if="planError" class="text-xs text-rose-600 dark:text-rose-300">{{ planError }}</p>
                <div v-else-if="plan" class="rounded-xl border border-slate-200 dark:border-slate-700 p-3 bg-white/70 dark:bg-slate-800/40">
                  <div class="flex items-center justify-between">
                    <span class="text-xs text-slate-600 dark:text-slate-300">{{ hasPendingFileChanges ? '有可用更新' : '无需更新' }}</span>
                    <button class="px-4 py-2 rounded-lg text-xs text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50" :disabled="!hasPendingFileChanges || installing" @click="installUpdate">开始更新</button>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
