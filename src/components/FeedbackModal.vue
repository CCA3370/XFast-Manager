<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div v-if="show" class="fixed inset-0 z-[1200] flex items-center justify-center">
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm"></div>

        <div
          role="dialog"
          aria-modal="true"
          class="relative w-full max-w-xl mx-4 bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-700 shadow-2xl overflow-hidden"
          @click.stop
        >
          <div
            class="px-5 py-4 border-b border-gray-200 dark:border-gray-700 flex items-start justify-between"
          >
            <div>
              <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                <AnimatedText>{{ $t('feedback.submitTitle') }}</AnimatedText>
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                <AnimatedText>{{ $t('feedback.submitSubtitle') }}</AnimatedText>
              </p>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors p-1"
              :title="$t('common.close')"
              @click="requestClose"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          <div class="px-5 py-4 space-y-4 max-h-[65vh] overflow-y-auto">
            <div class="space-y-1">
              <label class="text-sm font-medium text-gray-700 dark:text-gray-200">
                {{ $t('feedback.fieldTitle') }}
              </label>
              <input
                v-model="title"
                type="text"
                maxlength="120"
                class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                :placeholder="$t('feedback.fieldTitlePlaceholder')"
              />
            </div>

            <div class="space-y-1">
              <label class="text-sm font-medium text-gray-700 dark:text-gray-200">
                {{ $t('feedback.fieldType') }}
              </label>
              <select
                v-model="type"
                class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="bug">{{ $t('feedback.types.bug') }}</option>
                <option value="feature-request">{{ $t('feedback.types.featureRequest') }}</option>
                <option value="improvement">{{ $t('feedback.types.improvement') }}</option>
                <option value="other">{{ $t('feedback.types.other') }}</option>
              </select>
            </div>

            <div class="space-y-1">
              <label class="text-sm font-medium text-gray-700 dark:text-gray-200">
                {{ $t('feedback.fieldContent') }}
              </label>
              <textarea
                v-model="content"
                rows="8"
                maxlength="5000"
                class="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-y min-h-[160px]"
                :placeholder="$t('feedback.fieldContentPlaceholder')"
              ></textarea>
              <p class="text-[11px] text-gray-400 dark:text-gray-500 text-right">
                {{ content.length }}/5000
              </p>
            </div>
          </div>

          <div
            class="px-5 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-end gap-2"
          >
            <button
              class="px-3 py-2 rounded-lg text-sm font-medium border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              :disabled="isSubmitting"
              @click="requestClose"
            >
              <AnimatedText>{{ $t('common.cancel') }}</AnimatedText>
            </button>
            <button
              class="px-3 py-2 rounded-lg text-sm font-medium bg-blue-600 hover:bg-blue-700 disabled:opacity-60 text-white transition-colors flex items-center gap-2"
              :disabled="isSubmitting"
              @click="submit"
            >
              <svg
                v-if="isSubmitting"
                class="w-4 h-4 animate-spin"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                />
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                />
              </svg>
              <AnimatedText>{{ $t('feedback.submitAction') }}</AnimatedText>
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import AnimatedText from './AnimatedText.vue'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useIssueTrackerStore } from '@/stores/issueTracker'
import { getItem, removeItem, setItem, STORAGE_KEYS, type FeedbackType } from '@/services/storage'
import { logError } from '@/services/logger'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'dirty-change', value: boolean): void
  (e: 'submitted', issueNumber: number): void
}>()

interface FeedbackDraft {
  title: string
  type: FeedbackType
  content: string
}

const { t } = useI18n()
const toast = useToastStore()
const modal = useModalStore()
const issueTrackerStore = useIssueTrackerStore()

const title = ref('')
const type = ref<FeedbackType>('bug')
const content = ref('')
const isSubmitting = ref(false)

const isDirty = computed(() => {
  return title.value.trim().length > 0 || content.value.trim().length > 0
})

function closeNow() {
  emit('dirty-change', false)
  emit('update:show', false)
}

function requestClose() {
  if (!isDirty.value) {
    closeNow()
    return
  }

  modal.showConfirm({
    title: t('feedback.discardTitle'),
    message: t('feedback.discardMessage'),
    warning: t('feedback.discardWarning'),
    confirmText: t('feedback.discardConfirm'),
    cancelText: t('feedback.discardCancel'),
    type: 'warning',
    onConfirm: async () => {
      await clearDraft()
      closeNow()
    },
    onCancel: () => {},
  })
}

async function loadDraft() {
  const draft = await getItem<FeedbackDraft>(STORAGE_KEYS.FEEDBACK_DRAFT)
  if (!draft) return

  title.value = draft.title || ''
  type.value = draft.type || 'bug'
  content.value = draft.content || ''
}

async function persistDraft() {
  if (!props.show) return
  if (!title.value.trim() && !content.value.trim()) {
    await removeItem(STORAGE_KEYS.FEEDBACK_DRAFT)
    return
  }
  await setItem(STORAGE_KEYS.FEEDBACK_DRAFT, {
    title: title.value,
    type: type.value,
    content: content.value,
  } satisfies FeedbackDraft)
}

async function clearDraft() {
  await removeItem(STORAGE_KEYS.FEEDBACK_DRAFT)
  title.value = ''
  type.value = 'bug'
  content.value = ''
}

async function submit() {
  if (isSubmitting.value) return

  const normalizedTitle = title.value.trim()
  const normalizedContent = content.value.trim()
  if (!normalizedTitle) {
    toast.warning(t('feedback.validationTitleRequired'))
    return
  }
  if (!normalizedContent) {
    toast.warning(t('feedback.validationContentRequired'))
    return
  }

  isSubmitting.value = true
  try {
    const result = await invoke<{ issue_url: string; issue_number: number; issue_title: string }>(
      'create_feedback_issue',
      {
        feedbackTitle: normalizedTitle,
        feedbackType: type.value,
        feedbackContent: normalizedContent,
      },
    )

    await issueTrackerStore.appendTrackedIssue({
      issueNumber: result.issue_number,
      issueTitle: result.issue_title || `[Feedback] ${normalizedTitle}`,
      issueUrl: result.issue_url,
      source: 'feedback',
      feedbackType: type.value,
      feedbackContentPreview: normalizedContent.slice(0, 200),
    })

    await clearDraft()
    toast.success(t('feedback.submitSuccess'))
    emit('submitted', result.issue_number)
    closeNow()
  } catch (error) {
    toast.error(t('feedback.submitFailed'))
    logError(`Feedback submit failed: ${error}`, 'feedback')
  } finally {
    isSubmitting.value = false
  }
}

function onKeyDown(event: KeyboardEvent) {
  if (!props.show) return
  if (event.key === 'Escape') {
    event.preventDefault()
    requestClose()
  }
}

watch(
  () => props.show,
  async (visible) => {
    if (visible) {
      await loadDraft()
      window.addEventListener('keydown', onKeyDown)
    } else {
      window.removeEventListener('keydown', onKeyDown)
    }
  },
  { immediate: true },
)

watch([title, type, content], async () => {
  emit('dirty-change', isDirty.value)
  await persistDraft()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown)
})
</script>

<style scoped>
.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}
</style>
