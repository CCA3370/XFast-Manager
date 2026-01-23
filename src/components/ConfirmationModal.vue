<template>
  <Teleport to="body">
    <transition name="modal">
      <div class="modal-overlay animate-fade-in" @click.self>
        <div class="modal-content animate-scale-in" @click.stop>
          <!-- Header -->
          <div class="modal-header mb-2 flex-shrink-0">
            <div class="flex items-center justify-between">
              <div class="flex items-center space-x-2">
                <div class="w-7 h-7 bg-gradient-to-r from-green-500 to-blue-600 rounded-lg flex items-center justify-center">
                  <svg class="w-3.5 h-3.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-base font-bold text-gray-900 dark:text-white"><AnimatedText>{{ $t('modal.confirmInstallation') }}</AnimatedText></h3>
                  <p class="text-gray-500 dark:text-gray-400 text-xs leading-tight"><AnimatedText>{{ $t('modal.installToXplane') }}</AnimatedText></p>
                </div>
              </div>
              <div class="flex items-center space-x-1.5 px-2.5 py-1 bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/20 rounded-lg">
                <svg class="w-3.5 h-3.5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"></path>
                </svg>
                <span class="text-xs font-semibold text-blue-700 dark:text-blue-300">
                  <AnimatedText>{{ store.enabledTasksCount }}/{{ store.currentTasks.length }}</AnimatedText>
                </span>
              </div>
            </div>
          </div>

          <!-- Size Warning Banner (only show if any size warnings exist) -->
          <div v-if="store.hasSizeWarnings" class="mb-2 p-2 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-lg flex-shrink-0">
            <div class="flex items-start space-x-1.5">
              <svg class="w-3.5 h-3.5 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
              </svg>
              <div class="flex-1">
                <span class="font-medium text-xs text-red-700 dark:text-red-100"><AnimatedText>{{ $t('modal.sizeWarningTitle') }}</AnimatedText></span>
                <p class="text-xs text-red-600 dark:text-red-200/70 leading-tight"><AnimatedText>{{ $t('modal.sizeWarningDesc') }}</AnimatedText></p>
              </div>
            </div>
          </div>

          <!-- Tasks List -->
          <div class="tasks-list mb-2 flex-1 min-h-0 overflow-y-auto custom-scrollbar">
            <div
              v-for="task in store.currentTasks"
              :key="task.id"
              class="task-item bg-white dark:bg-gray-800/50 border border-gray-200 dark:border-white/10 rounded-lg p-2 mb-1.5 hover:border-blue-400 dark:hover:border-blue-400/30 transition-colors duration-200 cursor-pointer"
              :class="{ 'opacity-50': !store.getTaskEnabled(task.id) }"
              @click="toggleTaskEnabled(task.id)"
            >
              <div class="flex items-start gap-2">
                <!-- Checkbox with better styling -->
                <div class="flex-shrink-0 pt-0.5">
                  <div class="custom-checkbox" :class="{ 'checked': store.getTaskEnabled(task.id) }">
                    <svg v-if="store.getTaskEnabled(task.id)" class="w-2.5 h-2.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"></path>
                    </svg>
                  </div>
                </div>

                <!-- Task Content -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5 mb-0.5">
                    <span class="type-badge" :class="getTypeBadgeClass(task.type)">
                      {{ task.type }}
                    </span>
                    <span class="font-medium text-gray-900 dark:text-white text-xs truncate">{{ task.displayName }}</span>
                  </div>
                  <div class="flex items-center space-x-1 text-xs text-gray-500 dark:text-gray-400">
                    <svg class="w-2.5 h-2.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z"></path>
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5a2 2 0 012-2h4a2 2 0 012 2v2H8V5z"></path>
                    </svg>
                    <span class="truncate text-xs"><AnimatedText>{{ $t('modal.targetPath') }}</AnimatedText>: {{ getRelativePath(task.targetPath) }}</span>
                  </div>

                  <!-- Conflict warning with install mode toggle switch -->
                  <div v-if="task.conflictExists" class="mt-1.5">
                    <div class="flex items-center space-x-1.5 text-xs text-yellow-600 dark:text-yellow-400 mb-1.5">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                      </svg>
                      <span><AnimatedText>{{ $t('modal.folderExists') }}</AnimatedText></span>
                    </div>

                    <!-- Install mode toggle switch with mode name on the right -->
                    <div class="flex items-center gap-2 text-xs" @click.stop>
                      <label class="install-mode-switch flex-shrink-0" :class="{ 'disabled': !store.getTaskEnabled(task.id) }">
                        <input
                          type="checkbox"
                          :checked="!store.getTaskOverwrite(task.id)"
                          :disabled="!store.getTaskEnabled(task.id)"
                          @change="setTaskInstallMode(task.id, !store.getTaskOverwrite(task.id))"
                        >
                        <span class="switch-slider"></span>
                      </label>
                      <span class="font-medium flex-shrink-0" :class="store.getTaskOverwrite(task.id) ? 'text-blue-600 dark:text-blue-400' : 'text-emerald-600 dark:text-emerald-400'">
                        <AnimatedText>{{ store.getTaskOverwrite(task.id) ? $t('modal.directOverwrite') : $t('modal.cleanInstall') }}</AnimatedText>
                      </span>

                      <!-- Backup options inline for Aircraft clean install -->
                      <div v-if="task.type === 'Aircraft' && !store.getTaskOverwrite(task.id)"
                           class="flex items-center gap-2 ml-2 pl-2 border-l border-emerald-300 dark:border-emerald-500/30">
                        <label class="backup-checkbox-label" :class="{ 'disabled': !store.getTaskEnabled(task.id) }">
                          <input
                            type="checkbox"
                            :checked="getBackupLiveries(task.id)"
                            :disabled="!store.getTaskEnabled(task.id)"
                            @change="setBackupLiveries(task.id, !getBackupLiveries(task.id))"
                            class="backup-checkbox-input"
                          >
                          <span class="backup-checkbox-custom">
                            <svg class="backup-checkbox-icon" viewBox="0 0 12 12" fill="none">
                              <path d="M2 6L5 9L10 3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                          </span>
                          <span class="backup-checkbox-text">
                            <AnimatedText>{{ $t('modal.backupLiveries') }}</AnimatedText>
                          </span>
                        </label>
                        <label class="backup-checkbox-label" :class="{ 'disabled': !store.getTaskEnabled(task.id) || !hasConfigPatterns }">
                          <input
                            type="checkbox"
                            :checked="getBackupConfigFiles(task.id)"
                            :disabled="!store.getTaskEnabled(task.id) || !hasConfigPatterns"
                            @change="setBackupConfigFiles(task.id, !getBackupConfigFiles(task.id))"
                            class="backup-checkbox-input"
                          >
                          <span class="backup-checkbox-custom">
                            <svg class="backup-checkbox-icon" viewBox="0 0 12 12" fill="none">
                              <path d="M2 6L5 9L10 3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                          </span>
                          <span class="backup-checkbox-text" :title="!hasConfigPatterns ? $t('modal.noConfigPatternsHint') : ''">
                            <AnimatedText>{{ $t('modal.backupConfigFiles') }}</AnimatedText>
                          </span>
                        </label>
                      </div>
                    </div>
                  </div>

                  <!-- Navdata cycle comparison -->
                  <div v-if="task.type === 'Navdata' && task.conflictExists && task.existingNavdataInfo"
                       class="mt-1.5 p-2 bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/20 rounded">
                    <div class="flex items-center space-x-2 text-xs">
                      <svg class="w-3.5 h-3.5 text-blue-500 dark:text-blue-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                      <span class="text-blue-700 dark:text-blue-200">
                        <AnimatedText>{{ $t('modal.existingCycle') }}</AnimatedText>:
                        <span class="font-medium">{{ formatNavdataCycle(task.existingNavdataInfo) }}</span>
                        <span class="mx-1.5">→</span>
                        <AnimatedText>{{ $t('modal.newCycle') }}</AnimatedText>:
                        <span class="font-medium">{{ formatNavdataCycle(task.newNavdataInfo) }}</span>
                      </span>
                    </div>
                  </div>

                  <!-- Size warning with confirmation checkbox -->
                  <div v-if="task.sizeWarning" class="mt-1.5 p-2 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded">
                    <div class="flex items-start space-x-2">
                      <svg class="w-3.5 h-3.5 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                      </svg>
                      <div class="flex-1 min-w-0">
                        <p class="text-xs text-red-700 dark:text-red-300">{{ parseSizeWarning(task.sizeWarning).message }}</p>
                        <label class="flex items-center space-x-1.5 mt-1.5 cursor-pointer" @click.stop>
                          <input
                            type="checkbox"
                            :checked="store.getTaskSizeConfirmed(task.id)"
                            @change="toggleTaskSizeConfirm(task.id)"
                            class="w-3 h-3 rounded border-red-300 dark:border-red-500/50 bg-white dark:bg-red-500/10 text-red-600 dark:text-red-500 focus:ring-red-500 dark:focus:ring-red-500/50"
                          >
                          <span class="text-xs text-red-700 dark:text-red-200"><AnimatedText>{{ $t('modal.confirmTrustArchive') }}</AnimatedText></span>
                        </label>
                      </div>
                    </div>
                  </div>

                  <!-- Livery aircraft not found warning -->
                  <div v-if="task.type === 'Livery' && task.liveryAircraftFound === false" class="mt-1.5 p-2 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 rounded">
                    <div class="flex items-start space-x-2">
                      <svg class="w-3.5 h-3.5 text-amber-500 dark:text-amber-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                      </svg>
                      <div class="flex-1 min-w-0">
                        <span class="font-medium text-xs text-amber-700 dark:text-amber-100"><AnimatedText>{{ $t('modal.liveryAircraftNotFound') }}</AnimatedText></span>
                        <p class="text-xs text-amber-600 dark:text-amber-200/70 leading-tight"><AnimatedText>{{ $t('modal.liveryAircraftNotFoundDesc') }}</AnimatedText></p>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex justify-end gap-1.5 flex-shrink-0 pt-1.5">
            <button
              @click="$emit('close')"
              class="px-3 py-1.5 bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-700 text-gray-700 dark:text-white rounded-lg transition-all duration-200 hover:scale-105 text-xs font-medium flex items-center space-x-1"
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
              <span><AnimatedText>{{ $t('common.cancel') }}</AnimatedText></span>
            </button>
            <button
              @click="$emit('confirm')"
              :disabled="installDisabled"
              :class="[
                'px-3 py-1.5 rounded-lg transition-all duration-200 text-xs font-medium flex items-center space-x-1',
                installDisabled
                  ? 'bg-gray-300 dark:bg-gray-600 cursor-not-allowed opacity-50 text-gray-500 dark:text-gray-400'
                  : 'bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700 hover:scale-105 text-white'
              ]"
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
              </svg>
              <span><AnimatedText>{{ $t('modal.startInstallation') }}</AnimatedText></span>
            </button>
          </div>
        </div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { AddonType, NavdataInfo } from '@/types'
import AnimatedText from '@/components/AnimatedText.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const store = useAppStore()

defineEmits(['close', 'confirm'])

// Get relative path from X-Plane root
function getRelativePath(fullPath: string): string {
  const xplanePath = store.xplanePath
  if (!xplanePath || !fullPath.startsWith(xplanePath)) {
    return fullPath
  }
  // Remove X-Plane path and leading slash/backslash
  let relativePath = fullPath.substring(xplanePath.length)
  if (relativePath.startsWith('/') || relativePath.startsWith('\\')) {
    relativePath = relativePath.substring(1)
  }
  return relativePath
}

// Format Navdata cycle for display
function formatNavdataCycle(info: NavdataInfo | undefined): string {
  if (!info) return t('modal.unknown')

  // Prefer airac, fallback to cycle, fallback to name
  if (info.airac) return `AIRAC ${info.airac}`
  if (info.cycle) return `Cycle ${info.cycle}`
  return info.name
}

// Parse size warning message to get human-readable text
function parseSizeWarning(warning: string): { type: 'ratio' | 'size', message: string } {
  if (warning.startsWith('SUSPICIOUS_RATIO:')) {
    const parts = warning.split(':')
    const ratio = parts[1]
    const size = parseFloat(parts[2]) / 1024 / 1024 / 1024
    return {
      type: 'ratio',
      message: t('modal.suspiciousRatio', { ratio, size: size.toFixed(2) })
    }
  } else if (warning.startsWith('LARGE_SIZE:')) {
    const size = warning.split(':')[1]
    return {
      type: 'size',
      message: t('modal.largeSize', { size })
    }
  }
  return { type: 'size', message: warning }
}

// Check if install button should be disabled
const installDisabled = computed(() => {
  // Disable if no tasks are enabled
  if (store.enabledTasksCount === 0) return true
  // Disable if there are size warnings that haven't been confirmed
  return store.hasSizeWarnings && !store.allSizeWarningsConfirmed
})

// Check if there are any config file patterns configured
const hasConfigPatterns = computed(() => {
  const patterns = store.getConfigFilePatterns()
  return patterns && patterns.length > 0
})

// Set install mode for individual task
// cleanInstall = false (should_overwrite = false): Clean install - delete old folder
// cleanInstall = true (should_overwrite = true): Direct overwrite - keep existing files
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
    default:
      return 'bg-gray-600'
  }
}
</script>

<style scoped>
/* Modal animations */
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
    transform: scale(0.9) translateY(-20px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.animate-scale-in {
  animation: scale-in 0.4s ease-out;
}

.animate-fade-in {
  animation: fade-in 0.3s ease-out;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* Modal overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

/* Modal content */
.modal-content {
  background: white;
  border-radius: 0.75rem;
  padding: 0.875rem;
  max-width: 520px;
  width: 90%;
  border: 1px solid rgba(229, 231, 235, 1);
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
  max-height: 88vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dark .modal-content {
  background: linear-gradient(135deg, rgba(31, 41, 55, 0.95), rgba(17, 24, 39, 0.95));
  border: 1px solid rgba(59, 130, 246, 0.3);
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.8);
}

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
  transition: border-color 0.2s ease, opacity 0.2s ease;
}

.task-item:hover {
  /* Remove float effect, only change border color */
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
  content: "";
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

.type-badge.bg-gray-600 {
  background: linear-gradient(135deg, rgba(75, 85, 99, 0.8), rgba(107, 114, 128, 0.8));
  color: white;
}

/* Hover scale utility */
.hover\:scale-102:hover {
  transform: scale(1.02);
}

/* Toggle Switch */
.toggle-switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  flex-shrink: 0;
}

.toggle-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(75, 85, 99, 0.8);
  transition: 0.3s;
  border-radius: 24px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: 0.3s;
  border-radius: 50%;
}

input:checked + .toggle-slider {
  background: linear-gradient(135deg, rgba(234, 179, 8, 0.8), rgba(245, 158, 11, 0.8));
}

input:checked + .toggle-slider:before {
  transform: translateX(20px);
}

/* Smaller toggle for global overwrite */
.toggle-switch-sm {
  width: 36px;
  height: 20px;
}

.toggle-switch-sm .toggle-slider:before {
  height: 14px;
  width: 14px;
}

.toggle-switch-sm input:checked + .toggle-slider:before {
  transform: translateX(16px);
}

/* Extra small toggle for individual tasks */
.toggle-switch-xs {
  width: 28px;
  height: 16px;
}

.toggle-switch-xs .toggle-slider:before {
  height: 12px;
  width: 12px;
  left: 2px;
  bottom: 2px;
}

.toggle-switch-xs input:checked + .toggle-slider:before {
  transform: translateX(12px);
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
