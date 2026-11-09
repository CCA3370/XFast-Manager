<template>
  <Teleport to="body">
    <transition name="modal">
      <div class="modal-overlay" @click="handleCancel">
        <div class="modal-content" @click.stop>
          <!-- Header -->
          <div class="flex shrink-0 items-center space-x-2 mb-4">
            <div class="w-8 h-8 bg-orange-600 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"
                ></path>
              </svg>
            </div>
            <div class="min-w-0">
              <h3 class="text-base font-bold text-gray-900 dark:text-white">
                {{ $t('patch.title') }}
              </h3>
              <p
                class="text-orange-600 dark:text-orange-300/80 text-xs mt-0.5 truncate"
                :title="archiveName"
              >
                {{ archiveName }}
              </p>
            </div>
          </div>

          <div
            class="patch-scroll-body min-h-0 flex-1 space-y-4 overflow-y-auto custom-scrollbar pr-1"
          >
            <div
              v-if="errorMsg"
              role="alert"
              class="sticky top-0 z-10 max-h-24 overflow-y-auto break-words rounded-lg border border-red-300 bg-red-100 p-2 text-xs font-medium text-red-700 dark:border-red-500/50 dark:bg-gray-900 dark:text-red-400"
            >
              {{ errorMsg }}
            </div>
            <!-- Apply to -->
            <section>
              <label class="block text-xs font-semibold text-gray-700 dark:text-gray-200 mb-1.5">
                {{ $t('patch.applyTo') }}
              </label>
              <div v-if="detecting" class="text-xs text-gray-500 dark:text-gray-400 py-2">
                {{ $t('patch.detecting') }}
              </div>
              <template v-else-if="candidates.length">
                <select
                  v-model="selectedFolder"
                  :disabled="building"
                  class="w-full px-3 py-2 bg-white dark:bg-gray-900/70 border border-gray-200 dark:border-gray-700/50 rounded-lg text-gray-900 dark:text-white text-sm focus:border-orange-400 focus:ring-2 focus:ring-orange-500/30 transition-all"
                  @change="onAircraftChange"
                >
                  <option value="" disabled>{{ $t('patch.selectAircraft') }}</option>
                  <option v-for="c in candidates" :key="c.folderName" :value="c.folderName">
                    {{ c.displayName }}
                  </option>
                </select>
                <p
                  v-if="recommendedFolder && selectedFolder === recommendedFolder"
                  class="text-xs text-emerald-600 dark:text-emerald-400 mt-1.5 flex items-center gap-1"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  {{ $t('patch.autoRecommended') }}
                </p>
                <p
                  v-else-if="!selectedFolder"
                  class="text-xs text-amber-600 dark:text-amber-400 mt-1.5"
                >
                  {{ $t('patch.noReliableMatch') }}
                </p>
              </template>
              <div v-else class="text-xs text-amber-600 dark:text-amber-400 py-2">
                {{ $t('patch.noAircraft') }}
              </div>
            </section>

            <!-- What will happen -->
            <section v-if="selectedFolder" class="min-h-36">
              <label class="block text-xs font-semibold text-gray-700 dark:text-gray-200 mb-1.5">
                {{ $t('patch.summaryTitle') }}
              </label>
              <div
                v-if="inferring || summarizing"
                class="text-xs text-gray-500 dark:text-gray-400 py-2"
              >
                {{ $t('patch.inferring') }}
              </div>
              <div
                v-else-if="totalFiles > 0"
                class="bg-gray-100 dark:bg-gray-800/60 border border-gray-200 dark:border-gray-700/50 rounded-lg p-3 space-y-2"
              >
                <div class="flex items-start gap-2 text-xs text-gray-700 dark:text-gray-200">
                  <svg
                    class="w-4 h-4 flex-shrink-0 text-orange-500 mt-px"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M12 4v16m8-8H4"
                    />
                  </svg>
                  <span>{{
                    $t('patch.filesApplied', { count: totalFiles, aircraft: selectedDisplayName })
                  }}</span>
                </div>
                <div
                  v-if="overwriteCount > 0"
                  class="flex items-start gap-2 text-xs text-amber-700 dark:text-amber-300"
                >
                  <svg
                    class="w-4 h-4 flex-shrink-0 mt-px"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M12 9v2m0 4h.01M5 19h14a2 2 0 001.84-2.75L13.74 4a2 2 0 00-3.48 0L3.16 16.25A2 2 0 005 19z"
                    />
                  </svg>
                  <span>{{ $t('patch.filesOverwritten', { count: overwriteCount }) }}</span>
                </div>
                <label class="flex items-center gap-2 cursor-pointer pt-1">
                  <input
                    v-model="backupEnabled"
                    type="checkbox"
                    class="w-4 h-4 accent-orange-600"
                  />
                  <span class="text-xs font-medium text-gray-700 dark:text-gray-200">
                    {{ $t('patch.backupLabel') }}
                  </span>
                </label>
                <p v-if="backupEnabled" class="text-[11px] text-gray-500 dark:text-gray-400 ml-6">
                  {{ $t('patch.backupHint') }}
                </p>
              </div>
            </section>

            <!-- Advanced: file mappings -->
            <section v-if="selectedFolder">
              <button
                type="button"
                class="flex items-center gap-1.5 text-xs font-semibold text-gray-700 dark:text-gray-200 hover:text-orange-600 dark:hover:text-orange-400"
                @click="advancedOpen = !advancedOpen"
              >
                <svg
                  class="w-3.5 h-3.5 transition-transform"
                  :class="advancedOpen ? 'rotate-90' : ''"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 5l7 7-7 7"
                  />
                </svg>
                {{ $t('patch.advancedMappings') }}
                <span v-if="mappings.length" class="text-gray-400 dark:text-gray-500 font-normal">
                  ({{ mappings.length }})
                </span>
              </button>

              <div v-show="advancedOpen" class="mt-2">
                <div class="flex flex-wrap items-center justify-end gap-x-3 gap-y-1 mb-1.5">
                  <button
                    v-if="hasRememberedMappings"
                    type="button"
                    class="text-xs text-gray-600 hover:underline dark:text-gray-300"
                    :disabled="inferring || building"
                    @click="resetMappings"
                  >
                    {{ $t('patch.resetMappings') }}
                  </button>
                  <button
                    type="button"
                    class="text-xs text-orange-600 dark:text-orange-400 hover:underline"
                    :disabled="inferring || building"
                    @click="addMapping()"
                  >
                    + {{ $t('patch.addMapping') }}
                  </button>
                </div>
                <p v-if="memoryError" class="mb-1.5 text-[11px] text-amber-700 dark:text-amber-300">
                  {{ $t('patch.mappingMemoryError') }}
                </p>
                <p
                  v-else-if="hasRememberedMappings"
                  class="mb-1.5 text-[11px] text-gray-500 dark:text-gray-400"
                >
                  {{ $t('patch.rememberedMappings') }}
                </p>

                <div v-if="inferring" class="text-xs text-gray-500 dark:text-gray-400 py-2">
                  {{ $t('patch.inferring') }}
                </div>

                <div v-else class="space-y-2">
                  <div
                    v-for="(m, i) in mappings"
                    :key="m.id"
                    class="bg-gray-100 dark:bg-gray-800/60 border border-gray-200 dark:border-gray-700/50 rounded-lg p-2.5"
                  >
                    <div
                      class="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)_auto] items-start gap-2"
                    >
                      <PatchSourcePicker
                        v-model="m.archiveSubpath"
                        :paths="archiveTree"
                        :disabled="building"
                      />
                      <svg
                        class="w-4 h-4 mt-1.5 text-gray-400"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M14 5l7 7m0 0l-7 7m7-7H3"
                        />
                      </svg>
                      <PatchDestinationInput
                        v-model="m.destSubpath"
                        :directories="aircraftDirectories"
                        :disabled="building"
                      />
                      <button
                        type="button"
                        class="mt-1.5 text-gray-400 hover:text-red-500 disabled:opacity-50"
                        :disabled="building"
                        :title="$t('common.delete')"
                        @click="removeMapping(i)"
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
                    <p class="mt-1.5 break-all text-[11px] text-gray-500 dark:text-gray-400">
                      {{ m.archiveSubpath || $t('patch.archiveRoot') }} →
                      {{
                        m.destSubpath && m.destSubpath !== '/'
                          ? m.destSubpath
                          : $t('patch.aircraftRoot')
                      }}
                      ·
                      {{ describeMapping(mappingPreviews[i]) }}
                    </p>
                  </div>
                </div>

                <!-- Unmapped -->
                <div
                  v-if="!inferring && unmapped.length"
                  class="mt-2 p-2 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/30 rounded-lg"
                >
                  <p class="text-[11px] font-medium text-amber-700 dark:text-amber-300 mb-1">
                    {{ $t('patch.unmappedHint') }}
                  </p>
                  <div class="flex flex-wrap gap-1.5">
                    <button
                      v-for="u in unmapped"
                      :key="u"
                      type="button"
                      class="text-[11px] px-2 py-0.5 bg-white dark:bg-gray-800 border border-amber-300 dark:border-amber-500/40 rounded hover:border-orange-400 text-gray-700 dark:text-gray-200"
                      @click="addMapping(u)"
                    >
                      + {{ u }}
                    </button>
                  </div>
                </div>
              </div>
            </section>
          </div>

          <!-- Actions -->
          <div class="flex shrink-0 justify-end gap-2 pt-4">
            <button
              class="px-3 py-2 bg-gray-200 dark:bg-gray-700/80 hover:bg-gray-300 dark:hover:bg-gray-600/80 rounded-lg text-xs font-medium text-gray-700 dark:text-white"
              @click="handleCancel"
            >
              {{ $t('common.cancel') }}
            </button>
            <button
              :disabled="!canInstall"
              :class="[
                'px-3 py-2 rounded-lg text-xs font-medium flex items-center gap-1.5',
                canInstall
                  ? 'bg-orange-600 hover:bg-orange-700 text-white'
                  : 'bg-gray-300 dark:bg-gray-700/50 text-gray-500 cursor-not-allowed',
              ]"
              @click="confirmInstall"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 4v12m0 0l-4-4m4 4l4-4M4 20h16"
                />
              </svg>
              {{ building ? $t('patch.preparing') : $t('patch.install') }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import PatchSourcePicker from './PatchSourcePicker.vue'
import PatchDestinationInput from './PatchDestinationInput.vue'
import {
  buildPatchInstallTasks,
  detectPatchTargetAircraft,
  inferPatchMappings,
  summarizePatchInstall,
} from '@/services/patch-api'
import { loadPatchMappings, savePatchMappings } from '@/services/patch-mapping-memory'
import { getErrorMessage } from '@/types'
import type {
  InstallTask,
  PatchAircraftCandidate,
  PatchMappingInput,
  PatchMappingPreview,
} from '@/types'

const props = defineProps<{
  archivePath: string
  presetAircraftFolder?: string | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'install', tasks: InstallTask[]): void
}>()

const { t } = useI18n()
const store = useAppStore()

interface EditableMapping extends PatchMappingInput {
  id: number
}

const detecting = ref(true)
const inferring = ref(false)
const summarizing = ref(false)
const building = ref(false)
const errorMsg = ref('')

const candidates = ref<PatchAircraftCandidate[]>([])
const recommendedFolder = ref<string | null>(null)
const selectedFolder = ref('')

const archiveTree = ref<string[]>([])
const aircraftDirectories = ref<string[]>([])
const unmapped = ref<string[]>([])
const mappings = ref<EditableMapping[]>([])
const mappingsFolder = ref('')
const mappingPreviews = ref<PatchMappingPreview[]>([])
const hasRememberedMappings = ref(false)
const memoryError = ref(false)

// Install summary (what will happen) — kept current via summarizePatchInstall.
const totalFiles = ref(0)
const overwriteCount = ref(0)
const planId = ref('')

const backupEnabled = ref(true)
// The raw mapping editor is power-user surface area, hidden unless it's needed.
const advancedOpen = ref(false)

const archiveName = computed(() => props.archivePath.split(/[/\\]/).pop() || props.archivePath)

const selectedDisplayName = computed(
  () =>
    candidates.value.find((c) => c.folderName === selectedFolder.value)?.displayName ||
    selectedFolder.value,
)

const canInstall = computed(
  () =>
    !!selectedFolder.value &&
    mappingsFolder.value === selectedFolder.value &&
    mappings.value.length > 0 &&
    totalFiles.value > 0 &&
    !!planId.value &&
    !summarizing.value &&
    !building.value &&
    !inferring.value,
)

let summarizeTimer: ReturnType<typeof setTimeout> | null = null
let mounted = true
let inferenceRequest = 0
let summaryRequest = 0
let nextMappingId = 0
let mappingKey = ''
let automaticMappings: PatchMappingInput[] = []
let queuedMemorySnapshot: string | undefined
let memoryRequest = 0

onMounted(runDetect)
onUnmounted(() => {
  mounted = false
  inferenceRequest++
  invalidateSummary()
})

// Any change to the mappings (re-inference or manual edit) re-previews the summary.
watch(mappings, scheduleSummarize, { deep: true, flush: 'sync' })

async function runDetect() {
  detecting.value = true
  errorMsg.value = ''
  try {
    const result = await detectPatchTargetAircraft(props.archivePath, store.xplanePath)
    if (!mounted) return
    candidates.value = result.candidates
    recommendedFolder.value = result.recommendedFolder ?? null

    // The aircraft management entry is an explicit target. Otherwise only
    // preselect when the comparison identified a clear winner.
    const preset = props.presetAircraftFolder?.replaceAll('\\', '/')
    if (preset && candidates.value.some((c) => c.folderName === preset)) {
      selectedFolder.value = preset
    } else if (!preset && recommendedFolder.value) {
      selectedFolder.value = recommendedFolder.value
    }

    if (selectedFolder.value) {
      await runInfer(selectedFolder.value)
    }
  } catch (e) {
    if (mounted) errorMsg.value = getErrorMessage(e)
  } finally {
    if (mounted) detecting.value = false
  }
}

async function runInfer(folder: string) {
  const request = ++inferenceRequest
  const isCurrent = () => mounted && request === inferenceRequest && folder === selectedFolder.value
  mappingsFolder.value = ''
  mappingKey = ''
  hasRememberedMappings.value = false
  memoryError.value = false
  backupEnabled.value = true
  inferring.value = true
  mappings.value = []
  archiveTree.value = []
  aircraftDirectories.value = []
  unmapped.value = []
  invalidateSummary()
  if (!folder) {
    inferring.value = false
    return
  }
  errorMsg.value = ''
  try {
    const plan = await inferPatchMappings(props.archivePath, store.xplanePath, folder)
    if (!isCurrent()) return
    let remembered: PatchMappingInput[] | null = null
    let canRemember = true
    try {
      remembered = await loadPatchMappings(plan.mappingKey)
    } catch {
      canRemember = false
    }
    if (!isCurrent()) return
    mappingKey = canRemember ? plan.mappingKey : ''
    memoryError.value = !canRemember
    queuedMemorySnapshot = JSON.stringify(remembered)
    hasRememberedMappings.value = remembered !== null
    automaticMappings = plan.suggestedMappings.map(({ archiveSubpath, destSubpath }) => ({
      archiveSubpath,
      destSubpath,
    }))
    archiveTree.value = plan.archiveTree
    aircraftDirectories.value = plan.aircraftDirectories
    mappingsFolder.value = folder
    inferring.value = false
    setMappings(remembered ?? automaticMappings)
    // Surface the editor automatically when the plan needs a human decision.
    advancedOpen.value =
      remembered !== null ||
      plan.unmapped.length > 0 ||
      plan.suggestedMappings.some(
        (m) => m.reason === 'noAnchorDefaultRoot' || m.confidence === 'low',
      )
  } catch (e) {
    if (isCurrent()) errorMsg.value = getErrorMessage(e)
  } finally {
    if (isCurrent()) inferring.value = false
  }
}

function onAircraftChange() {
  if (!building.value) runInfer(selectedFolder.value)
}

function mappingInputs(): PatchMappingInput[] {
  return mappings.value.map(({ archiveSubpath, destSubpath }) => ({ archiveSubpath, destSubpath }))
}

function setMappings(inputs: PatchMappingInput[]) {
  mappings.value = inputs.map((input) => ({ ...input, id: nextMappingId++ }))
}

async function rememberMappings(inputs: PatchMappingInput[] | null) {
  const key = mappingKey
  if (!key) return
  const snapshot = JSON.stringify(inputs)
  if (snapshot === queuedMemorySnapshot) return
  if (queuedMemorySnapshot === 'null' && snapshot === JSON.stringify(automaticMappings)) return
  queuedMemorySnapshot = snapshot
  const request = ++memoryRequest
  const inference = inferenceRequest
  const isCurrent = () => mounted && request === memoryRequest && inference === inferenceRequest
  try {
    await savePatchMappings(key, inputs)
    if (isCurrent()) {
      hasRememberedMappings.value = inputs !== null
      memoryError.value = false
    }
  } catch {
    if (isCurrent()) {
      queuedMemorySnapshot = undefined
      memoryError.value = true
    }
  }
}

function resetMappings() {
  void rememberMappings(null)
  setMappings(automaticMappings)
}

function invalidateSummary() {
  summaryRequest++
  if (summarizeTimer) clearTimeout(summarizeTimer)
  summarizeTimer = null
  totalFiles.value = 0
  overwriteCount.value = 0
  planId.value = ''
  mappingPreviews.value = []
  unmapped.value = []
  summarizing.value = false
}

function scheduleSummarize() {
  invalidateSummary()
  if (
    !mounted ||
    inferring.value ||
    mappingsFolder.value !== selectedFolder.value ||
    !selectedFolder.value
  ) {
    return
  }
  // Invalidate in-flight previews immediately, including during the debounce.
  const request = summaryRequest
  errorMsg.value = ''
  if (!mappings.value.length) {
    unmapped.value = [...new Set(archiveTree.value.map((path) => path.split('/')[0]!))]
    void rememberMappings([])
    return
  }
  summarizing.value = true
  summarizeTimer = setTimeout(() => {
    summarizeTimer = null
    void runSummarize(request)
  }, 350)
}

async function runSummarize(request: number) {
  const folder = selectedFolder.value
  const inputs = mappingInputs()
  const isCurrent = () => mounted && request === summaryRequest && folder === selectedFolder.value
  if (!isCurrent() || !folder || !inputs.length) return
  try {
    const summary = await summarizePatchInstall({
      archivePath: props.archivePath,
      xplanePath: store.xplanePath,
      aircraftFolder: folder,
      mappings: inputs,
    })
    if (!isCurrent()) return
    totalFiles.value = summary.totalFiles
    overwriteCount.value = summary.overwriteCount
    planId.value = summary.planId
    mappingPreviews.value = summary.mappingPreviews
    unmapped.value = summary.unmapped
    void rememberMappings(inputs)
  } catch (e) {
    if (isCurrent()) errorMsg.value = getErrorMessage(e)
  } finally {
    if (isCurrent()) summarizing.value = false
  }
}

function addMapping(archiveSubpath = '') {
  if (inferring.value || building.value || mappingsFolder.value !== selectedFolder.value) return
  // Suggest liveries as the destination when the source folder looks like one.
  const dest = /(^|\/)liveries$/i.test(archiveSubpath) ? 'liveries' : ''
  mappings.value.push({ archiveSubpath, destSubpath: dest, id: nextMappingId++ })
}

function removeMapping(i: number) {
  mappings.value.splice(i, 1)
}

function describeMapping(preview: PatchMappingPreview | undefined): string {
  if (summarizing.value) return t('patch.mappingChecking')
  if (!preview) return t('patch.mappingUnavailable')
  return t(preview.destinationExists ? 'patch.mappingExistingFolder' : 'patch.mappingNewFolder', {
    overwrite: preview.overwriteCount,
    added: preview.fileCount - preview.overwriteCount,
  })
}

async function confirmInstall() {
  if (!canInstall.value) return
  building.value = true
  errorMsg.value = ''
  try {
    const tasks = await buildPatchInstallTasks({
      archivePath: props.archivePath,
      xplanePath: store.xplanePath,
      aircraftFolder: selectedFolder.value,
      mappings: mappingInputs(),
      backupOverwritten: backupEnabled.value,
      expectedPlanId: planId.value,
    })
    if (mounted) emit('install', tasks)
  } catch (e) {
    if (mounted) {
      building.value = false
      scheduleSummarize()
      errorMsg.value = getErrorMessage(e)
    }
  }
}

function handleCancel() {
  if (building.value) return
  emit('close')
}
</script>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.3s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.modal-content {
  display: flex;
  flex-direction: column;
  height: min(40rem, calc(100% - 2rem));
  background: white;
  border-radius: 1.25rem;
  padding: 1.75rem;
  max-width: 640px;
  width: 92%;
  border: 1px solid rgba(209, 213, 219, 1);
  color: #111827;
}
:root.dark .modal-content {
  background: linear-gradient(135deg, rgba(17, 24, 39, 0.98), rgba(31, 41, 55, 0.98));
  border: 1px solid rgba(55, 65, 81, 0.5);
  color: #f3f4f6;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 8px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(31, 41, 55, 0.3);
  border-radius: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: linear-gradient(180deg, rgba(249, 115, 22, 0.6), rgba(234, 88, 12, 0.6));
  border-radius: 4px;
}
</style>
