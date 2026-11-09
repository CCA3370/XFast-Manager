import { onBeforeUnmount, ref } from 'vue'
import { importFlightPlan } from '@/services/flightPlanImport'
import type {
  FlightPlanConflict,
  FlightPlanConflictDecision,
  FlightPlanImportOutcome,
} from '@/types/flightPlans'

export function useFlightPlanImport() {
  const conflict = ref<FlightPlanConflict | null>(null)
  let pending: ((decision: FlightPlanConflictDecision) => void) | null = null
  let disposed = false
  let tail = Promise.resolve()

  function resolveConflict(decision: FlightPlanConflictDecision) {
    const resolve = pending
    pending = null
    conflict.value = null
    resolve?.(decision)
  }

  function importPlan(xplanePath: string, source: string): Promise<FlightPlanImportOutcome> {
    const task = tail.then(() => {
      if (disposed)
        return { status: 'cancelled' as const, name: source.split(/[\\/]/).pop() ?? source }
      return importFlightPlan(
        xplanePath,
        source,
        (details) =>
          new Promise((resolve) => {
            if (disposed) resolve({ action: 'cancel', remember: false })
            else {
              pending = resolve
              conflict.value = details
            }
          }),
      )
    })
    tail = task.then(
      () => {},
      () => {},
    )
    return task
  }

  onBeforeUnmount(() => {
    disposed = true
    resolveConflict({ action: 'cancel', remember: false })
  })

  return { conflict, importPlan, resolveConflict }
}
