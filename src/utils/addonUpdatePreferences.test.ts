import { describe, expect, it } from 'vitest'
import {
  addonUpdateItemKey,
  getAddonUpdateBetaPreference,
  getAddonUpdateChannelOverrides,
  normalizeAddonUpdateItemBetaPreferences,
} from './addonUpdatePreferences'

describe('addon update beta preferences', () => {
  it('keeps both explicit channel overrides and leaves unconfigured items unset', () => {
    const preferences = normalizeAddonUpdateItemBetaPreferences({
      'aircraft:Beta Aircraft': true,
      'plugin:Stable Plugin': false,
      invalid: 'yes',
    })

    expect(preferences).toEqual({ 'aircraft:Beta Aircraft': true, 'plugin:Stable Plugin': false })
    expect(getAddonUpdateBetaPreference(preferences, 'aircraft', 'Beta Aircraft')).toBe(true)
    expect(getAddonUpdateBetaPreference(preferences, 'plugin', 'Stable Plugin')).toBe(false)
    expect(getAddonUpdateBetaPreference(preferences, 'aircraft', 'Other')).toBeUndefined()
    expect(
      getAddonUpdateChannelOverrides(preferences, 'plugin', [
        { folderName: 'Stable Plugin' },
        { folderName: 'Beta Aircraft' },
        { folderName: 'Other' },
      ]),
    ).toEqual({ betaFolders: [], stableFolders: ['Stable Plugin'] })
  })

  it('uses the same item key contract for every update consumer', () => {
    expect(addonUpdateItemKey('scenery', 'Global Scenery')).toBe('scenery:Global Scenery')
  })
})
