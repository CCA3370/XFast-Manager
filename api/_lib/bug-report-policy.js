import { isReportableError } from '../../shared/error-report-policy.js'

const LEGACY_VERIFICATION_MESSAGE =
  'Verification failed: Verification failed: 1 files still failing after retries'

// Old clients omitted the error category or root cause from these dialogs.
// Require the matching modal and failure records from the same operation.
function isLegacyOperationalReport(errorTitle, errorMessage, logs) {
  if (typeof logs !== 'string') return false

  const errors = Array.from(
    logs.slice(-50_000).matchAll(/^\[[\d\- :]+\] \[ERROR\] \[([\w-]+)\] ([^\r\n]+)\r?$/gm),
    ([, context, message]) => ({ context, message }),
  )
  const modalIndex = errors.findLastIndex(
    ({ context, message }) => context === 'ui' && message.startsWith('[Modal] '),
  )
  const modalText = errorTitle ? `${errorTitle}: ${errorMessage}` : errorMessage
  if (modalIndex < 1 || errors[modalIndex].message !== `[Modal] ${modalText}`) return false

  const previous = errors[modalIndex - 1]
  if (
    !errorTitle &&
    previous.context === 'scenery' &&
    previous.message.startsWith('Failed to delete scenery entry [permission_denied]: ')
  ) {
    return true
  }

  if (
    !errorTitle ||
    errorMessage !== LEGACY_VERIFICATION_MESSAGE ||
    previous.context !== 'installation' ||
    previous.message !== modalText
  ) {
    return false
  }

  const installerIndex = errors.findLastIndex(
    ({ context, message }, index) =>
      index < modalIndex && context === 'installer' && message === `Failed to install ${modalText}`,
  )
  const summary = errors[installerIndex - 1]
  const hashError = errors[installerIndex - 2]
  return (
    summary?.context === 'installer' &&
    summary.message === 'Verification failed: 1 files' &&
    hashError?.context === 'verifier' &&
    hashError.message.startsWith('Failed to compute hash for ') &&
    hashError.message.endsWith('(os error 225)')
  )
}

export function canAutoReport({ errorTitle, errorMessage, errorCode, reportable, logs }) {
  return (
    isReportableError({ code: errorCode, message: errorMessage, reportable }) &&
    (Boolean(errorCode) || !isLegacyOperationalReport(errorTitle, errorMessage, logs))
  )
}

export function rejectOperationalReport(res) {
  return res.status(422).json({
    code: 'bug_report_not_allowed',
    error: 'This operational error is not eligible for automatic bug reporting.',
  })
}
