<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    disabled?: boolean
    ariaLabel?: string
    size?: 'sm' | 'md' | 'lg'
    activeClass?: string
    inactiveClass?: string
    stopPropagation?: boolean
  }>(),
  {
    disabled: false,
    ariaLabel: '',
    size: 'md',
    activeClass: 'bg-blue-500 hover:bg-blue-600 dark:bg-blue-500 dark:hover:bg-blue-400',
    inactiveClass: 'bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500',
    stopPropagation: true,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const sizeConfig = computed(() => {
  switch (props.size) {
    case 'sm':
      return {
        track: 'h-4 w-7 p-0.5',
        knob: 'h-3 w-3',
        on: 'translate-x-3',
      }
    case 'lg':
      return {
        track: 'h-6 w-11 p-0.5',
        knob: 'h-5 w-5',
        on: 'translate-x-5',
      }
    default:
      return {
        track: 'h-5 w-9 p-0.5',
        knob: 'h-4 w-4',
        on: 'translate-x-4',
      }
  }
})

function handleToggle(event: MouseEvent) {
  if (props.stopPropagation) {
    event.stopPropagation()
  }
  if (props.disabled) return
  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :aria-label="ariaLabel || undefined"
    class="relative inline-flex shrink-0 items-center rounded-full transition-colors duration-200 ease-out disabled:opacity-40 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 focus-visible:ring-offset-1 dark:focus-visible:ring-offset-gray-800"
    :class="[sizeConfig.track, modelValue ? activeClass : inactiveClass]"
    :disabled="disabled"
    @click="handleToggle"
  >
    <span
      class="rounded-full bg-white shadow-sm transition-transform duration-200 ease-out"
      :class="[sizeConfig.knob, modelValue ? sizeConfig.on : 'translate-x-0']"
    />
  </button>
</template>
