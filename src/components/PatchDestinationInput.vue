<template>
  <div class="min-w-0">
    <input
      ref="field"
      v-model="model"
      type="text"
      :disabled="disabled"
      :aria-label="$t('patch.destinationPath')"
      :aria-describedby="open ? suggestionsId : undefined"
      :placeholder="$t('patch.destRootPlaceholder')"
      autocomplete="off"
      spellcheck="false"
      class="w-full rounded-lg border border-gray-200 bg-white px-2.5 py-1.5 text-xs text-gray-900 focus-visible:outline-2 focus-visible:outline-orange-500 disabled:opacity-60 dark:border-gray-700 dark:bg-gray-900/70 dark:text-white"
      @focus="showSuggestions = true"
      @input="showSuggestions = true"
      @keydown.down.prevent="suggestionList?.querySelector('button')?.focus()"
      @keydown.esc.stop.prevent="showSuggestions = false"
    />
    <PatchPathPopover
      v-if="open"
      :id="suggestionsId"
      :anchor="field"
      @close="showSuggestions = false"
      @escape="dismiss"
    >
      <ul ref="suggestionList" class="space-y-0.5">
        <li v-for="path in suggestions" :key="path">
          <button
            type="button"
            :title="path"
            class="w-full break-all rounded-md px-2 py-1.5 text-start text-xs text-gray-700 hover:bg-orange-50 focus-visible:outline-2 focus-visible:outline-orange-500 dark:text-gray-200 dark:hover:bg-orange-500/15"
            @click="choose(path)"
          >
            {{ path }}
          </button>
        </li>
      </ul>
    </PatchPathPopover>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, useId } from 'vue'
import PatchPathPopover from './PatchPathPopover.vue'

const props = defineProps<{ directories: string[]; disabled: boolean }>()
const model = defineModel<string>({ required: true })
const suggestionsId = useId()
const field = ref<HTMLInputElement | null>(null)
const suggestionList = ref<HTMLUListElement | null>(null)
const showSuggestions = ref(false)

const suggestions = computed(() => {
  if (!model.value || props.disabled) return []
  const rooted = model.value.startsWith('/')
  const input = model.value.replaceAll('\\', '/').toLowerCase()
  const query = rooted ? input.slice(1) : input
  const leafSearch = !rooted && !query.includes('/')
  return props.directories
    .flatMap((path) => {
      const display = rooted ? '/' + path : path
      const lower = path.toLowerCase()
      if (lower === query) return display === model.value ? [] : [{ path: display, rank: -1 }]
      if (lower.startsWith(query)) return [{ path: display, rank: 0 }]
      if (leafSearch && lower.split('/').at(-1)?.startsWith(query))
        return [{ path: display, rank: 1 }]
      return []
    })
    .sort((a, b) => a.rank - b.rank || a.path.length - b.path.length)
    .slice(0, 2)
    .map(({ path }) => path)
})

const open = computed(() => showSuggestions.value && suggestions.value.length > 0)

function choose(path: string) {
  model.value = path
  dismiss()
}

function dismiss() {
  field.value?.focus()
  showSuggestions.value = false
}
</script>
