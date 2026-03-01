import { invoke } from '@tauri-apps/api/core'
import { logError } from '@/services/logger'
import { getItem, setItem, STORAGE_KEYS, type TrackedIssue } from '@/services/storage'
import { useIssueTrackerStore } from '@/stores/issueTracker'

export interface BugReportToast {
  success: (message: string) => void
  error: (message: string) => void
}

export interface SubmitBugReportParams {
  errorTitle: string
  errorMessage: string
  category?: string
  timeoutMs?: number
  t: (key: string, values?: Record<string, unknown>) => string
  toast: BugReportToast
}

const DEFAULT_BUG_REPORT_TIMEOUT_MS = 20000

function buildFallbackBugReportUrl(errorTitle: string, errorMessage: string, logs: string): string {
  const fallbackTitle = `[Bug]: ${(errorTitle || errorMessage).slice(0, 80)}`
  const fallbackBody = [
    '### Bug Report (Auto-submitted)',
    '',
    '**Error Message**',
    '```',
    errorMessage,
    '```',
    '',
    '**Logs**',
    '<details>',
    '<summary>Click to expand logs</summary>',
    '',
    '```',
    logs.slice(0, 5000),
    '```',
    '</details>',
  ].join('\n')

  return `https://github.com/CCA3370/XFast-Manager/issues/new?template=bug_report.yml&labels=${encodeURIComponent('bug')}&title=${encodeURIComponent(fallbackTitle)}&body=${encodeURIComponent(fallbackBody)}`
}

async function trackReportedIssue(
  issueNumber: number,
  issueTitle: string,
  issueUrl: string,
  feedbackPreview: string,
) {
  if (issueNumber <= 0) return

  try {
    const now = new Date().toISOString()
    const newEntry: TrackedIssue = {
      issueNumber,
      issueTitle,
      issueUrl,
      state: 'open',
      commentCount: 0,
      reportedAt: now,
      lastCheckedAt: now,
      source: 'auto-report',
      feedbackType: 'bug',
      feedbackContentPreview: feedbackPreview.slice(0, 200),
    }

    try {
      const issueTrackerStore = useIssueTrackerStore()
      await issueTrackerStore.appendTrackedIssue(newEntry)
      return
    } catch (storeErr) {
      logError(`Failed to append tracked issue via store, fallback to storage: ${storeErr}`, 'bug-report')
    }

    const existing = (await getItem<TrackedIssue[]>(STORAGE_KEYS.REPORTED_ISSUES)) ?? []
    const deduplicated = existing.filter((issue) => issue.issueNumber !== issueNumber)
    const updated = [newEntry, ...deduplicated].slice(0, 100)
    await setItem(STORAGE_KEYS.REPORTED_ISSUES, updated)
  } catch (trackErr) {
    logError(`Failed to track reported issue: ${trackErr}`, 'bug-report')
  }
}

export async function submitBugReport(params: SubmitBugReportParams): Promise<void> {
  const { errorTitle, errorMessage, category = 'Other', timeoutMs, t, toast } = params

  let logs: string
  try {
    logs = await invoke<string>('get_all_logs')
  } catch {
    logs = '(failed to retrieve logs)'
  }

  const fallbackTitle = `[Bug]: ${(errorTitle || errorMessage).slice(0, 80)}`
  const fallbackUrl = buildFallbackBugReportUrl(errorTitle, errorMessage, logs)
  let submitTimeoutId: ReturnType<typeof setTimeout> | null = null

  try {
    const result = await Promise.race<{ issue_url: string; issue_number: number }>([
      invoke<{ issue_url: string; issue_number: number }>('create_bug_report_issue', {
        errorTitle,
        errorMessage,
        logs,
        category,
      }),
      new Promise<{ issue_url: string; issue_number: number }>((_, reject) => {
        submitTimeoutId = setTimeout(() => {
          reject(new Error('BUG_REPORT_SUBMIT_TIMEOUT'))
        }, timeoutMs ?? DEFAULT_BUG_REPORT_TIMEOUT_MS)
      }),
    ])

    toast.success(t('modal.bugReportSubmitted'))
    await invoke('open_url', { url: result.issue_url })
    await trackReportedIssue(result.issue_number, fallbackTitle, result.issue_url, errorMessage)
  } catch {
    try {
      await invoke('open_url', { url: fallbackUrl })
      toast.success(t('modal.bugReportOpened'))
    } catch (e2) {
      logError(`Failed to open bug report URL: ${e2}`, 'bug-report')
      toast.error(t('modal.bugReportFailed'))
    }
  } finally {
    if (submitTimeoutId !== null) {
      clearTimeout(submitTimeoutId)
    }
  }
}
