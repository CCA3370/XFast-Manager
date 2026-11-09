import { getItem, removeItem, setItem, STORAGE_KEYS } from './storage'
import type { PatchMappingInput } from '@/types'

let pendingWrite = Promise.resolve()

function isMapping(value: unknown): value is PatchMappingInput {
  return (
    typeof value === 'object' &&
    value !== null &&
    'archiveSubpath' in value &&
    typeof value.archiveSubpath === 'string' &&
    'destSubpath' in value &&
    typeof value.destSubpath === 'string'
  )
}

export async function loadPatchMappings(key: string): Promise<PatchMappingInput[] | null> {
  await pendingWrite
  const value = await getItem<unknown>(STORAGE_KEYS.PATCH_MAPPING_PREFIX + key)
  return Array.isArray(value) && value.every(isMapping) ? value : null
}

export function savePatchMappings(
  key: string,
  mappings: PatchMappingInput[] | null,
): Promise<void> {
  const storageKey = STORAGE_KEYS.PATCH_MAPPING_PREFIX + key
  const write = pendingWrite.then(() =>
    mappings === null ? removeItem(storageKey) : setItem(storageKey, mappings),
  )
  pendingWrite = write.catch(() => {})
  return write
}
