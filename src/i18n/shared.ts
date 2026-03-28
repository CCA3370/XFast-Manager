export const LOCALE_STORAGE_KEY = 'xfast.locale'

export const SUPPORTED_LOCALES = ['zh', 'en', 'es', 'fr', 'de', 'ja', 'pt', 'hi', 'ar', 'ru', 'ko'] as const

export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number]

export const LOCALE_OPTIONS: Array<{ value: SupportedLocale; label: string }> = [
  { value: 'zh', label: '简体中文' },
  { value: 'en', label: 'English' },
  { value: 'es', label: 'Español' },
  { value: 'fr', label: 'Français' },
  { value: 'de', label: 'Deutsch' },
  { value: 'ja', label: '日本語' },
  { value: 'pt', label: 'Português' },
  { value: 'hi', label: 'हिन्दी' },
  { value: 'ar', label: 'العربية' },
  { value: 'ru', label: 'Русский' },
  { value: 'ko', label: '한국어' },
]

function matchLocale(input?: string | null): SupportedLocale | null {
  const locale = input?.trim().toLowerCase()

  if (!locale) return null
  if (locale.startsWith('zh')) return 'zh'
  if (locale.startsWith('en')) return 'en'
  if (locale.startsWith('es')) return 'es'
  if (locale.startsWith('fr')) return 'fr'
  if (locale.startsWith('de')) return 'de'
  if (locale.startsWith('ja')) return 'ja'
  if (locale.startsWith('pt')) return 'pt'
  if (locale.startsWith('hi')) return 'hi'
  if (locale.startsWith('ar')) return 'ar'
  if (locale.startsWith('ru')) return 'ru'
  if (locale.startsWith('ko')) return 'ko'

  return null
}

export function normalizeLocale(input?: string | null): SupportedLocale {
  return matchLocale(input) ?? 'en'
}

export function getStoredLocale(): SupportedLocale | null {
  if (typeof window === 'undefined') return null

  const stored = window.localStorage.getItem(LOCALE_STORAGE_KEY)
  return matchLocale(stored)
}

export function getSystemLocale(): SupportedLocale {
  if (typeof navigator === 'undefined') return 'en'

  const candidates = [
    ...(Array.isArray(navigator.languages) ? navigator.languages : []),
    navigator.language,
  ]

  for (const candidate of candidates) {
    const matched = matchLocale(candidate)
    if (matched) return matched
  }

  return 'en'
}

export function getInitialLocale(): SupportedLocale {
  return getStoredLocale() ?? getSystemLocale()
}

export function persistLocale(locale: SupportedLocale) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(LOCALE_STORAGE_KEY, locale)
}
