<template>
  <div class="h-full overflow-hidden p-5">
    <div class="h-full flex flex-col gap-4">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-xl font-semibold text-gray-900 dark:text-white">
            <AnimatedText>{{ $t('feedback.myFeedbackTitle') }}</AnimatedText>
          </h1>
          <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
            <AnimatedText>{{ $t('feedback.myFeedbackSubtitle') }}</AnimatedText>
          </p>
        </div>
        <button
          class="px-3 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium transition-colors"
          @click="feedbackStore.openSubmitModal()"
        >
          <AnimatedText>{{ $t('feedback.newFeedback') }}</AnimatedText>
        </button>
      </div>

      <div class="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-[320px_1fr] gap-4">
        <section
          class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md overflow-hidden flex flex-col"
        >
          <div class="px-4 py-3 border-b border-gray-200 dark:border-white/5">
            <h2 class="text-sm font-semibold text-gray-800 dark:text-gray-100">
              {{ $t('feedback.recordListTitle') }}
            </h2>
          </div>

          <div v-if="records.length === 0" class="flex-1 flex items-center justify-center px-4">
            <p class="text-sm text-gray-500 dark:text-gray-400 text-center">
              {{ $t('feedback.emptyRecords') }}
            </p>
          </div>

          <div v-else class="flex-1 overflow-y-auto p-2 space-y-2">
            <button
              v-for="record in records"
              :key="record.issueNumber"
              class="w-full text-left p-3 rounded-lg border transition-colors"
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
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                #{{ record.issueNumber }} · {{ formatDate(record.reportedAt) }}
              </p>
              <p v-if="record.feedbackContentPreview" class="text-xs text-gray-600 dark:text-gray-300 mt-1 line-clamp-2">
                {{ record.feedbackContentPreview }}
              </p>
            </button>
          </div>
        </section>

        <section
          class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md overflow-hidden flex flex-col"
        >
          <div class="px-4 py-3 border-b border-gray-200 dark:border-white/5">
            <div v-if="selectedRecord" class="flex items-center justify-between gap-2">
              <div class="min-w-0">
                <h2 class="text-sm font-semibold text-gray-800 dark:text-gray-100 truncate">
                  {{ selectedRecord.issueTitle }}
                </h2>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  #{{ selectedRecord.issueNumber }}
                </p>
              </div>
              <button
                class="px-2.5 py-1.5 text-xs rounded-md bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 transition-colors"
                @click="openIssue(selectedRecord.issueUrl)"
              >
                {{ $t('issueTracker.viewIssue') }} ↗
              </button>
            </div>
            <h2 v-else class="text-sm font-semibold text-gray-800 dark:text-gray-100">
              {{ $t('feedback.detailTitle') }}
            </h2>
          </div>

          <div v-if="!selectedRecord" class="flex-1 flex items-center justify-center px-4">
            <p class="text-sm text-gray-500 dark:text-gray-400">
              {{ $t('feedback.selectHint') }}
            </p>
          </div>

          <div v-else class="flex-1 min-h-0 flex flex-col">
            <div class="px-4 py-3 border-b border-gray-200 dark:border-white/5 text-xs text-gray-500 dark:text-gray-400 flex flex-wrap items-center gap-3">
              <span>{{ $t('feedback.typeLabel') }}: {{ displayFeedbackType(selectedRecord.feedbackType) }}</span>
              <span>{{ $t('feedback.commentCount') }}: {{ detailIssue?.comments ?? selectedRecord.commentCount }}</span>
              <span>{{ $t('feedback.updatedAt') }}: {{ formatDate(detailIssue?.updated_at || selectedRecord.lastCheckedAt) }}</span>
            </div>

            <div class="flex-1 min-h-0 overflow-y-auto px-4 py-3 space-y-3">
              <div v-if="loadingDetail" class="text-sm text-gray-500 dark:text-gray-400">
                {{ $t('feedback.loadingDetail') }}
              </div>
              <div v-else-if="comments.length === 0" class="text-sm text-gray-500 dark:text-gray-400">
                {{ $t('feedback.noComments') }}
              </div>
              <div
                v-for="comment in comments"
                :key="comment.id + '-' + comment.created_at"
                class="rounded-lg border border-gray-200 dark:border-white/10 bg-gray-50 dark:bg-gray-900/30 p-3"
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
                class="px-3 py-1.5 rounded-md text-xs bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 transition-colors"
                @click="loadMoreComments"
              >
                {{ $t('feedback.loadMoreComments') }}
              </button>
              <p v-if="loadingMoreComments" class="text-xs text-gray-500 dark:text-gray-400">
                {{ $t('feedback.loadingMoreComments') }}
              </p>
            </div>

            <div class="px-4 py-3 border-t border-gray-200 dark:border-white/5 space-y-2">
              <textarea
                v-model="newComment"
                rows="3"
                maxlength="3000"
                class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-y"
                :placeholder="$t('feedback.commentPlaceholder')"
              ></textarea>
              <div class="flex items-center justify-between">
                <p class="text-[11px] text-gray-400 dark:text-gray-500">{{ newComment.length }}/3000</p>
                <button
                  class="px-3 py-1.5 rounded-md bg-blue-600 hover:bg-blue-700 disabled:opacity-60 text-white text-sm font-medium transition-colors"
                  :disabled="submittingComment || !newComment.trim()"
                  @click="submitComment"
                >
                  {{ submittingComment ? $t('feedback.sendingComment') : $t('feedback.sendComment') }}
                </button>
              </div>
            </div>
          </div>
        </section>
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
  try {
    await invoke('open_url', { url })
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
  selectedIssueNumber.value = issueNumber
  await loadIssueDetail(issueNumber)
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
      selectedIssueNumber.value = null
      detailIssue.value = null
      comments.value = []
      return
    }

    if (!selectedIssueNumber.value || !nextRecords.some((record) => record.issueNumber === selectedIssueNumber.value)) {
      await selectIssue(nextRecords[0].issueNumber)
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

  if (records.value.length > 0) {
    await selectIssue(records.value[0].issueNumber)
  }
})
</script>
