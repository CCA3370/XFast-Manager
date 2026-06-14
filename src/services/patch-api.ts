import { invokeCommand } from './api'
import type {
  InstallTask,
  PatchMappingInput,
  PatchPlan,
  PatchTargetDetection,
} from '@/types'

/**
 * Typed wrappers around the backend patch-install commands.
 *
 * Patch install is a two-step judgement: detect which installed aircraft a
 * patch belongs to (by aligning its files against each aircraft), then infer
 * how the archive maps into that aircraft. The user confirms/edits, and we
 * build overlay install tasks that run through the normal install pipeline.
 */

/** Level 1 — rank installed aircraft by how well the patch aligns with each. */
export function detectPatchTargetAircraft(
  archivePath: string,
  xplanePath: string,
): Promise<PatchTargetDetection> {
  return invokeCommand<PatchTargetDetection>('detect_patch_target_aircraft', {
    archivePath,
    xplanePath,
  })
}

/** Level 2 — infer archive -> aircraft subpath mappings for the chosen aircraft. */
export function inferPatchMappings(
  archivePath: string,
  xplanePath: string,
  aircraftFolder: string,
): Promise<PatchPlan> {
  return invokeCommand<PatchPlan>('infer_patch_mappings', {
    archivePath,
    xplanePath,
    aircraftFolder,
  })
}

/** Build overlay install tasks from the user-confirmed mappings. */
export function buildPatchInstallTasks(params: {
  archivePath: string
  password?: string | null
  xplanePath: string
  aircraftFolder: string
  mappings: PatchMappingInput[]
  backupOverwritten: boolean
}): Promise<InstallTask[]> {
  return invokeCommand<InstallTask[]>('build_patch_install_tasks', {
    archivePath: params.archivePath,
    password: params.password ?? null,
    xplanePath: params.xplanePath,
    aircraftFolder: params.aircraftFolder,
    mappings: params.mappings,
    backupOverwritten: params.backupOverwritten,
  })
}

/** Restore a patch backup session, undoing the overwritten files. */
export function revertPatch(backupSessionDir: string): Promise<number> {
  return invokeCommand<number>('revert_patch', { backupSessionDir })
}
