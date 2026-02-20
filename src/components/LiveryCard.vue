<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import ConfirmModal from '@/components/ConfirmModal.vue'
import type { LiveryInfo } from '@/types'
import { useContextMenu } from '@/composables/useContextMenu'
import type { ContextMenuItem } from '@/composables/useContextMenu'

const props = defineProps<{
  livery: LiveryInfo
}>()

const emit = defineEmits<{
  (e: 'delete', folderName: string): void
  (e: 'preview', iconSrc: string): void
  (e: 'open-folder', folderName: string): void
}>()

const { t } = useI18n()
const showDeleteConfirm = ref(false)
const isDeleting = ref(false)
const imageError = ref(false)
const contextMenu = useContextMenu()

const iconSrc = props.livery.iconPath ? convertFileSrc(props.livery.iconPath) : null

function handleImageError() {
  imageError.value = true
}

function handleDeleteConfirm() {
  isDeleting.value = true
  emit('delete', props.livery.folderName)
}

function handleContextMenu(event: MouseEvent) {
  const menuItems: ContextMenuItem[] = []

  if (iconSrc && !imageError.value) {
    menuItems.push({
      id: 'preview',
      label: t('contextMenu.previewImage'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"/></svg>'
    })
  }

  menuItems.push({
    id: 'open-folder',
    label: t('contextMenu.openFolder'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/></svg>',
    dividerAfter: true
  })

  menuItems.push({
    id: 'delete',
    label: t('common.delete'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>',
    danger: true
  })

  contextMenu.show(event, menuItems, (id: string) => {
    switch (id) {
      case 'preview':
        if (iconSrc) emit('preview', iconSrc)
        break
      case 'open-folder':
        emit('open-folder', props.livery.folderName)
        break
      case 'delete':
        showDeleteConfirm.value = true
        break
    }
  })
}
</script>

<template>
  <div class="group bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden transition-all hover:shadow-md" @contextmenu.prevent="handleContextMenu">
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
        class="flex-shrink-0 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-red-100 dark:hover:bg-red-900/30 transition-all"
        :title="t('common.delete')"
        @click.stop="showDeleteConfirm = true"
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
