<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import ConfirmModal from '@/components/ConfirmModal.vue'
import type { LuaScriptInfo } from '@/types'

const { t } = useI18n()
const router = useRouter()
const appStore = useAppStore()
const modalStore = useModalStore()
const toastStore = useToastStore()

const scripts = ref<LuaScriptInfo[]>([])
const isLoading = ref(true)
const loadError = ref(false)
const togglingItems = ref<Set<string>>(new Set())

// Delete state
const showDeleteConfirmModal = ref(false)
const deleteTarget = ref<LuaScriptInfo | null>(null)
const isDeleting = ref(false)

const scriptCount = computed(() => scripts.value.length)

async function loadScripts() {
  if (!appStore.xplanePath) return

  isLoading.value = true
  loadError.value = false
  try {
    scripts.value = await invoke<LuaScriptInfo[]>('get_lua_scripts', {
      xplanePath: appStore.xplanePath
    })
  } catch (e) {
    loadError.value = true
    modalStore.showError(t('scripts.loadFailed') + ': ' + String(e))
  } finally {
    isLoading.value = false
  }
}

async function handleToggle(fileName: string) {
  if (!appStore.xplanePath || togglingItems.value.has(fileName)) return

  togglingItems.value.add(fileName)
  try {
    const newEnabled = await invoke<boolean>('toggle_lua_script', {
      xplanePath: appStore.xplanePath,
      fileName
    })

    // Update local state
    const script = scripts.value.find(s => s.fileName === fileName)
    if (script) {
      const oldExt = script.enabled ? '.lua' : '.xfml'
      const newExt = newEnabled ? '.lua' : '.xfml'
      script.enabled = newEnabled
      script.fileName = script.fileName.replace(oldExt, newExt)
    }
  } catch (e) {
    modalStore.showError(t('scripts.toggleFailed') + ': ' + String(e))
  } finally {
    togglingItems.value.delete(fileName)
  }
}

function confirmDelete(script: LuaScriptInfo) {
  deleteTarget.value = script
  showDeleteConfirmModal.value = true
}

async function handleDeleteConfirm() {
  if (!appStore.xplanePath || !deleteTarget.value) return

  isDeleting.value = true
  try {
    await invoke('delete_lua_script', {
      xplanePath: appStore.xplanePath,
      fileName: deleteTarget.value.fileName
    })

    // Remove from local array
    scripts.value = scripts.value.filter(s => s.fileName !== deleteTarget.value!.fileName)
    toastStore.success(t('scripts.deleteSuccess'))

    // Navigate back if no scripts remaining
    if (scripts.value.length === 0) {
      router.push('/management?tab=plugin')
    }
  } catch (e) {
    modalStore.showError(t('scripts.deleteFailed') + ': ' + String(e))
  } finally {
    isDeleting.value = false
    showDeleteConfirmModal.value = false
    deleteTarget.value = null
  }
}

function goBack() {
  router.push('/management?tab=plugin')
}

onMounted(() => {
  loadScripts()
})
</script>

<template>
  <div class="scripts-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Header -->
    <div class="mb-4 flex-shrink-0 flex items-center gap-3">
      <button
        @click="goBack"
        class="p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
        :title="t('onboarding.back')"
      >
        <svg class="w-5 h-5 text-gray-600 dark:text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>
      <div class="flex-1 min-w-0">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 truncate">
          {{ t('scripts.title') }}
        </h2>
      </div>
      <span
        v-if="!isLoading"
        class="flex-shrink-0 px-2 py-0.5 rounded text-xs font-medium text-emerald-700 dark:text-emerald-300 bg-emerald-100 dark:bg-emerald-900/30"
      >
        {{ scriptCount }} {{ t('scripts.scriptCount') }}
      </span>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      <!-- Loading state -->
      <div v-if="isLoading" class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>

      <!-- Error state -->
      <div v-else-if="loadError" class="text-center py-12">
        <p class="text-gray-600 dark:text-gray-400">{{ t('scripts.loadFailed') }}</p>
      </div>

      <!-- Empty state -->
      <div v-else-if="scripts.length === 0" class="text-center py-12">
        <p class="text-gray-600 dark:text-gray-400">{{ t('scripts.noScripts') }}</p>
      </div>

      <!-- Script list -->
      <div v-else class="space-y-1.5 px-1">
        <div
          v-for="script in scripts"
          :key="script.fileName"
          class="flex items-center gap-3 p-2 rounded-lg border transition-all hover:bg-gray-50 dark:hover:bg-gray-700/30"
          :class="script.enabled
            ? 'bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700'
            : 'bg-gray-50 dark:bg-gray-900/50 border-gray-200/50 dark:border-gray-700/50 opacity-60'"
        >
          <!-- Toggle switch -->
          <button
            @click="handleToggle(script.fileName)"
            :disabled="togglingItems.has(script.fileName)"
            class="flex-shrink-0 w-9 h-5 rounded-full relative transition-colors disabled:opacity-70"
            :class="script.enabled ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'"
          >
            <span
              v-if="togglingItems.has(script.fileName)"
              class="absolute inset-0 flex items-center justify-center"
            >
              <svg class="w-3 h-3 animate-spin text-white" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
            </span>
            <span
              v-else
              class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
              :class="script.enabled ? 'left-4.5' : 'left-0.5'"
            />
          </button>

          <!-- Script display name -->
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate" :title="script.fileName">
              {{ script.displayName }}
            </div>
          </div>

          <!-- Status text -->
          <span
            class="flex-shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium"
            :class="script.enabled
              ? 'text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900/30'
              : 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800/50'"
          >
            {{ script.enabled ? t('scripts.enabled') : t('scripts.disabled') }}
          </span>

          <!-- Delete button -->
          <button
            @click.stop="confirmDelete(script)"
            class="flex-shrink-0 p-0.5 rounded hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
            :title="t('common.delete')"
          >
            <svg class="w-3.5 h-3.5 text-gray-400 hover:text-red-500 dark:text-gray-500 dark:hover:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <ConfirmModal
      v-model:show="showDeleteConfirmModal"
      :title="t('scripts.deleteConfirmTitle')"
      :message="t('scripts.deleteConfirmMessage')"
      :item-name="deleteTarget?.fileName || ''"
      :confirm-text="t('common.delete')"
      :loading-text="t('common.deleting')"
      :is-loading="isDeleting"
      variant="danger"
      @confirm="handleDeleteConfirm"
    />
  </div>
</template>

<style scoped>
.scripts-view {
  background: linear-gradient(to bottom, rgba(248, 250, 252, 0.5), rgba(241, 245, 249, 0.5));
}

.dark .scripts-view {
  background: linear-gradient(to bottom, rgba(17, 24, 39, 0.5), rgba(31, 41, 55, 0.5));
}
</style>
