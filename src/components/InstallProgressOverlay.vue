<template>
  <div class="absolute inset-0 z-20 bg-white/90 dark:bg-gray-900/80 backdrop-blur-md rounded-2xl flex items-center justify-center p-6 transition-colors duration-300">
    <div class="w-full max-w-md space-y-4 text-center">
      <!-- Installing State with Progress -->
      <div class="space-y-3">
        <!-- Circular Progress -->
        <div class="relative w-20 h-20 mx-auto">
          <svg class="w-full h-full -rotate-90" viewBox="0 0 80 80">
            <!-- SVG filter for glow effect -->
            <defs>
              <filter id="progress-glow" x="-50%" y="-50%" width="200%" height="200%">
                <feGaussianBlur stdDeviation="2" result="blur"/>
                <feMerge>
                  <feMergeNode in="blur"/>
                  <feMergeNode in="SourceGraphic"/>
                </feMerge>
              </filter>
            </defs>
            <circle cx="40" cy="40" r="36" stroke-width="5" fill="none"
              class="text-emerald-500/20 dark:text-emerald-500/30" stroke="currentColor"/>
            <circle cx="40" cy="40" r="36" stroke-width="5" fill="none"
              class="text-emerald-500 dark:text-emerald-400 progress-circle" stroke="currentColor"
              :stroke-dasharray="CIRCLE_CIRCUMFERENCE"
              :stroke-dashoffset="progressOffset"
              stroke-linecap="round"
              filter="url(#progress-glow)"/>
          </svg>
          <span class="absolute inset-0 flex items-center justify-center text-lg font-bold text-emerald-600 dark:text-emerald-400 progress-text">
            {{ percentage }}%
          </span>
        </div>

        <!-- Task Info -->
        <div class="text-center">
          <h3 class="text-xl font-bold text-gray-900 dark:text-white">
            <AnimatedText>{{ $t('home.installing') }}</AnimatedText>
          </h3>
          <p class="text-sm text-gray-600 dark:text-gray-300 mt-1 transition-opacity duration-150">
            {{ taskName }}
          </p>
        </div>

        <!-- Linear Progress Bar -->
        <div class="w-full max-w-xs mx-auto">
          <div class="relative h-1.5 my-2">
            <!-- Background track -->
            <div class="absolute inset-0 bg-gray-200 dark:bg-gray-700 rounded-full"></div>
            <!-- Glow effect layer (behind the bar) -->
            <div class="absolute inset-y-0 left-0 rounded-full progress-bar-glow"
              :style="{ width: percentage + '%' }"/>
            <!-- Progress fill -->
            <div class="absolute inset-y-0 left-0 bg-emerald-500 dark:bg-emerald-400 rounded-full progress-bar"
              :style="{ width: percentage + '%' }"/>
          </div>
          <div class="flex justify-between text-xs text-gray-400 dark:text-gray-500 mt-1">
            <span class="progress-text">{{ processedMB }} MB</span>
            <span class="progress-text">{{ totalMB }} MB</span>
          </div>
        </div>

        <!-- Task Progress -->
        <p class="text-xs text-center text-gray-500 dark:text-gray-400 progress-text">
          {{ $t('home.taskProgress', { current: taskProgress }) }}
        </p>

        <!-- Task Control Buttons -->
        <div class="flex justify-center gap-3 mt-4">
          <!-- Skip Button -->
          <button
            @click="$emit('skip')"
            class="px-4 py-2 bg-yellow-500/10 hover:bg-yellow-500/20 dark:bg-yellow-500/20 dark:hover:bg-yellow-500/30 text-yellow-700 dark:text-yellow-400 text-sm font-medium rounded-lg transition-all duration-200 border border-yellow-500/30 hover:border-yellow-500/50 flex items-center gap-2 shadow-sm hover:shadow-md"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7"></path>
            </svg>
            <AnimatedText>{{ $t('taskControl.skipTask') }}</AnimatedText>
          </button>

          <!-- Cancel Button -->
          <button
            @click="$emit('cancel')"
            class="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 dark:bg-red-500/20 dark:hover:bg-red-500/30 text-red-700 dark:text-red-400 text-sm font-medium rounded-lg transition-all duration-200 border border-red-500/30 hover:border-red-500/50 flex items-center gap-2 shadow-sm hover:shadow-md"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
            <AnimatedText>{{ $t('taskControl.cancelAll') }}</AnimatedText>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AnimatedText from '@/components/AnimatedText.vue'

// Circle circumference for progress calculation (2 * PI * radius)
const CIRCLE_CIRCUMFERENCE = 226

const props = defineProps<{
  percentage: string
  taskName: string
  processedMB: string
  totalMB: string
  taskProgress: string
}>()

defineEmits<{
  skip: []
  cancel: []
}>()

const progressOffset = computed(() => {
  return CIRCLE_CIRCUMFERENCE - CIRCLE_CIRCUMFERENCE * (parseFloat(props.percentage) / 100)
})
</script>

<style scoped>
.progress-circle {
  transition: stroke-dashoffset 0.3s ease-out;
}

.progress-bar {
  transition: width 0.3s ease-out;
}

.progress-bar-glow {
  background: linear-gradient(90deg, transparent, rgba(16, 185, 129, 0.4));
  filter: blur(4px);
  transition: width 0.3s ease-out;
}

.progress-text {
  font-variant-numeric: tabular-nums;
}
</style>
