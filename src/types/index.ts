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

/**
 * Parse a Tauri invoke error to check if it's a structured ApiError
 * @param error The error from invoke (typically a string)
 * @returns Parsed ApiError if structured, or null if it's a plain string error
 */
export function parseApiError(error: unknown): ApiError | null {
  if (typeof error !== 'string') {
    return null
  }

  // Try to parse as JSON (structured error)
  try {
    const parsed = JSON.parse(error)
    if (parsed && typeof parsed === 'object' && 'code' in parsed && 'message' in parsed) {
      return parsed as ApiError
    }
  } catch {
    // Not JSON, it's a plain string error
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
}

/** Represents a nested archive within another archive */
export interface NestedArchiveInfo {
  /** Path within parent archive (e.g., "aircraft/A330.zip") */
  internalPath: string;
  /** Password for this specific nested archive (if different from parent) */
  password?: string;
  /** Archive format: "zip", "7z", or "rar" */
  format: string;
}

/** Extraction chain for nested archives (outer to inner order) */
export interface ExtractionChain {
  /** Ordered list of archives to extract (outer to inner) */
  archives: NestedArchiveInfo[];
  /** Final internal root after all extractions */
  finalInternalRoot?: string;
}

export interface NavdataInfo {
  name: string;
  cycle?: string;
  airac?: string;
}

export interface VersionInfo {
  version?: string;
}

export interface InstallTask {
  id: string;
  type: AddonType;
  sourcePath: string;
  targetPath: string;
  displayName: string;
  conflictExists?: boolean;
  /** For archives: the root folder path inside the archive to extract from */
  archiveInternalRoot?: string;
  /** For nested archives: extraction chain (takes precedence over archiveInternalRoot) */
  extractionChain?: ExtractionChain;
  /** Whether to overwrite existing folder (delete before install) */
  shouldOverwrite?: boolean;
  /** Password for encrypted archives */
  password?: string;
  /** Estimated uncompressed size in bytes (for archives) */
  estimatedSize?: number;
  /** Size warning message if archive is suspiciously large or has high compression ratio */
  sizeWarning?: string;
  /** Whether user has confirmed they trust this archive (for large/suspicious archives) */
  sizeConfirmed?: boolean;
  /** For Navdata: existing cycle info (if conflict exists) */
  existingNavdataInfo?: NavdataInfo;
  /** For Navdata: new cycle info to be installed */
  newNavdataInfo?: NavdataInfo;
  /** For Aircraft/Plugin: existing version info (if conflict exists) */
  existingVersionInfo?: VersionInfo;
  /** For Aircraft/Plugin: new version info to be installed */
  newVersionInfo?: VersionInfo;
  /** Whether to backup liveries during clean install (Aircraft only) */
  backupLiveries?: boolean;
  /** Whether to backup configuration files during clean install (Aircraft only) */
  backupConfigFiles?: boolean;
  /** Glob patterns for config files to backup (Aircraft only) */
  configFilePatterns?: string[];
  /** Whether to backup navdata during clean install (Navdata only) */
  backupNavdata?: boolean;
  /** For Livery: the aircraft type this livery belongs to (e.g., "FF777") */
  liveryAircraftType?: string;
  /** For Livery: whether the target aircraft is installed */
  liveryAircraftFound?: boolean;
}

export interface AnalysisResult {
  tasks: InstallTask[];
  errors: string[];
  /** List of archive paths that require a password */
  passwordRequired: string[];
  /** Map of nested archive paths to their parent archive */
  nestedPasswordRequired?: Record<string, string>;
}

export interface ConflictInfo {
  task: InstallTask;
  existingVersion?: string;
  newVersion?: string;
}

export type InstallPhase = 'calculating' | 'installing' | 'verifying' | 'finalizing';

export interface InstallProgress {
  percentage: number;
  totalBytes: number;
  processedBytes: number;
  currentTaskIndex: number;
  totalTasks: number;
  currentTaskName: string;
  currentFile?: string | null;
  phase: InstallPhase;
  /** Verification progress (0-100), only present during verifying phase */
  verificationProgress?: number;
}

export interface TaskResult {
  taskId: string;
  taskName: string;
  success: boolean;
  errorMessage?: string;
}

export interface InstallResult {
  totalTasks: number;
  successfulTasks: number;
  failedTasks: number;
  taskResults: TaskResult[];
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  isUpdateAvailable: boolean;
  releaseNotes: string;
  releaseUrl: string;
  publishedAt: string;
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
  folderName: string;
  category: SceneryCategory;
  subPriority: number;
  lastModified: number;
  hasAptDat: boolean;
  hasDsf: boolean;
  hasLibraryTxt: boolean;
  hasTextures: boolean;
  hasObjects: boolean;
  textureCount: number;
  indexedAt: number;
  requiredLibraries: string[];
  missingLibraries: string[];
  enabled: boolean;
  sortOrder: number;
}

export interface SceneryIndexStats {
  totalPackages: number;
  byCategory: Record<string, number>;
  lastUpdated: number;
}

export interface SceneryIndexStatus {
  indexExists: boolean;
  totalPackages: number;
}

export interface SceneryIndexScanResult {
  indexExists: boolean;
  added: number;
  removed: number;
  updated: number;
}

export interface SceneryManagerEntry {
  folderName: string;
  category: SceneryCategory;
  subPriority: number;
  enabled: boolean;
  sortOrder: number;
  missingLibraries: string[];
  requiredLibraries: string[];
}

export interface SceneryManagerData {
  entries: SceneryManagerEntry[];
  totalCount: number;
  enabledCount: number;
  missingDepsCount: number;
  needsSync: boolean;
}

// ========== Management Types ==========

export interface AircraftInfo {
  folderName: string;
  displayName: string;
  acfFile: string;
  enabled: boolean;
  hasLiveries: boolean;
  liveryCount: number;
  version?: string;
  updateUrl?: string;
  latestVersion?: string;
  hasUpdate: boolean;
  cfgDisabled?: boolean;
}

export interface PluginInfo {
  folderName: string;
  displayName: string;
  xplFiles: string[];
  enabled: boolean;
  platform: string;
  version?: string;
  updateUrl?: string;
  latestVersion?: string;
  hasUpdate: boolean;
  cfgDisabled?: boolean;
}

export interface NavdataManagerInfo {
  folderName: string;
  providerName: string;
  cycle?: string;
  airac?: string;
  enabled: boolean;
}

export interface BackupFileEntry {
  relativePath: string;
  checksum: string;
  size: number;
}

export interface NavdataBackupVerification {
  providerName: string;
  cycle?: string;
  airac?: string;
  backupTime: string;
  files: BackupFileEntry[];
  fileCount: number;
}

export interface NavdataBackupInfo {
  folderName: string;
  verification: NavdataBackupVerification;
}

export interface ManagementData<T> {
  entries: T[];
  totalCount: number;
  enabledCount: number;
}

export type ManagementTab = 'aircraft' | 'plugin' | 'navdata' | 'scenery';

export type ManagementItemType = 'aircraft' | 'plugin' | 'navdata';
