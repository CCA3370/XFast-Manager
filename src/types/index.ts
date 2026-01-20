export enum AddonType {
  Aircraft = 'Aircraft',
  /** Scenery with Earth nav data (.dsf files) */
  Scenery = 'Scenery',
  /** Scenery library with library.txt */
  SceneryLibrary = 'SceneryLibrary',
  Plugin = 'Plugin',
  Navdata = 'Navdata',
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
  /** Whether to backup liveries during clean install (Aircraft only) */
  backupLiveries?: boolean;
  /** Whether to backup configuration files during clean install (Aircraft only) */
  backupConfigFiles?: boolean;
  /** Glob patterns for config files to backup (Aircraft only) */
  configFilePatterns?: string[];
}

export interface AnalysisResult {
  tasks: InstallTask[];
  errors: string[];
  /** List of archive paths that require a password */
  passwordRequired: string[];
  /** Map of nested archive paths to their parent archive */
  nestedPasswordRequired?: Record<string, string>;
}

export interface NavdataInfo {
  name: string;
  cycle?: string;
  airac?: string;
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
  Orthophotos = 'Orthophotos',
  Mesh = 'Mesh',
  Other = 'Other',
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
