<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import ConfirmModal from '@/components/ConfirmModal.vue'
import type { LiveryInfo } from '@/types'

const props = defineProps<{
  livery: LiveryInfo
}>()

const emit = defineEmits<{
  (e: 'delete', folderName: string): void
  (e: 'preview', iconSrc: string): void
}>()

const { t } = useI18n()
const showDeleteConfirm = ref(false)
const isDeleting = ref(false)
const imageError = ref(false)

const iconSrc = props.livery.iconPath ? convertFileSrc(props.livery.iconPath) : null

function handleImageError() {
  imageError.value = true
}

function handleDeleteConfirm() {
  isDeleting.value = true
  emit('delete', props.livery.folderName)
}
</script>

<template>
  <div class="group bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden transition-all hover:shadow-md">
    <!-- Image area -->
    <div
      class="aspect-[16/10] bg-gray-100 dark:bg-gray-900 relative overflow-hidden"
      :class="{ 'cursor-pointer': iconSrc && !imageError }"
      @click="iconSrc && !imageError && emit('preview', iconSrc)"
    >
      <img
        v-if="iconSrc && !imageError"
        :src="iconSrc"
        :alt="livery.displayName"
        class="w-full h-full object-cover"
        @error="handleImageError"
      />
      <!-- Placeholder when no image -->
      <div v-else class="w-full h-full flex items-center justify-center text-gray-300 dark:text-gray-600">
        <svg class="w-12 h-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
      </div>
    </div>

    <!-- Footer -->
    <div class="px-2.5 py-2 flex items-center gap-2">
      <span class="flex-1 min-w-0 text-xs font-medium text-gray-900 dark:text-gray-100 truncate" :title="livery.folderName">
        {{ livery.displayName }}
      </span>
      <button
        @click.stop="showDeleteConfirm = true"
        class="flex-shrink-0 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-red-100 dark:hover:bg-red-900/30 transition-all"
        :title="t('common.delete')"
      >
        <svg class="w-3.5 h-3.5 text-gray-400 hover:text-red-500 dark:text-gray-500 dark:hover:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      </button>
    </div>
  </div>

  <!-- Delete Confirmation Modal -->
  <ConfirmModal
    v-model:show="showDeleteConfirm"
    :title="t('livery.deleteConfirmTitle')"
    :message="t('livery.deleteConfirmMessage')"
    :item-name="livery.folderName"
    :confirm-text="t('common.delete')"
    :loading-text="t('common.deleting')"
    :is-loading="isDeleting"
    variant="danger"
    @confirm="handleDeleteConfirm"
  />
</template>
