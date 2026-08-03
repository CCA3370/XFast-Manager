import type {
  DoctorCheckOutcome,
  DoctorCheckResult,
  DoctorCompleteness,
  DoctorRunState,
  DoctorRunSummary,
  DoctorSection,
  DoctorSeverity,
} from '@/types/doctor'

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
