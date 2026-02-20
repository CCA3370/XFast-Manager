<template>
  <div class="completion-view">
    <!-- Icon placeholder (no icon rendered here, animation becomes the icon) -->
    <div class="icon-container h-20 mb-6"></div>

    <!-- Title -->
    <h2 class="text-xl font-bold text-center text-gray-900 dark:text-white mb-3">
      <AnimatedText>{{ statusTitle }}</AnimatedText>
    </h2>

    <!-- Statistics -->
    <div class="stats flex justify-center gap-6 mb-4">
      <div v-if="!allFailed" class="stat-item text-center">
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

    <!-- Failed Tasks Summary with View Details Button -->
    <div v-if="failedTasks.length > 0" class="mt-4 mb-3">
      <button
        class="w-full py-3 px-4 bg-gradient-to-r from-red-500/20 to-red-600/20 hover:from-red-500/30 hover:to-red-600/30 border border-red-500/30 hover:border-red-500/50 text-red-600 dark:text-red-400 font-medium rounded-lg transition-all duration-200 flex items-center justify-between group"
        @click="showFailedTasksModal = true"
      >
        <div class="flex items-center space-x-2">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
          </svg>
          <span><AnimatedText>{{ $t('completion.viewFailedTasks') }}</AnimatedText></span>
        </div>
        <svg class="w-5 h-5 transform group-hover:translate-x-1 transition-transform" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
        </svg>
      </button>
    </div>

    <!-- Failed Tasks Modal -->
    <FailedTasksModal
      :visible="showFailedTasksModal"
      :failed-tasks="failedTasks"
      @close="showFailedTasksModal = false"
    />

    <!-- Confirm Button -->
    <button
      class="confirm-button w-full py-2.5 px-4 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white font-medium rounded-lg transition-all duration-200 shadow-md hover:shadow-lg mt-2"
      @click="$emit('confirm')"
    >
      <AnimatedText>{{ $t('completion.confirm') }}</AnimatedText>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { InstallResult } from '@/types'
import AnimatedText from './AnimatedText.vue'
import FailedTasksModal from './FailedTasksModal.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  result: InstallResult
}>()

defineEmits<{
  confirm: []
}>()

const showFailedTasksModal = ref(false)

const allSuccess = computed(() => props.result.failedTasks === 0)
const allFailed = computed(() => props.result.successfulTasks === 0)

const statusTitle = computed(() => {
  if (allSuccess.value) {
    return t('completion.allSuccess')
  } else if (allFailed.value) {
    return t('completion.allFailed')
  } else {
    return t('completion.partialSuccess')
  }
})

const failedTasks = computed(() =>
  props.result.taskResults.filter(task => !task.success)
)
</script>

<style scoped>
.completion-view {
  padding: 1rem;
}
</style>
