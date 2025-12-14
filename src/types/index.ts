export enum AddonType {
  Aircraft = 'Aircraft',
  Scenery = 'Scenery',
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
}

export interface AnalysisResult {
  tasks: InstallTask[];
  errors: string[];
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
