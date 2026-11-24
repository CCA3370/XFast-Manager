<template>
  <Teleport to="body">
    <dialog
      v-if="ad"
      ref="dialog"
      :dir="locale === 'ar' ? 'rtl' : 'ltr'"
      aria-labelledby="startup-ad-title"
      class="startup-ad-dialog m-auto rounded-xl border border-gray-200 bg-white p-0 text-gray-900 shadow-xl dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100"
      @cancel.prevent="dismiss"
    >
      <header class="flex items-center justify-between gap-3 px-4 py-2">
        <h2 id="startup-ad-title" class="text-sm font-medium text-gray-600 dark:text-gray-300">
          {{ t('startupAd.label') }}
        </h2>
        <button
          ref="closeButton"
          type="button"
          class="rounded-md p-2 text-gray-600 hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-blue-500 dark:text-gray-300 dark:hover:bg-gray-800"
          :aria-label="t('common.close')"
          @click="dismiss"
        >
          <svg aria-hidden="true" class="size-4" fill="none" viewBox="0 0 24 24">
            <path
              d="m6 6 12 12M18 6 6 18"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-width="2"
            />
          </svg>
        </button>
      </header>

      <div class="flex justify-center px-4">
        <button
          type="button"
          class="startup-ad-image-button rounded-lg focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500"
          :aria-label="t('startupAd.open', { alt: ad.alt })"
          :aria-disabled="opening"
          @click="openAd"
        >
          <img
            :src="ad.imageUrl"
            :alt="ad.alt"
            class="startup-ad-image rounded-lg"
            draggable="false"
          />
        </button>
      </div>

      <footer class="grid gap-2 px-4 py-3 text-sm">
        <label class="flex w-fit cursor-pointer items-start gap-2">
          <input
            type="checkbox"
            :checked="hidden"
            class="mt-0.5 size-4 shrink-0 accent-blue-600"
            @change="changeHidden"
          />
          <span>{{ t('startupAd.hide') }}</span>
        </label>
        <p v-if="openFailed" role="alert" class="text-red-700 dark:text-red-300">
          {{ t('startupAd.openFailed') }}
        </p>
        <p v-if="saveFailed" role="alert" class="text-red-700 dark:text-red-300">
          {{ t('startupAd.saveFailed') }}
        </p>
      </footer>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { parseStartupAdUrl, setStartupAdSeen, type StartupAd } from '@/services/startupAds'

const props = defineProps<{ ad: StartupAd | null }>()
const emit = defineEmits<{ dismiss: [] }>()
const { t, locale } = useI18n()

const dialog = ref<HTMLDialogElement | null>(null)
const closeButton = ref<HTMLButtonElement | null>(null)
const hidden = ref(false)
const opening = ref(false)
const openFailed = ref(false)
const saveFailed = ref(false)
let savedHidden = false
let preferenceChange = 0

watch(
  () => props.ad,
  async (ad) => {
    hidden.value = false
    savedHidden = false
    opening.value = false
    openFailed.value = false
    saveFailed.value = false
    if (!ad) {
      dialog.value?.close()
      return
    }

    await nextTick()
    const element = dialog.value
    if (props.ad !== ad || !element) return
    if (!element.open) element.showModal()
    closeButton.value?.focus()
  },
  { immediate: true },
)

function dismiss() {
  dialog.value?.close()
  emit('dismiss')
}

async function openAd() {
  const ad = props.ad
  if (!ad || opening.value) return
  const url = parseStartupAdUrl(ad.url)
  if (!url) {
    openFailed.value = true
    return
  }

  opening.value = true
  openFailed.value = false
  try {
    await invoke('open_url', { url })
    if (props.ad === ad && dialog.value?.open) dismiss()
  } catch {
    if (props.ad === ad) openFailed.value = true
  } finally {
    if (props.ad === ad) opening.value = false
  }
}

async function changeHidden(event: Event) {
  const ad = props.ad
  if (!ad) return

  const checked = (event.target as HTMLInputElement).checked
  const change = ++preferenceChange
  hidden.value = checked
  saveFailed.value = false
  try {
    await setStartupAdSeen(ad.id, checked)
    if (props.ad === ad) savedHidden = checked
  } catch {
    if (props.ad === ad && preferenceChange === change) {
      hidden.value = savedHidden
      saveFailed.value = true
    }
  }
}

onBeforeUnmount(() => dialog.value?.close())
</script>

<style scoped>
.startup-ad-dialog {
  width: max-content;
  max-width: min(42rem, calc(100% - 2rem));
  max-height: calc(100% - 2rem);
  overflow-y: auto;
  overscroll-behavior: contain;
}

.startup-ad-dialog::backdrop {
  background: rgb(0 0 0 / 45%);
}

.startup-ad-image-button {
  max-width: 100%;
}

.startup-ad-image {
  display: block;
  max-width: 100%;
  max-height: min(29rem, 65vh);
  width: auto;
  height: auto;
  object-fit: contain;
}
</style>
