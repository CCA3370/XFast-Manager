import de from '../src/i18n/de.ts'
import en from '../src/i18n/en.ts'
import es from '../src/i18n/es.ts'
import fr from '../src/i18n/fr.ts'
import ja from '../src/i18n/ja.ts'
import pt from '../src/i18n/pt.ts'
import hi from '../src/i18n/hi.ts'
import ar from '../src/i18n/ar.ts'
import ru from '../src/i18n/ru.ts'
import ko from '../src/i18n/ko.ts'
import zh from '../src/i18n/zh.ts'

function flattenMessages(value, prefix = '', output = {}) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return output
  }

  for (const [key, child] of Object.entries(value)) {
    const nextKey = prefix ? `${prefix}.${key}` : key
    if (child && typeof child === 'object' && !Array.isArray(child)) {
      flattenMessages(child, nextKey, output)
    } else {
      output[nextKey] = child
    }
  }

  return output
}

function extractPlaceholders(value) {
  if (typeof value !== 'string') return []
  const matches = value.match(/\{[^}]+\}/g) ?? []
  return [...new Set(matches)].sort()
}

function isSuspiciousUntranslated(locale, key, baseValue, localeValue) {
  if (typeof baseValue !== 'string' || typeof localeValue !== 'string') return false
  if (localeValue !== baseValue) return false

  const ignoredKeys = new Set([
    'common.test',
    'common.offline',
    'common.latest',
    'common.info',
    'common.warning',
    'common.error',
    'common.success',
    'geo.continents.Asia',
    'geo.continents.Europe',
    'commandPalette.categoryNav',
    'map.procedures.tabs.sid',
  ])

  if (ignoredKeys.has(key)) {
    return false
  }

  const ignoredValues = [
    /^X-Plane$/i,
    /^XFast Manager$/i,
    /^Log\.txt$/i,
    /^DMP$/i,
    /^ZIP$/i,
    /^RAR$/i,
    /^7z$/i,
    /^API$/i,
    /^CPU$/i,
    /^GPU$/i,
    /^SDK$/i,
    /^ICAO$/i,
    /^IATA$/i,
    /^ILS$/i,
    /^VATSIM$/i,
    /^Lua$/i,
    /^SimBrief$/i,
    /^Navigraph$/i,
    /^OpenStreetMap$/i,
  ]

  if (ignoredValues.some((pattern) => pattern.test(localeValue.trim()))) {
    return false
  }

  return /[A-Za-z]{3,}/.test(localeValue)
}

// Detect common English words left untranslated in non-Latin locales
const PARTIAL_TRANSLATION_WORDS = new Set([
  'Info', 'Cancel', 'Failed', 'Complete', 'Edit', 'All', 'Name', 'Alert',
  'Thread', 'Plugin', 'Aircraft', 'File', 'Install', 'Update', 'Delete',
  'Save', 'Close', 'Error', 'Search', 'Filter', 'Settings', 'Download',
  'Status', 'Version', 'Total', 'Source', 'Target', 'Current', 'Speed',
  'Progress', 'Available', 'Required', 'Missing', 'Enable', 'Disable',
  'Start', 'Stop', 'Reset', 'Clear', 'Apply', 'Confirm', 'Back', 'Next',
  'Title', 'Description', 'Details', 'Options', 'Help', 'About',
  'Import', 'Export', 'Backup', 'Restore', 'Scan', 'Analyze',
  'Category', 'Report', 'Summary', 'Overview', 'General', 'Custom',
  'Crash', 'Memory', 'Process', 'Performance', 'Network', 'Connection',
  'Theme', 'Light', 'Dark', 'Language', 'Notification', 'Message',
])

const NON_LATIN_LOCALES = new Set(['zh', 'ja', 'ko', 'ar', 'hi', 'ru'])

function detectPartialTranslation(locale, key, baseValue, localeValue) {
  if (!NON_LATIN_LOCALES.has(locale)) return false
  if (typeof baseValue !== 'string' || typeof localeValue !== 'string') return false
  if (localeValue === baseValue) return false // already caught by isSuspiciousUntranslated

  // Strip placeholders and known brand names before checking
  const cleaned = localeValue
    .replace(/\{[^}]+\}/g, '')
    .replace(/X-Plane|XFast Manager|SimBrief|Navigraph|OpenStreetMap|FlyWithLua|XPRealistic|FMOD/g, '')
    .replace(/ICAO|IATA|VATSIM|IVAO|METAR|TAF|NOTAM/g, '')
    .replace(/VOR|NDB|DME|RNAV|RNP|ILS|SID|STAR/g, '')
    .replace(/CSL|OBJ8|SDK|API|CPU|GPU|DMP|ZIP|RAR/g, '')
    .replace(/Beta|RC|GND|TWR|APP|CTR|DSF|FMOD|SSL/g, '')
    .replace(/Program Files/g, '')
    .replace(/https?:\/\/[^\s]*/g, '')
    .replace(/\.\w{1,4}\b/g, '') // file extensions like .cfg, .dmp

  // Find remaining English words that should have been translated
  const words = cleaned.match(/\b[A-Z][a-z]{2,}\b/g) || []
  return words.some((w) => PARTIAL_TRANSLATION_WORDS.has(w))
}

const locales = { en, zh, es, fr, de, ja, pt, hi, ar, ru, ko }
const baseLocale = 'en'
const baseMessages = flattenMessages(locales[baseLocale])
const baseKeys = Object.keys(baseMessages).sort()

let hasMismatch = false
const suspiciousByLocale = {}

for (const [locale, messages] of Object.entries(locales)) {
  if (locale === baseLocale) continue

  const flattened = flattenMessages(messages)
  const localeKeys = Object.keys(flattened).sort()
  const missing = baseKeys.filter((key) => !(key in flattened))
  const extra = localeKeys.filter((key) => !(key in baseMessages))
  const placeholderMismatch = []
  const suspicious = []
  const partiallyTranslated = []

  for (const key of baseKeys) {
    if (!(key in flattened)) continue

    const baseValue = baseMessages[key]
    const localeValue = flattened[key]
    const basePlaceholders = extractPlaceholders(baseValue)
    const localePlaceholders = extractPlaceholders(localeValue)

    if (basePlaceholders.join('|') !== localePlaceholders.join('|')) {
      placeholderMismatch.push({
        key,
        expected: basePlaceholders,
        actual: localePlaceholders,
      })
    }

    if (isSuspiciousUntranslated(locale, key, baseValue, localeValue)) {
      suspicious.push(key)
    }

    if (detectPartialTranslation(locale, key, baseValue, localeValue)) {
      partiallyTranslated.push(key)
    }
  }

  if (!missing.length && !extra.length && !placeholderMismatch.length && !partiallyTranslated.length) {
    suspiciousByLocale[locale] = suspicious
    continue
  }

  hasMismatch = true
  console.error(`i18n mismatch detected for locale "${locale}":`)

  if (missing.length) {
    console.error(`  Missing keys (${missing.length}):`)
    for (const key of missing) {
      console.error(`    - ${key}`)
    }
  }

  if (extra.length) {
    console.error(`  Extra keys (${extra.length}):`)
    for (const key of extra) {
      console.error(`    - ${key}`)
    }
  }

  if (placeholderMismatch.length) {
    console.error(`  Placeholder mismatches (${placeholderMismatch.length}):`)
    for (const item of placeholderMismatch) {
      console.error(
        `    - ${item.key}: expected [${item.expected.join(', ')}], got [${item.actual.join(', ')}]`,
      )
    }
  }

  if (partiallyTranslated.length) {
    console.error(`  Partially translated (${partiallyTranslated.length}):`)
    for (const key of partiallyTranslated) {
      console.error(`    - ${key}: ${JSON.stringify(flattened[key]).slice(0, 80)}`)
    }
  }

  suspiciousByLocale[locale] = suspicious
}

if (hasMismatch) {
  process.exit(1)
}

for (const [locale, suspicious] of Object.entries(suspiciousByLocale)) {
  if (!suspicious.length) continue

  console.warn(
    `i18n warning for locale "${locale}": ${suspicious.length} value(s) still match English exactly. Sample: ${suspicious
      .slice(0, 8)
      .join(', ')}`,
  )
}

console.log(
  `i18n key coverage OK: ${Object.keys(locales).length} locales, ${baseKeys.length} keys per locale`,
)
