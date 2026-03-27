export const LOCALE_STORAGE_KEY = 'xfast.locale'

export const SUPPORTED_LOCALES = ['zh', 'en', 'es', 'fr', 'de', 'ja'] as const

export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number]

export const LOCALE_OPTIONS: Array<{ value: SupportedLocale; label: string }> = [
  { value: 'zh', label: '简体中文' },
  { value: 'en', label: 'English' },
  { value: 'es', label: 'Español' },
  { value: 'fr', label: 'Français' },
  { value: 'de', label: 'Deutsch' },
  { value: 'ja', label: '日本語' },
]

export function normalizeLocale(input?: string | null): SupportedLocale {
  const locale = input?.trim().toLowerCase()

  if (!locale) return 'en'
  if (locale.startsWith('zh')) return 'zh'
  if (locale.startsWith('en')) return 'en'
  if (locale.startsWith('es')) return 'es'
  if (locale.startsWith('fr')) return 'fr'
  if (locale.startsWith('de')) return 'de'
  if (locale.startsWith('ja')) return 'ja'

  return 'en'
}

export function getStoredLocale(): SupportedLocale | null {
  if (typeof window === 'undefined') return null

  const stored = window.localStorage.getItem(LOCALE_STORAGE_KEY)
  return stored ? normalizeLocale(stored) : null
}

export function getSystemLocale(): SupportedLocale {
  if (typeof navigator === 'undefined') return 'en'
  return normalizeLocale(navigator.language || 'en')
}

export function getInitialLocale(): SupportedLocale {
  return getStoredLocale() ?? getSystemLocale()
}

export function persistLocale(locale: SupportedLocale) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(LOCALE_STORAGE_KEY, locale)
}
