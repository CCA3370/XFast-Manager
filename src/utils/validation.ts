/**
 * Shared validation utilities
 */

export interface GlobValidationResult {
  isValid: boolean
  errorKey: string | null
}

/**
 * Validate a glob pattern and return an error message key if invalid.
 * The error key should be used with i18n to get the localized message.
 *
 * @param pattern - The glob pattern to validate
 * @returns Error key string if invalid, null if valid
 */
export function validateGlobPattern(pattern: string): string | null {
  // Check for empty pattern
  if (!pattern || pattern.trim() === '') {
    return null // Empty is OK, will be filtered
  }

  // Check for unbalanced brackets
  let bracketDepth = 0
  let braceDepth = 0

  for (let i = 0; i < pattern.length; i++) {
    const char = pattern[i]
    const prevChar = i > 0 ? pattern[i - 1] : ''

    // Skip escaped characters
    if (prevChar === '\\') continue

    if (char === '[') bracketDepth++
    if (char === ']') bracketDepth--
    if (char === '{') braceDepth++
    if (char === '}') braceDepth--

    // Check for negative depth (closing before opening)
    if (bracketDepth < 0) return 'settings.patternUnbalancedBracket'
    if (braceDepth < 0) return 'settings.patternUnbalancedBrace'
  }

  // Check final balance
  if (bracketDepth !== 0) return 'settings.patternUnbalancedBracket'
  if (braceDepth !== 0) return 'settings.patternUnbalancedBrace'

  // Check for invalid characters in pattern
  if (pattern.includes('//')) return 'settings.patternInvalidSlash'

  return null
}

/**
 * Validate a list of glob patterns.
 *
 * @param patterns - Array of patterns to validate
 * @returns Object with errors record (index -> error key) and valid patterns array
 */
export function validateGlobPatterns(patterns: string[]): {
  errors: Record<number, string>
  validPatterns: string[]
} {
  const errors: Record<number, string> = {}
  const validPatterns: string[] = []

  patterns.forEach((pattern, index) => {
    const trimmed = pattern.trim()
    if (trimmed === '') return

    const error = validateGlobPattern(trimmed)
    if (error) {
      errors[index] = error
    } else {
      validPatterns.push(trimmed)
    }
  })

  return { errors, validPatterns }
}
