<template>
  <Teleport to="body">
    <transition name="modal">
      <div class="modal-overlay animate-fade-in" @click="$emit('close')">
        <div class="modal-content animate-scale-in" @click.stop>
          <!-- Header -->
          <div class="modal-header mb-3 flex-shrink-0">
            <div class="flex items-center space-x-2">
              <div class="w-8 h-8 bg-gradient-to-r from-green-500 to-blue-600 rounded-lg flex items-center justify-center">
                <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
              </div>
              <div>
                <h3 class="text-lg font-bold bg-gradient-to-r from-green-400 to-blue-400 bg-clip-text text-transparent"><AnimatedText>{{ $t('modal.confirmInstallation') }}</AnimatedText></h3>
                <p class="text-gray-400 text-xs"><AnimatedText>{{ $t('modal.installToXplane') }}</AnimatedText></p>
              </div>
            </div>
          </div>

          <!-- Global Overwrite Toggle (only show if any conflicts exist) -->
          <div v-if="store.hasConflicts" class="mb-2 p-2.5 bg-yellow-500/10 border border-yellow-500/20 rounded-lg flex-shrink-0">
            <div class="flex items-center justify-between">
              <div class="flex items-center space-x-2">
                <svg class="w-4 h-4 text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                </svg>
                <div>
                  <span class="font-medium text-sm text-yellow-100"><AnimatedText>{{ $t('modal.overwriteAll') }}</AnimatedText></span>
                  <p class="text-xs text-yellow-200/70"><AnimatedText>{{ $t('modal.overwriteAllDesc') }}</AnimatedText></p>
                </div>
              </div>
              <label class="toggle-switch toggle-switch-sm">
                <input
                  type="checkbox"
                  :checked="globalOverwrite"
                  @change="toggleGlobalOverwrite"
                >
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <!-- Size Warning Banner (only show if any size warnings exist) -->
          <div v-if="store.hasSizeWarnings" class="mb-2 p-2.5 bg-red-500/10 border border-red-500/20 rounded-lg flex-shrink-0">
            <div class="flex items-start space-x-2">
              <svg class="w-4 h-4 text-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
              </svg>
              <div class="flex-1">
                <span class="font-medium text-sm text-red-100"><AnimatedText>{{ $t('modal.sizeWarningTitle') }}</AnimatedText></span>
                <p class="text-xs text-red-200/70"><AnimatedText>{{ $t('modal.sizeWarningDesc') }}</AnimatedText></p>
              </div>
            </div>
          </div>

          <!-- Tasks List -->
          <div class="tasks-list mb-3 flex-1 min-h-0 overflow-y-auto custom-scrollbar">
            <div
              v-for="task in store.currentTasks"
              :key="task.id"
              class="task-item bg-gradient-to-r from-gray-800/50 to-gray-900/50 backdrop-blur-sm border border-white/10 rounded-lg p-3 mb-2 hover:border-blue-400/30 transition-all duration-200"
              :class="{ 'opacity-50': !store.getTaskEnabled(task.id) }"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1">
                    <span class="type-badge" :class="getTypeBadgeClass(task.type)">
                      {{ task.type }}
                    </span>
                    <span class="font-medium text-white text-sm truncate">{{ task.displayName }}</span>
                  </div>
                  <div class="flex items-center space-x-1.5 text-xs text-gray-400">
                    <svg class="w-3 h-3 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z"></path>
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5a2 2 0 012-2h4a2 2 0 012 2v2H8V5z"></path>
                    </svg>
                    <span class="truncate"><AnimatedText>{{ $t('modal.targetPath') }}</AnimatedText>: {{ task.targetPath }}</span>
                  </div>
                  <!-- Conflict warning with individual toggle -->
                  <div v-if="task.conflictExists" class="mt-1.5">
                    <div class="flex items-center justify-between">
                      <div class="flex items-center space-x-1.5 text-xs text-yellow-400">
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                        </svg>
                        <span><AnimatedText>{{ $t('modal.folderExists') }}</AnimatedText></span>
                      </div>
                      <!-- Individual overwrite toggle -->
                      <label class="toggle-switch toggle-switch-xs">
                        <input
                          type="checkbox"
                          :checked="store.getTaskOverwrite(task.id)"
                          @change="toggleTaskOverwrite(task.id)"
                        >
                        <span class="toggle-slider"></span>
                      </label>
                    </div>
                    <!-- Show overwrite info if enabled -->
                    <div v-if="store.getTaskOverwrite(task.id)" class="mt-0.5 text-xs text-orange-400">
                      <span v-if="task.type === 'Aircraft'"><AnimatedText>{{ $t('modal.overwriteAircraftNote') }}</AnimatedText></span>
                      <span v-else><AnimatedText>{{ $t('modal.overwriteNote') }}</AnimatedText></span>
                    </div>
                  </div>
                  <!-- Navdata cycle comparison (only for Navdata with conflict and existing info) -->
                  <div v-if="task.type === 'Navdata' && task.conflictExists && task.existingNavdataInfo"
                       class="mt-1.5 p-2 bg-blue-500/10 border border-blue-500/20 rounded">
                    <div class="flex items-center space-x-2 text-xs">
                      <svg class="w-3.5 h-3.5 text-blue-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                      <span class="text-blue-200">
                        <AnimatedText>{{ $t('modal.existingCycle') }}</AnimatedText>:
                        <span class="font-medium">{{ formatNavdataCycle(task.existingNavdataInfo) }}</span>
                        <span class="mx-1.5">→</span>
                        <AnimatedText>{{ $t('modal.newCycle') }}</AnimatedText>:
                        <span class="font-medium">{{ formatNavdataCycle(task.newNavdataInfo) }}</span>
                      </span>
                    </div>
                  </div>
                  <!-- Size warning with confirmation checkbox -->
                  <div v-if="task.sizeWarning" class="mt-1.5 p-2 bg-red-500/10 border border-red-500/20 rounded">
                    <div class="flex items-start space-x-2">
                      <svg class="w-3.5 h-3.5 text-red-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                      </svg>
                      <div class="flex-1 min-w-0">
                        <p class="text-xs text-red-300">{{ parseSizeWarning(task.sizeWarning).message }}</p>
                        <label class="flex items-center space-x-1.5 mt-1.5 cursor-pointer">
                          <input
                            type="checkbox"
                            :checked="store.getTaskSizeConfirmed(task.id)"
                            @change="toggleTaskSizeConfirm(task.id)"
                            class="w-3 h-3 rounded border-red-500/50 bg-red-500/10 text-red-500 focus:ring-red-500/50"
                          >
                          <span class="text-xs text-red-200"><AnimatedText>{{ $t('modal.confirmTrustArchive') }}</AnimatedText></span>
                        </label>
                      </div>
                    </div>
                  </div>
                </div>
                <!-- Task Enable Toggle (right side) -->
                <div class="flex-shrink-0">
                  <label class="toggle-switch toggle-switch-sm">
                    <input
                      type="checkbox"
                      :checked="store.getTaskEnabled(task.id)"
                      @change="toggleTaskEnabled(task.id)"
                    >
                    <span class="toggle-slider"></span>
                  </label>
                </div>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex justify-end gap-2 flex-shrink-0 pt-2">
            <button
              @click="$emit('close')"
              class="px-4 py-2 bg-gradient-to-r from-gray-600 to-gray-700 hover:from-gray-700 hover:to-gray-800 rounded-lg transition-all duration-200 hover:scale-105 text-sm font-medium flex items-center space-x-1.5"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
              <span><AnimatedText>{{ $t('common.cancel') }}</AnimatedText></span>
            </button>
            <button
              @click="$emit('confirm')"
              :disabled="installDisabled"
              :class="[
                'px-4 py-2 rounded-lg transition-all duration-200 text-sm font-medium flex items-center space-x-1.5',
                installDisabled
                  ? 'bg-gray-600 cursor-not-allowed opacity-50'
                  : 'bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700 hover:scale-105 hover:shadow-lg hover:shadow-green-500/25'
              ]"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { AddonType, NavdataInfo } from '@/types'
import AnimatedText from '@/components/AnimatedText.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const store = useAppStore()

defineEmits(['close', 'confirm'])

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

// Global overwrite state (local to modal, syncs with store)
const globalOverwrite = ref(false)

// Toggle global overwrite for all conflicting tasks
function toggleGlobalOverwrite() {
  globalOverwrite.value = !globalOverwrite.value
  store.setGlobalOverwrite(globalOverwrite.value)
}

// Toggle individual task overwrite
function toggleTaskOverwrite(taskId: string) {
  const currentValue = store.getTaskOverwrite(taskId)
  store.setTaskOverwrite(taskId, !currentValue)

  // Update global toggle state based on individual toggles
  updateGlobalToggleState()
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

// Update global toggle to reflect individual states
function updateGlobalToggleState() {
  const conflictingTasks = store.currentTasks.filter(t => t.conflictExists)
  if (conflictingTasks.length === 0) {
    globalOverwrite.value = false
    return
  }
  const allOverwrite = conflictingTasks.every(t => store.getTaskOverwrite(t.id))
  globalOverwrite.value = allOverwrite
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
      return 'bg-orange-600'
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
  background: linear-gradient(135deg, rgba(31, 41, 55, 0.95), rgba(17, 24, 39, 0.95));
  border-radius: 1rem;
  padding: 1.25rem;
  max-width: 560px;
  width: 90%;
  border: 1px solid rgba(59, 130, 246, 0.3);
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.8);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Custom scrollbar */
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
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
  transition: all 0.3s ease;
}

.task-item:hover {
  transform: translateY(-2px);
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
}

/* Type badges */
.type-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem 0.5rem;
  border-radius: 0.375rem;
  font-size: 0.65rem;
  font-weight: 600;
  text-transform: uppercase;
  backdrop-filter: blur(10px);
  flex-shrink: 0;
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

.type-badge.bg-orange-600 {
  background: linear-gradient(135deg, rgba(249, 115, 22, 0.8), rgba(251, 146, 60, 0.8));
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
</style>
