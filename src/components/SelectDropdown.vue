<script setup lang="ts">
import { computed, ref } from 'vue'
import { onClickOutside } from '@vueuse/core'

interface Option {
  value: string | number
  label: string
}

const props = withDefaults(
  defineProps<{
    modelValue: string | number
    options: Option[]
    disabled?: boolean
    ariaLabel?: string
  }>(),
  {
    disabled: false,
    ariaLabel: '',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
}>()

const open = ref(false)
const dropdownRef = ref<HTMLElement | null>(null)

onClickOutside(dropdownRef, () => (open.value = false))

const selectedLabel = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? '',
)

function select(value: string | number) {
  emit('update:modelValue', value)
  open.value = false
}

function toggle() {
  if (props.disabled) return
  open.value = !open.value
}
</script>

<template>
  <div ref="dropdownRef" class="relative">
    <button
      type="button"
      :aria-label="ariaLabel || undefined"
      :disabled="disabled"
      class="flex w-full items-center justify-between gap-2 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 transition-all duration-150 hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500/20 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:hover:border-gray-500 dark:focus:ring-blue-400/20"
      @click="toggle"
    >
      <span class="truncate">{{ selectedLabel }}</span>
      <svg
        class="h-4 w-4 shrink-0 text-gray-400 transition-transform duration-200"
        :class="{ 'rotate-180': open }"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        aria-hidden="true"
      >
        <path stroke-linecap="round" stroke-linejoin="round" d="m6 9 6 6 6-6" />
      </svg>
    </button>
    <Transition
      enter-active-class="transition-all duration-150 ease-out"
      enter-from-class="opacity-0 -translate-y-1"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all duration-100 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 -translate-y-1"
    >
      <ul
        v-if="open"
        class="absolute z-10 mt-1 w-full overflow-auto rounded-lg border border-gray-200 bg-white py-1 shadow-lg max-h-60 dark:border-gray-600 dark:bg-gray-800 dark:shadow-xl"
      >
        <li v-for="opt in options" :key="opt.value">
          <button
            type="button"
            class="flex w-full items-center px-3 py-1.5 text-left text-sm transition-colors hover:bg-blue-50 dark:hover:bg-blue-900/30"
            :class="
              opt.value === modelValue
                ? 'font-medium text-blue-600 dark:text-blue-400'
                : 'text-gray-700 dark:text-gray-300'
            "
            @click="select(opt.value)"
          >
            {{ opt.label }}
            <svg
              v-if="opt.value === modelValue"
              class="ml-auto h-4 w-4 text-blue-500 dark:text-blue-400"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              aria-hidden="true"
            >
              <path stroke-linecap="round" stroke-linejoin="round" d="m5 13 4 4L19 7" />
            </svg>
          </button>
        </li>
      </ul>
    </Transition>
  </div>
</template>
