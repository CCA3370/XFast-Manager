import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getItem, setItem, STORAGE_KEYS, type TrackedIssue } from '@/services/storage'
import { logDebug, logError } from '@/services/logger'

interface IssueUpdateResult {
  state: string
  total_comments: number
  new_comments: Array<{ author: string; body: string; created_at: string }>
}

export interface IssueUpdate {
  issue: TrackedIssue
  closed: boolean
  newComments: Array<{ author: string; body: string; created_at: string }>
}

export const useIssueTrackerStore = defineStore('issueTracker', () => {
  const pendingUpdates = ref<IssueUpdate[]>([])

  async function checkAllTrackedIssues(): Promise<void> {
    const tracked = (await getItem<TrackedIssue[]>(STORAGE_KEYS.REPORTED_ISSUES)) ?? []
    const unconfirmed = (await getItem<IssueUpdate[]>(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES)) ?? []

    logDebug(
      `[issueTracker] tracked: ${tracked.length}, unconfirmed: ${unconfirmed.length}`,
      'issue-tracker',
    )

    if (tracked.length === 0 && unconfirmed.length === 0) {
      logDebug('[issueTracker] nothing to check', 'issue-tracker')
      return
    }

    const now = new Date().toISOString()

    // Build a unified map of all issues to check.
    // Tracked open issues use their lastCheckedAt.
    // Closed-but-unconfirmed issues use the lastCheckedAt stored in the unconfirmed entry
    // so we can still pick up new comments posted after the previous check.
    const issuesToCheck = new Map<number, { issue: TrackedIssue; isTracked: boolean }>()
    for (const issue of tracked) {
      issuesToCheck.set(issue.issueNumber, { issue, isTracked: true })
    }
    for (const update of unconfirmed) {
      if (!issuesToCheck.has(update.issue.issueNumber)) {
        issuesToCheck.set(update.issue.issueNumber, { issue: update.issue, isTracked: false })
      }
    }

    // Index existing unconfirmed entries by issue number
    const unconfirmedMap = new Map<number, IssueUpdate>()
    for (const u of unconfirmed) {
      unconfirmedMap.set(u.issue.issueNumber, u)
    }

    // Run all checks
    const checkResults = new Map<number, IssueUpdateResult>()
    const remaining: TrackedIssue[] = []

    for (const [issueNumber, { issue, isTracked }] of issuesToCheck) {
      logDebug(
        `[issueTracker] checking #${issueNumber} since ${issue.lastCheckedAt}`,
        'issue-tracker',
      )
      try {
        const result = await invoke<IssueUpdateResult>('check_issue_updates', {
          issueNumber,
          since: issue.lastCheckedAt,
        })
        logDebug(
          `[issueTracker] #${issueNumber} state=${result.state} new_comments=${result.new_comments.length}`,
          'issue-tracker',
        )
        checkResults.set(issueNumber, result)

        if (isTracked && result.state !== 'closed') {
          remaining.push({
            ...issue,
            state: 'open',
            commentCount: result.total_comments,
            lastCheckedAt: now,
          })
        }
      } catch (e) {
        logError(`[issueTracker] check failed for #${issueNumber}: ${e}`, 'issue-tracker')
        if (isTracked) {
          remaining.push(issue) // keep tracking, retry next time
        }
      }
    }

    // Build merged unconfirmed list
    const newUnconfirmed: IssueUpdate[] = []
    const processedNumbers = new Set<number>()

    // Merge existing unconfirmed entries with fresh check results
    for (const existing of unconfirmed) {
      const issueNumber = existing.issue.issueNumber
      processedNumbers.add(issueNumber)

      const result = checkResults.get(issueNumber)
      if (result) {
        // Append any new comments to the already-unconfirmed ones
        const mergedComments = [...existing.newComments, ...result.new_comments]
        const closed = existing.closed || result.state === 'closed'
        newUnconfirmed.push({
          issue: { ...existing.issue, lastCheckedAt: now },
          closed,
          newComments: mergedComments,
        })
      } else {
        // Check failed or no new activity — preserve existing unconfirmed data unchanged
        newUnconfirmed.push(existing)
      }
    }

    // Add brand-new updates not previously in unconfirmed
    for (const [issueNumber, result] of checkResults) {
      if (processedNumbers.has(issueNumber)) continue
      const closed = result.state === 'closed'
      const hasNewComments = result.new_comments.length > 0
      if (closed || hasNewComments) {
        const issueInfo = issuesToCheck.get(issueNumber)!
        newUnconfirmed.push({
          issue: { ...issueInfo.issue, lastCheckedAt: now },
          closed,
          newComments: result.new_comments,
        })
      }
    }

    logDebug(
      `[issueTracker] updates: ${newUnconfirmed.length}, remaining tracked: ${remaining.length}`,
      'issue-tracker',
    )

    await setItem(STORAGE_KEYS.REPORTED_ISSUES, remaining)
    await setItem(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES, newUnconfirmed)

    if (newUnconfirmed.length > 0) {
      pendingUpdates.value = newUnconfirmed
    }
  }

  async function confirmUpdates(): Promise<void> {
    await setItem(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES, [])
    pendingUpdates.value = []
  }

  return { pendingUpdates, checkAllTrackedIssues, confirmUpdates }
})
