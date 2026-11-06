import { invoke } from '@tauri-apps/api/core'
import { getItem, setItem, STORAGE_KEYS } from './storage'
import type {
  FlightPlanConflict,
  FlightPlanConflictAction,
  FlightPlanConflictDecision,
  FlightPlanImportOutcome,
  FlightPlanImportResult,
} from '@/types/flightPlans'

export async function loadFlightPlanConflictAction(): Promise<FlightPlanConflictAction | null> {
  const value = await getItem<unknown>(STORAGE_KEYS.FLIGHT_PLAN_CONFLICT_ACTION)
  return value === 'replace' || value === 'rename' ? value : null
}

export async function saveFlightPlanConflictAction(action: FlightPlanConflictAction | null) {
  await setItem(STORAGE_KEYS.FLIGHT_PLAN_CONFLICT_ACTION, action)
}

export async function importFlightPlan(
  xplanePath: string,
  source: string,
  resolveConflict: (conflict: FlightPlanConflict) => Promise<FlightPlanConflictDecision>,
): Promise<FlightPlanImportOutcome> {
  const conflictAction = await loadFlightPlanConflictAction()
  const request = (action: FlightPlanConflictAction | null) =>
    invoke<FlightPlanImportResult>('import_flight_plan', {
      xplanePath,
      source,
      conflictAction: action,
    })
  let result = await request(conflictAction)
  let remembered: FlightPlanConflictAction | null = null
  while (result.status === 'conflict') {
    const decision = await resolveConflict(result)
    if (decision.action === 'cancel') return { status: 'cancelled', name: result.name }
    remembered = decision.remember ? decision.action : null
    result = await request(decision.action)
  }
  if (remembered) {
    try {
      await saveFlightPlanConflictAction(remembered)
    } catch {
      // The file was already imported: report the preference failure separately.
      return { ...result, preferenceSaveFailed: true }
    }
  }
  return result
}
