<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useLockStore } from '@/stores/lock'
import ToggleSwitch from '@/components/ToggleSwitch.vue'
import type { SceneryManagerEntry } from '@/types'
import { SceneryCategory, getErrorMessage } from '@/types'
import { useContextMenu } from '@/composables/useContextMenu'
import type { ContextMenuItem } from '@/composables/useContextMenu'

const props = withDefaults(
  defineProps<{
    entry: SceneryManagerEntry
    index: number
    totalCount: number
    disableReorder?: boolean
    disableMoveDown?: boolean
  }>(),
  {
    disableReorder: false,
    disableMoveDown: false,
  },
)

const emit = defineEmits<{
  (e: 'toggle-enabled', folderName: string): void
  (e: 'move-up', folderName: string): void
  (e: 'move-down', folderName: string): void
  (e: 'show-missing-libs', entry: SceneryManagerEntry): void
  (e: 'show-duplicate-tiles', entry: SceneryManagerEntry): void
  (e: 'show-delete-confirm', entry: SceneryManagerEntry): void
  (e: 'update', folderName: string): void
}>()

const { t } = useI18n()
const appStore = useAppStore()
const modalStore = useModalStore()
const lockStore = useLockStore()
const contextMenu = useContextMenu()

// Category display config
const categoryConfig = computed(() => {
  const cat =
    props.entry.category === SceneryCategory.FixedHighPriority &&
    props.entry.originalCategory &&
    props.entry.originalCategory !== SceneryCategory.FixedHighPriority
      ? props.entry.originalCategory
      : props.entry.category

  switch (cat) {
    case SceneryCategory.FixedHighPriority:
      return {
        label: 'SAM',
        color: 'text-purple-700 dark:text-purple-300',
        bgColor: 'bg-purple-100 dark:bg-purple-900/30',
      }
    case SceneryCategory.Airport:
      return {
        label: t('sceneryManager.categoryAirport'),
        color: 'text-blue-700 dark:text-blue-300',
        bgColor: 'bg-blue-100 dark:bg-blue-900/30',
      }
    case SceneryCategory.DefaultAirport:
      return {
        label: t('sceneryManager.categoryDefaultAirport'),
        color: 'text-gray-600 dark:text-gray-400',
        bgColor: 'bg-gray-100 dark:bg-gray-800/50',
      }
    case SceneryCategory.Library:
      return {
        label: t('sceneryManager.categoryLibrary'),
        color: 'text-green-700 dark:text-green-300',
        bgColor: 'bg-green-100 dark:bg-green-900/30',
      }
    case SceneryCategory.Overlay:
      return {
        label: t('sceneryManager.categoryOverlay'),
        color: 'text-yellow-700 dark:text-yellow-300',
        bgColor: 'bg-yellow-100 dark:bg-yellow-900/30',
      }
    case SceneryCategory.AirportMesh:
      return {
        label: t('sceneryManager.categoryAirportMesh'),
        color: 'text-cyan-700 dark:text-cyan-300',
        bgColor: 'bg-cyan-100 dark:bg-cyan-900/30',
      }
    case SceneryCategory.Mesh:
      return {
        label: t('sceneryManager.categoryMesh'),
        color: 'text-red-700 dark:text-red-300',
        bgColor: 'bg-red-100 dark:bg-red-900/30',
      }
    case SceneryCategory.Other:
      return {
        label: t('sceneryManager.categoryOther'),
        color: 'text-gray-600 dark:text-gray-400',
        bgColor: 'bg-gray-100 dark:bg-gray-800/50',
      }
    case SceneryCategory.Unrecognized:
      return {
        label: t('sceneryManager.categoryUnrecognized'),
        color: 'text-red-600 dark:text-red-400',
        bgColor: 'bg-red-100 dark:bg-red-900/40',
      }
    default:
      return {
        label: t('sceneryManager.categoryOther'),
        color: 'text-gray-600 dark:text-gray-400',
        bgColor: 'bg-gray-100 dark:bg-gray-800/50',
      }
  }
})

const hasMissingDeps = computed(() => props.entry.missingLibraries.length > 0)
const hasDuplicateTiles = computed(
  () => props.entry.duplicateTiles && props.entry.duplicateTiles.length > 0,
)
const hasDuplicateAirports = computed(
  () => props.entry.duplicateAirports && props.entry.duplicateAirports.length > 0,
)
const hasDuplicates = computed(() => hasDuplicateTiles.value || hasDuplicateAirports.value)
const duplicatesCount = computed(() => {
  const all = new Set<string>()
  if (props.entry.duplicateTiles) {
    for (const p of props.entry.duplicateTiles) all.add(p)
  }
  if (props.entry.duplicateAirports) {
    for (const p of props.entry.duplicateAirports) all.add(p)
  }
  return all.size
})
const isFirst = computed(() => props.index === 0)
const isLast = computed(() => props.index === props.totalCount - 1)

// Lock state
const isItemLocked = computed(() => lockStore.isLocked('scenery', props.entry.folderName))

function handleToggleLock() {
  lockStore.toggleLock('scenery', props.entry.folderName)
}

async function handleDoubleClick() {
  if (!appStore.xplanePath) {
    modalStore.showError(t('sceneryManager.noXplanePath'))
    return
  }

  try {
    await invoke('open_scenery_folder', {
      xplanePath: appStore.xplanePath,
      folderName: props.entry.folderName,
    })
  } catch (error) {
    modalStore.showError(t('sceneryManager.openFolderFailed') + ': ' + getErrorMessage(error))
  }
}

function handleClick(event: Event) {
  // Don't trigger if clicking on interactive elements
  const target = event.target as HTMLElement
  if (target.closest('button') || target.closest('.drag-handle')) {
    return
  }

  // If has missing libraries, show modal on single click
  if (hasMissingDeps.value) {
    event.stopPropagation()
    emit('show-missing-libs', props.entry)
  } else if (hasDuplicates.value) {
    // If has duplicates (but no missing deps), show that modal
    event.stopPropagation()
    emit('show-duplicate-tiles', props.entry)
  }
}

function handleContextMenu(event: MouseEvent) {
  const menuItems: ContextMenuItem[] = []

  menuItems.push({
    id: 'toggle-enabled',
    label: props.entry.enabled ? t('contextMenu.disable') : t('contextMenu.enable'),
    icon: props.entry.enabled
      ? '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>'
      : '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>',
  })

  menuItems.push({
    id: 'open-folder',
    label: t('contextMenu.openFolder'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/></svg>',
  })

  menuItems.push({
    id: 'update',
    label: t('management.startUpdate'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>',
  })

  if (!props.disableReorder) {
    menuItems.push({
      id: 'move-up',
      label: t('sceneryManager.moveUp'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/></svg>',
      disabled: isFirst.value,
    })
    menuItems.push({
      id: 'move-down',
      label: t('sceneryManager.moveDown'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>',
      disabled: isLast.value || props.disableMoveDown,
    })
  }

  if (hasMissingDeps.value) {
    menuItems.push({
      id: 'show-missing-libs',
      label: t('sceneryManager.missingLibraries'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>',
    })
  }

  if (hasDuplicates.value) {
    menuItems.push({
      id: 'show-duplicate-tiles',
      label: t('sceneryManager.duplicates'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"/></svg>',
    })
  }

  // Divider before lock/delete
  if (menuItems.length > 0) {
    menuItems[menuItems.length - 1].dividerAfter = true
  }

  menuItems.push({
    id: 'toggle-lock',
    label: isItemLocked.value ? t('management.unlock') : t('management.lock'),
    icon: isItemLocked.value
      ? '<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"/></svg>'
      : '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z"/></svg>',
  })

  menuItems.push({
    id: 'delete',
    label: t('sceneryManager.delete'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>',
    danger: true,
  })

  contextMenu.show(event, menuItems, (id: string) => {
    switch (id) {
      case 'toggle-enabled':
        emit('toggle-enabled', props.entry.folderName)
        break
      case 'open-folder':
        handleDoubleClick()
        break
      case 'update':
        emit('update', props.entry.folderName)
        break
      case 'move-up':
        emit('move-up', props.entry.folderName)
        break
      case 'move-down':
        emit('move-down', props.entry.folderName)
        break
      case 'show-missing-libs':
        emit('show-missing-libs', props.entry)
        break
      case 'show-duplicate-tiles':
        emit('show-duplicate-tiles', props.entry)
        break
      case 'toggle-lock':
        handleToggleLock()
        break
      case 'delete':
        emit('show-delete-confirm', props.entry)
        break
    }
  })
}
</script>

<template>
  <div
    class="flex items-center gap-2 p-2 rounded-lg border transition-all hover:bg-gray-50 dark:hover:bg-gray-700/30"
    :class="[
      entry.category === 'Unrecognized'
        ? entry.enabled
          ? 'bg-red-50/50 dark:bg-red-900/10 border-red-300 dark:border-red-700'
          : 'bg-red-50/30 dark:bg-red-900/5 border-red-300 dark:border-red-700'
        : entry.enabled
          ? 'bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700'
          : 'bg-gray-50 dark:bg-gray-900/50 border-gray-200/50 dark:border-gray-700/50 opacity-60',
      hasMissingDeps || hasDuplicates ? 'cursor-pointer' : '',
    ]"
    @click="handleClick"
    @dblclick="handleDoubleClick"
    @contextmenu.prevent="handleContextMenu"
  >
    <!-- Drag handle -->
    <div
      v-if="!props.disableReorder"
      class="cursor-grab active:cursor-grabbing text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 drag-handle select-none"
    >
      <svg
        class="w-4 h-4 pointer-events-none"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8h16M4 16h16" />
      </svg>
    </div>

    <!-- Enable/Disable toggle -->
    <ToggleSwitch
      :model-value="entry.enabled"
      active-class="bg-blue-500"
      inactive-class="bg-gray-300 dark:bg-gray-600"
      :aria-label="entry.enabled ? t('contextMenu.disable') : t('contextMenu.enable')"
      @update:model-value="emit('toggle-enabled', entry.folderName)"
    />

    <!-- Folder name -->
    <div class="flex-1 min-w-0">
      <div
        class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate"
        :title="entry.folderName"
      >
        {{ entry.folderName }}
      </div>
    </div>

    <!-- Missing dependencies warning (before category badge) -->
    <div
      v-if="hasMissingDeps"
      class="flex-shrink-0 flex items-center gap-0.5 px-1.5 py-0.5 rounded text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 cursor-pointer"
      :title="t('sceneryManager.clickToViewMissingLibs')"
      @click.stop="emit('show-missing-libs', entry)"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        />
      </svg>
      <span class="text-[10px] font-medium">{{ entry.missingLibraries.length }}</span>
    </div>

    <!-- Duplicate warning badge -->
    <div
      v-if="hasDuplicates"
      class="flex-shrink-0 flex items-center gap-0.5 px-1.5 py-0.5 rounded text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-900/20 cursor-pointer"
      :title="t('sceneryManager.clickToViewDuplicates')"
      @click.stop="emit('show-duplicate-tiles', entry)"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"
        />
      </svg>
      <span class="text-[10px] font-medium">{{ duplicatesCount }}</span>
    </div>

    <!-- Category badge -->
    <span
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium"
      :class="[categoryConfig.color, categoryConfig.bgColor]"
    >
      {{ categoryConfig.label }}
    </span>

    <!-- Geo info badge (continent) -->
    <span
      v-if="entry.continent"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700"
      :title="entry.continent"
    >
      {{ entry.continent }}
    </span>

    <!-- Move buttons -->
    <div v-if="!props.disableReorder" class="flex-shrink-0 flex gap-0.5">
      <button
        :disabled="isFirst"
        class="p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        :title="t('sceneryManager.moveUp')"
        @click="emit('move-up', entry.folderName)"
      >
        <svg
          class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
        </svg>
      </button>
      <button
        :disabled="isLast || props.disableMoveDown"
        class="p-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        :title="t('sceneryManager.moveDown')"
        @click="emit('move-down', entry.folderName)"
      >
        <svg
          class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </button>
    </div>

    <!-- Lock button -->
    <button
      class="flex-shrink-0 p-0.5 rounded transition-colors"
      :class="
        isItemLocked
          ? 'text-amber-500 dark:text-amber-400 hover:bg-amber-100 dark:hover:bg-amber-900/30'
          : 'text-gray-400 dark:text-gray-500 hover:text-amber-500 dark:hover:text-amber-400 hover:bg-amber-100 dark:hover:bg-amber-900/30'
      "
      :title="isItemLocked ? t('management.unlock') : t('management.lock')"
      @click.stop="handleToggleLock"
    >
      <!-- Locked icon (solid) -->
      <svg v-if="isItemLocked" class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
        <path
          d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"
        />
      </svg>
      <!-- Unlocked icon (outline) -->
      <svg v-else class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z"
        />
      </svg>
    </button>

    <!-- Delete button -->
    <button
      class="flex-shrink-0 p-0.5 rounded hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
      :title="t('sceneryManager.delete')"
      @click.stop="emit('show-delete-confirm', entry)"
    >
      <svg
        class="w-3.5 h-3.5 text-gray-400 hover:text-red-500 dark:text-gray-500 dark:hover:text-red-400"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
        />
      </svg>
    </button>
  </div>
</template>
