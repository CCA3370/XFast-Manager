<template>
  <div
    class="absolute inset-0 z-20 bg-white/95 dark:bg-gray-900/95 backdrop-blur-md rounded-2xl flex flex-col p-4 transition-colors duration-300 overflow-hidden"
  >
    <!-- Header: Title + Total Progress -->
    <div class="flex-shrink-0 space-y-3 mb-4">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-bold text-gray-900 dark:text-white flex items-center gap-2">
          <!-- Installing spinner -->
          <svg
            v-if="!isComplete"
            class="w-5 h-5 text-emerald-500 animate-spin"
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
            ></circle>
            <path
              class="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
          <!-- Completion icon -->
          <div
            v-else-if="completionStatus === 'success'"
            class="w-5 h-5 rounded-full bg-emerald-500 flex items-center justify-center"
          >
            <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M5 13l4 4L19 7"
              ></path>
            </svg>
          </div>
          <div
            v-else-if="completionStatus === 'partial'"
            class="w-5 h-5 rounded-full bg-yellow-500 flex items-center justify-center"
          >
            <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M12 9v2m0 4h.01"
              ></path>
            </svg>
          </div>
          <div v-else class="w-5 h-5 rounded-full bg-red-500 flex items-center justify-center">
            <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M6 18L18 6M6 6l12 12"
              ></path>
            </svg>
          </div>
          <AnimatedText>{{ headerTitle }}</AnimatedText>
        </h3>
      </div>

      <!-- Total Progress Bar -->
      <div class="relative">
        <div class="h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-150 ease-out"
            :class="getProgressBarClass()"
            :style="progressBarStyle"
          ></div>
        </div>
        <!-- Animated switch between percentage and completion count -->
        <div class="absolute right-0 -top-6 h-4 overflow-hidden">
          <transition name="slide-up" mode="out-in">
            <span
              v-if="!isComplete"
              key="percentage"
              class="block text-xs font-medium tabular-nums"
              :class="getProgressTextClass()"
            >
              {{ displayPercentage }}%
            </span>
            <span
              v-else
              key="completed"
              class="block text-xs font-medium tabular-nums"
              :class="getCompletionCountClass()"
            >
              {{ completedCount }}/{{ tasks.length }} {{ $t('home.completed') || 'completed' }}
            </span>
          </transition>
        </div>
      </div>
    </div>

    <!-- Task List -->
    <div class="flex-1 min-h-0 overflow-y-auto pr-1 custom-scrollbar" :class="sizeConfig.gap">
      <div
        v-for="(task, index) in tasks"
        :key="task.id"
        class="task-item rounded-lg transition-all duration-300"
        :class="[
          sizeConfig.padding,
          getTaskItemClass(index),
          { 'cursor-pointer': isTaskClickable(index) },
        ]"
        @click="handleTaskClick(index)"
      >
        <div class="flex items-center gap-2.5">
          <!-- Task Type Icon -->
          <div
            class="rounded-lg flex items-center justify-center flex-shrink-0 shadow-sm"
            :class="[sizeConfig.iconSize, getIconBgClass(task.type)]"
          >
            <!-- Aircraft - Plane icon from Lucide -->
            <svg
              v-if="task.type === AddonType.Aircraft"
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M17.8 19.2 16 11l3.5-3.5C21 6 21.5 4 21 3c-1-.5-3 0-4.5 1.5L13 8 4.8 6.2c-.5-.1-.9.1-1.1.5l-.3.5c-.2.5-.1 1 .3 1.3L9 12l-2 3H4l-1 1 3 2 2 3 1-1v-3l3-2 3.5 5.3c.3.4.8.5 1.3.3l.5-.2c.4-.3.6-.7.5-1.2z"
              />
            </svg>
            <!-- Scenery - Globe icon from Lucide -->
            <svg
              v-else-if="task.type === AddonType.Scenery"
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
              <path d="M2 12h20" />
            </svg>
            <!-- Scenery Library - Layers icon from Lucide -->
            <svg
              v-else-if="task.type === AddonType.SceneryLibrary"
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z"
              />
              <path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12" />
              <path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17" />
            </svg>
            <!-- Plugin - Zap icon from Lucide -->
            <svg
              v-else-if="task.type === AddonType.Plugin"
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"
              />
            </svg>
            <!-- Navdata - Map icon from Lucide -->
            <svg
              v-else-if="task.type === AddonType.Navdata"
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"
              />
              <path d="M15 5.764v15" />
              <path d="M9 3.236v15" />
            </svg>
            <!-- Livery - Palette icon from Lucide -->
            <svg
              v-else-if="task.type === AddonType.Livery"
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z"
              />
              <circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
              <circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
              <circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
              <circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
            </svg>
            <!-- Default - Box icon -->
            <svg
              v-else
              class="text-white"
              :class="sizeConfig.svgSize"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
            </svg>
          </div>

          <!-- Task Info -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-1.5 min-w-0">
              <span
                class="font-semibold text-gray-900 dark:text-white truncate leading-tight min-w-0"
                :class="sizeConfig.nameSize"
              >
                {{ task.displayName }}
              </span>
              <!-- Version number (if available) -->
              <span
                v-if="task.newVersionInfo?.version"
                class="text-gray-400 dark:text-gray-500 flex-shrink-0"
                :class="sizeConfig.typeSize"
              >
                v{{ task.newVersionInfo.version }}
              </span>
              <!-- Click hint for failed tasks -->
              <span
                v-if="isTaskFailed(index) && isComplete"
                class="text-xs text-red-400 dark:text-red-500 flex-shrink-0"
              >
                ({{ $t('completion.viewFailedTasks') || 'View Details' }})
              </span>
            </div>
            <div class="flex items-center gap-1 -mt-0.5">
              <span class="text-gray-500 dark:text-gray-400" :class="sizeConfig.typeSize">
                {{ getTaskTypeLabel(task.type) }}
              </span>
              <span class="text-gray-300 dark:text-gray-600">•</span>
              <span :class="[sizeConfig.typeSize, getStatusTextClass(index)]">
                {{ getTaskStatusText(index) }}
              </span>
              <!-- Per-task size info -->
              <template v-if="getTaskSizeText(task, index)">
                <span class="text-gray-300 dark:text-gray-600">•</span>
                <span
                  class="text-gray-400 dark:text-gray-500 tabular-nums"
                  :class="sizeConfig.typeSize"
                >
                  {{ getTaskSizeText(task, index) }}
                </span>
              </template>
            </div>

            <!-- Current Task Progress Bar (only during installation) -->
            <div v-if="!isComplete && isTaskActive(index)" class="mt-1.5">
              <div class="h-1 bg-gray-200 dark:bg-gray-600 rounded-full overflow-hidden">
                <div
                  class="h-full rounded-full transition-all duration-150 ease-out bg-gradient-to-r from-blue-500 to-blue-400"
                  :style="{ width: getActiveTaskPercentage(index).toFixed(1) + '%' }"
                ></div>
              </div>
              <div class="flex justify-end items-center mt-0.5">
                <span class="text-[10px] font-medium tabular-nums text-blue-600 dark:text-blue-400">
                  {{ getActiveTaskPercentage(index).toFixed(1) }}%
                </span>
              </div>
            </div>
          </div>

          <!-- Status Icon -->
          <div class="flex-shrink-0">
            <!-- Completed: Green Checkmark -->
            <div
              v-if="isTaskCompleted(index)"
              class="rounded-full bg-emerald-500 flex items-center justify-center shadow-sm"
              :class="sizeConfig.statusIconSize"
            >
              <svg
                class="text-white"
                :class="sizeConfig.statusInnerSize"
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
            <!-- Failed: Red X -->
            <div
              v-else-if="isTaskFailed(index)"
              class="rounded-full bg-red-500 flex items-center justify-center shadow-sm"
              :class="sizeConfig.statusIconSize"
            >
              <svg
                class="text-white"
                :class="sizeConfig.statusInnerSize"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="3"
                  d="M6 18L18 6M6 6l12 12"
                ></path>
              </svg>
            </div>
            <!-- Current: Spinning (only during installation) -->
            <div
              v-else-if="!isComplete && isTaskActive(index)"
              class="rounded-full bg-blue-500 flex items-center justify-center shadow-sm"
              :class="sizeConfig.statusIconSize"
            >
              <svg
                class="text-white animate-spin"
                :class="sizeConfig.statusInnerSize"
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
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
            </div>
            <!-- Waiting: Empty Circle -->
            <div
              v-else
              class="rounded-full border-2 border-gray-300 dark:border-gray-600 flex items-center justify-center"
              :class="sizeConfig.statusIconSize"
            >
              <div
                class="rounded-full bg-gray-300 dark:bg-gray-600"
                :class="sizeConfig.waitingDotSize"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer: Action Buttons -->
    <div
      v-if="isComplete"
      class="flex-shrink-0 flex justify-center gap-3 pt-4 border-t border-gray-200 dark:border-gray-700/50 mt-4"
    >
      <!-- Confirm Button (after completion) -->
      <button
        class="px-6 py-2 text-sm font-medium rounded-lg transition-all duration-200 flex items-center gap-2 shadow-sm hover:shadow-md"
        :class="getConfirmButtonClass()"
        @click="$emit('confirm')"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M5 13l4 4L19 7"
          ></path>
        </svg>
        <AnimatedText>{{ $t('completion.confirm') }}</AnimatedText>
      </button>
    </div>

    <!-- Error Detail Modal -->
    <Teleport to="body">
      <transition name="fade">
        <div
          v-if="showErrorModal"
          class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
          @click.self="showErrorModal = false"
        >
          <div
            class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-md w-full p-6 space-y-4 animate-scale-in"
          >
            <!-- Header -->
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-10 h-10 rounded-full bg-red-100 dark:bg-red-500/20 flex items-center justify-center flex-shrink-0"
              >
                <svg
                  class="w-5 h-5 text-red-500"
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
              </div>
              <div class="min-w-0">
                <h4 class="font-semibold text-gray-900 dark:text-white truncate">
                  {{ errorModalTask?.displayName }}
                </h4>
                <p class="text-sm text-gray-500 dark:text-gray-400">
                  {{ $t('home.failed') || 'Failed' }}
                </p>
              </div>
            </div>

            <!-- Error Message -->
            <div
              class="bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-lg p-3"
            >
              <p class="text-sm text-red-700 dark:text-red-300 break-words select-text">
                {{ errorModalMessage }}
              </p>
            </div>

            <!-- Buttons -->
            <div class="flex justify-end gap-2">
              <!-- Copy Button -->
              <button
                class="px-4 py-2 text-sm font-medium rounded-lg transition-colors flex items-center gap-2"
                :class="
                  copied
                    ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-500/20 dark:text-emerald-400'
                    : 'bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200'
                "
                @click="copyErrorMessage"
              >
                <svg
                  v-if="!copied"
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                  ></path>
                </svg>
                <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 13l4 4L19 7"
                  ></path>
                </svg>
                {{ copied ? $t('copy.copied') : $t('copy.copy') }}
              </button>
              <!-- Close Button -->
              <button
                class="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-sm font-medium rounded-lg transition-colors"
                @click="showErrorModal = false"
              >
                {{ $t('common.close') }}
              </button>
            </div>
          </div>
        </div>
      </transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AnimatedText from '@/components/AnimatedText.vue'
import { AddonType, type InstallTask, type InstallResult, type ParallelTaskProgress } from '@/types'

const { t } = useI18n()

// Size level based on task count
type SizeLevel = 'large' | 'medium' | 'compact'

interface SizeConfig {
  iconSize: string
  svgSize: string
  statusIconSize: string
  statusInnerSize: string
  padding: string
  gap: string
  nameSize: string
  typeSize: string
  waitingDotSize: string
}

const props = defineProps<{
  // Original props
  percentage: string
  taskName: string
  processedMB: string
  totalMB: string
  taskProgress: string
  // New props for task list display
  tasks: InstallTask[]
  currentTaskIndex: number
  // Completion state
  isComplete: boolean
  installResult: InstallResult | null
  // Current task percentage from backend
  currentTaskPercentage: number
  // Current task bytes
  currentTaskProcessedMB: string
  currentTaskTotalMB: string
  // Parallel mode props
  activeTasks?: ParallelTaskProgress[]
  completedTaskCount?: number
  completedTaskIds?: string[]
}>()

defineEmits<{
  confirm: []
}>()

// Computed: size level based on task count
const sizeLevel = computed<SizeLevel>(() => {
  const count = props.tasks.length
  if (count <= 3) return 'large'
  if (count <= 6) return 'medium'
  return 'compact'
})

// Computed: size configuration based on size level
const sizeConfig = computed<SizeConfig>(() => {
  const configs: Record<SizeLevel, SizeConfig> = {
    large: {
      iconSize: 'w-12 h-12', // 48px
      svgSize: 'w-7 h-7', // 28px
      statusIconSize: 'w-8 h-8', // 32px
      statusInnerSize: 'w-4 h-4',
      padding: 'p-3',
      gap: 'space-y-2',
      nameSize: 'text-sm',
      typeSize: 'text-xs',
      waitingDotSize: 'w-3 h-3',
    },
    medium: {
      iconSize: 'w-9 h-9', // 36px (current)
      svgSize: 'w-5 h-5', // 20px
      statusIconSize: 'w-6 h-6', // 24px
      statusInnerSize: 'w-3.5 h-3.5',
      padding: 'p-2',
      gap: 'space-y-1.5',
      nameSize: 'text-sm',
      typeSize: 'text-xs',
      waitingDotSize: 'w-2 h-2',
    },
    compact: {
      iconSize: 'w-7 h-7', // 28px
      svgSize: 'w-4 h-4', // 16px
      statusIconSize: 'w-5 h-5', // 20px
      statusInnerSize: 'w-3 h-3',
      padding: 'p-1.5',
      gap: 'space-y-1',
      nameSize: 'text-sm',
      typeSize: 'text-xs',
      waitingDotSize: 'w-1.5 h-1.5',
    },
  }
  return configs[sizeLevel.value]
})

// Error modal state
const showErrorModal = ref(false)
const errorModalTask = ref<InstallTask | null>(null)
const errorModalMessage = ref('')
const copied = ref(false)
let copyResetTimer: ReturnType<typeof setTimeout> | null = null

// Copy error message to clipboard
async function copyErrorMessage() {
  try {
    const textToCopy = `${errorModalTask.value?.displayName || 'Task'}: ${errorModalMessage.value}`
    await navigator.clipboard.writeText(textToCopy)
    copied.value = true

    // Reset copied state after 2 seconds
    if (copyResetTimer) {
      clearTimeout(copyResetTimer)
    }
    copyResetTimer = setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy:', err)
  }
}

// Computed: display percentage (100% when complete)
const displayPercentage = computed(() => {
  if (props.isComplete) return '100.0'
  return props.percentage
})

// Computed: progress bar style with full-width gradient effect
// The gradient should span the entire progress bar, so as progress increases,
// the head moves through the color gradient (blue -> green)
const progressBarStyle = computed(() => {
  const percent = parseFloat(displayPercentage.value) || 0.1
  // Scale background size so the gradient spans the full container width
  // When progress is 50%, background-size should be 200% to make gradient appear full width
  const bgSize = Math.min(10000, (100 / percent) * 100)
  return {
    width: displayPercentage.value + '%',
    backgroundSize: bgSize + '% 100%',
  }
})

// Computed: completion status
const completionStatus = computed<'success' | 'partial' | 'failed'>(() => {
  if (!props.installResult) return 'success'
  if (props.installResult.failedTasks === 0) return 'success'
  if (props.installResult.successfulTasks > 0) return 'partial'
  return 'failed'
})

// Computed: header title
const headerTitle = computed(() => {
  if (!props.isComplete) {
    return t('home.installing')
  }
  if (completionStatus.value === 'success') {
    return t('completion.allSuccess')
  }
  if (completionStatus.value === 'partial') {
    return t('completion.partialSuccess')
  }
  return t('completion.allFailed')
})

// Parallel mode detection
const isParallel = computed(() => props.activeTasks !== undefined && props.activeTasks !== null)

// [DEBUG] Watch parallel mode props for detailed logging
watch(
  () => ({
    activeTasks: props.activeTasks,
    completedTaskIds: props.completedTaskIds,
    completedTaskCount: props.completedTaskCount,
    percentage: props.percentage,
    currentTaskIndex: props.currentTaskIndex,
  }),
  (val) => {
    if (!props.tasks.length) return
    const mode = isParallel.value ? 'PARALLEL' : 'SERIAL'
    console.log(`[OVERLAY ${mode}] percentage=${val.percentage}, currentTaskIndex=${val.currentTaskIndex}, completedTaskCount=${val.completedTaskCount}`)
    if (isParallel.value) {
      console.log(`[OVERLAY PARALLEL] activeTasks(${val.activeTasks?.length ?? 0}):`, val.activeTasks?.map(t => `[${t.taskIndex}]${t.taskName} phase=${t.phase} ${t.percentage}%`))
      console.log(`[OVERLAY PARALLEL] completedTaskIds:`, val.completedTaskIds)
    }
    // Log per-task status
    props.tasks.forEach((task, i) => {
      const active = isTaskActive(i)
      const completed = isTaskCompleted(i)
      const failed = isTaskFailed(i)
      const status = getTaskStatusText(i)
      const pct = getActiveTaskPercentage(i)
      console.log(`[OVERLAY ${mode}] task[${i}] id=${task.id} name=${task.displayName} => active=${active} completed=${completed} failed=${failed} status="${status}" pct=${pct}%`)
    })
  },
  { deep: true }
)

// Check if a task is currently active (installing) in parallel mode
function isTaskActive(index: number): boolean {
  if (isParallel.value) {
    return props.activeTasks?.some((at) => at.taskIndex === index) ?? false
  }
  return index === props.currentTaskIndex
}

// Get the active task progress for a specific task index
function getActiveTaskPercentage(index: number): number {
  if (isParallel.value) {
    const at = props.activeTasks?.find((t) => t.taskIndex === index)
    return at?.percentage ?? 0
  }
  return props.currentTaskPercentage
}

// Computed: count of completed tasks
const completedCount = computed(() => {
  if (props.isComplete && props.installResult) {
    return props.installResult.successfulTasks
  }
  if (isParallel.value) {
    return props.completedTaskCount ?? 0
  }
  // During installation, tasks before currentTaskIndex are considered in progress or completed
  return props.currentTaskIndex
})

// Get task result from installResult
function getTaskResult(taskId: string) {
  if (!props.installResult) return null
  return props.installResult.taskResults.find((r) => r.taskId === taskId)
}

// Check if a task is completed successfully
function isTaskCompleted(index: number): boolean {
  const task = props.tasks[index]
  if (!task) return false

  if (props.isComplete && props.installResult) {
    const result = getTaskResult(task.id)
    return result?.success === true
  }

  if (isParallel.value) {
    // In parallel mode: check if task ID is in completedTaskIds
    return props.completedTaskIds?.includes(task.id) ?? false
  }

  // During installation: tasks before current index are assumed complete
  return index < props.currentTaskIndex
}

// Check if a task failed
function isTaskFailed(index: number): boolean {
  const task = props.tasks[index]
  if (!task) return false

  if (props.isComplete && props.installResult) {
    const result = getTaskResult(task.id)
    return result?.success === false
  }

  return false
}

// Check if task is clickable (only failed tasks when complete)
function isTaskClickable(index: number): boolean {
  return props.isComplete && isTaskFailed(index)
}

// Handle task click
function handleTaskClick(index: number) {
  if (!isTaskClickable(index)) return

  const task = props.tasks[index]
  if (!task) return

  const result = getTaskResult(task.id)
  if (!result) return

  errorModalTask.value = task
  errorModalMessage.value = result.errorMessage || t('completion.unknownError')
  copied.value = false // Reset copied state for new modal
  showErrorModal.value = true
}

// Get task item container class based on status
function getTaskItemClass(index: number): string {
  if (isTaskCompleted(index)) {
    return 'bg-emerald-50/80 dark:bg-emerald-500/10 border border-emerald-200/50 dark:border-emerald-500/20'
  }
  if (isTaskFailed(index)) {
    return 'bg-red-50/80 dark:bg-red-500/10 border border-red-200/50 dark:border-red-500/20'
  }
  if (!props.isComplete && isTaskActive(index)) {
    return 'bg-blue-50/80 dark:bg-blue-500/10 border-2 border-blue-400/50 dark:border-blue-500/40 shadow-sm'
  }
  // Waiting
  return 'bg-gray-50/50 dark:bg-gray-800/30 border border-gray-200/50 dark:border-gray-700/30 opacity-60'
}

// Get task status text
function getTaskStatusText(index: number): string {
  if (isTaskCompleted(index)) {
    return t('home.installed') || 'Installed'
  }
  if (isTaskFailed(index)) {
    return t('home.failed') || 'Failed'
  }
  if (!props.isComplete && isTaskActive(index)) {
    // Check if the task is in verifying phase
    if (isParallel.value) {
      const at = props.activeTasks?.find((t) => t.taskIndex === index)
      if (at?.phase === 'verifying') {
        return t('home.verifying') || 'Verifying...'
      }
    }
    return t('home.installingNow') || 'Installing...'
  }
  return t('home.waiting') || 'Waiting'
}

// Get status text color class
function getStatusTextClass(index: number): string {
  if (isTaskCompleted(index)) {
    return 'text-emerald-600 dark:text-emerald-400 font-medium'
  }
  if (isTaskFailed(index)) {
    return 'text-red-600 dark:text-red-400 font-medium'
  }
  if (!props.isComplete && isTaskActive(index)) {
    return 'text-blue-600 dark:text-blue-400 font-medium'
  }
  return 'text-gray-400 dark:text-gray-500'
}

// Get progress bar class based on completion status
function getProgressBarClass(): string {
  if (!props.isComplete) {
    return 'progress-bar-gradient progress-bar-glow'
  }
  if (completionStatus.value === 'success') {
    return 'progress-bar-gradient'
  }
  if (completionStatus.value === 'partial') {
    return 'bg-gradient-to-r from-yellow-500 to-orange-400'
  }
  return 'bg-gradient-to-r from-red-500 to-red-400'
}

// Get progress text class
function getProgressTextClass(): string {
  if (!props.isComplete) {
    return 'text-blue-600 dark:text-blue-400'
  }
  if (completionStatus.value === 'success') {
    return 'text-emerald-600 dark:text-emerald-400'
  }
  if (completionStatus.value === 'partial') {
    return 'text-yellow-600 dark:text-yellow-400'
  }
  return 'text-red-600 dark:text-red-400'
}

// Get completion count class
function getCompletionCountClass(): string {
  if (completionStatus.value === 'success') {
    return 'text-emerald-600 dark:text-emerald-400'
  }
  if (completionStatus.value === 'partial') {
    return 'text-yellow-600 dark:text-yellow-400'
  }
  return 'text-red-600 dark:text-red-400'
}

// Get confirm button class based on completion status
function getConfirmButtonClass(): string {
  if (completionStatus.value === 'success') {
    return 'bg-emerald-500 hover:bg-emerald-600 text-white'
  }
  if (completionStatus.value === 'partial') {
    return 'bg-yellow-500 hover:bg-yellow-600 text-white'
  }
  return 'bg-red-500 hover:bg-red-600 text-white'
}

// Get icon background class based on addon type
function getIconBgClass(type: AddonType): string {
  const classes: Record<AddonType, string> = {
    [AddonType.Aircraft]: 'bg-gradient-to-br from-blue-500 to-blue-600',
    [AddonType.Scenery]: 'bg-gradient-to-br from-emerald-500 to-emerald-600',
    [AddonType.SceneryLibrary]: 'bg-gradient-to-br from-teal-500 to-teal-600',
    [AddonType.Plugin]: 'bg-gradient-to-br from-purple-500 to-purple-600',
    [AddonType.Navdata]: 'bg-gradient-to-br from-amber-500 to-amber-600',
    [AddonType.Livery]: 'bg-gradient-to-br from-pink-500 to-pink-600',
    [AddonType.LuaScript]: 'bg-gradient-to-br from-cyan-500 to-cyan-600',
  }
  return classes[type] || 'bg-gradient-to-br from-gray-500 to-gray-600'
}

// Get per-task size text for display
function getTaskSizeText(task: InstallTask, index: number): string {
  // For the active task, show dynamic progress bytes
  if (!props.isComplete && isTaskActive(index)) {
    if (isParallel.value) {
      // In parallel mode, we don't have per-task byte data
      const pct = getActiveTaskPercentage(index)
      if (pct > 0) return `${pct.toFixed(0)}%`
      return ''
    }
    // Only show if we have meaningful data (totalMB > 0)
    if (props.currentTaskTotalMB !== '0.0') {
      return `${props.currentTaskProcessedMB} / ${props.currentTaskTotalMB} MB`
    }
    return ''
  }
  // For completed/waiting/failed tasks, show estimated size if available
  if (task.estimatedSize && task.estimatedSize > 0) {
    return `${(task.estimatedSize / 1048576).toFixed(1)} MB`
  }
  return ''
}

// Get localized label for addon type
function getTaskTypeLabel(type: AddonType): string {
  const labels: Record<AddonType, string> = {
    [AddonType.Aircraft]: t('addonType.Aircraft') || 'Aircraft',
    [AddonType.Scenery]: t('addonType.Scenery') || 'Scenery',
    [AddonType.SceneryLibrary]: t('addonType.SceneryLibrary') || 'Scenery Library',
    [AddonType.Plugin]: t('addonType.Plugin') || 'Plugin',
    [AddonType.Navdata]: t('addonType.Navdata') || 'Navdata',
    [AddonType.Livery]: t('addonType.Livery') || 'Livery',
    [AddonType.LuaScript]: t('addonType.LuaScript') || 'Lua Script',
  }
  return labels[type] || type
}
</script>

<style scoped>
/* Progress bar gradient - blue to green like the cover image */
/* The gradient spans the full bar width; background-size is controlled via inline style */
.progress-bar-gradient {
  background: linear-gradient(90deg, #3b82f6 0%, #10b981 100%);
  background-position: left;
}

.progress-bar-glow {
  box-shadow:
    0 0 12px rgba(59, 130, 246, 0.5),
    0 0 20px rgba(16, 185, 129, 0.3);
}

.tabular-nums {
  font-variant-numeric: tabular-nums;
}

/* Custom scrollbar */
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(156, 163, 175, 0.3);
  border-radius: 3px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(156, 163, 175, 0.5);
}

.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(75, 85, 99, 0.5);
}

.dark .custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(75, 85, 99, 0.7);
}

/* Task item hover effect */
.task-item {
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease;
}

.task-item:not(.opacity-60):hover {
  transform: translateX(2px);
}

/* Modal animations */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* Slide-up transition for percentage → completion count */
.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-up-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.slide-up-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

@keyframes scale-in {
  0% {
    opacity: 0;
    transform: scale(0.9);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

.animate-scale-in {
  animation: scale-in 0.2s ease-out;
}
</style>
