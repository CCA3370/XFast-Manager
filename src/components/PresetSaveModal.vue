<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div
        v-if="true"
        class="fixed inset-0 z-[80] flex items-center justify-center"
      >
        <div
          class="absolute inset-0 bg-black/30 dark:bg-black/50 backdrop-blur-sm"
          @click="$emit('close')"
        ></div>
        <div
          class="relative w-full max-w-md bg-white dark:bg-gray-800 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 p-6"
        >
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            {{ $t('presets.saveTitle') }}
          </h2>

          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                {{ $t('presets.nameLabel') }}
              </label>
              <input
                ref="nameInput"
                v-model="name"
                type="text"
                :placeholder="$t('presets.namePlaceholder')"
                class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 outline-none focus:ring-2 focus:ring-blue-500/30"
                @keydown.enter="submit"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                {{ $t('presets.descriptionLabel') }}
              </label>
              <textarea
                v-model="description"
                :placeholder="$t('presets.descriptionPlaceholder')"
                rows="3"
                class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 outline-none focus:ring-2 focus:ring-blue-500/30 resize-none"
              ></textarea>
            </div>
          </div>

          <div class="flex justify-end gap-2 mt-6">
            <button
              class="px-4 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              @click="$emit('close')"
            >
              {{ $t('common.cancel') }}
            </button>
            <button
              class="px-4 py-2 text-sm rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors disabled:opacity-50"
              :disabled="!name.trim()"
              @click="submit"
            >
              {{ $t('common.save') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'

const emit = defineEmits<{
  close: []
  save: [name: string, description: string]
}>()

const name = ref('')
const description = ref('')
const nameInput = ref<HTMLInputElement | null>(null)

onMounted(() => {
  nextTick(() => nameInput.value?.focus())
})

function submit() {
  if (!name.value.trim()) return
  emit('save', name.value.trim(), description.value.trim())
}
</script>

<style scoped>
.modal-fade-enter-active {
  transition: opacity 0.15s ease;
}
.modal-fade-leave-active {
  transition: opacity 0.1s ease;
}
.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}
</style>
