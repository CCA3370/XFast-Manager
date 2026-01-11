export enum AddonType {
  Aircraft = 'Aircraft',
  /** Scenery with Earth nav data (.dsf files) */
  Scenery = 'Scenery',
  /** Scenery library with library.txt */
  SceneryLibrary = 'SceneryLibrary',
  Plugin = 'Plugin',
  Navdata = 'Navdata',
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

export type InstallPhase = 'Calculating' | 'Installing' | 'Finalizing';

export interface InstallProgress {
  percentage: number;
  totalBytes: number;
  processedBytes: number;
  currentTaskIndex: number;
  totalTasks: number;
  currentTaskName: string;
  currentFile?: string;
  phase: InstallPhase;
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
