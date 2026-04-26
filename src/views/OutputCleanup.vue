<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6">
    <div class="flex items-start justify-between gap-4 mb-4">
      <div>
        <router-link
          to="/disk-usage"
          class="inline-flex items-center gap-1 text-xs text-slate-500 dark:text-slate-400 hover:text-blue-600 dark:hover:text-blue-300 transition-colors mb-2"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {{ $t('diskUsage.title') }}
        </router-link>
        <div class="flex items-center gap-2">
          <h1 class="text-xl font-bold text-gray-900 dark:text-white">
            {{ $t('diskUsage.outputCleanup.title') }}
          </h1>
        </div>
        <p class="text-sm text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
          {{ $t('diskUsage.outputCleanup.subtitle') }}
        </p>
      </div>

      <div class="flex items-center gap-2 pt-6">
        <button
          class="text-xs px-3 py-1.5 rounded-lg border border-slate-200 dark:border-white/10 text-slate-600 dark:text-slate-300 hover:bg-white dark:hover:bg-white/10 transition-colors disabled:opacity-50"
          :disabled="store.isCleanupScanning || store.isCleanupRefreshing"
          @click="refreshCleanup"
        >
          {{
            store.isCleanupRefreshing
              ? $t('diskUsage.outputCleanup.refreshing')
              : $t('diskUsage.outputCleanup.refresh')
          }}
        </button>
        <button
          class="text-xs px-3 py-1.5 rounded-lg border border-slate-200 dark:border-white/10 text-slate-600 dark:text-slate-300 hover:bg-white dark:hover:bg-white/10 transition-colors"
          @click="resetCleanupSelectionToDefaults"
        >
          {{ $t('diskUsage.outputCleanup.selectRecommended') }}
        </button>
        <button
          class="text-xs px-3 py-1.5 rounded-lg bg-rose-600 text-white hover:bg-rose-700 transition-colors disabled:opacity-50"
          :disabled="selectedCleanupItems.length === 0 || store.isCleaning"
          @click="confirmCleanSelected"
        >
          {{
            store.isCleaning
              ? $t('diskUsage.outputCleanup.cleaning')
              : $t('diskUsage.outputCleanup.cleanSelected', {
                  size: formatSize(selectedCleanupBytes),
                })
          }}
        </button>
      </div>
    </div>

    <section
      aria-labelledby="output-cleanup-title"
      class="flex-1 min-h-0 rounded-2xl border border-slate-200 dark:border-white/10 bg-white/85 dark:bg-slate-900/45 shadow-sm overflow-hidden flex flex-col"
    >
      <div
        class="px-4 py-3 border-b border-slate-200 dark:border-white/10 bg-gradient-to-r from-slate-50 to-cyan-50/70 dark:from-slate-900 dark:to-cyan-950/20"
      >
        <h2 id="output-cleanup-title" class="sr-only">
          {{ $t('diskUsage.outputCleanup.title') }}
        </h2>

        <div
          v-if="store.outputCleanupReport"
          class="grid grid-cols-3 gap-2 text-xs text-slate-600 dark:text-slate-300"
        >
          <div class="rounded-xl bg-white/80 dark:bg-white/5 px-3 py-1.5">
            <div class="text-[11px] uppercase tracking-wide text-slate-400">
              {{ $t('diskUsage.outputCleanup.available') }}
            </div>
            <div class="font-mono text-sm text-slate-900 dark:text-white">
              {{ formatSize(store.outputCleanupReport.totalBytes) }}
            </div>
          </div>
          <div class="rounded-xl bg-white/80 dark:bg-white/5 px-3 py-1.5">
            <div class="text-[11px] uppercase tracking-wide text-slate-400">
              {{ $t('diskUsage.outputCleanup.selected') }}
            </div>
            <div class="font-mono text-sm text-slate-900 dark:text-white">
              {{ formatSize(selectedCleanupBytes) }}
            </div>
          </div>
          <div class="rounded-xl bg-white/80 dark:bg-white/5 px-3 py-1.5">
            <div class="text-[11px] uppercase tracking-wide text-slate-400">
              {{ $t('diskUsage.outputCleanup.fileCount') }}
            </div>
            <div class="font-mono text-sm text-slate-900 dark:text-white">
              {{ selectedCleanupFiles }}
            </div>
          </div>
        </div>

        <p v-if="store.cleanupError" class="mt-3 text-xs text-rose-600 dark:text-rose-400">
          {{ store.cleanupError }}
        </p>
      </div>

      <div
        v-if="store.isCleanupScanning && !store.outputCleanupReport"
        class="px-4 py-6 text-sm text-slate-500 dark:text-slate-400"
      >
        {{ $t('diskUsage.outputCleanup.loading') }}
      </div>

      <div
        v-else-if="groupedCleanupItems.length === 0"
        class="px-4 py-6 text-sm text-slate-500 dark:text-slate-400"
      >
        {{ $t('diskUsage.outputCleanup.empty') }}
      </div>

      <div v-else class="flex-1 overflow-y-auto p-3 space-y-3">
        <div v-for="group in groupedCleanupItems" :key="group.level" class="space-y-1.5">
          <div class="flex items-center justify-between">
            <h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">
              {{ cleanupLevelLabel(group.level) }}
            </h3>
            <span class="text-[11px] text-slate-400">
              {{ group.items.length }} {{ $t('diskUsage.items') }}
            </span>
          </div>

          <div class="grid gap-2">
            <label
              v-for="item in group.items"
              :key="item.id"
              class="group relative flex items-start gap-2.5 rounded-xl border px-3 py-2 transition-colors cursor-pointer"
              :class="cleanupItemClass(item)"
            >
              <input
                type="checkbox"
                class="peer sr-only"
                :checked="selectedCleanupIds.has(item.id)"
                @change="toggleCleanupSelected(item.id, $event)"
              />
              <span
                class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-lg border transition-all peer-focus-visible:ring-2 peer-focus-visible:ring-blue-400 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-white dark:peer-focus-visible:ring-offset-slate-900"
                :class="cleanupCheckboxClass(item)"
                aria-hidden="true"
              >
                <svg
                  v-if="selectedCleanupIds.has(item.id)"
                  class="h-3.5 w-3.5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="3"
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </span>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-medium text-slate-900 dark:text-white">
                    {{ cleanupItemName(item) }}
                  </span>
                  <span
                    class="text-[11px] px-1.5 py-0.5 rounded-md"
                    :class="cleanupLevelBadgeClass(item.level)"
                  >
                    {{ cleanupLevelLabel(item.level) }}
                  </span>
                </div>
                <p class="mt-0.5 text-xs text-slate-500 dark:text-slate-400 leading-snug">
                  {{ cleanupItemDescription(item) }}
                </p>
                <p
                  v-if="cleanupWarningText(item)"
                  class="mt-1 text-xs text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 rounded-lg px-2 py-1"
                >
                  {{ cleanupWarningText(item) }}
                </p>
                <div class="mt-1 flex flex-wrap items-center gap-1.5 text-[11px]">
                  <span class="font-mono text-slate-500 dark:text-slate-400">
                    {{ item.relativePath }}
                  </span>
                  <span class="text-slate-400">·</span>
                  <span class="font-mono text-slate-600 dark:text-slate-300">
                    {{ formatSize(item.sizeBytes) }}
                  </span>
                  <span class="text-slate-400">·</span>
                  <span class="text-slate-500 dark:text-slate-400">
                    {{ item.fileCount }} {{ $t('diskUsage.outputCleanup.files') }}
                  </span>
                </div>
              </div>
              <button
                v-if="!item.recognized"
                type="button"
                class="text-xs px-2.5 py-1 rounded-lg border border-slate-200 dark:border-white/10 text-blue-600 dark:text-blue-300 hover:bg-blue-50 dark:hover:bg-blue-500/10 transition-colors"
                @click.prevent="openUnknownSubmission(item)"
              >
                {{ $t('diskUsage.outputCleanup.submitUnknown') }}
              </button>
            </label>
          </div>
        </div>
      </div>
    </section>

    <div
      v-if="unknownSubmissionItem"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/45 px-4"
      @click.self="closeUnknownSubmission"
    >
      <div
        class="w-full max-w-lg rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 shadow-2xl overflow-hidden"
      >
        <div class="px-5 py-4 border-b border-slate-200 dark:border-white/10">
          <h3 class="text-base font-semibold text-slate-900 dark:text-white">
            {{ $t('diskUsage.outputCleanup.submitTitle') }}
          </h3>
          <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
            {{ unknownSubmissionItem.relativePath }}
          </p>
        </div>
        <div class="p-5 space-y-4">
          <div>
            <label class="block text-xs font-medium text-slate-600 dark:text-slate-300 mb-1">
              {{ $t('diskUsage.outputCleanup.expectedLevel') }}
            </label>
            <select
              v-model="unknownSubmissionLevel"
              class="w-full rounded-lg border border-slate-200 dark:border-white/10 bg-white dark:bg-slate-950 text-sm text-slate-900 dark:text-white px-3 py-2"
            >
              <option value="recommended">
                {{ $t('diskUsage.outputCleanup.levelRecommended') }}
              </option>
              <option value="cleanable">{{ $t('diskUsage.outputCleanup.levelCleanable') }}</option>
              <option value="cautious">{{ $t('diskUsage.outputCleanup.levelCautious') }}</option>
            </select>
          </div>
          <div>
            <label class="block text-xs font-medium text-slate-600 dark:text-slate-300 mb-1">
              {{ $t('diskUsage.outputCleanup.descriptionLabel') }}
            </label>
            <textarea
              v-model="unknownSubmissionDescription"
              rows="5"
              class="w-full rounded-lg border border-slate-200 dark:border-white/10 bg-white dark:bg-slate-950 text-sm text-slate-900 dark:text-white px-3 py-2 resize-none"
              :placeholder="$t('diskUsage.outputCleanup.descriptionPlaceholder')"
            ></textarea>
          </div>
        </div>
        <div
          class="px-5 py-4 border-t border-slate-200 dark:border-white/10 flex justify-end gap-2"
        >
          <button
            class="px-3 py-1.5 rounded-lg text-sm text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-white/10 transition-colors"
            @click="closeUnknownSubmission"
          >
            {{ $t('common.cancel') }}
          </button>
          <button
            class="px-3 py-1.5 rounded-lg text-sm bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 transition-colors"
            :disabled="submittingUnknown || !unknownSubmissionDescription.trim()"
            @click="submitUnknownItem"
          >
            {{
              submittingUnknown
                ? $t('diskUsage.outputCleanup.submitting')
                : $t('diskUsage.outputCleanup.submit')
            }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  useDiskUsageStore,
  type OutputCleanupItem,
  type OutputCleanupLevel,
  type OutputCleanupSubmissionLevel,
} from '@/stores/diskUsage'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useIssueTrackerStore } from '@/stores/issueTracker'

const { t } = useI18n()

const store = useDiskUsageStore()
const appStore = useAppStore()
const toast = useToastStore()
const modal = useModalStore()
const issueTracker = useIssueTrackerStore()

const selectedCleanupIds = ref<Set<string>>(new Set())
const previousCleanupItemIds = ref<Set<string>>(new Set())
const cleanupSelectionInitialized = ref(false)
const unknownSubmissionItem = ref<OutputCleanupItem | null>(null)
const unknownSubmissionLevel = ref<OutputCleanupSubmissionLevel>('cleanable')
const unknownSubmissionDescription = ref('')
const submittingUnknown = ref(false)

const CLEANUP_LEVELS: OutputCleanupLevel[] = ['recommended', 'cleanable', 'cautious', 'unknown']

function isCleanupItemVisible(item: OutputCleanupItem): boolean {
  return item.exists && item.sizeBytes > 0
}

const cleanupItems = computed(() =>
  (store.outputCleanupReport?.items ?? []).filter(isCleanupItemVisible),
)

const groupedCleanupItems = computed(() =>
  CLEANUP_LEVELS.map((level) => ({
    level,
    items: cleanupItems.value.filter((item) => item.level === level),
  })).filter((group) => group.items.length > 0),
)

const selectedCleanupItems = computed(() =>
  cleanupItems.value.filter((item) => selectedCleanupIds.value.has(item.id)),
)

const selectedCleanupBytes = computed(() =>
  selectedCleanupItems.value.reduce((total, item) => total + item.sizeBytes, 0),
)

const selectedCleanupFiles = computed(() =>
  selectedCleanupItems.value.reduce((total, item) => total + item.fileCount, 0),
)

watch(
  () => store.outputCleanupReport,
  (report) => {
    if (!report) {
      selectedCleanupIds.value = new Set()
      previousCleanupItemIds.value = new Set()
      cleanupSelectionInitialized.value = false
      return
    }

    const previousSelected = selectedCleanupIds.value
    const previousIds = previousCleanupItemIds.value
    const nextSelected = new Set<string>()
    const nextIds = new Set<string>()

    for (const item of report.items) {
      if (!isCleanupItemVisible(item)) continue
      nextIds.add(item.id)

      if (!cleanupSelectionInitialized.value) {
        if (item.defaultSelected) nextSelected.add(item.id)
      } else if (previousSelected.has(item.id)) {
        nextSelected.add(item.id)
      } else if (!previousIds.has(item.id) && item.defaultSelected) {
        nextSelected.add(item.id)
      }
    }

    selectedCleanupIds.value = nextSelected
    previousCleanupItemIds.value = nextIds
    cleanupSelectionInitialized.value = true
  },
)

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(2)} GB`
}

function cleanupItemName(item: OutputCleanupItem): string {
  if (!item.recognized) return item.displayName
  return t(`diskUsage.outputCleanup.items.${item.id}.name`, item.displayName)
}

function cleanupItemDescription(item: OutputCleanupItem): string {
  if (!item.recognized) {
    return t('diskUsage.outputCleanup.unknownDescription')
  }
  return t(`diskUsage.outputCleanup.items.${item.id}.description`, item.description)
}

function cleanupWarningText(item: OutputCleanupItem): string {
  if (!item.warningKey) return ''
  return t(`diskUsage.outputCleanup.warnings.${item.warningKey}`, '')
}

function cleanupLevelLabel(level: OutputCleanupLevel): string {
  const key = {
    recommended: 'diskUsage.outputCleanup.levelRecommended',
    cleanable: 'diskUsage.outputCleanup.levelCleanable',
    cautious: 'diskUsage.outputCleanup.levelCautious',
    unknown: 'diskUsage.outputCleanup.levelUnknown',
  }[level]
  return t(key)
}

function cleanupLevelBadgeClass(level: OutputCleanupLevel): string {
  if (level === 'recommended')
    return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-500/15 dark:text-emerald-300'
  if (level === 'cleanable')
    return 'bg-blue-100 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300'
  if (level === 'cautious')
    return 'bg-amber-100 text-amber-700 dark:bg-amber-500/15 dark:text-amber-300'
  return 'bg-slate-100 text-slate-600 dark:bg-white/10 dark:text-slate-300'
}

function cleanupCheckboxClass(item: OutputCleanupItem): string {
  if (selectedCleanupIds.value.has(item.id)) {
    return 'border-blue-500 bg-blue-600 text-white shadow-sm shadow-blue-500/20 dark:border-blue-400 dark:bg-blue-500'
  }
  return 'border-slate-300 bg-white text-transparent group-hover:border-blue-400 dark:border-white/20 dark:bg-slate-950/80 dark:group-hover:border-blue-400/70'
}

function cleanupItemClass(item: OutputCleanupItem): string {
  const selected = selectedCleanupIds.value.has(item.id)
  if (selected) {
    return 'border-blue-300 dark:border-blue-400/40 bg-blue-50/80 dark:bg-blue-500/10'
  }
  return 'border-slate-200 dark:border-white/10 bg-white dark:bg-white/[0.03] hover:bg-slate-50 dark:hover:bg-white/[0.06]'
}

function setCleanupSelected(id: string, selected: boolean) {
  const next = new Set(selectedCleanupIds.value)
  if (selected) {
    next.add(id)
  } else {
    next.delete(id)
  }
  selectedCleanupIds.value = next
}

function toggleCleanupSelected(id: string, event: Event) {
  const input = event.target as HTMLInputElement | null
  setCleanupSelected(id, input?.checked ?? false)
}

function resetCleanupSelectionToDefaults() {
  const next = new Set<string>()
  for (const item of cleanupItems.value) {
    if (item.defaultSelected) {
      next.add(item.id)
    }
  }
  selectedCleanupIds.value = next
}

async function scanCleanup(resetToEmbedded = false) {
  if (!appStore.xplanePath) return
  await store.scanOutputCleanup(resetToEmbedded)
  void store.refreshOutputCleanupItems()
}

async function refreshCleanup() {
  await scanCleanup(false)
}

function confirmCleanSelected() {
  if (selectedCleanupItems.value.length === 0) {
    toast.warning(t('diskUsage.outputCleanup.noSelection'))
    return
  }

  const warnings = selectedCleanupItems.value
    .map((item) => cleanupWarningText(item))
    .filter((warning) => warning.trim().length > 0)

  modal.showConfirm({
    title: t('diskUsage.outputCleanup.confirmTitle'),
    message: t('diskUsage.outputCleanup.confirmMessage', {
      count: selectedCleanupItems.value.length,
      size: formatSize(selectedCleanupBytes.value),
    }),
    warning: warnings.length > 0 ? warnings.join('\n') : undefined,
    confirmText: t('diskUsage.outputCleanup.confirmClean'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: () => {
      void cleanSelected()
    },
    onCancel: () => {},
  })
}

async function cleanSelected() {
  try {
    const result = await store.cleanOutputItems(
      selectedCleanupItems.value.map((item) => ({
        id: item.id,
        relativePath: item.relativePath,
      })),
    )
    toast.success(
      t('diskUsage.outputCleanup.cleanSuccess', {
        size: formatSize(result.totalDeletedBytes),
        count: result.totalDeletedFiles,
      }),
    )
    selectedCleanupIds.value = new Set()
    cleanupSelectionInitialized.value = false
    await store.scanOutputCleanup()
    void store.refreshOutputCleanupItems()
  } catch (e) {
    toast.error(String(e))
  }
}

function openUnknownSubmission(item: OutputCleanupItem) {
  unknownSubmissionItem.value = item
  unknownSubmissionLevel.value = 'cleanable'
  unknownSubmissionDescription.value = ''
}

function closeUnknownSubmission(force = false) {
  if (submittingUnknown.value && !force) return
  unknownSubmissionItem.value = null
  unknownSubmissionDescription.value = ''
}

async function submitUnknownItem() {
  const item = unknownSubmissionItem.value
  if (!item) return

  submittingUnknown.value = true
  try {
    const result = await store.submitUnknownOutputCleanupItem({
      folderName: item.folderName,
      relativePath: item.relativePath,
      description: unknownSubmissionDescription.value,
      expectedLevel: unknownSubmissionLevel.value,
      sizeBytes: item.sizeBytes,
      fileCount: item.fileCount,
    })

    if (result.issue_number > 0) {
      await issueTracker.appendTrackedIssue({
        issueNumber: result.issue_number,
        issueTitle:
          result.issue_title || t('diskUsage.outputCleanup.issueTitle', { name: item.folderName }),
        issueUrl: result.issue_url,
        source: 'output-cleanup-item',
        feedbackContentPreview: unknownSubmissionDescription.value.slice(0, 160),
      })
    }

    toast.success(t('diskUsage.outputCleanup.submitSuccess'))
    closeUnknownSubmission(true)
  } catch (e) {
    toast.error(String(e))
  } finally {
    submittingUnknown.value = false
  }
}

onMounted(() => {
  void scanCleanup(true)
})
</script>
