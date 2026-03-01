import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  getItem,
  setItem,
  STORAGE_KEYS,
  type FeedbackType,
  type TrackedIssue,
  type TrackedIssueSource,
} from '@/services/storage'
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

export interface IssueDetailComment {
  id: number
  author: string
  body: string
  created_at: string
  updated_at: string
}

export interface IssueDetailIssue {
  number: number
  title: string
  state: string
  html_url: string
  comments: number
  created_at: string
  updated_at: string
}

export interface IssueDetailResult {
  issue: IssueDetailIssue
  comments: IssueDetailComment[]
  page: number
  per_page: number
  has_more: boolean
  from_cache?: boolean
}

const MAX_TRACKED_ISSUES = 100

function inferIssueSource(issue: TrackedIssue): TrackedIssueSource {
  if (issue.source) return issue.source
  if (issue.feedbackType) return 'feedback'
  const title = issue.issueTitle || ''
  if (title.startsWith('[Feedback]')) return 'feedback'
  if (title.startsWith('[Library Link]')) return 'library-link'
  return 'auto-report'
}

function normalizeTrackedIssue(issue: TrackedIssue): TrackedIssue {
  const now = new Date().toISOString()
  const reportedAt = issue.reportedAt || now
  return {
    issueNumber: issue.issueNumber,
    issueTitle: issue.issueTitle,
    issueUrl: issue.issueUrl,
    state: issue.state === 'closed' ? 'closed' : 'open',
    commentCount: typeof issue.commentCount === 'number' ? issue.commentCount : 0,
    reportedAt,
    lastCheckedAt: issue.lastCheckedAt || reportedAt,
    source: inferIssueSource(issue),
    feedbackType: issue.feedbackType,
    feedbackContentPreview: issue.feedbackContentPreview,
    appVersion: issue.appVersion,
    os: issue.os,
    arch: issue.arch,
  }
}

export const useIssueTrackerStore = defineStore('issueTracker', () => {
  const pendingUpdates = ref<IssueUpdate[]>([])
  const trackedIssues = ref<TrackedIssue[]>([])
  const isInitialized = ref(false)

  const feedbackRecords = computed(() =>
    trackedIssues.value.filter((issue) => {
      const source = inferIssueSource(issue)
      return source === 'feedback' || source === 'auto-report'
    }),
  )

  const hasFeedbackRecords = computed(() => feedbackRecords.value.length > 0)
  const hasSubmittedFeedback = computed(() =>
    trackedIssues.value.some((issue) => inferIssueSource(issue) === 'feedback'),
  )

  async function initStore(): Promise<void> {
    if (isInitialized.value) return

    const storedTracked = (await getItem<TrackedIssue[]>(STORAGE_KEYS.REPORTED_ISSUES)) ?? []
    const normalized = storedTracked
      .map((issue) => normalizeTrackedIssue(issue))
      .filter((issue) => issue.issueNumber > 0 && issue.issueUrl)
    trackedIssues.value = normalized

    const storedUnconfirmed =
      (await getItem<IssueUpdate[]>(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES)) ?? []
    pendingUpdates.value = storedUnconfirmed

    // Persist normalized shape so old records gain source/default fields.
    await setItem(STORAGE_KEYS.REPORTED_ISSUES, normalized)

    isInitialized.value = true
  }

  async function appendTrackedIssue(
    issue: Omit<TrackedIssue, 'state' | 'commentCount' | 'reportedAt' | 'lastCheckedAt'> & {
      state?: TrackedIssue['state']
      commentCount?: number
      reportedAt?: string
      lastCheckedAt?: string
      source?: TrackedIssueSource
      feedbackType?: FeedbackType
    },
  ): Promise<void> {
    if (!isInitialized.value) {
      await initStore()
    }

    const now = new Date().toISOString()
    const incoming = normalizeTrackedIssue({
      ...issue,
      state: issue.state ?? 'open',
      commentCount: issue.commentCount ?? 0,
      reportedAt: issue.reportedAt ?? now,
      lastCheckedAt: issue.lastCheckedAt ?? now,
    })

    const current = [...trackedIssues.value]
    const existingIndex = current.findIndex((item) => item.issueNumber === incoming.issueNumber)
    if (existingIndex >= 0) {
      const existing = current[existingIndex]
      current.splice(existingIndex, 1)
      current.unshift(
        normalizeTrackedIssue({
          ...existing,
          ...incoming,
          reportedAt: existing.reportedAt || incoming.reportedAt,
        }),
      )
    } else {
      current.unshift(incoming)
    }

    trackedIssues.value = current.slice(0, MAX_TRACKED_ISSUES)
    await setItem(STORAGE_KEYS.REPORTED_ISSUES, trackedIssues.value)
  }

  async function checkAllTrackedIssues(): Promise<void> {
    if (!isInitialized.value) {
      await initStore()
    }

    const tracked = [...trackedIssues.value]
    const existingUnconfirmed =
      (await getItem<IssueUpdate[]>(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES)) ?? []

    logDebug(
      `[issueTracker] tracked: ${tracked.length}, unconfirmed: ${existingUnconfirmed.length}`,
      'issue-tracker',
    )

    if (tracked.length === 0 && existingUnconfirmed.length === 0) {
      logDebug('[issueTracker] nothing to check', 'issue-tracker')
      pendingUpdates.value = []
      return
    }

    const now = new Date().toISOString()
    const existingUnconfirmedMap = new Map<number, IssueUpdate>()
    for (const update of existingUnconfirmed) {
      existingUnconfirmedMap.set(update.issue.issueNumber, update)
    }

    const nextTracked: TrackedIssue[] = []
    const nextUnconfirmedMap = new Map<number, IssueUpdate>()

    for (const issue of tracked) {
      logDebug(
        `[issueTracker] checking #${issue.issueNumber} since ${issue.lastCheckedAt}`,
        'issue-tracker',
      )
      try {
        const result = await invoke<IssueUpdateResult>('check_issue_updates', {
          issueNumber: issue.issueNumber,
          since: issue.lastCheckedAt,
        })
        logDebug(
          `[issueTracker] #${issue.issueNumber} state=${result.state} new_comments=${result.new_comments.length}`,
          'issue-tracker',
        )

        const updatedIssue = normalizeTrackedIssue({
          ...issue,
          state: result.state === 'closed' ? 'closed' : 'open',
          commentCount: result.total_comments,
          lastCheckedAt: now,
        })
        nextTracked.push(updatedIssue)

        const wasClosed = issue.state === 'closed'
        const isClosedNow = result.state === 'closed'
        const justClosed = !wasClosed && isClosedNow
        const existing = existingUnconfirmedMap.get(issue.issueNumber)
        const mergedComments = [...(existing?.newComments ?? []), ...result.new_comments]
        const closed = (existing?.closed ?? false) || justClosed
        if (closed || mergedComments.length > 0) {
          nextUnconfirmedMap.set(issue.issueNumber, {
            issue: updatedIssue,
            closed,
            newComments: mergedComments,
          })
        }
      } catch (e) {
        logError(`[issueTracker] check failed for #${issue.issueNumber}: ${e}`, 'issue-tracker')
        nextTracked.push(issue)
        const existing = existingUnconfirmedMap.get(issue.issueNumber)
        if (existing) {
          nextUnconfirmedMap.set(issue.issueNumber, existing)
        }
      }
    }

    // Preserve unconfirmed updates for records no longer tracked.
    for (const [issueNumber, update] of existingUnconfirmedMap) {
      if (!nextUnconfirmedMap.has(issueNumber)) {
        nextUnconfirmedMap.set(issueNumber, update)
      }
    }

    const nextUnconfirmed = Array.from(nextUnconfirmedMap.values())

    logDebug(
      `[issueTracker] updates: ${nextUnconfirmed.length}, tracked total: ${nextTracked.length}`,
      'issue-tracker',
    )

    trackedIssues.value = nextTracked
    pendingUpdates.value = nextUnconfirmed
    await setItem(STORAGE_KEYS.REPORTED_ISSUES, nextTracked)
    await setItem(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES, nextUnconfirmed)
  }

  async function confirmUpdates(): Promise<void> {
    await setItem(STORAGE_KEYS.UNCONFIRMED_ISSUE_UPDATES, [])
    pendingUpdates.value = []
  }

  async function getIssueDetail(
    issueNumber: number,
    page = 1,
    perPage = 30,
  ): Promise<IssueDetailResult> {
    try {
      return await invoke<IssueDetailResult>('get_issue_detail', {
        issueNumber,
        page,
        perPage,
      })
    } catch (error) {
      const message = String(error || '')
      const lowered = message.toLowerCase()
      const isRateLimited403 = lowered.includes('403')

      if (!isRateLimited403) {
        throw error
      }

      const cached = trackedIssues.value.find((item) => item.issueNumber === issueNumber)
      if (!cached) {
        throw error
      }

      logDebug(
        `[issueTracker] getIssueDetail fallback to cache for #${issueNumber}: ${message}`,
        'issue-tracker',
      )

      return {
        issue: {
          number: cached.issueNumber,
          title: cached.issueTitle || `#${cached.issueNumber}`,
          state: cached.state === 'closed' ? 'closed' : 'open',
          html_url: cached.issueUrl,
          comments: typeof cached.commentCount === 'number' ? cached.commentCount : 0,
          created_at: cached.reportedAt || '',
          updated_at: cached.lastCheckedAt || cached.reportedAt || '',
        },
        comments: [],
        page: 1,
        per_page: perPage,
        has_more: false,
        from_cache: true,
      }
    }
  }

  async function postIssueComment(issueNumber: number, commentBody: string): Promise<void> {
    await invoke<{ ok: boolean }>('post_issue_comment', {
      issueNumber,
      commentBody,
    })
  }

  return {
    pendingUpdates,
    trackedIssues,
    feedbackRecords,
    hasFeedbackRecords,
    hasSubmittedFeedback,
    isInitialized,
    initStore,
    appendTrackedIssue,
    checkAllTrackedIssues,
    confirmUpdates,
    getIssueDetail,
    postIssueComment,
  }
})
