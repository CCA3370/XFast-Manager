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
          <div
            class="px-5 py-4 border-b border-gray-200 dark:border-gray-700 flex items-start justify-between"
          >
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

          <div
            class="px-5 py-4 max-h-[55vh] overflow-y-auto text-sm leading-relaxed text-gray-700 dark:text-gray-200 markdown-content"
            v-html="renderedMarkdown"
          ></div>

          <div
            class="px-5 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-end gap-2"
          >
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
import { computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { marked } from 'marked'
import AnimatedText from './AnimatedText.vue'
import { useUpdateStore } from '@/stores/update'
import { logError } from '@/services/logger'

const updateStore = useUpdateStore()

const renderedMarkdown = computed(() => {
  const markdown = updateStore.postUpdateReleaseNotes || ''
  return marked(markdown)
})

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

.markdown-content :deep(*:first-child) {
  margin-top: 0 !important;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4) {
  font-weight: 600;
  margin-top: 1.5em;
  margin-bottom: 0.75em;
  line-height: 1.25;
}

.markdown-content :deep(h1) {
  font-size: 1.5rem;
  border-bottom: 2px solid rgb(229 231 235);
  padding-bottom: 0.3em;
}

:root.dark .markdown-content :deep(h1) {
  border-bottom-color: rgb(55 65 81);
}

.markdown-content :deep(h2) {
  font-size: 1.25rem;
  border-bottom: 1px solid rgb(229 231 235);
  padding-bottom: 0.3em;
}

:root.dark .markdown-content :deep(h2) {
  border-bottom-color: rgb(55 65 81);
}

.markdown-content :deep(h3) {
  font-size: 1.125rem;
}

.markdown-content :deep(h4) {
  font-size: 1rem;
}

.markdown-content :deep(p) {
  margin-bottom: 1em;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin-bottom: 1em;
  padding-left: 2em;
}

.markdown-content :deep(ul) {
  list-style-type: disc;
}

.markdown-content :deep(ol) {
  list-style-type: decimal;
}

.markdown-content :deep(li) {
  margin-bottom: 0.5em;
}

.markdown-content :deep(li > ul),
.markdown-content :deep(li > ol) {
  margin-top: 0.5em;
}

.markdown-content :deep(a) {
  color: rgb(37 99 235);
  text-decoration: none;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

:root.dark .markdown-content :deep(a) {
  color: rgb(96 165 250);
}

.markdown-content :deep(code) {
  background-color: rgb(243 244 246);
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  font-size: 0.875rem;
  font-family:
    ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace;
}

:root.dark .markdown-content :deep(code) {
  background-color: rgb(31 41 55);
}

.markdown-content :deep(pre) {
  background-color: rgb(243 244 246);
  padding: 0.75rem;
  border-radius: 0.5rem;
  overflow-x: auto;
  margin-bottom: 1em;
}

:root.dark .markdown-content :deep(pre) {
  background-color: rgb(31 41 55);
}

.markdown-content :deep(pre code) {
  background-color: transparent;
  padding: 0;
}

.markdown-content :deep(blockquote) {
  border-left: 4px solid rgb(209 213 219);
  padding-left: 1rem;
  font-style: italic;
  margin-bottom: 1em;
}

:root.dark .markdown-content :deep(blockquote) {
  border-left-color: rgb(75 85 99);
}

.markdown-content :deep(hr) {
  border-color: rgb(229 231 235);
  margin: 1.5em 0;
}

:root.dark .markdown-content :deep(hr) {
  border-color: rgb(55 65 81);
}

.markdown-content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin-bottom: 1em;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  border: 1px solid rgb(209 213 219);
  padding: 0.5rem 0.75rem;
}

:root.dark .markdown-content :deep(th),
:root.dark .markdown-content :deep(td) {
  border-color: rgb(75 85 99);
}

.markdown-content :deep(th) {
  background-color: rgb(243 244 246);
  font-weight: 600;
}

:root.dark .markdown-content :deep(th) {
  background-color: rgb(31 41 55);
}

.markdown-content :deep(strong) {
  font-weight: 700;
  color: rgb(17 24 39);
}

:root.dark .markdown-content :deep(strong) {
  color: rgb(249 250 251);
}

.markdown-content :deep(em) {
  font-style: italic;
}
</style>
