import { describe, expect, it } from 'vitest'
import {
  addonUpdateItemKey,
  hasAddonUpdateBetaPreference,
  normalizeAddonUpdateItemBetaPreferences,
} from './addonUpdatePreferences'

describe('addon update beta preferences', () => {
  it('keeps only explicitly enabled item preferences', () => {
    const preferences = normalizeAddonUpdateItemBetaPreferences({
      'aircraft:Beta Aircraft': true,
      'plugin:Stable Plugin': false,
      invalid: 'yes',
    })

    expect(preferences).toEqual({ 'aircraft:Beta Aircraft': true })
    expect(hasAddonUpdateBetaPreference(preferences, 'aircraft', 'Beta Aircraft')).toBe(true)
    expect(hasAddonUpdateBetaPreference(preferences, 'plugin', 'Stable Plugin')).toBe(false)
  })

  it('uses the same item key contract for every update consumer', () => {
    expect(addonUpdateItemKey('scenery', 'Global Scenery')).toBe('scenery:Global Scenery')
  })
})
