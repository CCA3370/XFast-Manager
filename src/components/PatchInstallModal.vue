<template>
  <Teleport to="body">
    <transition name="modal">
      <div class="modal-overlay" @click="handleCancel">
        <div class="modal-content animate-scale-in" @click.stop>
          <!-- Header -->
          <div class="flex items-center space-x-2 mb-4">
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
              <p class="text-orange-600 dark:text-orange-300/80 text-xs mt-0.5 truncate" :title="archiveName">
                {{ archiveName }}
              </p>
            </div>
          </div>

          <!-- Error -->
          <div
            v-if="errorMsg"
            class="mb-3 p-2 bg-red-100 dark:bg-red-500/20 border border-red-300 dark:border-red-500/50 rounded-lg text-xs font-medium text-red-600 dark:text-red-400"
          >
            {{ errorMsg }}
          </div>

          <div class="space-y-4 max-h-[60vh] overflow-y-auto custom-scrollbar pr-1">
            <!-- Target aircraft -->
            <section>
              <label class="block text-xs font-semibold text-gray-700 dark:text-gray-200 mb-1.5">
                {{ $t('patch.targetAircraft') }}
              </label>
              <div v-if="detecting" class="text-xs text-gray-500 dark:text-gray-400 py-2">
                {{ $t('patch.detecting') }}
              </div>
              <template v-else-if="candidates.length">
                <select
                  v-model="selectedFolder"
                  class="w-full px-3 py-2 bg-white dark:bg-gray-900/70 border border-gray-200 dark:border-gray-700/50 rounded-lg text-gray-900 dark:text-white text-sm focus:border-orange-400 focus:ring-2 focus:ring-orange-500/30 transition-all"
                  @change="onAircraftChange"
                >
                  <option v-for="c in candidates" :key="c.folderName" :value="c.folderName">
                    {{ candidateLabel(c) }}
                  </option>
                </select>
                <p
                  v-if="recommendedFolder && selectedFolder === recommendedFolder"
                  class="text-xs text-emerald-600 dark:text-emerald-400 mt-1.5 flex items-center gap-1"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                  </svg>
                  {{ $t('patch.autoRecommended') }}
                </p>
              </template>
              <div v-else class="text-xs text-amber-600 dark:text-amber-400 py-2">
                {{ $t('patch.noAircraft') }}
              </div>
            </section>

            <!-- Mappings -->
            <section v-if="selectedFolder">
              <div class="flex items-center justify-between mb-1.5">
                <label class="block text-xs font-semibold text-gray-700 dark:text-gray-200">
                  {{ $t('patch.mappings') }}
                </label>
                <button
                  type="button"
                  class="text-xs text-orange-600 dark:text-orange-400 hover:underline"
                  @click="addMapping()"
                >
                  + {{ $t('patch.addMapping') }}
                </button>
              </div>

              <div v-if="inferring" class="text-xs text-gray-500 dark:text-gray-400 py-2">
                {{ $t('patch.inferring') }}
              </div>

              <div v-else class="space-y-2">
                <div
                  v-for="(m, i) in mappings"
                  :key="i"
                  class="bg-gray-100 dark:bg-gray-800/60 border border-gray-200 dark:border-gray-700/50 rounded-lg p-2.5"
                >
                  <div class="flex items-center gap-2">
                    <input
                      v-model="m.archiveSubpath"
                      list="patch-archive-dirs"
                      :placeholder="$t('patch.archiveRootPlaceholder')"
                      class="flex-1 min-w-0 px-2 py-1.5 bg-white dark:bg-gray-900/70 border border-gray-200 dark:border-gray-700/50 rounded text-gray-900 dark:text-white text-xs"
                    />
                    <svg class="w-4 h-4 text-gray-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3" />
                    </svg>
                    <input
                      v-model="m.destSubpath"
                      list="patch-dest-dirs"
                      :placeholder="$t('patch.destRootPlaceholder')"
                      class="flex-1 min-w-0 px-2 py-1.5 bg-white dark:bg-gray-900/70 border border-gray-200 dark:border-gray-700/50 rounded text-gray-900 dark:text-white text-xs"
                    />
                    <button
                      type="button"
                      class="text-gray-400 hover:text-red-500 flex-shrink-0"
                      :title="$t('common.delete')"
                      @click="removeMapping(i)"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </div>
                  <div class="flex items-center gap-2 mt-1.5 pl-0.5">
                    <span
                      v-if="m.confidence"
                      class="text-[10px] font-semibold px-1.5 py-0.5 rounded"
                      :class="confidenceClass(m.confidence)"
                    >
                      {{ $t('patch.confidence.' + m.confidence) }}
                    </span>
                    <span class="text-[11px] text-gray-500 dark:text-gray-400 truncate">
                      {{ describeMapping(m) }}
                    </span>
                  </div>
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
            </section>

            <!-- Backup toggle -->
            <section v-if="selectedFolder">
              <label class="flex items-center gap-2 cursor-pointer">
                <input v-model="backupEnabled" type="checkbox" class="w-4 h-4 accent-orange-600" />
                <span class="text-xs font-medium text-gray-700 dark:text-gray-200">
                  {{ $t('patch.backupLabel') }}
                </span>
              </label>
              <p class="text-[11px] text-gray-500 dark:text-gray-400 mt-1 ml-6">
                {{ $t('patch.backupHint') }}
              </p>
            </section>
          </div>

          <!-- Shared datalists -->
          <datalist id="patch-archive-dirs">
            <option v-for="d in archiveTree" :key="d" :value="d"></option>
          </datalist>
          <datalist id="patch-dest-dirs">
            <option value="liveries"></option>
            <option v-for="d in aircraftSubdirs" :key="d" :value="d"></option>
          </datalist>

          <!-- Actions -->
          <div class="flex justify-end gap-2 pt-4">
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
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v12m0 0l-4-4m4 4l4-4M4 20h16" />
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
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import {
  buildPatchInstallTasks,
  detectPatchTargetAircraft,
  inferPatchMappings,
} from '@/services/patch-api'
import { getErrorMessage } from '@/types'
import type { InstallTask, PatchAircraftCandidate, PatchConfidence } from '@/types'

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

interface EditableMapping {
  archiveSubpath: string
  destSubpath: string
  confidence?: PatchConfidence
  reason?: string
  fileCount?: number
}

const detecting = ref(true)
const inferring = ref(false)
const building = ref(false)
const errorMsg = ref('')

const candidates = ref<PatchAircraftCandidate[]>([])
const recommendedFolder = ref<string | null>(null)
const selectedFolder = ref('')

const archiveTree = ref<string[]>([])
const aircraftSubdirs = ref<string[]>([])
const unmapped = ref<string[]>([])
const mappings = ref<EditableMapping[]>([])

const backupEnabled = ref(true)

const archiveName = computed(() => props.archivePath.split(/[/\\]/).pop() || props.archivePath)

const canInstall = computed(
  () => !!selectedFolder.value && mappings.value.length > 0 && !building.value && !inferring.value,
)

onMounted(runDetect)

async function runDetect() {
  detecting.value = true
  errorMsg.value = ''
  try {
    const result = await detectPatchTargetAircraft(props.archivePath, store.xplanePath)
    candidates.value = result.candidates
    recommendedFolder.value = result.recommendedFolder ?? null

    // Pre-select: explicit preset > recommended > best candidate.
    const preset = props.presetAircraftFolder
    if (preset && candidates.value.some((c) => c.folderName === preset)) {
      selectedFolder.value = preset
    } else if (recommendedFolder.value) {
      selectedFolder.value = recommendedFolder.value
    } else if (candidates.value.length) {
      selectedFolder.value = candidates.value[0].folderName
    }

    if (selectedFolder.value) {
      await runInfer(selectedFolder.value)
    }
  } catch (e) {
    errorMsg.value = getErrorMessage(e)
  } finally {
    detecting.value = false
  }
}

async function runInfer(folder: string) {
  inferring.value = true
  errorMsg.value = ''
  try {
    const plan = await inferPatchMappings(props.archivePath, store.xplanePath, folder)
    archiveTree.value = plan.archiveTree
    aircraftSubdirs.value = plan.aircraftSubdirs
    unmapped.value = plan.unmapped
    mappings.value = plan.suggestedMappings.map((m) => ({
      archiveSubpath: m.archiveSubpath,
      destSubpath: m.destSubpath,
      confidence: m.confidence,
      reason: m.reason,
      fileCount: m.fileCount,
    }))
  } catch (e) {
    errorMsg.value = getErrorMessage(e)
  } finally {
    inferring.value = false
  }
}

function onAircraftChange() {
  if (selectedFolder.value) runInfer(selectedFolder.value)
}

function addMapping(archiveSubpath = '') {
  // Suggest liveries as the destination when the source folder looks like one.
  const dest = /(^|\/)liveries$/i.test(archiveSubpath) ? 'liveries' : ''
  mappings.value.push({ archiveSubpath, destSubpath: dest })
  // Once an unmapped entry is added as a mapping, drop it from the hint list.
  unmapped.value = unmapped.value.filter((u) => u !== archiveSubpath)
}

function removeMapping(i: number) {
  mappings.value.splice(i, 1)
}

function candidateLabel(c: PatchAircraftCandidate): string {
  if (c.sampleSize > 0 && c.matchedCount > 0) {
    return `${c.displayName} (${c.matchedCount}/${c.sampleSize})`
  }
  return c.displayName
}

function confidenceClass(c: PatchConfidence): string {
  switch (c) {
    case 'high':
      return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-500/20 dark:text-emerald-300'
    case 'medium':
      return 'bg-amber-100 text-amber-700 dark:bg-amber-500/20 dark:text-amber-300'
    default:
      return 'bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-300'
  }
}

function describeMapping(m: EditableMapping): string {
  const from = m.archiveSubpath || t('patch.archiveRoot')
  const to = m.destSubpath || t('patch.aircraftRoot')
  let reason = ''
  if (m.reason?.startsWith('matchedExisting:')) {
    reason = ' · ' + t('patch.reasonMatched', { count: Number(m.reason.split(':')[1]) || 0 })
  } else if (m.reason === 'liveryFolder') {
    reason = ' · ' + t('patch.reasonLivery')
  } else if (m.reason === 'noAnchorDefaultRoot') {
    reason = ' · ' + t('patch.reasonNoAnchor')
  } else if (m.reason === 'couldNotInspectArchive') {
    reason = ' · ' + t('patch.reasonNoListing')
  }
  return `${from} → ${to}${reason}`
}

async function confirmInstall() {
  if (!canInstall.value) return
  building.value = true
  errorMsg.value = ''
  try {
    // Deduplicate identical rows; allow root→root.
    const seen = new Set<string>()
    const inputs = mappings.value
      .map((m) => ({
        archiveSubpath: m.archiveSubpath.trim(),
        destSubpath: m.destSubpath.trim(),
      }))
      .filter((m) => {
        const key = `${m.archiveSubpath}|${m.destSubpath}`
        if (seen.has(key)) return false
        seen.add(key)
        return true
      })

    const tasks = await buildPatchInstallTasks({
      archivePath: props.archivePath,
      xplanePath: store.xplanePath,
      aircraftFolder: selectedFolder.value,
      mappings: inputs,
      backupOverwritten: backupEnabled.value,
    })
    emit('install', tasks)
  } catch (e) {
    errorMsg.value = getErrorMessage(e)
    building.value = false
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
  transition: all 0.3s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
.modal-enter-from .modal-content,
.modal-leave-to .modal-content {
  opacity: 0;
  transform: scale(0.9) translateY(-20px);
}
@keyframes scale-in {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}
.animate-scale-in {
  animation: scale-in 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
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
