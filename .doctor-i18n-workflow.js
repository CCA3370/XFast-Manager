export const meta = {
  name: 'translate-doctor-i18n',
  description: 'Translate English doctor namespace to 10 locales',
  phases: [{ title: 'Translate', detail: 'Parallel translation to 10 target locales' }],
}

phase('Translate')

const LOCALES = [
  { code: 'zh', name: 'Chinese (Simplified)' },
  { code: 'es', name: 'Spanish' },
  { code: 'fr', name: 'French' },
  { code: 'de', name: 'German' },
  { code: 'ja', name: 'Japanese' },
  { code: 'pt', name: 'Portuguese (Brazil)' },
  { code: 'hi', name: 'Hindi' },
  { code: 'ar', name: 'Arabic' },
  { code: 'ru', name: 'Russian' },
  { code: 'ko', name: 'Korean' },
]

// English source is read from src/i18n/en.ts by each agent (they have file tools).
const INSTRUCTION = [
  'You are translating UI strings for "XFast Manager", an X-Plane 12 addon manager.',
  'Read the English "doctor" namespace object from the file E:/3370/Desktop/XFast-Manager/src/i18n/en.ts',
  '(it begins with a line "  doctor: {" near the end of the file and ends at its matching "  },").',
  'Translate ALL string VALUES into the target language. Rules:',
  '- Keep every KEY unchanged (navTitle, checks, the dotted check ids like "integrity.path_invalid", etc).',
  '- Preserve every interpolation token EXACTLY: {time} {count} {line} {score} {module} {version} {gpu} {provider} {cycle} {expiry} {days} {cycles} {free} {size} {categories}.',
  '- Keep technical/proper nouns recognizable: X-Plane, Vulkan, ReShade, AIRAC, Navigraph, CIFP, VRAM, GPU, UAC, Program Files, scenery_packs.ini, apt.dat, Custom Data, Resources/plugins, dxgi.dll, d3d11.dll, vulkan-1.dll, Global Airports, Gateway, XFast Manager, SID/STAR, Steam.',
  '- Do NOT leave values identical to English unless they are pure proper nouns/technical terms — the i18n validator flags untranslated strings.',
  '- Use natural, concise wording a flight-sim user would expect in this language.',
  '- Escape single quotes/apostrophes inside values as \\\' so the result is valid TS (single-quoted strings).',
  'Return ONLY the raw TypeScript object literal, starting exactly with "doctor: {" and ending with the matching "}," — no markdown fences, no commentary, no surrounding object.',
].join('\n')

const results = await parallel(
  LOCALES.map((loc) => () =>
    agent(INSTRUCTION + '\n\nTARGET LANGUAGE: ' + loc.name + ' (locale code: ' + loc.code + ').', {
      label: 'translate:' + loc.code,
      phase: 'Translate',
      agentType: 'general-purpose',
    }),
  ),
)

return {
  translations: LOCALES.map((loc, i) => ({ code: loc.code, text: results[i] })).filter(
    (t) => t.text,
  ),
}
