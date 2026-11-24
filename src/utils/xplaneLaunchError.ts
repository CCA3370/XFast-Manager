import { getErrorMessage, parseApiError, type ApiError } from '@/types'

const SMART_APP_CONTROL_DIAGNOSTICS_URL =
  'https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/test-your-app-with-smart-app-control#checking-event-logs'

const CODE_INTEGRITY_EVENT_FIELDS = 'File Name · Process Name · PolicyName · PolicyId · Status'

interface LaunchErrorPresentation {
  error: ApiError | string
  title: string
  hideReport?: boolean
}

export function getXPlaneLaunchErrorPresentation(
  error: unknown,
  t: (key: string) => string,
): LaunchErrorPresentation {
  if (error instanceof Error && error.message === 'timeout') {
    return { error: t('home.launchTimeout'), title: '' }
  }

  const apiError = parseApiError(error)
  if (apiError?.code === 'application_control_blocked') {
    const key = 'home.applicationControlBlocked'
    const message = [
      t(`${key}.description`),
      t(`${key}.eventLog`),
      `${t(`${key}.eventDetails`)}\n${CODE_INTEGRITY_EVENT_FIELDS}`,
      t(`${key}.signatureDetails`),
      t(`${key}.resolution`),
      // The launch target is not necessarily the file named by event 3077.
      apiError.details ? `${t(`${key}.path`)} ${apiError.details}` : '',
      `${t(`${key}.systemError`)} ${apiError.message}`,
      `${t(`${key}.documentation`)} ${SMART_APP_CONTROL_DIAGNOSTICS_URL}`,
    ]
      .filter(Boolean)
      .join('\n\n')

    return {
      error: { ...apiError, message, reportable: false },
      title: t(`${key}.title`),
      hideReport: true,
    }
  }

  const message = getErrorMessage(error)
  if (message.startsWith('ELEVATION_REQUIRED:')) {
    const exePath = message.substring('ELEVATION_REQUIRED:'.length)
    return {
      error:
        t('home.elevationRequired.title') +
        '\n\n' +
        t('home.elevationRequired.causes') +
        '\n' +
        t('home.elevationRequired.cause1') +
        '\n' +
        t('home.elevationRequired.cause2') +
        '\n' +
        t('home.elevationRequired.cause3') +
        '\n\n' +
        t('home.elevationRequired.solutions') +
        '\n' +
        t('home.elevationRequired.solution1') +
        '\n' +
        t('home.elevationRequired.solution2') +
        '\n\n' +
        t('home.elevationRequired.path') +
        ' ' +
        exePath,
      title: '',
      hideReport: true,
    }
  }

  return { error: t('home.launchFailed') + ': ' + message, title: '' }
}
