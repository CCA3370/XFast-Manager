import { describe, expect, it } from 'vitest'
import { getErrorReportPolicy, shouldHideBugReportForMessage } from './index'

const operationalErrors = [
  'Error performing inpage operation. (os error 999)',
  '[internal] No such file or directory (os error 2)',
  'Operation not permitted (os error 1)',
  'The device is not ready. (os error 21)',
  'failed to fill whole buffer',
  'Insufficient disk space for atomic installation',
  'X-Plane executable not found in the selected folder',
  'Cannot install an add-on from inside the X-Plane directory',
  'Failed to scan (C:\\X-Plane 12\\Custom Scenery\\AXP_Florida_2): Cannot install from X-Plane directory. Please drag files from outside X-Plane folder',
  'Operation did not complete successfully because the file contains a virus (os error 225)',
  'Invalid or incomplete 7z archive',
  'File verification failed for 1 file(s): aircraft.acf: file content does not match the downloaded package',
]

describe('error report policy', () => {
  it.each(operationalErrors)('does not report an operational error: %s', (message) => {
    expect(shouldHideBugReportForMessage(message)).toBe(true)
    expect(getErrorReportPolicy(message).reportable).toBe(false)
  })

  it.each([
    'validation_failed',
    'permission_denied',
    'not_found',
    'corrupted_data',
    'network_error',
    'archive_error',
    'cancelled',
    'insufficient_space',
    'timeout',
  ] as const)('does not report structured %s errors', (code) => {
    expect(getErrorReportPolicy({ code, message: 'Example failure' }).reportable).toBe(false)
  })

  it('keeps unexpected internal failures reportable', () => {
    expect(
      getErrorReportPolicy({
        code: 'internal',
        message: 'Unexpected invariant violation while building an installation task',
      }).reportable,
    ).toBe(true)
  })
})
