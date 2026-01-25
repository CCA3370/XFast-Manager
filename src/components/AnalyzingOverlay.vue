<template>
  <div class="absolute inset-0 z-20 bg-white/90 dark:bg-gray-900/80 backdrop-blur-md rounded-2xl flex items-center justify-center p-6 transition-colors duration-300">
    <div class="w-full max-w-md space-y-4 text-center">
      <!-- Analyzing State -->
      <div class="space-y-4">
        <div class="relative w-20 h-20 mx-auto">
          <div class="absolute inset-0 border-4 border-blue-200 dark:border-blue-500/30 rounded-full"></div>
          <div class="absolute inset-0 border-4 border-blue-500 rounded-full border-t-transparent animate-spin"></div>
          <div class="absolute inset-0 flex items-center justify-center">
            <svg class="w-8 h-8 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
            </svg>
          </div>
        </div>
        <div>
          <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
            <AnimatedText>{{ $t('home.analyzing') }}</AnimatedText>
          </h3>
          <p class="text-gray-500 dark:text-gray-400 mt-2">
            <AnimatedText>{{ $t('home.pleaseWait') }}</AnimatedText>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AnimatedText from '@/components/AnimatedText.vue'
</script>
