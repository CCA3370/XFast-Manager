export interface ErrorReportPolicyInput {
  code?: string
  origin?: string
  operation?: string
  message?: string
  reportable?: boolean
}

export const REPORTABLE_ERROR_CODES: readonly string[]
export const NON_REPORTABLE_ERROR_CODES: readonly string[]
export function isOperationalErrorMessage(message: unknown): boolean
export function isReportableError(input?: ErrorReportPolicyInput): boolean
export function normalizeErrorFingerprintInput(input?: ErrorReportPolicyInput): string
