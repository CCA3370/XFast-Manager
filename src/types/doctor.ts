export type DoctorRunMode = 'quick' | 'full'

export type DoctorRunState = 'running' | 'completed' | 'cancelled'

export type DoctorSeverity = 'critical' | 'warning' | 'info' | 'ok'

export type DoctorCompleteness = 'complete' | 'partial' | 'cancelled'

export type DoctorCheckOutcome =
  | 'pass'
  | 'info'
  | 'warning'
  | 'critical'
  | 'unavailable'
  | 'notApplicable'
  | 'cancelled'

export type DoctorCheckRuntimeState = 'pending' | 'running' | 'completed' | 'cancelled'

export type DoctorSection =
  | 'installation'
  | 'stability'
  | 'scenery'
  | 'addons'
  | 'navdata'
  | 'performance'
  | 'storage'
  | 'system'
  | 'xfast'
  | 'updates'

export type DoctorRemediationKind = 'automatic' | 'navigate' | 'guide'

export type DoctorRemediationRisk = 'safe' | 'confirm' | 'destructive'

export interface DoctorEvidence {
  kind: 'text' | 'path' | 'log'
  value: string
  label?: string
  sensitive?: boolean
}

export interface DoctorRemediation {
  id: string
  kind: DoctorRemediationKind
  risk: DoctorRemediationRisk
  route?: string
  params?: Record<string, string | number>
}

export interface DoctorCheckResult {
  id: string
  section: DoctorSection
  outcome: DoctorCheckOutcome
  params?: Record<string, string | number>
  evidence?: DoctorEvidence[]
  remediation?: DoctorRemediation
  durationMs: number
}

export interface DoctorCheckRuntime {
  id: string
  section: DoctorSection
  state: DoctorCheckRuntimeState
  startedAt?: number
  completedAt?: number
}

export interface DoctorRunSummary {
  severity: DoctorSeverity
  completeness: DoctorCompleteness
  total: number
  eligible: number
  covered: number
  coveragePercent: number
  pass: number
  info: number
  warning: number
  critical: number
  unavailable: number
  notApplicable: number
  cancelled: number
}

export interface DoctorSystemSnapshot {
  os: string
  osVersion: string | null
  architecture: string
  cpuModel: string | null
  logicalCores: number
  totalMemoryBytes: number
  availableMemoryBytes: number
  xplaneVersion: string | null
  xplaneVersionRaw: string | null
  isBeta: boolean
  gpuModel: string | null
  gpuDriver: string | null
  xplaneFreeBytes: number | null
  xplaneTotalBytes: number | null
  appDataFreeBytes: number | null
  appDataTotalBytes: number | null
}

export interface DoctorRun {
  schemaVersion: 1
  id: string
  installationId: string
  mode: DoctorRunMode
  state: DoctorRunState
  startedAt: number
  completedAt: number | null
  durationMs: number
  appVersion: string
  checks: DoctorCheckResult[]
  summary: DoctorRunSummary
  system: DoctorSystemSnapshot | null
}

export interface DoctorHistoryFile {
  schemaVersion: 1
  installationByPath: Record<string, string>
  runsByInstallation: Record<string, DoctorRun[]>
}
