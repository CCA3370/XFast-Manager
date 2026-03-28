import { createI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import zh from './zh'
import en from './en'
import es from './es'
import fr from './fr'
import de from './de'
import ja from './ja'
import pt from './pt'
import hi from './hi'
import ar from './ar'
import ru from './ru'
import ko from './ko'
import { getInitialLocale, persistLocale, type SupportedLocale } from './shared'

const initialLocale = getInitialLocale()

function applyDocumentLocale(locale: SupportedLocale) {
  if (typeof document === 'undefined') return
  document.documentElement.lang = locale
  document.documentElement.dir = locale === 'ar' ? 'rtl' : 'ltr'
}

// Removed blocking invoke call from module top-level to improve startup speed
// Use syncLocaleToBackend() in App.vue onMounted instead

export const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: false,
  messages: {
    zh,
    en,
    es,
    fr,
    de,
    ja,
    pt,
    hi,
    ar,
    ru,
    ko,
  },
})

applyDocumentLocale(initialLocale)

// Non-blocking function to sync initial locale with backend (call in App.vue)
export function syncLocaleToBackend() {
  applyDocumentLocale(i18n.global.locale.value)
  invoke('set_log_locale', { locale: i18n.global.locale.value }).catch(() => {
    // Ignore errors during initialization
  })
}

// Helper function to sync locale with backend when user changes language
export async function setLocale(locale: SupportedLocale) {
  i18n.global.locale.value = locale
  applyDocumentLocale(locale)
  persistLocale(locale)
  try {
    await invoke('set_log_locale', { locale })
  } catch (e) {
    console.debug('Failed to set backend locale:', e)
  }
}

export default i18n
