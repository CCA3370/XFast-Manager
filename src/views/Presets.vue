<template>
  <div class="h-full flex flex-col px-6 pt-3 pb-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ $t('presets.title') }}
        </h1>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
          {{ $t('presets.subtitle') }}
        </p>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="text-sm px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
          @click="store.importPreset()"
        >
          {{ $t('presets.import') }}
        </button>
        <button
          class="text-sm px-3 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors"
          @click="showSaveModal = true"
        >
          {{ $t('presets.saveCurrent') }}
        </button>
      </div>
    </div>

    <!-- Preset cards -->
    <div class="flex-1 overflow-y-auto">
      <div v-if="store.presets.length > 0" class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div
          v-for="preset in store.presets"
          :key="preset.id"
          class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800/50 p-4 hover:shadow-md transition-shadow"
        >
          <div class="flex items-start justify-between mb-2">
            <div class="min-w-0">
              <h3 class="font-semibold text-gray-900 dark:text-white truncate">
                {{ preset.name }}
              </h3>
              <p
                v-if="preset.description"
                class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 line-clamp-2"
              >
                {{ preset.description }}
              </p>
            </div>
          </div>

          <!-- Addon counts -->
          <div class="flex flex-wrap gap-2 mt-3 text-xs text-gray-500 dark:text-gray-400">
            <span v-if="preset.addonCounts.aircraftTotal > 0">
              {{
                $t('presets.countAircraft', {
                  enabled: preset.addonCounts.aircraftEnabled,
                  total: preset.addonCounts.aircraftTotal,
                })
              }}
            </span>
            <span v-if="preset.addonCounts.pluginsTotal > 0">
              {{
                $t('presets.countPlugins', {
                  enabled: preset.addonCounts.pluginsEnabled,
                  total: preset.addonCounts.pluginsTotal,
                })
              }}
            </span>
            <span v-if="preset.addonCounts.sceneryTotal > 0">
              {{
                $t('presets.countScenery', {
                  enabled: preset.addonCounts.sceneryEnabled,
                  total: preset.addonCounts.sceneryTotal,
                })
              }}
            </span>
            <span v-if="preset.addonCounts.luaTotal > 0">
              {{
                $t('presets.countLua', {
                  enabled: preset.addonCounts.luaEnabled,
                  total: preset.addonCounts.luaTotal,
                })
              }}
            </span>
          </div>

          <!-- Timestamps -->
          <p class="text-[10px] text-gray-400 dark:text-gray-500 mt-2">
            {{ $t('presets.updated') }} {{ formatDate(preset.updatedAt) }}
          </p>

          <!-- Actions -->
          <div
            class="flex flex-wrap items-center gap-2 mt-3 pt-3 border-t border-gray-100 dark:border-gray-700/50"
          >
            <button
              class="flex-1 text-sm py-1.5 rounded-lg bg-blue-50 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-500/20 transition-colors font-medium"
              :disabled="store.isApplying"
              @click="handleApply(preset)"
            >
              {{ store.isApplying ? $t('presets.applying') : $t('presets.apply') }}
            </button>
            <button
              class="text-sm px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
              :disabled="store.isUpdatingSnapshot(preset.id)"
              @click="handleUpdateSnapshot(preset)"
            >
              {{
                store.isUpdatingSnapshot(preset.id)
                  ? $t('presets.updatingSnapshot')
                  : $t('presets.updateSnapshot')
              }}
            </button>
            <button
              class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 transition-colors"
              :title="$t('presets.export')"
              @click="store.exportPreset(preset.id)"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                />
              </svg>
            </button>
            <button
              class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
              :title="$t('common.delete')"
              @click="handleDelete(preset)"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                />
              </svg>
            </button>
          </div>
        </div>
      </div>

      <!-- Empty state -->
      <div
        v-else-if="!store.isLoading"
        class="flex flex-col items-center justify-center h-full text-gray-400 dark:text-gray-500"
      >
        <svg
          class="w-12 h-12 mb-3 opacity-50"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
          />
        </svg>
        <p class="text-sm">{{ $t('presets.empty') }}</p>
        <p class="text-xs mt-1">{{ $t('presets.emptyHint') }}</p>
      </div>

      <!-- Loading -->
      <div v-if="store.isLoading" class="flex justify-center py-12">
        <div
          class="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
        ></div>
      </div>
    </div>

    <!-- Save modal -->
    <PresetSaveModal v-if="showSaveModal" @close="showSaveModal = false" @save="handleSave" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { usePresetsStore, type PresetSummary } from '@/stores/presets'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import PresetSaveModal from '@/components/PresetSaveModal.vue'

const { t } = useI18n()
const store = usePresetsStore()
const modalStore = useModalStore()
const toastStore = useToastStore()
const showSaveModal = ref(false)

onMounted(() => {
  store.loadPresets()
})

function formatDate(epoch: number): string {
  return new Date(epoch * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

async function handleSave(name: string, description: string) {
  showSaveModal.value = false
  await store.savePreset(name, description || undefined)
}

function handleApply(preset: PresetSummary) {
  modalStore.showConfirm({
    title: t('presets.applyTitle'),
    message: t('presets.applyMessage', { name: preset.name }),
    confirmText: t('presets.apply'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: async () => {
      try {
        const result = await store.applyPreset(preset.id)
        if (result.errors.length > 0) {
          modalStore.showError(result.errors.join('\n'))
        }
      } catch (e) {
        modalStore.showError(String(e), t('presets.applyTitle'))
      }
    },
    onCancel: () => {},
  })
}

function handleUpdateSnapshot(preset: PresetSummary) {
  modalStore.showConfirm({
    title: t('presets.updateSnapshotTitle'),
    message: t('presets.updateSnapshotMessage', { name: preset.name }),
    confirmText: t('presets.updateSnapshot'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: async () => {
      try {
        await store.updatePreset(preset.id, undefined, undefined, true)
        toastStore.success(t('presets.updateSnapshotSuccess'))
      } catch (e) {
        modalStore.showError(String(e), t('presets.updateSnapshotFailed'))
      }
    },
    onCancel: () => {},
  })
}

function handleDelete(preset: PresetSummary) {
  modalStore.showConfirm({
    title: t('presets.deleteTitle'),
    message: t('presets.deleteMessage', { name: preset.name }),
    confirmText: t('common.delete'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: async () => {
      await store.deletePreset(preset.id)
    },
    onCancel: () => {},
  })
}
</script>
