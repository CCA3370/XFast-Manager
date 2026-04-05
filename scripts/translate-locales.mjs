import fs from 'node:fs'
import path from 'node:path'
import ts from 'typescript'

const ROOT = process.cwd()
const SOURCE_FILE = path.join(ROOT, 'src/i18n/en.ts')
const CACHE_FILE = path.join(ROOT, 'scripts/translation-cache.json')
const TARGET_LOCALES = process.argv.slice(2)

if (TARGET_LOCALES.length === 0) {
  console.error('Usage: node scripts/translate-locales.mjs <locale> [locale...]')
  process.exit(1)
}

const sourceText = fs.readFileSync(SOURCE_FILE, 'utf8')
const sourceFile = ts.createSourceFile(SOURCE_FILE, sourceText, ts.ScriptTarget.Latest, true)

const replacements = []

function visit(node) {
  if (
    (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) &&
    ts.isPropertyAssignment(node.parent) &&
    node.parent.initializer === node
  ) {
    replacements.push({
      text: node.text,
      start: node.getStart(sourceFile) + 1,
      end: node.getEnd() - 1,
    })
  }

  ts.forEachChild(node, visit)
}

visit(sourceFile)

const uniqueTexts = Array.from(new Set(replacements.map((item) => item.text)))
const cache = fs.existsSync(CACHE_FILE) ? JSON.parse(fs.readFileSync(CACHE_FILE, 'utf8')) : {}

function escapeSingleQuoted(value) {
  return value.replace(/\\/g, '\\\\').replace(/'/g, "\\'")
}

function protectPlaceholders(text) {
  const placeholders = []
  const protectedText = text.replace(/\{[^{}]+\}/g, (match) => {
    const token = `[[[PH_${placeholders.length}]]]`
    placeholders.push(match)
    return token
  })

  return { protectedText, placeholders }
}

function restorePlaceholders(text, placeholders) {
  return text.replace(/\[\[\[PH_(\d+)]]]/g, (_, index) => placeholders[Number(index)] ?? '')
}

function extractTranslatedText(responseText) {
  const payload = JSON.parse(responseText)
  if (!Array.isArray(payload) || !Array.isArray(payload[0])) {
    throw new Error('Unexpected translation response')
  }

  return payload[0].map((segment) => segment[0] ?? '').join('')
}

async function requestTranslation(text, locale) {
  const params = new URLSearchParams({
    client: 'gtx',
    sl: 'en',
    tl: locale,
    dt: 't',
    q: text,
  })

  const response = await fetch(`https://translate.googleapis.com/translate_a/single?${params}`, {
    headers: {
      'User-Agent': 'Mozilla/5.0',
    },
  })

  if (!response.ok) {
    throw new Error(`Translation request failed: ${response.status}`)
  }

  return extractTranslatedText(await response.text())
}

async function translateBatch(items, locale, batchIndex) {
  const delimiter = `[[[SEG_${locale}_${batchIndex}]]]`
  const protectedItems = items.map((item) => ({
    original: item,
    ...protectPlaceholders(item),
  }))
  const joined = protectedItems.map((item) => item.protectedText).join(`\n${delimiter}\n`)
  const translated = await requestTranslation(joined, locale)
  const parts = translated.split(delimiter)

  if (parts.length !== items.length) {
    throw new Error(`Batch split mismatch for ${locale} batch ${batchIndex}`)
  }

  return parts.map((part, index) =>
    restorePlaceholders(part.trim(), protectedItems[index].placeholders),
  )
}

async function translateTexts(locale) {
  cache[locale] ??= {}
  const missing = uniqueTexts.filter((text) => !cache[locale][text])
  const batches = []
  let currentBatch = []
  let currentLength = 0

  for (const text of missing) {
    const estimated = text.length + 24
    if (currentBatch.length >= 12 || currentLength + estimated > 1800) {
      batches.push(currentBatch)
      currentBatch = []
      currentLength = 0
    }

    currentBatch.push(text)
    currentLength += estimated
  }

  if (currentBatch.length > 0) {
    batches.push(currentBatch)
  }

  for (let index = 0; index < batches.length; index += 1) {
    const batch = batches[index]
    let translated

    try {
      translated = await translateBatch(batch, locale, index)
    } catch (error) {
      translated = []
      for (const item of batch) {
        const { protectedText, placeholders } = protectPlaceholders(item)
        const translatedItem = await requestTranslation(protectedText, locale)
        translated.push(restorePlaceholders(translatedItem.trim(), placeholders))
      }
      console.warn(`Batch fallback used for ${locale} batch ${index}: ${String(error)}`)
    }

    translated.forEach((value, itemIndex) => {
      cache[locale][batch[itemIndex]] = value
    })

    console.log(`[${locale}] ${index + 1}/${batches.length} batches complete`)
  }

  fs.writeFileSync(CACHE_FILE, JSON.stringify(cache, null, 2), 'utf8')
}

function buildLocaleContent(locale) {
  let output = sourceText
  const localeReplacements = [...replacements].reverse()

  for (const item of localeReplacements) {
    const translated = cache[locale][item.text]
    if (!translated) {
      throw new Error(`Missing translation for locale ${locale}: ${item.text}`)
    }

    output =
      output.slice(0, item.start) + escapeSingleQuoted(translated) + output.slice(item.end)
  }

  return output
}

for (const locale of TARGET_LOCALES) {
  await translateTexts(locale)
  const outputPath = path.join(ROOT, `src/i18n/${locale}.ts`)
  fs.writeFileSync(outputPath, buildLocaleContent(locale), 'utf8')
  console.log(`Wrote ${outputPath}`)
}
