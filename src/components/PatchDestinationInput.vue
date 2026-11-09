<template>
  <div class="group/destination min-w-0">
    <input
      ref="field"
      v-model="model"
      type="text"
      :disabled="disabled"
      :aria-label="$t('patch.destinationPath')"
      :aria-describedby="suggestions.length ? suggestionsId : undefined"
      :placeholder="$t('patch.destRootPlaceholder')"
      autocomplete="off"
      spellcheck="false"
      class="w-full rounded-lg border border-gray-200 bg-white px-2.5 py-1.5 text-xs text-gray-900 focus-visible:outline-2 focus-visible:outline-orange-500 disabled:opacity-60 dark:border-gray-700 dark:bg-gray-900/70 dark:text-white"
      @keydown.down.prevent="suggestionList?.querySelector('button')?.focus()"
    />
    <ul
      v-if="suggestions.length"
      :id="suggestionsId"
      ref="suggestionList"
      class="mt-1 hidden space-y-0.5 group-focus-within/destination:block"
    >
      <li v-for="path in suggestions" :key="path">
        <button
          type="button"
          :title="path"
          class="w-full break-all rounded-md bg-orange-50 px-2 py-1 text-start text-[11px] text-orange-800 hover:bg-orange-100 focus-visible:outline-2 focus-visible:outline-orange-500 dark:bg-orange-500/10 dark:text-orange-200 dark:hover:bg-orange-500/20"
          @click="choose(path)"
        >
          {{ path }}
        </button>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, useId } from 'vue'

const props = defineProps<{ directories: string[]; disabled: boolean }>()
const model = defineModel<string>({ required: true })
const suggestionsId = useId()
const field = ref<HTMLInputElement | null>(null)
const suggestionList = ref<HTMLUListElement | null>(null)

const suggestions = computed(() => {
  const query = model.value.replaceAll('\\', '/').toLowerCase()
  if (!query || props.disabled) return []
  const leafSearch = !query.includes('/')
  return props.directories
    .flatMap((path) => {
      const lower = path.toLowerCase()
      if (lower === query) return path === model.value ? [] : [{ path, rank: -1 }]
      if (lower.startsWith(query)) return [{ path, rank: 0 }]
      if (leafSearch && lower.split('/').at(-1)?.startsWith(query)) return [{ path, rank: 1 }]
      return []
    })
    .sort((a, b) => a.rank - b.rank || a.path.length - b.path.length)
    .slice(0, 2)
    .map(({ path }) => path)
})

function choose(path: string) {
  model.value = path
  field.value?.focus()
}
</script>
