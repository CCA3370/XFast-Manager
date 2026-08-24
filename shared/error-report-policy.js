export const REPORTABLE_ERROR_CODES = Object.freeze([
  'database_error',
  'migration_failed',
  'internal',
])

export const NON_REPORTABLE_ERROR_CODES = Object.freeze([
  'validation_failed',
  'permission_denied',
  'not_found',
  'conflict_exists',
  'corrupted_data',
  'network_error',
  'archive_error',
  'password_required',
  'incorrect_password',
  'cancelled',
  'insufficient_space',
  'security_violation',
  'timeout',
  'environment_error',
  'external_software_blocked',
])

const NON_REPORTABLE_CODE_SET = new Set(NON_REPORTABLE_ERROR_CODES)
const REPORTABLE_CODE_SET = new Set(REPORTABLE_ERROR_CODES)

const OPERATIONAL_ERROR_PATTERNS = Object.freeze([
  'invalid or incomplete zip archive',
  'invalid or incomplete 7z archive',
  'invalid or incomplete rar archive',
  'this archive appears to be incomplete',
  'this rar archive could not be extracted',
  'this 7z archive could not be extracted',
  'invalid zip archive',
  'could not find eocd',
  'not a rar archive',
  'badarchive@open',
  'ewrite@process',
  'failed to fill whole buffer',
  'file content does not match the downloaded package',
  'exec format error',
  'not runnable on this system',
  'not a valid windows executable',
  'invalid airport source path',
  'airport flatten source is no longer available',
  'airport flatten source was not found',
  'apt.dat not found:',
  'source file is no longer available:',
  'source path is not a regular file or directory:',
  'source path is neither file nor directory',
  'x-plane path does not exist',
  'x-plane folder does not exist',
  'x-plane executable not found',
  'x-plane installation was not found',
  'cannot install an add-on from inside the x-plane directory',
  'cannot install from x-plane directory',
  'cannot install from the x-plane directory',
  'no gateway scenery selected',
  '[permission_denied]',
  '(os error 1)',
  '(os error 2)',
  '(os error 5)',
  '(os error 21)',
  '(os error 225)',
  '(os error 483)',
  '(os error 999)',
  'no such file or directory',
  'operation not permitted',
  'permission denied',
  'access is denied',
  'contains a virus or potentially unwanted',
  'file contains a virus',
  'fatal device hardware error',
  'i/o device error',
  'the device is not ready',
  'custom scenery folder not found',
  'plugins folder not found',
  'custom data folder not found',
  'aircraft folder not found',
  'target directory does not exist',
  'failed to create target directory',
  'insufficient disk space',
  'not enough disk space',
  'no space left on device',
])

function normalizeCode(code) {
  return typeof code === 'string' ? code.trim().toLowerCase() : ''
}

export function isOperationalErrorMessage(message) {
  const lower = String(message || '').toLowerCase()

  if (OPERATIONAL_ERROR_PATTERNS.some((pattern) => lower.includes(pattern))) {
    return true
  }

  if (
    lower.includes('[cancelled]') ||
    lower.includes('[canceled]') ||
    lower.includes('cancelled by user') ||
    lower.includes('canceled by user') ||
    lower.includes('operation cancelled') ||
    lower.includes('operation canceled')
  ) {
    return true
  }

  return (
    lower.includes('[not_found]') &&
    [
      'folder not found',
      'livery folder not found',
      'file not found',
      'path not found',
      'directory not found',
      'source file',
      'source path',
      'target directory',
      'screenshot not found',
      'media file',
    ].some((pattern) => lower.includes(pattern))
  )
}

export function isReportableError({ code, message, reportable } = {}) {
  const normalizedCode = normalizeCode(code)

  if (reportable === false || NON_REPORTABLE_CODE_SET.has(normalizedCode)) {
    return false
  }

  if (isOperationalErrorMessage(message)) {
    return false
  }

  if (reportable === true || REPORTABLE_CODE_SET.has(normalizedCode)) {
    return true
  }

  // Legacy plain-string errors remain reportable unless they match a known operational failure.
  return true
}

export function normalizeErrorFingerprintInput({ code, origin, operation, message } = {}) {
  const normalizedMessage = String(message || '')
    .toLowerCase()
    .replace(/[a-z]:[\\/](?:[^\s:()[\]{}]+[\\/]?)+/gi, '<path>')
    .replace(/\/(?:[^\s:()[\]{}]+\/)+[^\s:()[\]{}]*/g, '<path>')
    .replace(/\b[0-9a-f]{8}-[0-9a-f-]{27,}\b/gi, '<id>')
    .replace(/\b0x[0-9a-f]+\b/gi, '<hex>')
    .replace(/\s+/g, ' ')
    .trim()

  return [
    normalizeCode(code) || 'legacy',
    String(origin || 'unknown')
      .trim()
      .toLowerCase(),
    String(operation || 'unknown')
      .trim()
      .toLowerCase(),
    normalizedMessage,
  ].join('|')
}
