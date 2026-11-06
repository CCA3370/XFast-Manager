export type FlightPlanConflictAction = 'replace' | 'rename'

export interface FlightPlanConflict {
  status: 'conflict'
  name: string
  existingSize: number
  incomingSize: number
}

export interface FlightPlanConflictDecision {
  action: FlightPlanConflictAction | 'cancel'
  remember: boolean
}

export type FlightPlanImportResult =
  | { status: 'imported'; name: string; renamed: boolean; replaced: boolean }
  | { status: 'duplicate'; name: string }
  | FlightPlanConflict

export type FlightPlanImportOutcome = (
  | Exclude<FlightPlanImportResult, FlightPlanConflict>
  | { status: 'cancelled'; name: string }
) & { preferenceSaveFailed?: boolean }
