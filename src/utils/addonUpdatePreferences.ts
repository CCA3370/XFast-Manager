import type { AddonUpdatableItemType } from '@/types'

export type AddonUpdateItemBetaPreferences = Record<string, boolean>

export function addonUpdateItemKey(itemType: AddonUpdatableItemType, folderName: string): string {
  return `${itemType}:${folderName}`
}

export function normalizeAddonUpdateItemBetaPreferences(
  value: unknown,
): AddonUpdateItemBetaPreferences {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {}

  return Object.fromEntries(
    Object.entries(value).filter(
      ([key, enabled]) => typeof key === 'string' && typeof enabled === 'boolean' && enabled,
    ),
  )
}

export function hasAddonUpdateBetaPreference(
  preferences: AddonUpdateItemBetaPreferences,
  itemType: AddonUpdatableItemType,
  folderName: string,
): boolean {
  return preferences[addonUpdateItemKey(itemType, folderName)] === true
}
