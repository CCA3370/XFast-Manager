import type { DoctorCheckResult } from '@/types/doctor'
import { doctorAdvice } from '@/i18n/doctorAdvice'

type AdviceKey = keyof typeof doctorAdvice

export function getDoctorAdvice(
  check: DoctorCheckResult,
  checks: readonly DoctorCheckResult[],
): AdviceKey[] | undefined {
  const { id, outcome, params, remediation } = check
  if (outcome === 'pass' || outcome === 'cancelled') return []

  switch (id) {
    case 'stability.last_session':
      if (params?.sessionEnd === 'running') return ['activeSession']
      if (outcome === 'unavailable') return ['sessionStateUnavailable']
      if (params?.sessionEnd === 'crash') {
        const oldLog = checks.some(
          (item) => item.id === 'stability.log_freshness' && item.outcome === 'info',
        )
        return [oldLog ? 'oldCrashLog' : 'saveCrashLog']
      }
      return outcome === 'info' ? ['unknownSession'] : []
    case 'stability.log_available':
      if (outcome === 'notApplicable') return ['missingLog']
      return outcome === 'unavailable' ? ['unrecognizedLog'] : []
    case 'stability.log_freshness':
      return outcome === 'info' ? ['oldLog'] : []
    case 'stability.crash_dump':
      if (remediation?.id === 'enable_dmp_analysis') return ['dumpDisabled']
      if (outcome === 'notApplicable') return []
      if (outcome === 'unavailable') return ['dumpUnavailable']
      if (params?.cause === 'plugin_crash' && params.module) return ['dumpPlugin']
      if (params?.cause === 'gpu_driver_crash') return ['dumpGpu']
      if (params?.cause === 'memory_exhaustion') return ['dumpMemory']
      return ['dumpInconclusive']
  }
}
