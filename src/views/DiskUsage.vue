<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ $t('diskUsage.title') }}
        </h1>
        <p v-if="store.report" class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
          {{ $t('diskUsage.scannedIn', { ms: store.report.scanDurationMs }) }}
        </p>
      </div>
      <button
        class="text-sm px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-2"
        :disabled="store.isScanning || store.isCleanupScanning"
        @click="scanAll"
      >
        <div
          v-if="store.isScanning || store.isCleanupScanning"
          class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
        ></div>
        {{
          store.isScanning || store.isCleanupScanning
            ? $t('diskUsage.scanning')
            : $t('diskUsage.scan')
        }}
      </button>
    </div>

    <!-- Content -->
    <div v-if="store.report" class="flex-1 overflow-y-auto">
      <!-- Chart + total -->
      <div class="flex items-center gap-8 mb-6">
        <DiskUsageChart
          :categories="chartData"
          :total-bytes="store.report.totalBytes"
          :size="180"
          :is-dark="isDark"
        />
        <div class="space-y-2">
          <div
            v-for="cat in store.report.categories"
            :key="cat.category"
            class="flex items-center gap-2 text-sm"
          >
            <div
              class="w-3 h-3 rounded-sm flex-shrink-0"
              :style="{ backgroundColor: categoryColor(cat.category) }"
            ></div>
            <span class="text-gray-700 dark:text-gray-300">{{
              diskCategoryLabel(cat.category)
            }}</span>
            <span class="text-gray-400 dark:text-gray-500 ml-auto">{{
              formatSize(cat.totalBytes)
            }}</span>
          </div>
        </div>
      </div>

      <!-- Output cleanup -->
      <section
        class="mb-6 rounded-2xl border border-slate-200 dark:border-white/10 bg-white/85 dark:bg-slate-900/45 overflow-hidden"
      >
        <div
          class="px-4 py-4 border-b border-slate-200 dark:border-white/10 bg-gradient-to-r from-slate-50 to-cyan-50/70 dark:from-slate-900 dark:to-cyan-950/20"
        >
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <div class="flex items-center gap-2">
                <h2 class="text-sm font-semibold text-slate-900 dark:text-white">
                  {{ $t('diskUsage.outputCleanup.title') }}
                </h2>
                <span
                  v-if="store.outputCleanupReport"
                  class="text-[11px] px-2 py-0.5 rounded-full bg-slate-200/80 dark:bg-white/10 text-slate-600 dark:text-slate-300"
                >
                  {{
                    $t('diskUsage.outputCleanup.source', {
                      source: cleanupSourceLabel(store.outputCleanupReport.source),
                      version: store.outputCleanupReport.version,
                    })
                  }}
                </span>
              </div>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
                {{ $t('diskUsage.outputCleanup.subtitle') }}
              </p>
            </div>
            <div class="flex items-center gap-2">
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

          <div
            v-if="store.outputCleanupReport"
            class="mt-3 grid grid-cols-3 gap-2 text-xs text-slate-600 dark:text-slate-300"
          >
            <div class="rounded-xl bg-white/80 dark:bg-white/5 px-3 py-2">
              <div class="text-[11px] uppercase tracking-wide text-slate-400">
                {{ $t('diskUsage.outputCleanup.available') }}
              </div>
              <div class="font-mono text-sm text-slate-900 dark:text-white">
                {{ formatSize(store.outputCleanupReport.totalBytes) }}
              </div>
            </div>
            <div class="rounded-xl bg-white/80 dark:bg-white/5 px-3 py-2">
              <div class="text-[11px] uppercase tracking-wide text-slate-400">
                {{ $t('diskUsage.outputCleanup.selected') }}
              </div>
              <div class="font-mono text-sm text-slate-900 dark:text-white">
                {{ formatSize(selectedCleanupBytes) }}
              </div>
            </div>
            <div class="rounded-xl bg-white/80 dark:bg-white/5 px-3 py-2">
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

        <div v-else class="p-4 space-y-4">
          <div v-for="group in groupedCleanupItems" :key="group.level" class="space-y-2">
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
                class="group flex items-start gap-3 rounded-xl border px-3 py-3 transition-colors cursor-pointer"
                :class="cleanupItemClass(item)"
              >
                <input
                  type="checkbox"
                  class="mt-1 h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-500"
                  :checked="selectedCleanupIds.has(item.id)"
                  :disabled="!item.exists || item.sizeBytes === 0"
                  @change="toggleCleanupSelected(item.id, $event)"
                />
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
                    <span
                      v-if="!item.exists"
                      class="text-[11px] px-1.5 py-0.5 rounded-md bg-slate-100 dark:bg-white/10 text-slate-500"
                    >
                      {{ $t('diskUsage.outputCleanup.notFound') }}
                    </span>
                  </div>
                  <p class="mt-1 text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
                    {{ cleanupItemDescription(item) }}
                  </p>
                  <p
                    v-if="cleanupWarningText(item)"
                    class="mt-2 text-xs text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 rounded-lg px-2 py-1"
                  >
                    {{ cleanupWarningText(item) }}
                  </p>
                  <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px]">
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

      <!-- Category details -->
      <div class="space-y-3">
        <details
          v-for="cat in store.report.categories"
          :key="cat.category"
          class="rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden"
        >
          <summary
            class="flex items-center justify-between px-4 py-3 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
          >
            <div class="flex items-center gap-2">
              <div
                class="w-2.5 h-2.5 rounded-sm"
                :style="{ backgroundColor: categoryColor(cat.category) }"
              ></div>
              <span class="font-medium text-gray-900 dark:text-white">{{
                diskCategoryLabel(cat.category)
              }}</span>
              <span class="text-xs text-gray-400 dark:text-gray-500">
                {{ cat.itemCount }} {{ $t('diskUsage.items') }}
              </span>
            </div>
            <span class="text-sm text-gray-600 dark:text-gray-400 font-mono">
              {{ formatSize(cat.totalBytes) }}
            </span>
          </summary>

          <div class="border-t border-gray-200 dark:border-gray-700">
            <div
              v-for="item in cat.items"
              :key="item.folderName"
              class="flex items-center gap-3 px-4 py-2 hover:bg-gray-50 dark:hover:bg-gray-800/30 transition-colors"
            >
              <div class="flex-1 min-w-0">
                <p class="text-sm text-gray-800 dark:text-gray-200 truncate">
                  {{ item.displayName }}
                </p>
                <div class="flex items-center gap-2 mt-0.5">
                  <div
                    class="flex-1 h-1.5 bg-gray-100 dark:bg-gray-700 rounded-full overflow-hidden"
                  >
                    <div
                      class="h-full rounded-full transition-all duration-300"
                      :style="{
                        width:
                          cat.totalBytes > 0 ? `${(item.sizeBytes / cat.totalBytes) * 100}%` : '0%',
                        backgroundColor: categoryColor(cat.category),
                      }"
                    ></div>
                  </div>
                  <span class="text-xs text-gray-400 dark:text-gray-500 font-mono flex-shrink-0">
                    {{ formatSize(item.sizeBytes) }}
                  </span>
                </div>
              </div>
              <button
                v-if="item.folderName"
                class="p-1 rounded text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                :title="$t('diskUsage.openFolder')"
                @click="openFolder(item.itemType, item.folderName)"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                  />
                </svg>
              </button>
            </div>
          </div>
        </details>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="!store.isScanning"
      class="flex-1 flex flex-col items-center justify-center text-gray-400 dark:text-gray-500"
    >
      <svg class="w-12 h-12 mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.5"
          d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"
        />
      </svg>
      <p class="text-sm">{{ $t('diskUsage.empty') }}</p>
      <p class="text-xs mt-1">{{ $t('diskUsage.emptyHint') }}</p>
    </div>

    <!-- Scanning overlay -->
    <div
      v-if="store.isScanning && !store.report"
      class="flex-1 flex flex-col items-center justify-center"
    >
      <div
        class="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mb-3"
      ></div>
      <p class="text-sm text-gray-500 dark:text-gray-400">{{ $t('diskUsage.scanning') }}</p>
    </div>

    <!-- Unknown item submission modal -->
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
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import {
  useDiskUsageStore,
  type OutputCleanupItem,
  type OutputCleanupLevel,
  type OutputCleanupSubmissionLevel,
} from '@/stores/diskUsage'
import { useAppStore } from '@/stores/app'
import { useThemeStore } from '@/stores/theme'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useIssueTrackerStore } from '@/stores/issueTracker'
import DiskUsageChart from '@/components/DiskUsageChart.vue'

const { t } = useI18n()

const store = useDiskUsageStore()
const appStore = useAppStore()
const themeStore = useThemeStore()
const toast = useToastStore()
const modal = useModalStore()
const issueTracker = useIssueTrackerStore()

const isDark = computed(() => themeStore.isDark)
const selectedCleanupIds = ref<Set<string>>(new Set())
const previousCleanupItemIds = ref<Set<string>>(new Set())
const cleanupSelectionInitialized = ref(false)
const unknownSubmissionItem = ref<OutputCleanupItem | null>(null)
const unknownSubmissionLevel = ref<OutputCleanupSubmissionLevel>('cleanable')
const unknownSubmissionDescription = ref('')
const submittingUnknown = ref(false)

const COLORS: Record<string, string> = {
  Aircraft: '#3b82f6',
  Plugins: '#8b5cf6',
  Scenery: '#10b981',
  Navdata: '#f59e0b',
  Screenshots: '#ef4444',
}

const CLEANUP_LEVELS: OutputCleanupLevel[] = ['recommended', 'cleanable', 'cautious', 'unknown']

function categoryColor(name: string): string {
  return COLORS[name] || '#6b7280'
}

const DISK_CATEGORY_KEY_MAP: Record<string, string> = {
  aircraft: 'diskUsage.categoryAircraft',
  plugin: 'diskUsage.categoryPlugins',
  plugins: 'diskUsage.categoryPlugins',
  scenery: 'diskUsage.categoryScenery',
  navdata: 'diskUsage.categoryNavdata',
  screenshot: 'diskUsage.categoryScreenshots',
  screenshots: 'diskUsage.categoryScreenshots',
}

function diskCategoryLabel(category: string): string {
  const key = DISK_CATEGORY_KEY_MAP[category.trim().toLowerCase()]
  return key ? t(key) : category
}

const chartData = computed(() => {
  if (!store.report) return []
  return store.report.categories.map((c) => ({
    name: diskCategoryLabel(c.category),
    bytes: c.totalBytes,
    color: categoryColor(c.category),
  }))
})

const cleanupItems = computed(() => store.outputCleanupReport?.items ?? [])

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
      nextIds.add(item.id)
      if (!item.exists || item.sizeBytes === 0) continue

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

function cleanupSourceLabel(source: string): string {
  return source === 'remote'
    ? t('diskUsage.outputCleanup.sourceRemote')
    : t('diskUsage.outputCleanup.sourceEmbedded')
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

function cleanupItemClass(item: OutputCleanupItem): string {
  const selected = selectedCleanupIds.value.has(item.id)
  if (!item.exists || item.sizeBytes === 0) {
    return 'border-slate-200 dark:border-white/10 bg-slate-50/80 dark:bg-white/[0.03] opacity-70'
  }
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
    if (item.defaultSelected && item.exists && item.sizeBytes > 0) {
      next.add(item.id)
    }
  }
  selectedCleanupIds.value = next
}

async function scanAll() {
  if (!appStore.xplanePath) return
  await Promise.all([store.scan(), store.scanOutputCleanup(true)])
  void store.refreshOutputCleanupItems()
}

async function refreshCleanup() {
  await store.scanOutputCleanup()
  void store.refreshOutputCleanupItems()
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
    await Promise.all([store.scan(), store.scanOutputCleanup()])
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

async function openFolder(itemType: string, folderName: string) {
  try {
    await invoke('open_management_folder', {
      xplanePath: appStore.xplanePath,
      itemType,
      folderName,
    })
  } catch {
    // silently ignore
  }
}

onMounted(() => {
  if (!appStore.xplanePath || store.isScanning) return
  void scanAll()
})
</script>
