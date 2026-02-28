<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type {
  AircraftInfo,
  PluginInfo,
  NavdataManagerInfo,
  NavdataBackupInfo,
  ManagementItemType,
} from '@/types'
import { getNavdataCycleStatus } from '@/utils/airac'
import ConfirmModal from '@/components/ConfirmModal.vue'
import { useLockStore } from '@/stores/lock'
import { useContextMenu } from '@/composables/useContextMenu'
import type { ContextMenuItem } from '@/composables/useContextMenu'

type EntryType = AircraftInfo | PluginInfo | NavdataManagerInfo

const props = withDefaults(
  defineProps<{
    entry: EntryType
    itemType: ManagementItemType
    isToggling?: boolean
    backupInfo?: NavdataBackupInfo | null
    selected?: boolean
    showCheckbox?: boolean
    isProtected?: boolean
  }>(),
  {
    isToggling: false,
    backupInfo: null,
    selected: false,
    showCheckbox: false,
    isProtected: false,
  },
)

const emit = defineEmits<{
  (e: 'toggle-enabled', folderName: string): void
  (e: 'delete', folderName: string): void
  (e: 'open-folder', folderName: string): void
  (e: 'restore-backup', backupInfo: NavdataBackupInfo): void
  (e: 'view-liveries', folderName: string): void
  (e: 'view-scripts', folderName: string): void
  (e: 'toggle-select', folderName: string): void
  (e: 'update', folderName: string): void
}>()

const { t } = useI18n()
const lockStore = useLockStore()
const contextMenu = useContextMenu()

const showDeleteConfirmModal = ref(false)
const isDeleting = ref(false)

// Lock state
const isItemLocked = computed(() => lockStore.isLocked(props.itemType, props.entry.folderName))

function handleToggleLock() {
  lockStore.toggleLock(props.itemType, props.entry.folderName)
}

// Type guards
function isAircraft(entry: EntryType): entry is AircraftInfo {
  return 'acfFile' in entry
}

function isPlugin(entry: EntryType): entry is PluginInfo {
  return 'xplFiles' in entry
}

function isNavdata(entry: EntryType): entry is NavdataManagerInfo {
  return 'providerName' in entry
}

// Display name
const displayName = computed(() => {
  if (isAircraft(props.entry)) {
    return props.entry.displayName
  } else if (isPlugin(props.entry)) {
    return props.entry.displayName
  } else if (isNavdata(props.entry)) {
    return props.entry.providerName
  }
  // This case should never be reached, but satisfies TypeScript
  return (props.entry as { folderName: string }).folderName
})

// Badge info
const badgeInfo = computed(() => {
  if (isAircraft(props.entry) && props.entry.hasLiveries) {
    return {
      text: `${props.entry.liveryCount} ${t('management.liveries')}`,
      color: 'text-blue-700 dark:text-blue-300',
      bgColor: 'bg-blue-100 dark:bg-blue-900/30',
    }
  } else if (isPlugin(props.entry) && props.entry.hasScripts) {
    return {
      text: `${props.entry.scriptCount} ${t('scripts.scriptCount')}`,
      color: 'text-emerald-700 dark:text-emerald-300',
      bgColor: 'bg-emerald-100 dark:bg-emerald-900/30',
    }
  } else if (isPlugin(props.entry)) {
    const platformColors: Record<string, { color: string; bgColor: string }> = {
      win: {
        color: 'text-blue-700 dark:text-blue-300',
        bgColor: 'bg-blue-100 dark:bg-blue-900/30',
      },
      mac: {
        color: 'text-gray-700 dark:text-gray-300',
        bgColor: 'bg-gray-100 dark:bg-gray-800/50',
      },
      lin: {
        color: 'text-orange-700 dark:text-orange-300',
        bgColor: 'bg-orange-100 dark:bg-orange-900/30',
      },
      multi: {
        color: 'text-green-700 dark:text-green-300',
        bgColor: 'bg-green-100 dark:bg-green-900/30',
      },
      unknown: {
        color: 'text-gray-600 dark:text-gray-400',
        bgColor: 'bg-gray-100 dark:bg-gray-800/50',
      },
    }
    const colors = platformColors[props.entry.platform] || platformColors.unknown
    return {
      text: props.entry.platform.toUpperCase(),
      ...colors,
    }
  } else if (isNavdata(props.entry)) {
    const cycleText = props.entry.cycle || props.entry.airac || ''
    if (cycleText) {
      return {
        text: cycleText,
        color: 'text-purple-700 dark:text-purple-300',
        bgColor: 'bg-purple-100 dark:bg-purple-900/30',
      }
    }
  }
  return null
})

// Navdata cycle status
const navdataCycleStatus = computed(() => {
  if (!isNavdata(props.entry)) return null
  const cycleText = props.entry.cycle || props.entry.airac
  return getNavdataCycleStatus(cycleText)
})

// Version info (for aircraft and plugins)
const versionInfo = computed(() => {
  if (isAircraft(props.entry) || isPlugin(props.entry)) {
    return props.entry.version || null
  }
  return null
})

// Update available (for aircraft and plugins)
const updateAvailable = computed(() => {
  if (isAircraft(props.entry) || isPlugin(props.entry)) {
    return props.entry.hasUpdate
  }
  return false
})

const latestVersion = computed(() => {
  if (isAircraft(props.entry) || isPlugin(props.entry)) {
    return props.entry.latestVersion || null
  }
  return null
})

function handleDoubleClick() {
  emit('open-folder', props.entry.folderName)
}

function handleClick() {
  if (isAircraft(props.entry) && props.entry.hasLiveries) {
    emit('view-liveries', props.entry.folderName)
  } else if (isPlugin(props.entry) && props.entry.hasScripts) {
    emit('view-scripts', props.entry.folderName)
  }
}

function handleDeleteConfirm() {
  isDeleting.value = true
  emit('delete', props.entry.folderName)
  // Parent will handle the actual deletion and close modal on success
  setTimeout(() => {
    isDeleting.value = false
    showDeleteConfirmModal.value = false
  }, 500)
}

function handleContextMenu(event: MouseEvent) {
  const menuItems: ContextMenuItem[] = []

  if (!isNavdata(props.entry) && !(props.isProtected && props.entry.enabled)) {
    menuItems.push({
      id: 'toggle-enabled',
      label: props.entry.enabled ? t('contextMenu.disable') : t('contextMenu.enable'),
      icon: props.entry.enabled
        ? '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>'
        : '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>',
    })
  }

  menuItems.push({
    id: 'open-folder',
    label: t('contextMenu.openFolder'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/></svg>',
  })

  if (isAircraft(props.entry) && props.entry.hasLiveries) {
    menuItems.push({
      id: 'view-liveries',
      label: t('contextMenu.viewLiveries'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"/></svg>',
    })
  }

  if (isPlugin(props.entry) && props.entry.hasScripts) {
    menuItems.push({
      id: 'view-scripts',
      label: t('contextMenu.viewScripts'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"/></svg>',
    })
  }

  if (updateAvailable.value) {
    menuItems.push({
      id: 'update',
      label: t('management.startUpdate'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>',
    })
  }

  if (isNavdata(props.entry) && props.backupInfo) {
    menuItems.push({
      id: 'restore-backup',
      label: t('management.restoreBackup'),
      icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>',
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
    label: t('common.delete'),
    icon: '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>',
    danger: true,
  })

  contextMenu.show(event, menuItems, (id: string) => {
    switch (id) {
      case 'toggle-enabled':
        emit('toggle-enabled', props.entry.folderName)
        break
      case 'open-folder':
        emit('open-folder', props.entry.folderName)
        break
      case 'view-liveries':
        emit('view-liveries', props.entry.folderName)
        break
      case 'view-scripts':
        emit('view-scripts', props.entry.folderName)
        break
      case 'update':
        emit('update', props.entry.folderName)
        break
      case 'restore-backup':
        if (props.backupInfo) emit('restore-backup', props.backupInfo)
        break
      case 'toggle-lock':
        handleToggleLock()
        break
      case 'delete':
        showDeleteConfirmModal.value = true
        break
    }
  })
}
</script>

<template>
  <div
    class="flex items-center gap-2 p-2 rounded-lg border transition-all hover:bg-gray-50 dark:hover:bg-gray-700/30"
    :class="[
      isNavdata(entry) || entry.enabled
        ? 'bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700'
        : 'bg-gray-50 dark:bg-gray-900/50 border-gray-200/50 dark:border-gray-700/50 opacity-60',
      (isAircraft(entry) && entry.hasLiveries) || (isPlugin(entry) && entry.hasScripts)
        ? 'cursor-pointer'
        : '',
    ]"
    @click="handleClick"
    @dblclick="handleDoubleClick"
    @contextmenu.prevent="handleContextMenu"
  >
    <!-- Selection checkbox -->
    <button
      v-if="showCheckbox"
      class="flex-shrink-0 w-4 h-4 rounded border-2 transition-all duration-150 flex items-center justify-center"
      :class="
        selected
          ? 'bg-blue-500 border-blue-500'
          : 'border-gray-300 dark:border-gray-500 hover:border-blue-400 dark:hover:border-blue-400'
      "
      @click.stop="emit('toggle-select', entry.folderName)"
    >
      <svg
        v-if="selected"
        class="w-3 h-3 text-white"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="3"
          d="M5 13l4 4L19 7"
        />
      </svg>
    </button>

    <!-- Enable/Disable toggle (not for navdata) -->
    <button
      v-if="!isNavdata(entry)"
      :disabled="isToggling || (isProtected && entry.enabled)"
      class="flex-shrink-0 w-9 h-5 rounded-full relative transition-colors disabled:opacity-70"
      :class="entry.enabled ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'"
      :title="isProtected && entry.enabled ? t('management.protectedAircraft') : undefined"
      @click.stop="emit('toggle-enabled', entry.folderName)"
    >
      <span v-if="isToggling" class="absolute inset-0 flex items-center justify-center">
        <svg class="w-3 h-3 animate-spin text-white" fill="none" viewBox="0 0 24 24">
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
      </span>
      <span
        v-else
        class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
        :class="entry.enabled ? 'left-4.5' : 'left-0.5'"
      />
    </button>

    <!-- Display name -->
    <div class="flex-1 min-w-0">
      <div
        class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate"
        :title="entry.folderName"
      >
        {{ displayName }}
      </div>
    </div>

    <!-- Version info (if available) -->
    <span
      v-if="versionInfo"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium"
      :class="
        updateAvailable
          ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-100 dark:bg-emerald-900/30'
          : 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700'
      "
      :title="updateAvailable ? `${versionInfo} → ${latestVersion}` : versionInfo"
    >
      {{ versionInfo }}
      <template v-if="updateAvailable"> → {{ latestVersion }} </template>
    </span>

    <button
      v-if="updateAvailable"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium text-white bg-emerald-500 hover:bg-emerald-600 transition-colors"
      :title="t('management.startUpdate')"
      @click.stop="emit('update', entry.folderName)"
    >
      {{ t('management.startUpdate') }}
    </button>

    <!-- Badge (liveries count / platform / cycle) -->
    <span
      v-if="badgeInfo"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium"
      :class="[badgeInfo.color, badgeInfo.bgColor]"
    >
      {{ badgeInfo.text }}
    </span>

    <!-- Navdata cycle status -->
    <span
      v-if="navdataCycleStatus === 'current'"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900/30"
    >
      {{ t('management.currentCycle') }}
    </span>
    <span
      v-else-if="navdataCycleStatus === 'outdated'"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900/30"
    >
      {{ t('management.outdatedCycle') }}
    </span>

    <!-- Restore backup button (navdata only, when backup exists) -->
    <button
      v-if="isNavdata(entry) && backupInfo"
      class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium text-amber-700 dark:text-amber-300 bg-amber-100 dark:bg-amber-900/30 hover:bg-amber-200 dark:hover:bg-amber-800/40 transition-colors"
      :title="`${t('management.restoreBackup')}: ${backupInfo.verification.cycle || backupInfo.verification.airac || ''}`"
      @click.stop="emit('restore-backup', backupInfo)"
    >
      {{ t('management.restoreBackup') }}
    </button>

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
      :title="t('common.delete')"
      @click.stop="showDeleteConfirmModal = true"
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

  <!-- Delete Confirmation Modal -->
  <ConfirmModal
    v-model:show="showDeleteConfirmModal"
    :title="t('management.deleteConfirmTitle')"
    :message="t('management.deleteConfirmMessage')"
    :item-name="entry.folderName"
    :confirm-text="t('common.delete')"
    :loading-text="t('common.deleting')"
    :is-loading="isDeleting"
    variant="danger"
    @confirm="handleDeleteConfirm"
  />
</template>
