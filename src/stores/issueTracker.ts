import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getItem, setItem, STORAGE_KEYS, type TrackedIssue } from '@/services/storage'

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
    if (!tracked || tracked.length === 0) return

    const updates: IssueUpdate[] = []
    const remaining: TrackedIssue[] = []

    for (const issue of tracked) {
      try {
        const result = await invoke<IssueUpdateResult>('check_issue_updates', {
          issueNumber: issue.issueNumber,
          since: issue.lastCheckedAt,
        })
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
      } catch {
        remaining.push(issue) // Keep on error, try again next time
      }
    }

    await setItem(STORAGE_KEYS.REPORTED_ISSUES, remaining)
    if (updates.length > 0) {
      pendingUpdates.value = updates
    }
  }

  function clearUpdates() {
    pendingUpdates.value = []
  }

  return { pendingUpdates, checkAllTrackedIssues, clearUpdates }
})
