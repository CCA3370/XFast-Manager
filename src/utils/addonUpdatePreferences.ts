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

function isXUpdaterTaggedUrl(url: string): boolean {
  return url.trim().toLowerCase().startsWith('x-updater:')
}

export function getUpdateProvider(item: {
  updateProvider?: string
  updateUrl?: string
}): string | null {
  const provider = item.updateProvider?.trim()
  if (provider) return provider

  const updateUrl = item.updateUrl?.trim()
  if (!updateUrl) return null
  return isXUpdaterTaggedUrl(updateUrl) ? 'x-updater' : 'skunkcrafts'
}

export function usesRemoteUpdateCheck(item: {
  updateProvider?: string
  updateUrl?: string
}): boolean {
  const provider = getUpdateProvider(item)
  return provider === 'skunkcrafts' || provider === 'zibo'
}
