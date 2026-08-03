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
      : counts.unavailable > 0
        ? 'partial'
        : 'complete'

  const eligible = checks.length - counts.notApplicable
  const covered = counts.pass + counts.info + counts.warning + counts.critical
  const coveragePercent = eligible === 0 ? 100 : Math.round((covered / eligible) * 100)

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
  return /^[A-Za-z]:\//.test(normalized) ? normalized.toLocaleLowerCase('en-US') : normalized
}

export function emptyDoctorHistory(): DoctorHistoryFile {
  return { schemaVersion: 1, installationByPath: {}, runsByInstallation: {} }
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
