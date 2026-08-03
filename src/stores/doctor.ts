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
  DoctorSection as HealthSection,
  DoctorSeverity as HealthSeverity,
} from '@/types/doctor'

export type DoctorSeverity = 'critical' | 'warning' | 'info' | 'ok'

export type DoctorSection =
  | 'integrity'
  | 'crashes'
  | 'navdata'
  | 'scenery'
  | 'plugins'
  | 'performance'
  | 'environment'
  | 'disk'
  | 'updates'

export type DoctorFixTier = 'safe' | 'confirm' | 'destructive' | 'none'

/** Compatibility shape for the existing Health page while the richer UI is mounted. */
export interface DoctorFinding {
  id: string
  section: DoctorSection
  severity: DoctorSeverity
  params?: Record<string, string | number>
  detail?: string[]
  fix?: {
    id: string
    tier: DoctorFixTier
    params?: Record<string, string | number>
  }
  route?: string
}

export type DoctorPhase = 'idle' | 'local' | 'network' | 'done'

export interface DoctorBatchFixResult {
  applied: number
  failed: number
}

const LOCAL_CONCURRENCY = 4
const NETWORK_CONCURRENCY = 3

const LEGACY_SECTION: Record<HealthSection, DoctorSection> = {
  installation: 'integrity',
  stability: 'crashes',
  scenery: 'scenery',
  addons: 'plugins',
  navdata: 'navdata',
  performance: 'performance',
  storage: 'disk',
  system: 'environment',
  xfast: 'integrity',
  updates: 'updates',
}

const LEGACY_SECTION_ORDER: DoctorSection[] = [
  'integrity',
  'crashes',
  'navdata',
  'scenery',
  'plugins',
  'performance',
  'environment',
  'disk',
  'updates',
]

const LEGACY_SEVERITY_ORDER: Record<DoctorSeverity, number> = {
  critical: 0,
  warning: 1,
  info: 2,
  ok: 3,
}

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

function outcomeSeverity(check: DoctorCheckResult): DoctorSeverity | null {
  if (check.outcome === 'critical' || check.outcome === 'warning' || check.outcome === 'info') {
    return check.outcome
  }
  if (check.outcome === 'unavailable') return 'info'
  return null
}

function legacyFinding(check: DoctorCheckResult): DoctorFinding | null {
  const severity = outcomeSeverity(check)
  if (!severity) return null
  const remediation = check.remediation
  return {
    id: check.id,
    section: LEGACY_SECTION[check.section],
    severity,
    params: check.params,
    detail: check.evidence?.map((item) => item.value),
    fix:
      remediation?.kind === 'automatic'
        ? {
            id: remediation.id,
            tier: remediation.risk,
            params: remediation.params,
          }
        : undefined,
    route: remediation?.route,
  }
}

export const useDoctorStore = defineStore('doctor', () => {
  const appStore = useAppStore()
  const lockStore = useLockStore()

  const phase = ref<DoctorPhase>('idle')
  const currentRun = ref<DoctorRun | null>(null)
  const history = ref<DoctorRun[]>([])
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

  // Compatibility projections for the pre-redesign view.
  const findings = computed(() =>
    (displayedRun.value?.checks ?? [])
      .map(legacyFinding)
      .filter((finding): finding is DoctorFinding => finding !== null),
  )
  const sortedFindings = computed(() =>
    [...findings.value].sort((a, b) => {
      const section =
        LEGACY_SECTION_ORDER.indexOf(a.section) - LEGACY_SECTION_ORDER.indexOf(b.section)
      return section || LEGACY_SEVERITY_ORDER[a.severity] - LEGACY_SEVERITY_ORDER[b.severity]
    }),
  )
  const findingsBySection = computed(() => {
    const groups = new Map<DoctorSection, DoctorFinding[]>()
    for (const finding of sortedFindings.value) {
      const list = groups.get(finding.section) ?? []
      list.push(finding)
      groups.set(finding.section, list)
    }
    return LEGACY_SECTION_ORDER.flatMap((section) => {
      const sectionFindings = groups.get(section)
      return sectionFindings?.length ? [{ section, findings: sectionFindings }] : []
    })
  })
  const counts = computed(() => ({
    critical: displayedRun.value?.summary.critical ?? 0,
    warning: displayedRun.value?.summary.warning ?? 0,
    info: displayedRun.value?.summary.info ?? 0,
    total: findings.value.length,
  }))
  const overallSeverity = computed<HealthSeverity>(
    () => displayedRun.value?.summary.severity ?? 'ok',
  )
  const systemInfo = computed(() => {
    const system = displayedRun.value?.system
    if (!system) return null
    return {
      xplaneVersion: system.xplaneVersion,
      xplaneVersionRaw: system.xplaneVersionRaw,
      isBeta: system.isBeta,
      gpuModel: system.gpuModel,
      gpuDriver: system.gpuDriver,
    }
  })
  const lastRun = computed(() => displayedRun.value?.completedAt ?? null)
  const runningChecks = computed(
    () =>
      new Set(
        checkRuntime.value
          .filter((runtime) => runtime.state === 'running')
          .map((runtime) => runtime.id),
      ),
  )

  function reset() {
    activeRunToken += 1
    cancelRequested = false
    cancelActiveWork?.()
    cancelActiveWork = null
    phase.value = 'idle'
    currentRun.value = null
    selectedRunId.value = null
    checkRuntime.value = []
    error.value = null
    xplaneRunning.value = false
    fixingId.value = null
    repairingAll.value = false
    appDataPath.value = null
    currentRunIsLive.value = false
    currentRunPath.value = null
    lastContext = null
  }

  async function loadHistory(): Promise<void> {
    const xplanePath = appStore.xplanePath
    selectedRunId.value = null
    if (!xplanePath) {
      history.value = []
      if (!isRunning.value) {
        currentRun.value = null
        currentRunPath.value = null
      }
      return
    }
    try {
      history.value = await loadDoctorRunsForPath(xplanePath)
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
      history.value = []
    }
  }

  function selectHistoryRun(runId: string | null) {
    selectedRunId.value = runId
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

  function setRuntimeState(definition: DoctorCheckDefinition, state: DoctorCheckRuntime['state']) {
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
      setRuntimeState(definition, 'cancelled')
      return [cancelledResult(definition)]
    }

    setRuntimeState(definition, 'running')
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
        setRuntimeState(definition, 'cancelled')
        return [{ ...cancelledResult(definition), durationMs }]
      }
      setRuntimeState(definition, 'completed')
      return results.map((result) => ({ ...result, durationMs }))
    } catch (reason) {
      const durationMs = Math.round(performance.now() - startedAt)
      if (
        reason instanceof DoctorCancelledError ||
        cancelRequested ||
        activeRunToken !== runToken
      ) {
        setRuntimeState(definition, 'cancelled')
        return [{ ...cancelledResult(definition), durationMs }]
      }
      setRuntimeState(definition, 'completed')
      logError(`Health check ${definition.id} failed: ${reason}`, 'doctor')
      return [unavailableResult(definition, reason, durationMs)]
    }
  }

  async function finishRun(runToken: number, state: 'completed' | 'cancelled') {
    if (activeRunToken !== runToken || !currentRun.value) return
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
        history.value = await saveDoctorRun(appStore.xplanePath, finished)
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

    let appVersion = 'unknown'
    try {
      appVersion = await invoke<string>('get_app_version')
    } catch (reason) {
      logError(`Health: app version unavailable: ${reason}`, 'doctor')
    }

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
      appVersion,
      checks: [],
      summary: summarizeDoctorRun([], 'running'),
      system: null,
    }
    currentRunIsLive.value = true
    currentRunPath.value = xplanePath

    try {
      const valid = await invoke<boolean>('validate_xplane_path', { path: xplanePath })
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
        await finishRun(runToken, 'completed')
        return
      }
    } catch (reason) {
      logError(`Health: path validation unavailable: ${reason}`, 'doctor')
    }

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
    if (networkDefinitions.length) phase.value = 'network'
    await runBounded(networkDefinitions, NETWORK_CONCURRENCY, async (definition) => {
      updateRunChecks(runToken, await executeDefinition(definition, runToken))
    })

    await finishRun(runToken, cancelRequested ? 'cancelled' : 'completed')
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

  async function executeAutomaticRemediation(remediation: DoctorRemediation): Promise<void> {
    const xplanePath = appStore.xplanePath
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

  async function recheckDefinitions(definitionIds: Set<string>): Promise<void> {
    if (!lastContext || !currentRun.value || definitionIds.size === 0) return
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
    await finishRun(runToken, 'completed')
  }

  async function applyRemediation(check: DoctorCheckResult, recheck = true): Promise<boolean> {
    const remediation = check.remediation
    if (
      !currentRunIsLive.value ||
      !remediation ||
      remediation.kind !== 'automatic' ||
      !appStore.xplanePath
    ) {
      return false
    }
    if (!(await ensureFilesystemFixIsSafe(remediation))) return false

    fixingId.value = check.id
    error.value = null
    try {
      await executeAutomaticRemediation(remediation)
      const definitionId = remediationDefinitionId(remediation.id)
      if (recheck && definitionId) await recheckDefinitions(new Set([definitionId]))
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
    if (repairingAll.value || !isDisplayingCurrentRun.value || !currentRunIsLive.value) {
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

    repairingAll.value = true
    error.value = null
    let applied = 0
    let failed = 0
    const definitionsToRecheck = new Set<string>()
    try {
      for (const check of checks) {
        if (!check.remediation) continue
        fixingId.value = check.id
        try {
          await executeAutomaticRemediation(check.remediation)
          applied += 1
          const definitionId = remediationDefinitionId(check.remediation.id)
          if (definitionId) definitionsToRecheck.add(definitionId)
        } catch (reason) {
          failed += 1
          logError(`Health: remediation ${check.remediation.id} failed: ${reason}`, 'doctor')
        }
      }
      await recheckDefinitions(definitionsToRecheck)
      return { applied, failed }
    } finally {
      fixingId.value = null
      repairingAll.value = false
    }
  }

  // Compatibility adapter for the old page.
  async function applyFix(finding: DoctorFinding): Promise<boolean> {
    const check = currentRun.value?.checks.find((item) => item.id === finding.id)
    return check ? applyRemediation(check) : false
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
      xplanePath: appStore.xplanePath,
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
    isStale,
    lastCompletedRun,
    findings,
    sortedFindings,
    findingsBySection,
    counts,
    overallSeverity,
    systemInfo,
    lastRun,
    runningChecks,
    loadHistory,
    selectHistoryRun,
    runDiagnostics,
    runAutomaticQuickCheck,
    cancelRun,
    applyRemediation,
    applyAllSafeFixes,
    applyFix,
    exportReport,
    reset,
  }
})
