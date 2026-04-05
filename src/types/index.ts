// ========== API Error Types ==========

/** Structured error codes matching backend ApiErrorCode */
export type ApiErrorCode =
  | 'validation_failed'
  | 'permission_denied'
  | 'not_found'
  | 'conflict_exists'
  | 'corrupted_data'
  | 'network_error'
  | 'archive_error'
  | 'password_required'
  | 'incorrect_password'
  | 'cancelled'
  | 'insufficient_space'
  | 'security_violation'
  | 'timeout'
  | 'internal'

/** Structured API error from backend */
export interface ApiError {
  code: ApiErrorCode
  message: string
  details?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function parseApiErrorObject(value: Record<string, unknown>): ApiError | null {
  const code = value.code
  const message = value.message
  const details = value.details

  if (typeof code === 'string' && typeof message === 'string') {
    return {
      code: code as ApiErrorCode,
      message,
      details: typeof details === 'string' ? details : undefined,
    }
  }

  if (message !== undefined) {
    const nested = parseApiError(message)
    if (nested) return nested
  }

  if (value.error !== undefined) {
    const nested = parseApiError(value.error)
    if (nested) return nested
  }

  return null
}

/**
 * Parse a Tauri invoke error to check if it's a structured ApiError
 * @param error The error from invoke (typically a string)
 * @returns Parsed ApiError if structured, or null if it's a plain string error
 */
export function parseApiError(error: unknown): ApiError | null {
  if (typeof error === 'string') {
    // Try to parse as JSON (structured error)
    try {
      const parsed = JSON.parse(error)
      if (isRecord(parsed)) {
        return parseApiErrorObject(parsed)
      }
    } catch {
      // Not JSON, it's a plain string error
    }

    return null
  }

  if (isRecord(error)) {
    return parseApiErrorObject(error)
  }

  if (error instanceof Error) {
    return parseApiError(error.message)
  }

  return null
}

/**
 * Check if an error has a specific error code
 */
export function isApiErrorCode(error: unknown, code: ApiErrorCode): boolean {
  const apiError = parseApiError(error)
  return apiError?.code === code
}

/**
 * Get the error message from either an ApiError or plain string
 */
export function getErrorMessage(error: unknown): string {
  const apiError = parseApiError(error)
  if (apiError) {
    return apiError.message
  }
  if (typeof error === 'string') {
    return error
  }
  if (error instanceof Error) {
    return error.message
  }
  if (isRecord(error)) {
    if (typeof error.message === 'string') {
      return error.message
    }
    if (typeof error.error === 'string') {
      return error.error
    }
    try {
      return JSON.stringify(error)
    } catch {
      // Ignore JSON serialization failures and fall back below.
    }
  }
  return String(error)
}

// ========== Addon Types ==========

export enum AddonType {
  Aircraft = 'Aircraft',
  /** Scenery with Earth nav data (.dsf files) */
  Scenery = 'Scenery',
  /** Scenery library with library.txt */
  SceneryLibrary = 'SceneryLibrary',
  Plugin = 'Plugin',
  Navdata = 'Navdata',
  /** Aircraft livery (auto-detected by pattern) */
  Livery = 'Livery',
  /** FlyWithLua Lua script */
  LuaScript = 'LuaScript',
}

/** Represents a nested archive within another archive */
export interface NestedArchiveInfo {
  /** Path within parent archive (e.g., "aircraft/A330.zip") */
  internalPath: string
  /** Password for this specific nested archive (if different from parent) */
  password?: string
  /** Archive format: "zip", "7z", or "rar" */
  format: string
}

/** Extraction chain for nested archives (outer to inner order) */
export interface ExtractionChain {
  /** Ordered list of archives to extract (outer to inner) */
  archives: NestedArchiveInfo[]
  /** Final internal root after all extractions */
  finalInternalRoot?: string
}

export interface NavdataInfo {
  name: string
  cycle?: string
  airac?: string
}

export interface VersionInfo {
  version?: string
}

export interface InstallTask {
  id: string
  type: AddonType
  sourcePath: string
  targetPath: string
  displayName: string
  conflictExists?: boolean
  /** For archives: the root folder path inside the archive to extract from */
  archiveInternalRoot?: string
  /** For nested archives: extraction chain (takes precedence over archiveInternalRoot) */
  extractionChain?: ExtractionChain
  /** Whether to overwrite existing folder (delete before install) */
  shouldOverwrite?: boolean
  /** Password for encrypted archives */
  password?: string
  /** Estimated uncompressed size in bytes (for archives) */
  estimatedSize?: number
  /** Size warning message if archive is suspiciously large or has high compression ratio */
  sizeWarning?: string
  /** Whether user has confirmed they trust this archive (for large/suspicious archives) */
  sizeConfirmed?: boolean
  /** For Navdata: existing cycle info (if conflict exists) */
  existingNavdataInfo?: NavdataInfo
  /** For Navdata: new cycle info to be installed */
  newNavdataInfo?: NavdataInfo
  /** For Aircraft/Plugin: existing version info (if conflict exists) */
  existingVersionInfo?: VersionInfo
  /** For Aircraft/Plugin: new version info to be installed */
  newVersionInfo?: VersionInfo
  /** Whether to backup liveries during clean install (Aircraft only) */
  backupLiveries?: boolean
  /** Whether to backup configuration files during clean install (Aircraft only) */
  backupConfigFiles?: boolean
  /** Glob patterns for config files to backup (Aircraft only) */
  configFilePatterns?: string[]
  /** Whether to backup navdata during clean install (Navdata only) */
  backupNavdata?: boolean
  /** For Livery: the aircraft type this livery belongs to (e.g., "FF777") */
  liveryAircraftType?: string
  /** For Livery: whether the target aircraft is installed */
  liveryAircraftFound?: boolean
  /** For LuaScript: whether FlyWithLua plugin is installed */
  flyWithLuaInstalled?: boolean
  /** For LuaScript: companion files/folders referenced by SCRIPT_DIRECTORY */
  companionPaths?: string[]
}

export interface AnalysisResult {
  tasks: InstallTask[]
  errors: string[]
  /** List of archive paths that require a password */
  passwordRequired: string[]
  /** Map of nested archive paths to their parent archive */
  nestedPasswordRequired?: Record<string, string>
}

export interface ConflictInfo {
  task: InstallTask
  existingVersion?: string
  newVersion?: string
}

export type InstallPhase = 'calculating' | 'installing' | 'verifying' | 'finalizing'

export interface InstallProgress {
  percentage: number
  totalBytes: number
  processedBytes: number
  currentTaskIndex: number
  totalTasks: number
  currentTaskName: string
  currentFile?: string | null
  phase: InstallPhase
  /** Verification progress (0-100), only present during verifying phase */
  verificationProgress?: number
  /** Current task progress percentage (0-100), represents progress of current task only */
  currentTaskPercentage: number
  /** Current task total bytes */
  currentTaskTotalBytes: number
  /** Current task processed bytes */
  currentTaskProcessedBytes: number
  /** Active tasks in parallel mode */
  activeTasks?: ParallelTaskProgress[]
  /** Count of completed tasks in parallel mode */
  completedTaskCount?: number
  /** IDs of completed tasks in parallel mode */
  completedTaskIds?: string[]
}

export interface ParallelTaskProgress {
  taskId: string
  taskIndex: number
  taskName: string
  phase: InstallPhase
  percentage: number
  currentFile?: string | null
}

export interface TaskResult {
  taskId: string
  taskName: string
  success: boolean
  errorMessage?: string
}

export interface InstallResult {
  totalTasks: number
  successfulTasks: number
  failedTasks: number
  taskResults: TaskResult[]
}

export interface UpdateInfo {
  currentVersion: string
  latestVersion: string
  isUpdateAvailable: boolean
  releaseNotes: string
  releaseUrl: string
  publishedAt: string
}

// ========== Scenery Auto-Sorting Types ==========

export enum SceneryCategory {
  FixedHighPriority = 'FixedHighPriority',
  Airport = 'Airport',
  DefaultAirport = 'DefaultAirport',
  Library = 'Library',
  Overlay = 'Overlay',
  AirportMesh = 'AirportMesh',
  Mesh = 'Mesh',
  Other = 'Other',
  Unrecognized = 'Unrecognized',
}

export interface SceneryPackageInfo {
  folderName: string
  category: SceneryCategory
  subPriority: number
  lastModified: number
  hasAptDat: boolean
  airportId?: string
  hasDsf: boolean
  hasLibraryTxt: boolean
  hasTextures: boolean
  hasObjects: boolean
  textureCount: number
  indexedAt: number
  requiredLibraries: string[]
  missingLibraries: string[]
  enabled: boolean
  sortOrder: number
}

export interface SceneryIndexStats {
  totalPackages: number
  byCategory: Record<string, number>
  lastUpdated: number
}

export interface SceneryIndexStatus {
  indexExists: boolean
  totalPackages: number
}

export interface SceneryIndexScanResult {
  indexExists: boolean
  added: string[]
  removed: string[]
  updated: string[]
}

export interface SceneryManagerEntry {
  folderName: string
  category: SceneryCategory
  subPriority: number
  enabled: boolean
  sortOrder: number
  updateUrl?: string
  missingLibraries: string[]
  requiredLibraries: string[]
  continent?: string
  duplicateTiles: string[]
  duplicateAirports: string[]
  airportId?: string
  originalCategory?: SceneryCategory
}

export interface SceneryManagerData {
  entries: SceneryManagerEntry[]
  totalCount: number
  enabledCount: number
  missingDepsCount: number
  duplicateTilesCount: number
  duplicateAirportsCount: number
  needsSync: boolean
  /** Raw tile overlap data (all overlaps, before XPME filtering) for real-time recalculation */
  tileOverlaps: Record<string, string[]>
}

// ========== Management Types ==========

export interface AircraftInfo {
  folderName: string
  displayName: string
  acfFile: string
  acfFiles: AircraftAcfFileInfo[]
  enabled: boolean
  hasMixedAcfStates: boolean
  hasLiveries: boolean
  liveryCount: number
  version?: string
  updateUrl?: string
  updateProvider?: 'skunkcrafts' | 'x-updater' | 'zibo'
  latestVersion?: string
  hasUpdate: boolean
  cfgDisabled?: boolean
}

export interface AircraftAcfFileInfo {
  fileName: string
  enabled: boolean
}

export interface LiveryInfo {
  folderName: string
  displayName: string
  iconPath: string | null
}

export interface PluginInfo {
  folderName: string
  displayName: string
  xplFiles: string[]
  enabled: boolean
  platform: string
  version?: string
  updateUrl?: string
  updateProvider?: 'skunkcrafts' | 'x-updater'
  latestVersion?: string
  hasUpdate: boolean
  cfgDisabled?: boolean
  hasScripts: boolean
  scriptCount: number
}

export type AddonUpdatableItemType = 'aircraft' | 'plugin' | 'scenery' | 'livery'

export interface AddonUpdateDrawerTask {
  itemType: AddonUpdatableItemType
  folderName: string
  displayName: string
  initialLocalVersion?: string
  initialTargetVersion?: string
}

export interface AddonUpdateOptions {
  useBeta: boolean
  includeLiveries: boolean
  applyBlacklist: boolean
  rollbackOnFailure: boolean
  parallelDownloads?: number
  channel?: 'stable' | 'beta' | 'alpha'
  freshInstall?: boolean
  preserveLiveries?: boolean
  preserveConfigFiles?: boolean
  chunkedDownloadEnabled?: boolean
  threadsPerTask?: number
  totalThreads?: number
}

export interface AddonUpdatePreview {
  provider?: string
  itemType: string
  folderName: string
  localVersion?: string
  targetVersion?: string
  selectedChannel: 'stable' | 'beta' | 'alpha' | string
  availableChannels: string[]
  changelog?: string
}

export type AddonManualDownloadReason = 'drive-limit' | 'release-page'

export type ZiboInstallMode = 'patch' | 'major-clean'

export interface AddonUpdatePlan {
  provider?: string
  itemType: string
  folderName: string
  localVersion?: string
  remoteVersion?: string
  remoteModule?: string
  manualDownloadUrl?: string
  manualDownloadReason?: AddonManualDownloadReason
  ziboInstallMode?: ZiboInstallMode
  remoteLocked: boolean
  hasUpdate: boolean
  estimatedDownloadBytes: number
  addFiles: string[]
  replaceFiles: string[]
  deleteFiles: string[]
  skipFiles: string[]
  warnings: string[]
  hasBetaConfig: boolean
}

export interface AddonUpdateResult {
  provider?: string
  success: boolean
  message: string
  itemType: string
  folderName: string
  localVersion?: string
  remoteVersion?: string
  updatedFiles: number
  deletedFiles: number
  skippedFiles: number
  rollbackUsed: boolean
}

export interface AddonUpdaterCredentials {
  login: string
  licenseKey: string
}

export interface AddonDiskSpaceInfo {
  freeBytes: number
  totalBytes: number
}

export interface LuaScriptInfo {
  fileName: string
  displayName: string
  enabled: boolean
}

export interface NavdataManagerInfo {
  folderName: string
  providerName: string
  cycle?: string
  airac?: string
  enabled: boolean
}

export interface BackupFileEntry {
  relativePath: string
  checksum: string
  size: number
}

export interface NavdataBackupVerification {
  providerName: string
  cycle?: string
  airac?: string
  backupTime: string
  files: BackupFileEntry[]
  fileCount: number
}

export interface NavdataBackupInfo {
  folderName: string
  verification: NavdataBackupVerification
}

export interface ManagementData<T> {
  entries: T[]
  totalCount: number
  enabledCount: number
}

export type ManagementTab = 'aircraft' | 'plugin' | 'navdata' | 'scenery'

export type ManagementItemType = 'aircraft' | 'plugin' | 'navdata'

export type ScreenshotMediaType = 'image' | 'video'

export interface ScreenshotMediaItem {
  id: string
  name: string
  fileName: string
  path: string
  mediaType: ScreenshotMediaType
  ext: string
  size: number
  modifiedAt: number
  width?: number | null
  height?: number | null
  duration?: number | null
  editable: boolean
  previewable: boolean
}

export interface ScreenshotCrop {
  x: number
  y: number
  width: number
  height: number
}

export interface ScreenshotEditParams {
  crop: ScreenshotCrop | null
  rotate: number
  exposure: number
  contrast: number
  saturation: number
  temperature: number
  highlights: number
  shadows: number
  sharpness: number
  denoise: number
}

// ========== CSL Management Types ==========

export type CslPackageStatus = 'checking' | 'not_installed' | 'needs_update' | 'up_to_date'

export interface CslPackageInfo {
  name: string
  total_size_bytes: number
  file_count: number
  description: string
  status: CslPackageStatus
  files_to_update: number
  update_size_bytes: number
  last_updated: string
}

export interface CslPath {
  path: string
  source: string
  plugin_name: string | null
}

export interface CslScanResult {
  packages: CslPackageInfo[]
  paths: CslPath[]
  server_version: string
}

export interface CslProgress {
  package_name: string
  current_file: number
  total_files: number
  current_file_name: string
  bytes_downloaded: number
  total_bytes: number
}

// ========== Gateway Management Types ==========

export interface GatewayAirportSearchResult {
  icao: string
  airportName: string | null
  sceneryCount: number | null
  recommendedSceneryId: number | null
  recommendedArtist: string | null
  recommendedAcceptedAt: string | null
}

export interface GatewayScenerySummary {
  sceneryId: number
  artist: string | null
  status: string | null
  approvedDate: string | null
  comment: string | null
  recommended: boolean
}

export interface GatewayAirportDetail {
  icao: string
  airportName: string | null
  sceneryCount: number | null
  recommendedSceneryId: number | null
  recommendedArtist: string | null
  recommendedAcceptedAt: string | null
  sceneries: GatewayScenerySummary[]
}

export interface GatewaySceneryDetail {
  sceneryId: number
  icao: string | null
  airportName: string | null
  status: string | null
  artist: string | null
  approvedDate: string | null
  comment: string | null
  features: string[]
}

export interface GatewayInstalledAirport {
  id: number
  airportIcao: string
  airportName: string
  sceneryId: number
  folderName: string
  artist: string | null
  approvedDate: string | null
  installedAt: number
  updateAvailable: boolean | null
  latestSceneryId: number | null
  latestArtist: string | null
  latestApprovedDate: string | null
}

export interface GatewayInstallWarning {
  kind: string
  message: string
}
