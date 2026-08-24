import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { useAppStore } from './app'
import { useLockStore } from './lock'
import { logError } from '@/services/logger'
import {
  createDoctorCheckDefinitions,
  type DoctorCheckContext,
  type DoctorCheckDefinition,
} from '@/services/doctorChecks'
import { loadDoctorRunsForPath, saveDoctorRun } from '@/services/doctorHistory'
import { buildDoctorReport, type DoctorReportFormat } from '@/utils/doctorReport'
import { isDoctorRunStale, sortDoctorChecks, summarizeDoctorRun } from '@/utils/doctor'
import type {
  DoctorCheckResult,
  DoctorCheckRuntime,
  DoctorRemediation,
  DoctorRun,
  DoctorRunMode,
} from '@/types/doctor'

export type DoctorPhase = 'idle' | 'local' | 'network' | 'done'

export interface DoctorBatchFixResult {
  applied: number
  failed: number
}

const LOCAL_CONCURRENCY = 4
const NETWORK_CONCURRENCY = 3
const APP_VERSION_TIMEOUT_MS = 5_000
const PATH_VALIDATION_TIMEOUT_MS = 10_000

const FILESYSTEM_REMEDIATIONS = new Set([
  'sort_scenery',
  'enable_global_airports',
  'apply_flatten',
  'refresh_scenery_index',
  'rebuild_scenery_index',
])

const DEFINITION_RESULT_IDS: Record<string, string[]> = {
  scenery: [
    'scenery.global_airports',
    'scenery.load_order',
    'scenery.dependencies',
    'scenery.overlaps',
    'scenery.duplicate_airports',
    'scenery.flatten',
  ],
  xfast: ['storage.app_data_space', 'xfast.app_data', 'xfast.database', 'xfast.scenery_index'],
}

class DoctorTimeoutError extends Error {
  constructor(checkId: string, timeoutMs: number) {
    super(`${checkId} timed out after ${timeoutMs} ms`)
    this.name = 'DoctorTimeoutError'
  }
}

class DoctorCancelledError extends Error {
  constructor() {
    super('Health run cancelled')
    this.name = 'DoctorCancelledError'
  }
}

function createRunId(): string {
  if (typeof globalThis.crypto?.randomUUID === 'function') return globalThis.crypto.randomUUID()
  return `health-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

function runWithTimeout<T>(
  promise: Promise<T>,
  checkId: string,
  timeoutMs: number,
  cancellation: Promise<void>,
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    let settled = false
    const finish = (callback: () => void) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      callback()
    }
    const timer = setTimeout(
      () => finish(() => reject(new DoctorTimeoutError(checkId, timeoutMs))),
      timeoutMs,
    )
    promise.then(
      (value) => finish(() => resolve(value)),
      (reason: unknown) => finish(() => reject(reason)),
    )
    cancellation.then(() => finish(() => reject(new DoctorCancelledError())))
  })
}

async function runBounded<T>(
  items: T[],
  concurrency: number,
  worker: (item: T) => Promise<void>,
): Promise<void> {
  let nextIndex = 0
  const workerCount = Math.min(Math.max(concurrency, 1), items.length)
  await Promise.all(
    Array.from({ length: workerCount }, async () => {
      while (nextIndex < items.length) {
        const item = items[nextIndex]
        nextIndex += 1
        await worker(item)
      }
    }),
  )
}

function cancelledResult(definition: DoctorCheckDefinition): DoctorCheckResult {
  return {
    id: `diagnostic.${definition.id}`,
    section: definition.section,
    outcome: 'cancelled',
    durationMs: 0,
  }
}

function unavailableResult(
  definition: DoctorCheckDefinition,
  error: unknown,
  durationMs: number,
): DoctorCheckResult {
  return {
    id: `diagnostic.${definition.id}`,
    section: definition.section,
    outcome: 'unavailable',
    durationMs,
    evidence: [{ kind: 'text', value: String(error) }],
  }
}

export const useDoctorStore = defineStore('doctor', () => {
  const appStore = useAppStore()
  const lockStore = useLockStore()

  const phase = ref<DoctorPhase>('idle')
  const currentRun = ref<DoctorRun | null>(null)
  const history = ref<DoctorRun[]>([])
  const isHistoryLoading = ref(false)
  const selectedRunId = ref<string | null>(null)
  const checkRuntime = ref<DoctorCheckRuntime[]>([])
  const error = ref<string | null>(null)
  const xplaneRunning = ref(false)
  const fixingId = ref<string | null>(null)
  const repairingAll = ref(false)
  const hasAutoRunThisSession = ref(false)
  const appDataPath = ref<string | null>(null)
  const currentRunIsLive = ref(false)
  const currentRunPath = ref<string | null>(null)

  let activeRunToken = 0
  let historyLoadToken = 0
  let cancelRequested = false
  let lastContext: DoctorCheckContext | null = null
  let cancelActiveWork: (() => void) | null = null
  let activeCancellation: Promise<void> = new Promise(() => {})

  function createCancellationSignal() {
    activeCancellation = new Promise<void>((resolve) => {
      cancelActiveWork = resolve
    })
  }

  const displayedRun = computed(() => {
    if (selectedRunId.value) {
      const selected = history.value.find((run) => run.id === selectedRunId.value)
      if (selected) return selected
    }
    return currentRun.value
  })

  const isRunning = computed(() => currentRun.value?.state === 'running')
  const isDisplayingCurrentRun = computed(() =>
    Boolean(currentRun.value && displayedRun.value?.id === currentRun.value.id),
  )
  const lastCompletedRun = computed(() => {
    if (currentRunIsLive.value && currentRun.value?.state !== 'running') return currentRun.value
    return history.value[0] ?? null
  })
  const isStale = computed(() => isDoctorRunStale(lastCompletedRun.value))
  const canRepairCurrentRun = computed(
    () =>
      Boolean(currentRun.value) &&
      currentRun.value?.state === 'completed' &&
      currentRunIsLive.value &&
      !selectedRunId.value &&
      currentRunPath.value === appStore.xplanePath &&
      !fixingId.value &&
      !repairingAll.value &&
      !xplaneRunning.value,
  )

  function discardCurrentRun() {
    activeRunToken += 1
    cancelRequested = true
    cancelActiveWork?.()
    cancelActiveWork = null
    phase.value = 'idle'
    currentRun.value = null
    selectedRunId.value = null
    checkRuntime.value = []
    xplaneRunning.value = false
    appDataPath.value = null
    currentRunIsLive.value = false
    currentRunPath.value = null
    lastContext = null
  }

  function reset() {
    historyLoadToken += 1
    discardCurrentRun()
    history.value = []
    isHistoryLoading.value = false
    error.value = null
    fixingId.value = null
    repairingAll.value = false
  }

  async function loadHistory(): Promise<void> {
    const xplanePath = appStore.xplanePath
    const requestToken = ++historyLoadToken
    selectedRunId.value = null

    if (currentRunPath.value && currentRunPath.value !== xplanePath) {
      discardCurrentRun()
    }

    if (!xplanePath) {
      history.value = []
      if (currentRun.value) discardCurrentRun()
      isHistoryLoading.value = false
      return
    }

    isHistoryLoading.value = true
    try {
      const loadedHistory = await loadDoctorRunsForPath(xplanePath)
      if (requestToken !== historyLoadToken || appStore.xplanePath !== xplanePath) return

      history.value = loadedHistory
      const preserveLiveResult =
        currentRunIsLive.value && currentRunPath.value === xplanePath && Boolean(currentRun.value)
      if (!isRunning.value && !preserveLiveResult) {
        currentRun.value = history.value[0] ?? null
        currentRunIsLive.value = false
        currentRunPath.value = xplanePath
        lastContext = null
        appDataPath.value = null
      }
    } catch (reason) {
      logError(`Health: history load failed: ${reason}`, 'doctor')
      if (requestToken !== historyLoadToken || appStore.xplanePath !== xplanePath) return
      history.value = []
      const preserveLiveResult =
        currentRunIsLive.value && currentRunPath.value === xplanePath && Boolean(currentRun.value)
      if (!preserveLiveResult) {
        currentRun.value = null
        currentRunIsLive.value = false
        currentRunPath.value = xplanePath
        lastContext = null
        appDataPath.value = null
      }
    } finally {
      if (requestToken === historyLoadToken) isHistoryLoading.value = false
    }
  }

  function selectHistoryRun(runId: string | null) {
    if (isRunning.value) return
    selectedRunId.value = runId && history.value.some((run) => run.id === runId) ? runId : null
  }

  function updateRunChecks(runToken: number, results: DoctorCheckResult[]) {
    if (activeRunToken !== runToken || !currentRun.value) return
    const checks = sortDoctorChecks([...currentRun.value.checks, ...results])
    currentRun.value = {
      ...currentRun.value,
      checks,
      summary: summarizeDoctorRun(checks, currentRun.value.state),
      system: lastContext?.system ?? currentRun.value.system,
    }
    xplaneRunning.value = lastContext?.xplaneRunning ?? xplaneRunning.value
    appDataPath.value = lastContext?.xfast?.appDataDir ?? appDataPath.value
  }

  function setRuntimeState(
    definition: DoctorCheckDefinition,
    state: DoctorCheckRuntime['state'],
    runToken: number,
  ) {
    if (activeRunToken !== runToken) return
    const index = checkRuntime.value.findIndex((runtime) => runtime.id === definition.id)
    const previous = index >= 0 ? checkRuntime.value[index] : null
    const now = Date.now()
    const runtime: DoctorCheckRuntime = {
      id: definition.id,
      section: definition.section,
      state,
      startedAt: state === 'running' ? now : previous?.startedAt,
      completedAt: state === 'completed' || state === 'cancelled' ? now : undefined,
    }
    if (index >= 0) checkRuntime.value.splice(index, 1, runtime)
    else checkRuntime.value.push(runtime)
  }

  async function executeDefinition(
    definition: DoctorCheckDefinition,
    runToken: number,
  ): Promise<DoctorCheckResult[]> {
    if (cancelRequested || activeRunToken !== runToken) {
      setRuntimeState(definition, 'cancelled', runToken)
      return [cancelledResult(definition)]
    }

    setRuntimeState(definition, 'running', runToken)
    const startedAt = performance.now()
    try {
      const results = await runWithTimeout(
        definition.run(),
        definition.id,
        definition.timeoutMs,
        activeCancellation,
      )
      const durationMs = Math.round(performance.now() - startedAt)
      if (cancelRequested || activeRunToken !== runToken) {
        setRuntimeState(definition, 'cancelled', runToken)
        return [{ ...cancelledResult(definition), durationMs }]
      }
      setRuntimeState(definition, 'completed', runToken)
      return results.map((result) => ({ ...result, durationMs }))
    } catch (reason) {
      const durationMs = Math.round(performance.now() - startedAt)
      if (
        reason instanceof DoctorCancelledError ||
        cancelRequested ||
        activeRunToken !== runToken
      ) {
        setRuntimeState(definition, 'cancelled', runToken)
        return [{ ...cancelledResult(definition), durationMs }]
      }
      setRuntimeState(definition, 'completed', runToken)
      logError(`Health check ${definition.id} failed: ${reason}`, 'doctor')
      return [unavailableResult(definition, reason, durationMs)]
    }
  }

  async function finishRun(runToken: number, state: 'completed' | 'cancelled', runPath: string) {
    if (activeRunToken !== runToken || !currentRun.value || currentRunPath.value !== runPath) {
      return
    }
    const completedAt = Date.now()
    const checks = sortDoctorChecks(currentRun.value.checks)
    const finished: DoctorRun = {
      ...currentRun.value,
      state,
      completedAt,
      durationMs: completedAt - currentRun.value.startedAt,
      checks,
      summary: summarizeDoctorRun(checks, state),
      installationId: lastContext?.installationId || currentRun.value.installationId,
      system: lastContext?.system ?? currentRun.value.system,
    }
    currentRun.value = finished
    phase.value = 'done'
    cancelActiveWork = null
    xplaneRunning.value = lastContext?.xplaneRunning ?? false
    appDataPath.value = lastContext?.xfast?.appDataDir ?? null

    if (state === 'completed' && finished.installationId) {
      try {
        const savedHistory = await saveDoctorRun(runPath, finished)
        if (
          activeRunToken === runToken &&
          currentRunPath.value === runPath &&
          appStore.xplanePath === runPath
        ) {
          history.value = savedHistory
        }
      } catch (reason) {
        logError(`Health: history save failed: ${reason}`, 'doctor')
      }
    }
  }

  async function runDiagnostics(mode: DoctorRunMode = 'quick'): Promise<void> {
    if (isRunning.value) return
    const xplanePath = appStore.xplanePath
    if (!xplanePath) {
      reset()
      error.value = 'xplane_path_missing'
      return
    }

    activeRunToken += 1
    const runToken = activeRunToken
    cancelRequested = false
    createCancellationSignal()
    selectedRunId.value = null
    error.value = null
    phase.value = 'local'

    const startedAt = Date.now()
    currentRun.value = {
      schemaVersion: 1,
      id: createRunId(),
      installationId: '',
      mode,
      state: 'running',
      startedAt,
      completedAt: null,
      durationMs: 0,
      appVersion: 'unknown',
      checks: [],
      summary: summarizeDoctorRun([], 'running'),
      system: null,
    }
    currentRunIsLive.value = true
    currentRunPath.value = xplanePath

    try {
      const appVersion = await runWithTimeout(
        invoke<string>('get_app_version'),
        'app_version',
        APP_VERSION_TIMEOUT_MS,
        activeCancellation,
      )
      if (activeRunToken !== runToken || currentRunPath.value !== xplanePath) return
      currentRun.value = currentRun.value ? { ...currentRun.value, appVersion } : null
    } catch (reason) {
      if (
        reason instanceof DoctorCancelledError ||
        cancelRequested ||
        activeRunToken !== runToken
      ) {
        await finishRun(runToken, 'cancelled', xplanePath)
        return
      }
      logError(`Health: app version unavailable: ${reason}`, 'doctor')
    }

    if (activeRunToken !== runToken || currentRunPath.value !== xplanePath) return

    try {
      const valid = await runWithTimeout(
        invoke<boolean>('validate_xplane_path', { path: xplanePath }),
        'path_validation',
        PATH_VALIDATION_TIMEOUT_MS,
        activeCancellation,
      )
      if (!valid) {
        updateRunChecks(runToken, [
          {
            id: 'installation.structure',
            section: 'installation',
            outcome: 'critical',
            durationMs: 0,
            remediation: {
              id: 'configure_xplane_path',
              kind: 'navigate',
              risk: 'safe',
              route: '/settings',
            },
          },
        ])
        await finishRun(runToken, 'completed', xplanePath)
        return
      }
    } catch (reason) {
      if (
        reason instanceof DoctorCancelledError ||
        cancelRequested ||
        activeRunToken !== runToken
      ) {
        await finishRun(runToken, 'cancelled', xplanePath)
        return
      }
      logError(`Health: path validation unavailable: ${reason}`, 'doctor')
    }

    if (activeRunToken !== runToken || currentRunPath.value !== xplanePath) return

    const context: DoctorCheckContext = {
      xplanePath,
      mode,
      crashAnalysisDmpEnabled: appStore.crashAnalysisDmpEnabled,
      crashAnalysisIgnoreDateCheck: appStore.crashAnalysisIgnoreDateCheck,
      installationId: '',
      xplaneRunning: false,
      environment: null,
      xfast: null,
      log: null,
      system: null,
    }
    lastContext = context

    const definitions = createDoctorCheckDefinitions(context).filter((definition) =>
      definition.modes.includes(mode),
    )
    checkRuntime.value = definitions.map((definition) => ({
      id: definition.id,
      section: definition.section,
      state: 'pending',
    }))

    const environment = definitions.find((definition) => definition.id === 'environment')
    if (environment) {
      updateRunChecks(runToken, await executeDefinition(environment, runToken))
    }

    const localDefinitions = definitions.filter(
      (definition) => definition.phase === 'local' && definition.id !== 'environment',
    )
    await runBounded(localDefinitions, LOCAL_CONCURRENCY, async (definition) => {
      updateRunChecks(runToken, await executeDefinition(definition, runToken))
    })

    const networkDefinitions = definitions.filter((definition) => definition.phase === 'network')
    if (networkDefinitions.length && activeRunToken === runToken) phase.value = 'network'
    await runBounded(networkDefinitions, NETWORK_CONCURRENCY, async (definition) => {
      updateRunChecks(runToken, await executeDefinition(definition, runToken))
    })

    await finishRun(runToken, cancelRequested ? 'cancelled' : 'completed', xplanePath)
  }

  async function runAutomaticQuickCheck(): Promise<void> {
    if (hasAutoRunThisSession.value) return
    hasAutoRunThisSession.value = true
    await runDiagnostics('quick')
  }

  function cancelRun() {
    if (!isRunning.value) return
    cancelRequested = true
    cancelActiveWork?.()
  }

  async function lockedSceneryFolders(): Promise<string[]> {
    if (!lockStore.isInitialized) await lockStore.initStore()
    return lockStore.getLockedItems('scenery')
  }

  async function ensureFilesystemFixIsSafe(remediation: DoctorRemediation): Promise<boolean> {
    if (!FILESYSTEM_REMEDIATIONS.has(remediation.id)) return true
    try {
      xplaneRunning.value = await invoke<boolean>('is_xplane_running')
    } catch (reason) {
      error.value = String(reason)
      return false
    }
    if (xplaneRunning.value) {
      error.value = 'xplane_running'
      return false
    }
    return true
  }

  async function executeAutomaticRemediation(
    remediation: DoctorRemediation,
    xplanePath: string,
  ): Promise<void> {
    switch (remediation.id) {
      case 'sort_scenery':
        await invoke('sort_scenery_packs', {
          xplanePath,
          lockedFolderNames: await lockedSceneryFolders(),
        })
        return
      case 'enable_global_airports':
        await invoke('update_scenery_entry', {
          xplanePath,
          folderName: '*GLOBAL_AIRPORTS*',
          enabled: true,
          sortOrder: null,
          category: null,
        })
        return
      case 'apply_flatten':
        await invoke('airport_flatten_apply_all_drifted', { xplanePath })
        return
      case 'refresh_scenery_index':
        await invoke('quick_scan_scenery_index', {
          xplanePath,
          lockedFolderNames: await lockedSceneryFolders(),
        })
        return
      case 'rebuild_scenery_index':
        await invoke('rebuild_scenery_index', { xplanePath })
        return
      default:
        throw new Error(`Unsupported automatic remediation: ${remediation.id}`)
    }
  }

  function remediationDefinitionId(remediationId: string): string | null {
    if (['sort_scenery', 'enable_global_airports', 'apply_flatten'].includes(remediationId)) {
      return 'scenery'
    }
    if (['refresh_scenery_index', 'rebuild_scenery_index'].includes(remediationId)) return 'xfast'
    return null
  }

  async function recheckDefinitions(definitionIds: Set<string>, runPath: string): Promise<void> {
    if (
      !lastContext ||
      lastContext.xplanePath !== runPath ||
      !currentRun.value ||
      currentRunPath.value !== runPath ||
      appStore.xplanePath !== runPath ||
      definitionIds.size === 0
    ) {
      return
    }
    const definitions = createDoctorCheckDefinitions(lastContext).filter((definition) =>
      definitionIds.has(definition.id),
    )
    if (!definitions.length) return

    activeRunToken += 1
    const runToken = activeRunToken
    cancelRequested = false
    createCancellationSignal()
    phase.value = 'local'
    currentRun.value = { ...currentRun.value, state: 'running', completedAt: null }
    checkRuntime.value = definitions.map((definition) => ({
      id: definition.id,
      section: definition.section,
      state: 'pending',
    }))

    const replacementIds = new Set(
      definitions.flatMap((definition) => DEFINITION_RESULT_IDS[definition.id] ?? []),
    )
    currentRun.value = {
      ...currentRun.value,
      checks: currentRun.value.checks.filter((check) => !replacementIds.has(check.id)),
    }
    await runBounded(definitions, LOCAL_CONCURRENCY, async (definition) => {
      updateRunChecks(runToken, await executeDefinition(definition, runToken))
    })
    await finishRun(runToken, cancelRequested ? 'cancelled' : 'completed', runPath)
  }

  async function applyRemediation(check: DoctorCheckResult, recheck = true): Promise<boolean> {
    const currentCheck = currentRun.value?.checks.find((item) => item.id === check.id)
    const remediation = currentCheck?.remediation
    const repairPath = currentRunPath.value
    if (!canRepairCurrentRun.value || !repairPath || remediation?.kind !== 'automatic') {
      return false
    }
    if (!(await ensureFilesystemFixIsSafe(remediation))) return false
    if (
      currentRunPath.value !== repairPath ||
      appStore.xplanePath !== repairPath ||
      currentRun.value?.state !== 'completed'
    ) {
      return false
    }

    fixingId.value = check.id
    error.value = null
    try {
      await executeAutomaticRemediation(remediation, repairPath)
      const definitionId = remediationDefinitionId(remediation.id)
      if (
        recheck &&
        definitionId &&
        currentRunPath.value === repairPath &&
        appStore.xplanePath === repairPath
      ) {
        await recheckDefinitions(new Set([definitionId]), repairPath)
      }
      return true
    } catch (reason) {
      logError(`Health: remediation ${remediation.id} failed: ${reason}`, 'doctor')
      error.value = String(reason)
      return false
    } finally {
      fixingId.value = null
    }
  }

  async function applyAllSafeFixes(): Promise<DoctorBatchFixResult> {
    const repairPath = currentRunPath.value
    if (!canRepairCurrentRun.value || !isDisplayingCurrentRun.value || !repairPath) {
      return { applied: 0, failed: 0 }
    }
    const checks = (currentRun.value?.checks ?? []).filter(
      (check) => check.remediation?.kind === 'automatic' && check.remediation.risk === 'safe',
    )
    if (!checks.length) return { applied: 0, failed: 0 }

    const filesystemRemediation = checks
      .map((check) => check.remediation)
      .find((remediation): remediation is DoctorRemediation =>
        Boolean(remediation && FILESYSTEM_REMEDIATIONS.has(remediation.id)),
      )
    if (filesystemRemediation && !(await ensureFilesystemFixIsSafe(filesystemRemediation))) {
      return { applied: 0, failed: checks.length }
    }
    if (currentRunPath.value !== repairPath || appStore.xplanePath !== repairPath) {
      return { applied: 0, failed: checks.length }
    }

    repairingAll.value = true
    error.value = null
    let applied = 0
    let failed = 0
    const definitionsToRecheck = new Set<string>()
    try {
      for (const check of checks) {
        if (!check.remediation) continue
        if (currentRunPath.value !== repairPath || appStore.xplanePath !== repairPath) {
          failed += checks.length - applied - failed
          break
        }
        fixingId.value = check.id
        try {
          await executeAutomaticRemediation(check.remediation, repairPath)
          applied += 1
          const definitionId = remediationDefinitionId(check.remediation.id)
          if (definitionId) definitionsToRecheck.add(definitionId)
        } catch (reason) {
          failed += 1
          logError(`Health: remediation ${check.remediation.id} failed: ${reason}`, 'doctor')
        }
      }
      if (currentRunPath.value === repairPath && appStore.xplanePath === repairPath) {
        await recheckDefinitions(definitionsToRecheck, repairPath)
      }
      return { applied, failed }
    } finally {
      fixingId.value = null
      repairingAll.value = false
    }
  }

  async function exportReport(
    exportPath: string,
    format: DoctorReportFormat,
    includeSensitive = false,
    labels?: {
      checkLabel?: (check: DoctorCheckResult) => string
      sectionLabel?: (check: DoctorCheckResult) => string
    },
  ): Promise<void> {
    const run = displayedRun.value
    if (!run) throw new Error('No health report is available')
    const content = buildDoctorReport(run, format, {
      xplanePath: currentRunPath.value ?? appStore.xplanePath,
      appDataPath: appDataPath.value,
      includeSensitive,
      ...labels,
    })
    await invoke('doctor_export_report', { exportPath, content })
  }

  return {
    phase,
    currentRun,
    displayedRun,
    history,
    isHistoryLoading,
    selectedRunId,
    checkRuntime,
    error,
    xplaneRunning,
    fixingId,
    repairingAll,
    hasAutoRunThisSession,
    appDataPath,
    currentRunIsLive,
    isRunning,
    isDisplayingCurrentRun,
    canRepairCurrentRun,
    isStale,
    lastCompletedRun,
    loadHistory,
    selectHistoryRun,
    runDiagnostics,
    runAutomaticQuickCheck,
    cancelRun,
    applyRemediation,
    applyAllSafeFixes,
    exportReport,
    reset,
  }
})
