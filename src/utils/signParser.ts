/**
 * X-Plane airport sign text parser
 *
 * Converts raw sign markup like `{@R,^l}18{^r}36_{^u}22-04`
 * into human-readable text (e.g. `←18→36 ↑22-04`).
 */

const GLYPH_MAP: Record<string, string> = {
  '^u': '↑',
  '^d': '↓',
  '^l': '←',
  '^r': '→',
  '^lu': '↖',
  '^ru': '↗',
  '^ld': '↙',
  '^rd': '↘',
  r1: 'Ⅰ',
  r2: 'Ⅱ',
  r3: 'Ⅲ',
  'no-entry': '⊘',
  critical: '◈',
  safety: '▣',
  hazard: '⚠',
  comma: ',',
}

type ColorDirective = 'Y' | 'R' | 'L' | 'B'

function isColorDirective(s: string): s is ColorDirective {
  return s === 'Y' || s === 'R' || s === 'L' || s === 'B'
}

/**
 * Parse X-Plane sign text and return a plain-text representation.
 *
 * If the sign has a back side (separated by `{@@}`), only the front is used
 * since map labels can only show one string.
 */
export function parseSignText(rawText: string): string {
  if (!rawText) return ''

  // Split front/back on {@@}
  const sepIdx = rawText.indexOf('{@@}')
  const frontText = sepIdx !== -1 ? rawText.slice(0, sepIdx) : rawText

  // Check for comma-delimited syntax: {@Y,^l,C}
  const commaMatch = frontText.match(/^\{@([YRLB]),(.+)\}$/)
  if (commaMatch) {
    return parseCommaDelimited(commaMatch[2])
  }

  return parseSignSide(frontText)
}

function parseSignSide(text: string): string {
  let result = ''
  let i = 0

  while (i < text.length) {
    if (text[i] === '{') {
      const closeIdx = text.indexOf('}', i)
      if (closeIdx === -1) {
        result += text.slice(i)
        break
      }

      const content = text.slice(i + 1, closeIdx)

      // Color directive: @Y, @R, @L, @B — skip, we only want text
      if (content.startsWith('@') && content.length === 2 && isColorDirective(content[1])) {
        // skip
      }
      // Glyph in braces
      else if (GLYPH_MAP[content]) {
        result += GLYPH_MAP[content]
      }
      // Unknown bracketed — ignore

      i = closeIdx + 1
    }
    // Standalone @ directive (without braces)
    else if (text[i] === '@' && i + 1 < text.length && isColorDirective(text[i + 1])) {
      i += 2
    }
    // Standalone ^ glyph codes
    else if (text[i] === '^' && i + 1 < text.length) {
      // Try 2-char suffix first (lu, ru, ld, rd)
      if (i + 2 < text.length) {
        const twoCharKey = `^${text[i + 1]}${text[i + 2]}`
        if (GLYPH_MAP[twoCharKey]) {
          result += GLYPH_MAP[twoCharKey]
          i += 3
          continue
        }
      }
      // Single char suffix (u, d, l, r)
      const oneCharKey = `^${text[i + 1]}`
      if (GLYPH_MAP[oneCharKey]) {
        result += GLYPH_MAP[oneCharKey]
        i += 2
      } else {
        result += text[i]
        i++
      }
    }
    // Underscore = space
    else if (text[i] === '_') {
      result += ' '
      i++
    }
    // Asterisk = bullet
    else if (text[i] === '*') {
      result += '•'
      i++
    }
    // Pipe = divider
    else if (text[i] === '|') {
      result += '|'
      i++
    }
    // Regular character
    else {
      result += text[i]
      i++
    }
  }

  return result.trim()
}

function parseCommaDelimited(content: string): string {
  const parts = content.split(',')
  let text = ''

  for (const part of parts) {
    const trimmed = part.trim()
    if (GLYPH_MAP[trimmed]) {
      text += GLYPH_MAP[trimmed]
    } else if (trimmed === '_') {
      text += ' '
    } else if (trimmed === '*') {
      text += '•'
    } else {
      text += trimmed
    }
  }

  return text.trim()
}
