<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div
        v-if="updateStore.showPostUpdateChangelog"
        class="fixed inset-0 z-[1150] flex items-center justify-center"
      >
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click="close"></div>

        <div
          class="relative w-full max-w-2xl mx-4 bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-700 shadow-2xl overflow-hidden"
        >
          <div class="px-5 py-4 border-b border-gray-200 dark:border-gray-700 flex items-start justify-between">
            <div>
              <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                <AnimatedText>
                  {{ $t('update.postUpdateTitle', { version: updateStore.postUpdateVersion }) }}
                </AnimatedText>
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                <AnimatedText>{{ $t('update.postUpdateSubtitle') }}</AnimatedText>
              </p>
            </div>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors p-1"
              :title="$t('common.close')"
              @click="close"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          <div class="px-5 py-4 max-h-[55vh] overflow-y-auto whitespace-pre-wrap text-sm leading-relaxed text-gray-700 dark:text-gray-200">
            {{ updateStore.postUpdateReleaseNotes }}
          </div>

          <div class="px-5 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-end gap-2">
            <button
              class="px-3 py-2 rounded-lg text-sm font-medium border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              @click="close"
            >
              <AnimatedText>{{ $t('common.close') }}</AnimatedText>
            </button>
            <button
              v-if="updateStore.postUpdateReleaseUrl"
              class="px-3 py-2 rounded-lg text-sm font-medium bg-blue-600 hover:bg-blue-700 text-white transition-colors"
              @click="openRelease"
            >
              <AnimatedText>{{ $t('update.viewOnGitHub') }}</AnimatedText>
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import AnimatedText from './AnimatedText.vue'
import { useUpdateStore } from '@/stores/update'
import { logError } from '@/services/logger'

const updateStore = useUpdateStore()

function close() {
  updateStore.dismissPostUpdateChangelog()
}

async function openRelease() {
  const url = updateStore.postUpdateReleaseUrl
  if (!url) return

  try {
    await invoke('open_url', { url })
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    logError(`Failed to open release URL: ${message}`, 'update')
  }
}
</script>

<style scoped>
.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}
</style>
