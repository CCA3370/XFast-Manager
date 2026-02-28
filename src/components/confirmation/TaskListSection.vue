<template>
  <div class="tasks-list mb-2 flex-1 min-h-0 overflow-y-auto custom-scrollbar">
    <div
      v-for="task in store.currentTasks"
      :key="task.id"
      class="task-item bg-white dark:bg-gray-800/50 border border-gray-200 dark:border-white/10 rounded-lg p-2 mb-1.5 hover:border-blue-400 dark:hover:border-blue-400/30 transition-colors duration-200"
      :class="{
        'opacity-50': !store.getTaskEnabled(task.id),
        'cursor-pointer': !isTaskDisabled(task),
        'cursor-not-allowed': isTaskDisabled(task),
      }"
      @click="!isTaskDisabled(task) && toggleTaskEnabled(task.id)"
    >
      <div class="flex items-start gap-2">
        <!-- Checkbox with better styling -->
        <div class="flex-shrink-0 pt-0.5">
          <div
            class="custom-checkbox"
            :class="{
              checked: store.getTaskEnabled(task.id),
              disabled: isTaskDisabled(task),
            }"
          >
            <svg
              v-if="store.getTaskEnabled(task.id)"
              class="w-2.5 h-2.5 text-white"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M5 13l4 4L19 7"
              ></path>
            </svg>
          </div>
        </div>

        <!-- Task Content -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5 mb-0.5 min-w-0">
            <span class="type-badge flex-shrink-0" :class="getTypeBadgeClass(task.type)">
              {{ task.type }}
            </span>
            <span class="font-medium text-gray-900 dark:text-white text-xs truncate min-w-0">{{
              task.displayName
            }}</span>
          </div>
          <div
            v-if="!isTaskDisabled(task)"
            class="flex items-center space-x-1 text-xs text-gray-500 dark:text-gray-400"
          >
            <svg
              class="w-2.5 h-2.5 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z"
              ></path>
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M8 5a2 2 0 012-2h4a2 2 0 012 2v2H8V5z"
              ></path>
            </svg>
            <span class="truncate text-xs"
              ><AnimatedText>{{ $t('modal.targetPath') }}</AnimatedText
              >: {{ getRelativePath(task.targetPath) }}</span
            >
          </div>

          <!-- Lua companion files/folders -->
          <div
            v-if="task.type === 'LuaScript' && hasLuaCompanions(task)"
            class="mt-1"
          >
            <button
              type="button"
              class="w-full flex items-center justify-between px-2 py-1 rounded bg-cyan-50 dark:bg-cyan-500/10 border border-cyan-200 dark:border-cyan-500/30 text-cyan-700 dark:text-cyan-300 text-xs"
              @click.stop="toggleLuaCompanions(task.id)"
            >
              <span class="flex items-center gap-1.5 min-w-0">
                <svg
                  class="w-3 h-3 flex-shrink-0 transition-transform duration-150"
                  :class="{ 'rotate-90': isLuaCompanionsExpanded(task.id) }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 5l7 7-7 7"
                  ></path>
                </svg>
                <span class="truncate">
                  {{ $t('luaCompanion.companionCount', { count: getLuaCompanions(task).length }) }}
                </span>
              </span>
              <span class="text-[10px] opacity-80">
                {{ $t('luaCompanion.expandCompanions') }}
              </span>
            </button>

            <div
              v-if="isLuaCompanionsExpanded(task.id)"
              class="mt-1 p-2 rounded border border-cyan-200 dark:border-cyan-500/30 bg-cyan-50/60 dark:bg-cyan-500/5 space-y-1"
            >
              <div class="text-[11px] font-medium text-cyan-700 dark:text-cyan-300">
                {{ $t('luaCompanion.companions') }}
              </div>
              <div
                v-for="companion in getLuaCompanions(task)"
                :key="`${task.id}-${companion}`"
                class="flex items-center gap-1.5 text-[11px] text-cyan-800 dark:text-cyan-200"
              >
                <svg
                  v-if="isLuaCompanionFile(companion)"
                  class="w-3.5 h-3.5 flex-shrink-0"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M7 2h7l5 5v13a2 2 0 01-2 2H7a2 2 0 01-2-2V4a2 2 0 012-2z"
                  ></path>
                </svg>
                <svg
                  v-else
                  class="w-3.5 h-3.5 flex-shrink-0"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-7l-2-2H5a2 2 0 00-2 2z"
                  ></path>
                </svg>
                <span class="break-all">{{ companion }}</span>
              </div>
            </div>
          </div>

          <!-- Conflict warning with install mode toggle switch (only for non-locked conflicts) -->
          <div v-if="task.conflictExists && !isLockedConflict(task)" class="mt-1.5">
            <div
              class="flex items-center space-x-1.5 text-xs text-yellow-600 dark:text-yellow-400 mb-1.5"
            >
              <svg
                class="w-3 h-3 flex-shrink-0"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                ></path>
              </svg>
              <span
                ><AnimatedText>{{
                  task.type === 'LuaScript' ? $t('modal.fileExists') : $t('modal.folderExists')
                }}</AnimatedText></span
              >
              <!-- Inline version comparison for Aircraft/Plugin -->
              <template
                v-if="
                  (task.type === 'Aircraft' || task.type === 'Plugin') &&
                  (task.existingVersionInfo?.version || task.newVersionInfo?.version)
                "
              >
                <span class="text-gray-400 dark:text-gray-500">·</span>
                <span class="text-blue-600 dark:text-blue-400">
                  {{ task.existingVersionInfo?.version || '?' }}
                  <span class="text-gray-400 dark:text-gray-500 mx-0.5">→</span>
                  {{ task.newVersionInfo?.version || '?' }}
                </span>
              </template>
              <!-- Inline cycle comparison for Navdata -->
              <template v-if="task.type === 'Navdata' && task.existingNavdataInfo">
                <span class="text-gray-400 dark:text-gray-500">·</span>
                <span class="text-blue-600 dark:text-blue-400">
                  {{ formatNavdataCycle(task.existingNavdataInfo) }}
                  <span class="text-gray-400 dark:text-gray-500 mx-0.5">→</span>
                  {{ formatNavdataCycle(task.newNavdataInfo) }}
                </span>
              </template>
            </div>

            <!-- Install mode toggle switch with mode name on the right -->
            <div class="flex items-center gap-2 text-xs" @click.stop>
              <label
                class="install-mode-switch flex-shrink-0"
                :class="{ disabled: !store.getTaskEnabled(task.id) }"
              >
                <input
                  type="checkbox"
                  :checked="!store.getTaskOverwrite(task.id)"
                  :disabled="!store.getTaskEnabled(task.id)"
                  @change="setTaskInstallMode(task.id, !store.getTaskOverwrite(task.id))"
                />
                <span class="switch-slider"></span>
              </label>
              <span
                class="font-medium flex-shrink-0"
                :class="
                  store.getTaskOverwrite(task.id)
                    ? 'text-blue-600 dark:text-blue-400'
                    : 'text-emerald-600 dark:text-emerald-400'
                "
              >
                <AnimatedText>{{
                  store.getTaskOverwrite(task.id)
                    ? $t('modal.directOverwrite')
                    : $t('modal.cleanInstall')
                }}</AnimatedText>
              </span>

              <!-- Backup options inline for Aircraft clean install -->
              <div
                v-if="task.type === 'Aircraft' && !store.getTaskOverwrite(task.id)"
                class="flex items-center gap-2 ml-2 pl-2 border-l border-emerald-300 dark:border-emerald-500/30"
              >
                <label
                  class="backup-checkbox-label"
                  :class="{ disabled: !store.getTaskEnabled(task.id) }"
                >
                  <input
                    type="checkbox"
                    :checked="getBackupLiveries(task.id)"
                    :disabled="!store.getTaskEnabled(task.id)"
                    class="backup-checkbox-input"
                    @change="setBackupLiveries(task.id, !getBackupLiveries(task.id))"
                  />
                  <span class="backup-checkbox-custom">
                    <svg class="backup-checkbox-icon" viewBox="0 0 12 12" fill="none">
                      <path
                        d="M2 6L5 9L10 3"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                  </span>
                  <span class="backup-checkbox-text">
                    <AnimatedText>{{ $t('modal.backupLiveries') }}</AnimatedText>
                  </span>
                </label>
                <label
                  class="backup-checkbox-label"
                  :class="{ disabled: !store.getTaskEnabled(task.id) || !hasConfigPatterns }"
                >
                  <input
                    type="checkbox"
                    :checked="getBackupConfigFiles(task.id)"
                    :disabled="!store.getTaskEnabled(task.id) || !hasConfigPatterns"
                    class="backup-checkbox-input"
                    @change="setBackupConfigFiles(task.id, !getBackupConfigFiles(task.id))"
                  />
                  <span class="backup-checkbox-custom">
                    <svg class="backup-checkbox-icon" viewBox="0 0 12 12" fill="none">
                      <path
                        d="M2 6L5 9L10 3"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                  </span>
                  <span
                    class="backup-checkbox-text"
                    :title="!hasConfigPatterns ? $t('modal.noConfigPatternsHint') : ''"
                  >
                    <AnimatedText>{{ $t('modal.backupConfigFiles') }}</AnimatedText>
                  </span>
                </label>
              </div>

              <!-- Backup option inline for Navdata clean install -->
              <div
                v-if="task.type === 'Navdata' && !store.getTaskOverwrite(task.id)"
                class="flex items-center gap-2 ml-2 pl-2 border-l border-emerald-300 dark:border-emerald-500/30"
              >
                <label
                  class="backup-checkbox-label"
                  :class="{ disabled: !store.getTaskEnabled(task.id) }"
                >
                  <input
                    type="checkbox"
                    :checked="store.getBackupNavdata(task.id)"
                    :disabled="!store.getTaskEnabled(task.id)"
                    class="backup-checkbox-input"
                    @change="store.setBackupNavdata(task.id, !store.getBackupNavdata(task.id))"
                  />
                  <span class="backup-checkbox-custom">
                    <svg class="backup-checkbox-icon" viewBox="0 0 12 12" fill="none">
                      <path
                        d="M2 6L5 9L10 3"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                  </span>
                  <span class="backup-checkbox-text">
                    <AnimatedText>{{ $t('modal.backupNavdata') }}</AnimatedText>
                  </span>
                </label>
              </div>
            </div>
          </div>

          <!-- Size warning with confirmation checkbox -->
          <div
            v-if="task.sizeWarning"
            class="mt-1.5 p-2 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded"
          >
            <div class="flex items-start space-x-2">
              <svg
                class="w-3.5 h-3.5 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                ></path>
              </svg>
              <div class="flex-1 min-w-0">
                <p class="text-xs text-red-700 dark:text-red-300">
                  {{ parseSizeWarning(task.sizeWarning).message }}
                </p>
                <label class="flex items-center space-x-1.5 mt-1.5 cursor-pointer" @click.stop>
                  <input
                    type="checkbox"
                    :checked="store.getTaskSizeConfirmed(task.id)"
                    class="w-3 h-3 rounded border-red-300 dark:border-red-500/50 bg-white dark:bg-red-500/10 text-red-600 dark:text-red-500 focus:ring-red-500 dark:focus:ring-red-500/50"
                    @change="toggleTaskSizeConfirm(task.id)"
                  />
                  <span class="text-xs text-red-700 dark:text-red-200"
                    ><AnimatedText>{{ $t('modal.confirmTrustArchive') }}</AnimatedText></span
                  >
                </label>
              </div>
            </div>
          </div>

          <!-- Livery aircraft not found warning -->
          <div
            v-if="task.type === 'Livery' && task.liveryAircraftFound === false"
            class="mt-1.5 flex items-center space-x-1.5 text-xs text-red-600 dark:text-red-400"
          >
            <svg
              class="w-3.5 h-3.5 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
              ></path>
            </svg>
            <span class="font-medium"
              ><AnimatedText>{{ $t('modal.liveryAircraftNotFound') }}</AnimatedText></span
            >
          </div>

          <!-- FlyWithLua not installed warning -->
          <div
            v-if="task.type === 'LuaScript' && task.flyWithLuaInstalled === false"
            class="mt-1.5 flex items-center space-x-1.5 text-xs text-red-600 dark:text-red-400"
          >
            <svg
              class="w-3.5 h-3.5 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
              ></path>
            </svg>
            <span class="font-medium"
              ><AnimatedText>{{ $t('modal.flyWithLuaRequired') }}</AnimatedText></span
            >
          </div>

          <!-- Locked conflict warning -->
          <div
            v-if="isLockedConflict(task)"
            class="mt-1.5 flex items-center space-x-1.5 text-xs text-amber-600 dark:text-amber-400"
          >
            <svg class="w-3.5 h-3.5 flex-shrink-0" fill="currentColor" viewBox="0 0 24 24">
              <path
                d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"
              />
            </svg>
            <span class="font-medium"
              ><AnimatedText>{{ $t('modal.targetLockedWarning') }}</AnimatedText></span
            >
          </div>

          <!-- Target path conflict warning -->
          <div
            v-if="store.isTaskInTargetPathConflict(task.id) && store.getTaskEnabled(task.id)"
            class="mt-1.5 flex items-center space-x-1.5 text-xs text-amber-600 dark:text-amber-400"
          >
            <svg
              class="w-3.5 h-3.5 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
              ></path>
            </svg>
            <span class="font-medium"
              ><AnimatedText>{{ $t('modal.targetPathConflictBadge') }}</AnimatedText></span
            >
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { useLockStore } from '@/stores/lock'
import { AddonType, NavdataInfo } from '@/types'
import type { InstallTask } from '@/types'
import AnimatedText from '@/components/AnimatedText.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const store = useAppStore()
const lockStore = useLockStore()

// Get relative path from X-Plane root
function getRelativePath(fullPath: string): string {
  const xplanePath = store.xplanePath
  if (!xplanePath || !fullPath.startsWith(xplanePath)) {
    return fullPath
  }
  let relativePath = fullPath.substring(xplanePath.length)
  if (relativePath.startsWith('/') || relativePath.startsWith('\\')) {
    relativePath = relativePath.substring(1)
  }
  return relativePath
}

// Truncate long text with ellipsis
function truncateText(text: string, maxLength: number = 30): string {
  if (text.length <= maxLength) return text
  return text.substring(0, maxLength) + '...'
}

// Format Navdata cycle for display
function formatNavdataCycle(info: NavdataInfo | undefined): string {
  if (!info) return t('modal.unknown')

  // Prefer airac, fallback to cycle, fallback to name (truncated)
  if (info.airac) return `AIRAC ${info.airac}`
  if (info.cycle) return `Cycle ${info.cycle}`
  return truncateText(info.name)
}

// Parse size warning message to get human-readable text
function parseSizeWarning(warning: string): { type: 'ratio' | 'size'; message: string } {
  if (warning.startsWith('SUSPICIOUS_RATIO:')) {
    const parts = warning.split(':')
    const ratio = parts[1]
    const size = parseFloat(parts[2]) / 1024 / 1024 / 1024
    return {
      type: 'ratio',
      message: t('modal.suspiciousRatio', { ratio, size: size.toFixed(2) }),
    }
  } else if (warning.startsWith('LARGE_SIZE:')) {
    const size = warning.split(':')[1]
    return {
      type: 'size',
      message: t('modal.largeSize', { size }),
    }
  }
  return { type: 'size', message: warning }
}

// Check if there are any config file patterns configured
const hasConfigPatterns = computed(() => {
  const patterns = store.getConfigFilePatterns()
  return patterns && patterns.length > 0
})

// Set install mode for individual task
function setTaskInstallMode(taskId: string, directOverwrite: boolean) {
  store.setTaskOverwrite(taskId, directOverwrite)
}

// Toggle individual task size confirmation
function toggleTaskSizeConfirm(taskId: string) {
  const currentValue = store.getTaskSizeConfirmed(taskId)
  store.setTaskSizeConfirmed(taskId, !currentValue)
}

// Toggle individual task enabled state
function toggleTaskEnabled(taskId: string) {
  const currentValue = store.getTaskEnabled(taskId)
  store.setTaskEnabled(taskId, !currentValue)
}

const expandedLuaCompanions = ref<Record<string, boolean>>({})

function hasLuaCompanions(task: InstallTask): boolean {
  return task.type === 'LuaScript' && getLuaCompanions(task).length > 0
}

function isLuaCompanionsExpanded(taskId: string): boolean {
  return !!expandedLuaCompanions.value[taskId]
}

function toggleLuaCompanions(taskId: string) {
  expandedLuaCompanions.value[taskId] = !expandedLuaCompanions.value[taskId]
}

function normalizeLuaCompanion(companion: string): string {
  return companion.trim().replace(/^[/\\]+/, '')
}

function isDisplayableLuaCompanion(companion: string): boolean {
  const normalized = normalizeLuaCompanion(companion)
  if (!normalized) return false

  const firstSegment = normalized.split(/[\\/]/).find(Boolean)
  return !!firstSegment && !/^\.+$/.test(firstSegment)
}

function getLuaCompanions(task: InstallTask): string[] {
  if (task.type !== 'LuaScript') return []

  const companions = task.companionPaths || []
  const deduped: string[] = []
  const seen = new Set<string>()

  for (const rawCompanion of companions) {
    if (!isDisplayableLuaCompanion(rawCompanion)) continue
    const normalized = normalizeLuaCompanion(rawCompanion)
    const key = normalized.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    deduped.push(normalized)
  }

  return deduped
}

function isLuaCompanionFile(companion: string): boolean {
  const normalized = normalizeLuaCompanion(companion)
  const name = normalized.split(/[\\/]/).pop() || normalized
  return /\.[A-Za-z0-9]+$/.test(name)
}

// Check if task is a livery without installed aircraft
function isLiveryWithoutAircraft(task: InstallTask): boolean {
  return task.type === 'Livery' && task.liveryAircraftFound === false
}

// Check if task is a Lua script without FlyWithLua installed
function isLuaWithoutFlyWithLua(task: InstallTask): boolean {
  return task.type === 'LuaScript' && task.flyWithLuaInstalled === false
}

// Check if task has a locked conflict (target exists and is locked)
function isLockedConflict(task: InstallTask): boolean {
  if (!task.conflictExists) return false
  return lockStore.isPathLocked(task.targetPath, store.xplanePath)
}

// Check if task should be disabled (livery without aircraft OR locked conflict OR lua without FlyWithLua)
function isTaskDisabled(task: InstallTask): boolean {
  return isLiveryWithoutAircraft(task) || isLockedConflict(task) || isLuaWithoutFlyWithLua(task)
}

// Get backup liveries setting for a task
function getBackupLiveries(taskId: string): boolean {
  return store.getTaskBackupSettings(taskId).liveries
}

// Set backup liveries setting for a task
function setBackupLiveries(taskId: string, value: boolean) {
  const current = store.getTaskBackupSettings(taskId)
  store.setTaskBackupSettings(taskId, value, current.configFiles)
}

// Get backup config files setting for a task
function getBackupConfigFiles(taskId: string): boolean {
  // Only return true if patterns are configured
  return hasConfigPatterns.value && store.getTaskBackupSettings(taskId).configFiles
}

// Set backup config files setting for a task
function setBackupConfigFiles(taskId: string, value: boolean) {
  const current = store.getTaskBackupSettings(taskId)
  store.setTaskBackupSettings(taskId, current.liveries, value)
}

function getTypeBadgeClass(type: AddonType) {
  switch (type) {
    case AddonType.Aircraft:
      return 'bg-blue-600'
    case AddonType.Scenery:
      return 'bg-green-600'
    case AddonType.SceneryLibrary:
      return 'bg-teal-600'
    case AddonType.Plugin:
      return 'bg-purple-600'
    case AddonType.Navdata:
      return 'bg-amber-600'
    case AddonType.Livery:
      return 'bg-pink-600'
    case AddonType.LuaScript:
      return 'bg-cyan-600'
    default:
      return 'bg-gray-600'
  }
}
</script>

<style scoped>
/* Custom scrollbar */
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 3px;
}

.dark .custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.1);
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(59, 130, 246, 0.5);
  border-radius: 3px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(59, 130, 246, 0.7);
}

/* Task items */
.task-item {
  transition:
    border-color 0.2s ease,
    opacity 0.2s ease;
}

/* Custom checkbox */
.custom-checkbox {
  width: 14px;
  height: 14px;
  border: 2px solid #d1d5db;
  border-radius: 0.25rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  background: white;
  cursor: pointer;
}

.dark .custom-checkbox {
  border-color: #4b5563;
  background: #374151;
}

.custom-checkbox.checked {
  background: #3b82f6;
  border-color: #3b82f6;
}

.dark .custom-checkbox.checked {
  background: #3b82f6;
  border-color: #3b82f6;
}

.custom-checkbox.disabled {
  background: #e5e7eb;
  border-color: #d1d5db;
  cursor: not-allowed;
}

.dark .custom-checkbox.disabled {
  background: #374151;
  border-color: #4b5563;
}

/* Install mode toggle switch */
.install-mode-switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 18px;
  cursor: pointer;
}

.install-mode-switch.disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.install-mode-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.install-mode-switch input:disabled {
  cursor: not-allowed;
}

.switch-slider {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  border-radius: 18px;
  transition: 0.3s;
}

/* Direct Overwrite (unchecked) - Blue background */
.switch-slider {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
}

.dark .switch-slider {
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
}

/* Clean Install (checked) - Green background */
.install-mode-switch input:checked + .switch-slider {
  background: linear-gradient(135deg, #10b981, #059669);
}

.dark .install-mode-switch input:checked + .switch-slider {
  background: linear-gradient(135deg, #059669, #047857);
}

/* Switch knob */
.switch-slider:before {
  content: '';
  position: absolute;
  height: 14px;
  width: 14px;
  left: 2px;
  bottom: 2px;
  background-color: white;
  border-radius: 50%;
  transition: 0.3s;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.install-mode-switch input:checked + .switch-slider:before {
  transform: translateX(18px);
}

/* Type badges */
.type-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  backdrop-filter: blur(10px);
  flex-shrink: 0;
  letter-spacing: 0.025em;
}

.type-badge.bg-blue-600 {
  background: linear-gradient(135deg, rgba(37, 99, 235, 0.8), rgba(59, 130, 246, 0.8));
  color: white;
}

.type-badge.bg-green-600 {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.8), rgba(74, 222, 128, 0.8));
  color: white;
}

.type-badge.bg-teal-600 {
  background: linear-gradient(135deg, rgba(13, 148, 136, 0.8), rgba(20, 184, 166, 0.8));
  color: white;
}

.type-badge.bg-purple-600 {
  background: linear-gradient(135deg, rgba(147, 51, 234, 0.8), rgba(168, 85, 247, 0.8));
  color: white;
}

.type-badge.bg-amber-600 {
  background: linear-gradient(135deg, rgba(217, 119, 6, 0.8), rgba(245, 158, 11, 0.8));
  color: white;
}

.type-badge.bg-pink-600 {
  background: linear-gradient(135deg, rgba(219, 39, 119, 0.8), rgba(236, 72, 153, 0.8));
  color: white;
}

.type-badge.bg-cyan-600 {
  background: linear-gradient(135deg, rgba(8, 145, 178, 0.8), rgba(6, 182, 212, 0.8));
  color: white;
}

.type-badge.bg-gray-600 {
  background: linear-gradient(135deg, rgba(75, 85, 99, 0.8), rgba(107, 114, 128, 0.8));
  color: white;
}

/* Backup checkbox styles */
.backup-checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  cursor: pointer;
  user-select: none;
  transition: opacity 0.2s ease;
}

.backup-checkbox-label.disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.backup-checkbox-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
  pointer-events: none;
}

.backup-checkbox-custom {
  position: relative;
  width: 14px;
  height: 14px;
  border: 2px solid #10b981;
  border-radius: 0.25rem;
  background: white;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.dark .backup-checkbox-custom {
  background: rgba(16, 185, 129, 0.1);
  border-color: rgba(16, 185, 129, 0.5);
}

.backup-checkbox-label:hover .backup-checkbox-custom {
  border-color: #059669;
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.1);
}

.dark .backup-checkbox-label:hover .backup-checkbox-custom {
  border-color: #10b981;
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15);
}

.backup-checkbox-icon {
  width: 10px;
  height: 10px;
  color: white;
  opacity: 0;
  transform: scale(0.5);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.backup-checkbox-input:checked + .backup-checkbox-custom {
  background: linear-gradient(135deg, #10b981, #059669);
  border-color: #10b981;
}

.dark .backup-checkbox-input:checked + .backup-checkbox-custom {
  background: linear-gradient(135deg, #10b981, #059669);
  border-color: #10b981;
}

.backup-checkbox-input:checked + .backup-checkbox-custom .backup-checkbox-icon {
  opacity: 1;
  transform: scale(1);
}

.backup-checkbox-label.disabled .backup-checkbox-custom {
  background: #e5e7eb;
  border-color: #d1d5db;
}

.dark .backup-checkbox-label.disabled .backup-checkbox-custom {
  background: #374151;
  border-color: #4b5563;
}

.backup-checkbox-text {
  font-size: 0.75rem;
  line-height: 1rem;
  color: #047857;
  font-weight: 500;
  transition: color 0.2s ease;
}

.dark .backup-checkbox-text {
  color: #6ee7b7;
}

.backup-checkbox-label:hover .backup-checkbox-text {
  color: #065f46;
}

.dark .backup-checkbox-label:hover .backup-checkbox-text {
  color: #86efac;
}

.backup-checkbox-label.disabled .backup-checkbox-text {
  color: #9ca3af;
}

.dark .backup-checkbox-label.disabled .backup-checkbox-text {
  color: #6b7280;
}
</style>
