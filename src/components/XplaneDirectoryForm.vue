<template>
  <form class="space-y-3" @submit.prevent="save()">
    <label for="xplane-directory" class="block text-sm font-medium">
      {{ t('settings.xplanePath') }}
    </label>
    <div class="flex flex-wrap gap-2">
      <input
        id="xplane-directory"
        v-model="pathInput"
        class="min-w-0 flex-1 basis-48 rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900/50"
        type="text"
        autocomplete="off"
        spellcheck="false"
        :aria-invalid="Boolean(errorKey || browseFailed)"
        aria-describedby="xplane-directory-help xplane-directory-status"
        :disabled="locked"
        @input="browseFailed = false"
      />
      <button
        type="button"
        class="rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium hover:bg-gray-100 disabled:opacity-50 dark:border-gray-700 dark:hover:bg-gray-800"
        :disabled="locked || browsing"
        @click="browse"
      >
        {{ t('common.browse') }}
      </button>
    </div>
    <p id="xplane-directory-help" class="text-sm text-gray-600 dark:text-gray-400">
      {{ t('settingsSetup.directoryHelp') }}
    </p>
    <p id="xplane-directory-status" role="status" class="text-sm" :class="feedbackClass">
      {{ feedback }}
    </p>
    <div class="flex flex-wrap gap-2">
      <button
        type="submit"
        class="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        :disabled="pending || locked || browsing"
      >
        {{ t(onboarding ? 'settingsSetup.start' : 'settings.savePath') }}
      </button>
      <button
        v-if="!onboarding && store.xplanePath"
        type="button"
        class="rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium hover:bg-gray-100 disabled:opacity-50 dark:border-gray-700 dark:hover:bg-gray-800"
        :disabled="pending || locked || browsing"
        @click="save(true)"
      >
        {{ t('settingsSetup.clearDirectory') }}
      </button>
    </div>
  </form>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { useXplaneDirectory } from '@/composables/useXplaneDirectory'
import { logError } from '@/services/logger'
import { useAppStore } from '@/stores/app'

const props = defineProps<{ onboarding?: boolean }>()
const emit = defineEmits<{ saved: [] }>()
const { t } = useI18n()
const store = useAppStore()
const { pathInput, errorKey, status, submit } = useXplaneDirectory(Boolean(props.onboarding))
const browsing = ref(false)
const browseFailed = ref(false)
const pending = computed(() => status.value === 'validating' || status.value === 'saving')
const locked = computed(
  () => status.value === 'saving' || (props.onboarding && status.value === 'saved'),
)

const feedback = computed(() => {
  if (browseFailed.value) return t('settingsSetup.browseFailed')
  if (errorKey.value) return t(errorKey.value)
  if (pending.value) return t('settings.saving')
  return status.value === 'saved' ? t('settings.saved') : ''
})
const feedbackClass = computed(() =>
  errorKey.value || browseFailed.value
    ? 'text-red-700 dark:text-red-400'
    : status.value === 'saved'
      ? 'text-emerald-700 dark:text-emerald-400'
      : 'text-gray-600 dark:text-gray-400',
)

async function browse() {
  browsing.value = true
  browseFailed.value = false
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.selectXplaneFolder'),
    })
    if (typeof selected === 'string') pathInput.value = selected
  } catch (error) {
    browseFailed.value = true
    logError(`Failed to open folder dialog: ${error}`, 'settings')
  } finally {
    browsing.value = false
  }
}

async function save(clear = false) {
  if (locked.value || browsing.value) return
  browseFailed.value = false
  if (await submit(clear)) emit('saved')
}
</script>
