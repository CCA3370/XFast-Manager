<template>
  <div class="min-w-0" @focusout="closeOnBlur" @keydown.esc.stop.prevent="close(true)">
    <button
      ref="trigger"
      type="button"
      :disabled="disabled"
      :aria-label="$t('patch.sourcePath') + ' · ' + (model || $t('patch.archiveRoot'))"
      :aria-expanded="open"
      :aria-controls="optionsId"
      class="flex w-full items-center gap-2 rounded-lg border border-gray-200 bg-white px-2.5 py-1.5 text-start text-xs text-gray-900 hover:border-orange-400 focus-visible:outline-2 focus-visible:outline-orange-500 disabled:opacity-60 dark:border-gray-700 dark:bg-gray-900/70 dark:text-white"
      @click="open ? close() : openPicker()"
    >
      <span class="min-w-0 flex-1 truncate" :title="model || $t('patch.archiveRoot')">
        {{ model || $t('patch.archiveRootPlaceholder') }}
      </span>
      <svg
        class="h-3.5 w-3.5 shrink-0 text-gray-500 transition-transform"
        :class="{ 'rotate-180': open }"
        viewBox="0 0 20 20"
        fill="none"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path d="m5 8 5 5 5-5" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" />
      </svg>
    </button>
    <div
      v-if="open"
      :id="optionsId"
      class="mt-1 rounded-lg border border-orange-200 bg-white p-1.5 shadow-sm dark:border-orange-500/30 dark:bg-gray-900"
    >
      <input
        ref="searchField"
        v-model="query"
        type="search"
        :aria-label="$t('patch.searchArchive')"
        :placeholder="$t('patch.searchArchive')"
        class="w-full rounded-md bg-gray-100 px-2 py-1.5 text-xs text-gray-900 focus-visible:outline-2 focus-visible:outline-orange-500 dark:bg-gray-800 dark:text-white"
        @keydown.enter.prevent="chooseFirst"
      />
      <ul v-if="choices.length" class="mt-1 max-h-40 space-y-0.5 overflow-auto">
        <li v-for="path in choices" :key="path">
          <button
            type="button"
            :aria-pressed="path === model"
            :title="path || $t('patch.archiveRoot')"
            class="w-full break-all rounded-md px-2 py-1.5 text-start text-xs text-gray-700 hover:bg-orange-50 aria-pressed:bg-orange-100 aria-pressed:text-orange-800 focus-visible:outline-2 focus-visible:outline-orange-500 dark:text-gray-200 dark:hover:bg-orange-500/15 dark:aria-pressed:bg-orange-500/20 dark:aria-pressed:text-orange-200"
            @click="choose(path)"
          >
            {{ path || $t('patch.archiveRoot') }}
          </button>
        </li>
      </ul>
      <p v-else class="px-2 py-1.5 text-xs text-gray-500 dark:text-gray-400">
        {{ $t('patch.noSourceResults') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, useId } from 'vue'

const props = defineProps<{ paths: string[]; disabled: boolean }>()
const model = defineModel<string>({ required: true })
const optionsId = useId()
const open = ref(false)
const query = ref('')
const trigger = ref<HTMLButtonElement | null>(null)
const searchField = ref<HTMLInputElement | null>(null)

const choices = computed(() => {
  const search = query.value.trim().replaceAll('\\', '/').toLowerCase()
  if (!search) return ['', ...props.paths.slice(0, 59)]
  const matches = props.paths.filter((path) => path.toLowerCase().includes(search))
  const exact = matches.find((path) => path.toLowerCase() === search)
  return exact
    ? [exact, ...matches.filter((path) => path !== exact).slice(0, 59)]
    : matches.slice(0, 60)
})

async function openPicker() {
  query.value = ''
  open.value = true
  await nextTick()
  searchField.value?.focus()
}

function close(restoreFocus = false) {
  open.value = false
  if (restoreFocus) trigger.value?.focus()
}

function closeOnBlur(event: FocusEvent) {
  const picker = event.currentTarget
  if (picker instanceof HTMLElement && !picker.contains(event.relatedTarget as Node | null)) close()
}

function choose(path: string) {
  model.value = path
  close(true)
}

function chooseFirst() {
  if (choices.value.length) choose(choices.value[0]!)
}
</script>
