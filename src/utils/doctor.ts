import type {
  DoctorCheckOutcome,
  DoctorCheckResult,
  DoctorCompleteness,
  DoctorHistoryFile,
  DoctorRun,
  DoctorRunState,
  DoctorRunSummary,
  DoctorSection,
  DoctorSeverity,
} from '@/types/doctor'

export const DOCTOR_HISTORY_LIMIT = 20
export const DOCTOR_RESULT_STALE_MS = 24 * 60 * 60 * 1000

export const DOCTOR_SECTION_ORDER: DoctorSection[] = [
  'installation',
  'stability',
  'scenery',
  'addons',
  'navdata',
  'performance',
  'storage',
  'system',
  'xfast',
  'updates',
]

const OUTCOME_ORDER: Record<DoctorCheckOutcome, number> = {
  critical: 0,
  warning: 1,
  unavailable: 2,
  info: 3,
  pass: 4,
  notApplicable: 5,
  cancelled: 6,
}

const DOCTOR_RUN_MODES = new Set(['quick', 'full'])
const DOCTOR_RUN_STATES = new Set(['completed'])
const DOCTOR_OUTCOMES = new Set(Object.keys(OUTCOME_ORDER))
const DOCTOR_EVIDENCE_KINDS = new Set(['text', 'path', 'log'])
const DOCTOR_REMEDIATION_KINDS = new Set(['automatic', 'navigate', 'guide'])
const DOCTOR_REMEDIATION_RISKS = new Set(['safe', 'confirm', 'destructive'])

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function isFiniteNonNegativeNumber(value: unknown): value is number {
  return isFiniteNumber(value) && value >= 0
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}

function isOptionalParams(value: unknown): boolean {
  return (
    value === undefined ||
    (isRecord(value) &&
      Object.values(value).every((item) => typeof item === 'string' || isFiniteNumber(item)))
  )
}

function isDoctorSystemSnapshot(value: unknown): boolean {
  if (!isRecord(value)) return false
  return (
    typeof value.os === 'string' &&
    isNullableString(value.osVersion) &&
    typeof value.architecture === 'string' &&
    isNullableString(value.cpuModel) &&
    isFiniteNonNegativeNumber(value.logicalCores) &&
    isFiniteNonNegativeNumber(value.totalMemoryBytes) &&
    isFiniteNonNegativeNumber(value.availableMemoryBytes) &&
    isNullableString(value.xplaneVersion) &&
    isNullableString(value.xplaneVersionRaw) &&
    typeof value.isBeta === 'boolean' &&
    isNullableString(value.gpuModel) &&
    isNullableString(value.gpuDriver) &&
    (value.xplaneFreeBytes === null || isFiniteNonNegativeNumber(value.xplaneFreeBytes)) &&
    (value.xplaneTotalBytes === null || isFiniteNonNegativeNumber(value.xplaneTotalBytes)) &&
    (value.appDataFreeBytes === null || isFiniteNonNegativeNumber(value.appDataFreeBytes)) &&
    (value.appDataTotalBytes === null || isFiniteNonNegativeNumber(value.appDataTotalBytes))
  )
}

function isDoctorCheckResult(value: unknown): value is DoctorCheckResult {
  if (!isRecord(value)) return false
  if (
    typeof value.id !== 'string' ||
    !DOCTOR_SECTION_ORDER.includes(value.section as DoctorSection)
  ) {
    return false
  }
  if (!DOCTOR_OUTCOMES.has(String(value.outcome))) return false
  if (!isFiniteNonNegativeNumber(value.durationMs) || !isOptionalParams(value.params)) return false
  if (
    value.evidence !== undefined &&
    (!Array.isArray(value.evidence) ||
      !value.evidence.every(
        (item) =>
          isRecord(item) &&
          DOCTOR_EVIDENCE_KINDS.has(String(item.kind)) &&
          typeof item.value === 'string' &&
          (item.label === undefined || typeof item.label === 'string') &&
          (item.sensitive === undefined || typeof item.sensitive === 'boolean'),
      ))
  ) {
    return false
  }
  if (
    value.remediation !== undefined &&
    (!isRecord(value.remediation) ||
      typeof value.remediation.id !== 'string' ||
      !DOCTOR_REMEDIATION_KINDS.has(String(value.remediation.kind)) ||
      !DOCTOR_REMEDIATION_RISKS.has(String(value.remediation.risk)) ||
      (value.remediation.route !== undefined && typeof value.remediation.route !== 'string') ||
      !isOptionalParams(value.remediation.params))
  ) {
    return false
  }
  return true
}

function isStoredDoctorRun(value: unknown): value is DoctorRun {
  if (!isRecord(value) || value.schemaVersion !== 1) return false
  if (
    typeof value.id !== 'string' ||
    !value.id ||
    typeof value.installationId !== 'string' ||
    !value.installationId ||
    typeof value.appVersion !== 'string' ||
    !DOCTOR_RUN_MODES.has(String(value.mode)) ||
    !DOCTOR_RUN_STATES.has(String(value.state)) ||
    !isFiniteNonNegativeNumber(value.startedAt) ||
    !isFiniteNonNegativeNumber(value.completedAt) ||
    !isFiniteNonNegativeNumber(value.durationMs) ||
    !Array.isArray(value.checks) ||
    !value.checks.every(isDoctorCheckResult) ||
    (value.system !== null && !isDoctorSystemSnapshot(value.system))
  ) {
    return false
  }
  return true
}

export function summarizeDoctorRun(
  checks: DoctorCheckResult[],
  runState: DoctorRunState,
): DoctorRunSummary {
  const counts: Record<DoctorCheckOutcome, number> = {
    pass: 0,
    info: 0,
    warning: 0,
    critical: 0,
    unavailable: 0,
    notApplicable: 0,
    cancelled: 0,
  }

  for (const check of checks) counts[check.outcome] += 1

  const severity: DoctorSeverity = counts.critical
    ? 'critical'
    : counts.warning
      ? 'warning'
      : counts.info
        ? 'info'
        : 'ok'

  const completeness: DoctorCompleteness =
    runState === 'cancelled' || counts.cancelled > 0
      ? 'cancelled'
      : runState === 'running' || counts.unavailable > 0
        ? 'partial'
        : 'complete'

  const eligible = checks.length - counts.notApplicable
  const covered = counts.pass + counts.info + counts.warning + counts.critical
  const coveragePercent =
    eligible === 0 ? (runState === 'running' ? 0 : 100) : Math.round((covered / eligible) * 100)

  return {
    severity,
    completeness,
    total: checks.length,
    eligible,
    covered,
    coveragePercent,
    ...counts,
  }
}

export function sortDoctorChecks(checks: DoctorCheckResult[]): DoctorCheckResult[] {
  return [...checks].sort((a, b) => {
    const sectionDelta =
      DOCTOR_SECTION_ORDER.indexOf(a.section) - DOCTOR_SECTION_ORDER.indexOf(b.section)
    if (sectionDelta !== 0) return sectionDelta
    const outcomeDelta = OUTCOME_ORDER[a.outcome] - OUTCOME_ORDER[b.outcome]
    return outcomeDelta || a.id.localeCompare(b.id)
  })
}

export function normalizeDoctorHistoryPath(path: string): string {
  const normalized = path.trim().replace(/\\/g, '/').replace(/\/+$/, '')
  return /^[A-Za-z]:\//.test(normalized) || normalized.startsWith('//')
    ? normalized.toLocaleLowerCase('en-US')
    : normalized
}

export function emptyDoctorHistory(): DoctorHistoryFile {
  return { schemaVersion: 1, installationByPath: {}, runsByInstallation: {} }
}

export function sanitizeDoctorHistory(value: unknown): DoctorHistoryFile {
  if (
    !isRecord(value) ||
    value.schemaVersion !== 1 ||
    !isRecord(value.installationByPath) ||
    !isRecord(value.runsByInstallation)
  ) {
    return emptyDoctorHistory()
  }

  const runsByInstallation: Record<string, DoctorRun[]> = {}
  for (const [installationId, candidateRuns] of Object.entries(value.runsByInstallation)) {
    if (!Array.isArray(candidateRuns)) continue
    const seenRunIds = new Set<string>()
    const runs = candidateRuns
      .filter(isStoredDoctorRun)
      .filter((run) => run.installationId === installationId)
      .sort((a, b) => b.startedAt - a.startedAt)
      .filter((run) => {
        if (seenRunIds.has(run.id)) return false
        seenRunIds.add(run.id)
        return true
      })
      .slice(0, DOCTOR_HISTORY_LIMIT)
      .map((run) => ({
        ...run,
        checks: sortDoctorChecks(run.checks),
        summary: summarizeDoctorRun(run.checks, run.state),
      }))
    if (runs.length) runsByInstallation[installationId] = runs
  }

  const installationByPath = Object.fromEntries(
    Object.entries(value.installationByPath).filter(
      ([path, installationId]) =>
        Boolean(path) &&
        typeof installationId === 'string' &&
        Boolean(runsByInstallation[installationId]),
    ),
  ) as Record<string, string>

  return { schemaVersion: 1, installationByPath, runsByInstallation }
}

export function addDoctorHistoryRun(
  history: DoctorHistoryFile,
  xplanePath: string,
  run: DoctorRun,
  limit = DOCTOR_HISTORY_LIMIT,
): DoctorHistoryFile {
  const pathKey = normalizeDoctorHistoryPath(xplanePath)
  const current = history.runsByInstallation[run.installationId] ?? []
  const runs = [run, ...current.filter((item) => item.id !== run.id)]
    .sort((a, b) => b.startedAt - a.startedAt)
    .slice(0, limit)

  return {
    schemaVersion: 1,
    installationByPath: { ...history.installationByPath, [pathKey]: run.installationId },
    runsByInstallation: { ...history.runsByInstallation, [run.installationId]: runs },
  }
}

export function getDoctorHistoryForPath(
  history: DoctorHistoryFile,
  xplanePath: string,
): DoctorRun[] {
  const installationId = history.installationByPath[normalizeDoctorHistoryPath(xplanePath)]
  return installationId ? (history.runsByInstallation[installationId] ?? []) : []
}

export function isDoctorRunStale(run: DoctorRun | null, now = Date.now()): boolean {
  if (!run?.completedAt) return true
  return now - run.completedAt >= DOCTOR_RESULT_STALE_MS
}
