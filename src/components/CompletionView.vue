<template>
  <div class="completion-view space-y-3">
    <!-- Icon -->
    <div class="icon-container">
      <div v-if="allSuccess" class="w-16 h-16 mx-auto bg-gradient-to-r from-green-500 to-emerald-600 rounded-full flex items-center justify-center">
        <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
        </svg>
      </div>
      <div v-else class="w-16 h-16 mx-auto bg-gradient-to-r from-yellow-500 to-orange-600 rounded-full flex items-center justify-center">
        <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
        </svg>
      </div>
    </div>

    <!-- Title -->
    <h2 class="text-xl font-bold text-center text-gray-900 dark:text-white">
      <AnimatedText>{{ allSuccess ? $t('completion.allSuccess') : $t('completion.partialSuccess') }}</AnimatedText>
    </h2>

    <!-- Statistics -->
    <div class="stats flex justify-center gap-6">
      <div class="stat-item text-center">
        <div class="text-2xl font-bold text-green-600 dark:text-green-400">
          {{ result.successfulTasks }}
        </div>
        <div class="text-xs text-gray-600 dark:text-gray-400">
          <AnimatedText>{{ $t('completion.successful') }}</AnimatedText>
        </div>
      </div>
      <div v-if="result.failedTasks > 0" class="stat-item text-center">
        <div class="text-2xl font-bold text-red-600 dark:text-red-400">
          {{ result.failedTasks }}
        </div>
        <div class="text-xs text-gray-600 dark:text-gray-400">
          <AnimatedText>{{ $t('completion.failed') }}</AnimatedText>
        </div>
      </div>
    </div>

    <!-- Failed Tasks List -->
    <div v-if="failedTasks.length > 0" class="failed-tasks mt-4">
      <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
        <AnimatedText>{{ $t('completion.failedTasks') }}</AnimatedText>
      </h3>
      <div class="space-y-1.5 max-h-32 overflow-y-auto custom-scrollbar">
        <div
          v-for="task in failedTasks"
          :key="task.taskId"
          class="failed-item flex items-center gap-2 p-2 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-lg"
        >
          <svg class="w-4 h-4 text-red-500 dark:text-red-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
          <span class="text-sm text-red-700 dark:text-red-200 truncate">{{ task.taskName }}</span>
        </div>
      </div>
    </div>

    <!-- Confirm Button -->
    <button
      @click="$emit('confirm')"
      class="confirm-button w-full py-2.5 px-4 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white font-medium rounded-lg transition-all duration-200 shadow-md hover:shadow-lg"
    >
      <AnimatedText>{{ $t('completion.confirm') }}</AnimatedText>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { InstallResult } from '@/types'
import AnimatedText from './AnimatedText.vue'

const props = defineProps<{
  result: InstallResult
}>()

defineEmits<{
  confirm: []
}>()

const allSuccess = computed(() => props.result.failedTasks === 0)

const failedTasks = computed(() =>
  props.result.taskResults.filter(task => !task.success)
)
</script>

<style scoped>
.completion-view {
  padding: 1rem;
}

.custom-scrollbar {
  scrollbar-width: thin;
  scrollbar-color: rgba(156, 163, 175, 0.5) transparent;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.5);
  border-radius: 3px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.7);
}

.dark .custom-scrollbar {
  scrollbar-color: rgba(75, 85, 99, 0.5) transparent;
}

.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: rgba(75, 85, 99, 0.5);
}

.dark .custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background-color: rgba(75, 85, 99, 0.7);
}
</style>
