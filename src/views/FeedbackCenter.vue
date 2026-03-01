<template>
  <div class="feedback-page h-full overflow-hidden pt-3 px-3 pb-1 md:pt-4 md:px-4 md:pb-1">
    <div class="h-full flex flex-col gap-3">
      <div class="feedback-header flex items-start justify-between gap-3">
        <div class="min-w-0">
          <h1 class="text-lg md:text-xl font-semibold text-gray-900 dark:text-white">
            <AnimatedText>{{ $t('feedback.myFeedbackTitle') }}</AnimatedText>
          </h1>
          <p class="feedback-header-subtitle text-xs md:text-sm text-gray-500 dark:text-gray-400 mt-1">
            <AnimatedText>{{ $t('feedback.myFeedbackSubtitle') }}</AnimatedText>
          </p>
        </div>
        <button
          class="px-2.5 py-1.5 md:px-3 md:py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white text-xs md:text-sm font-medium transition-colors flex-shrink-0"
          @click="feedbackStore.openSubmitModal()"
        >
          <AnimatedText>{{ $t('feedback.newFeedback') }}</AnimatedText>
        </button>
      </div>

      <div class="flex-1 min-h-0 flex flex-col gap-3">
        <section
          class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md overflow-hidden flex flex-col transition-all duration-300 ease-out"
          :class="selectedRecord ? 'flex-none' : 'flex-1 min-h-0'"
        >
          <div class="px-3 py-2 border-b border-gray-200 dark:border-white/5 flex items-center justify-between gap-2">
            <div class="min-w-0">
              <h2 class="text-sm font-semibold text-gray-800 dark:text-gray-100">
                {{ $t('feedback.recordListTitle') }}
              </h2>
              <p class="text-[11px] text-gray-500 dark:text-gray-400 mt-0.5">
                {{ records.length }}
              </p>
            </div>
          </div>

          <div v-if="records.length === 0" class="flex-1 flex items-center justify-center px-4">
            <p class="text-sm text-gray-500 dark:text-gray-400 text-center">
              {{ $t('feedback.emptyRecords') }}
            </p>
          </div>

          <div v-else-if="selectedRecord" class="p-1.5">
            <button
              class="w-full text-left p-2.5 rounded-lg border transition-colors border-blue-300 dark:border-blue-500/40 bg-blue-50 dark:bg-blue-500/10"
              @click="selectIssue(selectedRecord.issueNumber)"
            >
              <div class="flex items-start justify-between gap-2">
                <p class="text-sm font-medium text-gray-900 dark:text-gray-100 line-clamp-2">
                  {{ selectedRecord.issueTitle }}
                </p>
                <span
                  class="text-[10px] px-1.5 py-0.5 rounded font-semibold uppercase tracking-wide"
                  :class="
                    selectedRecord.state === 'closed'
                      ? 'bg-green-100 text-green-700 dark:bg-green-500/20 dark:text-green-300'
                      : 'bg-amber-100 text-amber-700 dark:bg-amber-500/20 dark:text-amber-300'
                  "
                >
                  {{ selectedRecord.state === 'closed' ? $t('feedback.statusClosed') : $t('feedback.statusOpen') }}
                </span>
              </div>
              <p class="text-[11px] text-gray-500 dark:text-gray-400 mt-1">
                #{{ selectedRecord.issueNumber }} · {{ formatDate(selectedRecord.reportedAt) }}
              </p>
            </button>
          </div>

          <div v-else class="flex-1 overflow-y-auto p-1.5 space-y-1.5">
            <button
              v-for="record in records"
              :key="record.issueNumber"
              class="w-full text-left p-2.5 rounded-lg border transition-colors"
              :class="
                selectedIssueNumber === record.issueNumber
                  ? 'border-blue-300 dark:border-blue-500/40 bg-blue-50 dark:bg-blue-500/10'
                  : 'border-gray-200 dark:border-white/10 bg-white dark:bg-gray-900/30 hover:bg-gray-50 dark:hover:bg-gray-800/50'
              "
              @click="selectIssue(record.issueNumber)"
            >
              <div class="flex items-start justify-between gap-2">
                <p class="text-sm font-medium text-gray-900 dark:text-gray-100 line-clamp-2">
                  {{ record.issueTitle }}
                </p>
                <span
                  class="text-[10px] px-1.5 py-0.5 rounded font-semibold uppercase tracking-wide"
                  :class="
                    record.state === 'closed'
                      ? 'bg-green-100 text-green-700 dark:bg-green-500/20 dark:text-green-300'
                      : 'bg-amber-100 text-amber-700 dark:bg-amber-500/20 dark:text-amber-300'
                  "
                >
                  {{ record.state === 'closed' ? $t('feedback.statusClosed') : $t('feedback.statusOpen') }}
                </span>
              </div>
              <p class="text-[11px] text-gray-500 dark:text-gray-400 mt-1">
                #{{ record.issueNumber }} · {{ formatDate(record.reportedAt) }}
              </p>
            </button>
          </div>
        </section>

        <Transition
          @before-enter="onDetailBeforeEnter"
          @enter="onDetailEnter"
          @before-leave="onDetailBeforeLeave"
          @leave="onDetailLeave"
          @after-leave="onDetailAfterLeave"
        >
          <section
            v-if="detailVisible && selectedRecord"
            class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md overflow-hidden flex flex-col flex-1 min-h-0"
          >
            <div class="px-3 py-2.5 border-b border-gray-200 dark:border-white/5">
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <h2 class="text-sm font-semibold text-gray-800 dark:text-gray-100 truncate">
                    {{ selectedRecord.issueTitle }}
                  </h2>
                  <p class="text-[11px] text-gray-500 dark:text-gray-400 mt-0.5 flex flex-wrap gap-x-2 gap-y-0.5">
                    <span>#{{ selectedRecord.issueNumber }}</span>
                    <span>{{ $t('feedback.typeLabel') }}: {{ displayFeedbackType(selectedRecord.feedbackType) }}</span>
                    <span>{{ $t('feedback.commentCount') }}: {{ detailIssue?.comments ?? selectedRecord.commentCount }}</span>
                    <span>{{ $t('feedback.updatedAt') }}: {{ formatDate(detailIssue?.updated_at || selectedRecord.lastCheckedAt) }}</span>
                  </p>
                </div>
                <div class="flex items-center gap-1.5 flex-shrink-0">
                  <button
                    class="px-2 py-1 text-xs rounded-md bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 transition-colors"
                    @click="openIssue(selectedRecord.issueUrl)"
                  >
                    {{ $t('issueTracker.viewIssue') }} ↗
                  </button>
                  <button
                    class="w-7 h-7 rounded-md bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-600 dark:text-gray-200 transition-colors flex items-center justify-center"
                    :title="$t('common.close')"
                    @click="requestCloseDetail"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                      />
                    </svg>
                  </button>
                </div>
              </div>
            </div>

            <div class="flex-1 min-h-0 flex flex-col">
              <div class="flex-1 min-h-0 overflow-y-auto px-3 py-2.5 space-y-2.5">
                <div v-if="loadingDetail" class="text-sm text-gray-500 dark:text-gray-400">
                  {{ $t('feedback.loadingDetail') }}
                </div>
                <div v-else-if="comments.length === 0" class="text-sm text-gray-500 dark:text-gray-400">
                  {{ $t('feedback.noComments') }}
                </div>
                <div
                  v-for="comment in comments"
                  :key="comment.id + '-' + comment.created_at"
                  class="rounded-lg border border-gray-200 dark:border-white/10 bg-gray-50 dark:bg-gray-900/30 p-2.5"
                >
                  <div class="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400 mb-1">
                    <span class="font-semibold text-gray-700 dark:text-gray-200">@{{ comment.author }}</span>
                    <span>· {{ formatDate(comment.created_at) }}</span>
                  </div>
                  <p class="text-sm text-gray-700 dark:text-gray-200 whitespace-pre-wrap break-words">
                    {{ comment.body }}
                  </p>
                </div>

                <button
                  v-if="hasMoreComments && !loadingMoreComments"
                  class="px-2.5 py-1.5 rounded-md text-xs bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 transition-colors"
                  @click="loadMoreComments"
                >
                  {{ $t('feedback.loadMoreComments') }}
                </button>
                <p v-if="loadingMoreComments" class="text-xs text-gray-500 dark:text-gray-400">
                  {{ $t('feedback.loadingMoreComments') }}
                </p>
              </div>

              <div class="feedback-composer px-3 py-2 border-t border-gray-200 dark:border-white/5 space-y-1.5">
                <textarea
                  v-model="newComment"
                  rows="2"
                  maxlength="3000"
                  class="feedback-comment-input w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none overflow-y-auto"
                  :placeholder="$t('feedback.commentPlaceholder')"
                ></textarea>
                <div class="flex items-center justify-between">
                  <p class="text-[11px] text-gray-400 dark:text-gray-500">{{ newComment.length }}/3000</p>
                  <button
                    class="px-2.5 py-1.5 rounded-md bg-blue-600 hover:bg-blue-700 disabled:opacity-60 text-white text-xs md:text-sm font-medium transition-colors"
                    :disabled="submittingComment || !newComment.trim()"
                    @click="submitComment"
                  >
                    {{ submittingComment ? $t('feedback.sendingComment') : $t('feedback.sendComment') }}
                  </button>
                </div>
              </div>
            </div>
          </section>
        </Transition>

        <div v-if="!selectedRecord && records.length > 0" class="px-1">
          <p class="text-xs text-gray-500 dark:text-gray-400">
            {{ $t('feedback.selectHint') }}
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import AnimatedText from '@/components/AnimatedText.vue'
import { useIssueTrackerStore, type IssueDetailComment, type IssueDetailIssue } from '@/stores/issueTracker'
import { useToastStore } from '@/stores/toast'
import { logError } from '@/services/logger'
import { useFeedbackStore } from '@/stores/feedback'

const route = useRoute()
const { t } = useI18n()
const issueTrackerStore = useIssueTrackerStore()
const toast = useToastStore()
const feedbackStore = useFeedbackStore()

const selectedIssueNumber = ref<number | null>(null)
const detailIssue = ref<IssueDetailIssue | null>(null)
const comments = ref<IssueDetailComment[]>([])
const loadingDetail = ref(false)
const loadingMoreComments = ref(false)
const hasMoreComments = ref(false)
const currentPage = ref(1)
const detailVisible = ref(false)
const pendingClearAfterLeave = ref(false)

const newComment = ref('')
const submittingComment = ref(false)

const records = computed(() => {
  return [...issueTrackerStore.feedbackRecords].sort((a, b) => {
    return new Date(b.reportedAt).getTime() - new Date(a.reportedAt).getTime()
  })
})

const selectedRecord = computed(() => {
  if (!selectedIssueNumber.value) return null
  return records.value.find((record) => record.issueNumber === selectedIssueNumber.value) || null
})

function finishDetailTransition(element: HTMLElement) {
  element.style.height = ''
  element.style.opacity = ''
  element.style.transform = ''
  element.style.transition = ''
  element.style.overflow = ''
}

function onDetailBeforeEnter(el: Element) {
  const element = el as HTMLElement
  element.style.height = '0px'
  element.style.opacity = '0'
  element.style.transform = 'translateY(8px)'
  element.style.overflow = 'hidden'
}

function onDetailEnter(el: Element, done: () => void) {
  const element = el as HTMLElement
  const targetHeight = element.scrollHeight
  const transition =
    'height 240ms cubic-bezier(0.2, 0.8, 0.2, 1), opacity 200ms ease, transform 240ms cubic-bezier(0.2, 0.8, 0.2, 1)'

  let finished = false
  const complete = () => {
    if (finished) return
    finished = true
    element.removeEventListener('transitionend', onTransitionEnd)
    if (fallbackTimer !== null) {
      clearTimeout(fallbackTimer)
    }
    finishDetailTransition(element)
    done()
  }

  const onTransitionEnd = (event: TransitionEvent) => {
    if (event.target === element && event.propertyName === 'height') {
      complete()
    }
  }

  const fallbackTimer = window.setTimeout(complete, 320)
  element.addEventListener('transitionend', onTransitionEnd)

  requestAnimationFrame(() => {
    element.style.transition = transition
    element.style.height = `${targetHeight}px`
    element.style.opacity = '1'
    element.style.transform = 'translateY(0)'
  })
}

function onDetailBeforeLeave(el: Element) {
  const element = el as HTMLElement
  element.style.height = `${element.scrollHeight}px`
  element.style.opacity = '1'
  element.style.transform = 'translateY(0)'
  element.style.overflow = 'hidden'
}

function onDetailLeave(el: Element, done: () => void) {
  const element = el as HTMLElement
  const transition =
    'height 220ms cubic-bezier(0.4, 0, 0.2, 1), opacity 180ms ease, transform 220ms cubic-bezier(0.4, 0, 0.2, 1)'

  let finished = false
  const complete = () => {
    if (finished) return
    finished = true
    element.removeEventListener('transitionend', onTransitionEnd)
    if (fallbackTimer !== null) {
      clearTimeout(fallbackTimer)
    }
    finishDetailTransition(element)
    done()
  }

  const onTransitionEnd = (event: TransitionEvent) => {
    if (event.target === element && event.propertyName === 'height') {
      complete()
    }
  }

  const fallbackTimer = window.setTimeout(complete, 320)
  element.addEventListener('transitionend', onTransitionEnd)

  // Force reflow before switching to collapsed state.
  void element.offsetHeight
  requestAnimationFrame(() => {
    element.style.transition = transition
    element.style.height = '0px'
    element.style.opacity = '0'
    element.style.transform = 'translateY(8px)'
  })
}

function formatDate(iso: string): string {
  if (!iso) return ''
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

function displayFeedbackType(type?: string): string {
  switch (type) {
    case 'bug':
      return t('feedback.types.bug')
    case 'feature-request':
      return t('feedback.types.featureRequest')
    case 'improvement':
      return t('feedback.types.improvement')
    default:
      return t('feedback.types.other')
  }
}

async function openIssue(url: string) {
  const issueRedirectApiBase =
    import.meta.env.VITE_XFAST_ISSUE_REDIRECT_API_URL ||
    'https://x-fast-manager.vercel.app/api/issue-redirect'
  let finalUrl = String(url || '').trim()
  if (!finalUrl) return

  try {
    const parsed = new URL(finalUrl)
    const numberFromQuery = Number(parsed.searchParams.get('number') || parsed.searchParams.get('issueNumber') || 0)
    if (Number.isFinite(numberFromQuery) && numberFromQuery > 0) {
      finalUrl = `${issueRedirectApiBase}?number=${numberFromQuery}`
    } else {
      const issueTail = parsed.pathname.split('/').pop() ?? ''
      const issueNumber = Number(issueTail)
      if (parsed.hostname.toLowerCase().includes('github.com') && Number.isFinite(issueNumber) && issueNumber > 0) {
        finalUrl = `${issueRedirectApiBase}?number=${issueNumber}`
      }
    }
  } catch {
    // keep original URL if parsing fails
  }

  try {
    await invoke('open_url', { url: finalUrl })
  } catch {
    // ignore
  }
}

async function loadIssueDetail(issueNumber: number, page = 1, append = false) {
  if (append) {
    loadingMoreComments.value = true
  } else {
    loadingDetail.value = true
  }

  try {
    const result = await issueTrackerStore.getIssueDetail(issueNumber, page, 30)
    detailIssue.value = result.issue
    hasMoreComments.value = result.has_more
    currentPage.value = page

    if (append) {
      comments.value = [...comments.value, ...result.comments]
    } else {
      comments.value = result.comments
    }
  } catch (error) {
    toast.error(t('feedback.loadDetailFailed'))
    logError(`Failed to load issue detail: ${error}`, 'feedback')
  } finally {
    loadingDetail.value = false
    loadingMoreComments.value = false
  }
}

async function selectIssue(issueNumber: number) {
  pendingClearAfterLeave.value = false
  detailVisible.value = true
  if (selectedIssueNumber.value === issueNumber && detailIssue.value) {
    return
  }
  selectedIssueNumber.value = issueNumber
  await loadIssueDetail(issueNumber)
}

function clearSelectionNow() {
  selectedIssueNumber.value = null
  detailIssue.value = null
  comments.value = []
  newComment.value = ''
  hasMoreComments.value = false
  currentPage.value = 1
}

function requestCloseDetail() {
  pendingClearAfterLeave.value = true
  detailVisible.value = false
}

function onDetailAfterLeave() {
  if (!pendingClearAfterLeave.value) return
  pendingClearAfterLeave.value = false
  clearSelectionNow()
}

async function loadMoreComments() {
  if (!selectedIssueNumber.value || loadingMoreComments.value || !hasMoreComments.value) return
  await loadIssueDetail(selectedIssueNumber.value, currentPage.value + 1, true)
}

async function submitComment() {
  if (!selectedIssueNumber.value || submittingComment.value) return
  const normalized = newComment.value.trim()
  if (!normalized) return

  submittingComment.value = true
  try {
    await issueTrackerStore.postIssueComment(selectedIssueNumber.value, normalized)
    toast.success(t('feedback.commentSent'))
    newComment.value = ''
    await loadIssueDetail(selectedIssueNumber.value, 1)
  } catch (error) {
    toast.error(t('feedback.commentSendFailed'))
    logError(`Failed to send issue comment: ${error}`, 'feedback')
  } finally {
    submittingComment.value = false
  }
}

watch(
  records,
  async (nextRecords) => {
    if (nextRecords.length === 0) {
      detailVisible.value = false
      pendingClearAfterLeave.value = false
      clearSelectionNow()
      return
    }

    if (
      selectedIssueNumber.value &&
      !nextRecords.some((record) => record.issueNumber === selectedIssueNumber.value)
    ) {
      if (detailVisible.value) {
        requestCloseDetail()
      } else {
        clearSelectionNow()
      }
    }
  },
  { immediate: true },
)

watch(
  () => route.query.issue,
  async (issueParam) => {
    const issueNumber = Number(issueParam || 0)
    if (!issueNumber) return
    if (records.value.some((record) => record.issueNumber === issueNumber)) {
      await selectIssue(issueNumber)
    }
  },
)

onMounted(async () => {
  await issueTrackerStore.initStore()

  const routeIssue = Number(route.query.issue || 0)
  if (routeIssue > 0) {
    const exists = records.value.some((record) => record.issueNumber === routeIssue)
    if (exists) {
      await selectIssue(routeIssue)
      return
    }
  }
})
</script>

<style scoped>
.feedback-comment-input {
  min-height: 72px;
  max-height: 160px;
}

@media (max-height: 860px) {
  .feedback-header-subtitle {
    display: none;
  }
}

@media (max-height: 760px) {
  .feedback-page {
    padding-top: 0.5rem;
    padding-bottom: 0.5rem;
  }

  .feedback-comment-input {
    min-height: 56px;
  }
}
</style>
