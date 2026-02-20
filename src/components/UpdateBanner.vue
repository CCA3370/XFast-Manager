<template>
  <transition name="slide-down">
    <div
      v-if="visible"
      class="flex-shrink-0 bg-green-50/90 dark:bg-green-500/10 backdrop-blur-md border border-green-200 dark:border-green-500/20 rounded-xl p-3 flex items-center space-x-3 shadow-lg shadow-green-500/5 transition-colors duration-300"
    >
      <!-- 左侧：图标 -->
      <div class="p-2 bg-green-100 dark:bg-green-500/20 rounded-lg flex-shrink-0">
        <svg class="w-5 h-5 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
        </svg>
      </div>

      <!-- 中间：文本内容 -->
      <div class="flex-1 min-w-0">
        <p class="text-sm font-medium text-green-800 dark:text-green-100 truncate">
          <AnimatedText>{{ $t('update.newVersionAvailable') }} v{{ updateInfo?.latestVersion }}</AnimatedText>
        </p>
        <p class="text-xs text-green-700 dark:text-green-200/70 truncate">
          <AnimatedText>{{ $t('update.lastChecked') }}: {{ formatPublishedDate(updateInfo?.publishedAt) }}</AnimatedText>
        </p>
      </div>

      <!-- 右侧：按钮组 -->
      <div class="flex items-center space-x-2">
        <!-- 查看详情按钮 -->
        <button
          class="flex-shrink-0 inline-flex items-center px-3 py-1.5 bg-green-200/50 dark:bg-green-500/20 hover:bg-green-200 dark:hover:bg-green-500/30 text-green-800 dark:text-green-200 text-xs font-medium rounded-lg transition-colors duration-200 border border-green-300 dark:border-green-500/30"
          @click="handleViewDetails"
        >
          <AnimatedText>{{ $t('update.viewDetails') }}</AnimatedText>
          <svg class="w-3 h-3 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
          </svg>
        </button>

        <!-- 关闭按钮 -->
        <button
          class="p-1.5 hover:bg-green-200/50 dark:hover:bg-green-500/20 rounded-lg transition-colors duration-200 text-green-600 dark:text-green-400"
          :title="$t('update.dismiss')"
          @click="handleDismiss"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import AnimatedText from './AnimatedText.vue'
import type { UpdateInfo } from '@/types'

interface Props {
  visible: boolean
  updateInfo: UpdateInfo | null
}

defineProps<Props>()

const { t } = useI18n()

const emit = defineEmits<{
  viewRelease: []
  dismiss: []
}>()

function handleViewDetails() {
  emit('viewRelease')
}

function handleDismiss() {
  emit('dismiss')
}

function formatPublishedDate(dateString?: string): string {
  if (!dateString) return ''

  try {
    const date = new Date(dateString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

    if (diffDays === 0) {
      return t('update.today')
    } else if (diffDays === 1) {
      return t('update.yesterday')
    } else if (diffDays < 7) {
      return t('update.daysAgo', { days: diffDays })
    } else {
      return date.toLocaleDateString()
    }
  } catch {
    return ''
  }
}
</script>

<style scoped>
.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.3s ease-out;
}

.slide-down-enter-from {
  opacity: 0;
  transform: translateY(-20px);
}

.slide-down-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
