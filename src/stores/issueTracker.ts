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
    const tracked = await getItem<TrackedIssue[]>(STORAGE_KEYS.REPORTED_ISSUES)
    logDebug(`[issueTracker] tracked issues in storage: ${JSON.stringify(tracked)}`, 'issue-tracker')

    if (!tracked || tracked.length === 0) {
      logDebug('[issueTracker] no tracked issues, skipping check', 'issue-tracker')
      return
    }

    const updates: IssueUpdate[] = []
    const remaining: TrackedIssue[] = []

    for (const issue of tracked) {
      logDebug(`[issueTracker] checking #${issue.issueNumber} since ${issue.lastCheckedAt}`, 'issue-tracker')
      try {
        const result = await invoke<IssueUpdateResult>('check_issue_updates', {
          issueNumber: issue.issueNumber,
          since: issue.lastCheckedAt,
        })
        logDebug(`[issueTracker] #${issue.issueNumber} state=${result.state} new_comments=${result.new_comments.length}`, 'issue-tracker')

        const closed = result.state === 'closed'
        const hasNewComments = result.new_comments.length > 0
        if (closed || hasNewComments) {
          updates.push({ issue, closed, newComments: result.new_comments })
        }
        if (!closed) {
          remaining.push({
            ...issue,
            state: 'open',
            commentCount: result.total_comments,
            lastCheckedAt: new Date().toISOString(),
          })
        }
        // Closed issues are dropped from tracking
      } catch (e) {
        logError(`[issueTracker] check failed for #${issue.issueNumber}: ${e}`, 'issue-tracker')
        remaining.push(issue)
      }
    }

    await setItem(STORAGE_KEYS.REPORTED_ISSUES, remaining)
    logDebug(`[issueTracker] updates found: ${updates.length}, remaining tracked: ${remaining.length}`, 'issue-tracker')

    if (updates.length > 0) {
      pendingUpdates.value = updates
    }
  }

  function clearUpdates() {
    pendingUpdates.value = []
  }

  return { pendingUpdates, checkAllTrackedIssues, clearUpdates }
})
