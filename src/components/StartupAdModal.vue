<template>
  <Teleport to="body">
    <dialog
      v-if="ad"
      ref="dialog"
      :dir="locale === 'ar' ? 'rtl' : 'ltr'"
      aria-labelledby="startup-ad-title"
      class="startup-ad-dialog m-auto rounded-2xl bg-white p-0 text-slate-900 shadow-2xl dark:bg-gray-900 dark:text-slate-100"
      @cancel.prevent="dismiss"
    >
      <header class="flex items-center justify-between gap-3 py-2.5 ps-5 pe-3">
        <h2 id="startup-ad-title" class="text-sm font-semibold text-slate-700 dark:text-slate-200">
          {{ t('startupAd.label') }}
        </h2>
        <button
          ref="closeButton"
          type="button"
          class="grid size-9 shrink-0 cursor-pointer place-items-center rounded-lg text-slate-600 transition-colors hover:bg-slate-100 hover:text-slate-900 active:bg-slate-200 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-blue-600 motion-reduce:transition-none dark:text-slate-300 dark:hover:bg-slate-800 dark:hover:text-white dark:active:bg-slate-700 dark:focus-visible:outline-blue-400"
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

      <div class="flex justify-center px-4 pb-4">
        <button
          type="button"
          class="startup-ad-image-button block min-w-0 max-w-full cursor-pointer rounded-xl border border-slate-200 bg-slate-50 transition-colors hover:border-blue-600 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-600 aria-disabled:cursor-wait motion-reduce:transition-none dark:border-slate-700 dark:bg-slate-950 dark:hover:border-blue-400 dark:focus-visible:outline-blue-400"
          :aria-label="t('startupAd.open', { alt: ad.alt })"
          :aria-disabled="opening"
          @click="openAd"
        >
          <img
            :src="ad.imageUrl"
            :alt="ad.alt"
            class="startup-ad-image rounded-[11px]"
            draggable="false"
          />
        </button>
      </div>

      <footer
        class="grid gap-2 border-t border-slate-200 bg-slate-50 px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950/35"
      >
        <label
          class="group flex min-h-10 w-fit max-w-full cursor-pointer items-start gap-2.5 rounded-lg px-2 py-2.5 leading-5 text-slate-700 transition-colors hover:bg-slate-200/65 active:bg-slate-200 motion-reduce:transition-none dark:text-slate-200 dark:hover:bg-slate-800 dark:active:bg-slate-700"
        >
          <input
            type="checkbox"
            :checked="hidden"
            class="startup-ad-checkbox peer sr-only"
            @change="changeHidden"
          />
          <span
            aria-hidden="true"
            class="startup-ad-checkmark grid size-5 shrink-0 place-items-center rounded-md border border-slate-500 bg-white text-white transition-colors group-hover:border-blue-600 peer-checked:border-blue-600 peer-checked:bg-blue-600 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-blue-600 motion-reduce:transition-none dark:border-slate-400 dark:bg-slate-900 dark:group-hover:border-blue-400 dark:peer-checked:border-blue-600 dark:peer-checked:bg-blue-600 dark:peer-focus-visible:outline-blue-400"
          >
            <svg class="size-3.5" fill="none" viewBox="0 0 20 20">
              <path
                d="m4.5 10 3.6 3.6 7.4-7.4"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
              />
            </svg>
          </span>
          <span>{{ t('startupAd.hide') }}</span>
        </label>
        <p v-if="openFailed" role="alert" class="px-2 pb-1 text-red-700 dark:text-red-300">
          {{ t('startupAd.openFailed') }}
        </p>
        <p v-if="saveFailed" role="alert" class="px-2 pb-1 text-red-700 dark:text-red-300">
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
  min-width: min(24rem, calc(100% - 2rem));
  max-width: min(42rem, calc(100% - 2rem));
  max-height: calc(100% - 2rem);
  overflow-y: auto;
  overscroll-behavior: contain;
}

.startup-ad-dialog::backdrop {
  background: rgb(2 6 23 / 49%);
}

.startup-ad-image {
  display: block;
  max-width: 100%;
  max-height: min(29rem, 65vh);
  width: auto;
  height: auto;
  object-fit: contain;
}

.startup-ad-checkmark svg {
  visibility: hidden;
}

.startup-ad-checkbox:checked + .startup-ad-checkmark svg {
  visibility: visible;
}

@media (forced-colors: active) {
  .startup-ad-checkbox {
    position: static;
    width: 1.25rem;
    height: 1.25rem;
    margin: 0;
    clip: auto;
  }

  .startup-ad-checkmark {
    display: none;
  }
}
</style>
