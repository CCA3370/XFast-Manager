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
    activeClass: 'bg-sky-500',
    inactiveClass: 'bg-gray-300 dark:bg-gray-600',
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
        on: 'translate-x-3.5',
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
        knob: 'h-3.5 w-3.5',
        on: 'translate-x-4.5',
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
    class="relative inline-flex shrink-0 items-center rounded-full transition-colors disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none"
    :class="[sizeConfig.track, modelValue ? activeClass : inactiveClass]"
    :disabled="disabled"
    @click="handleToggle"
  >
    <span
      class="rounded-full bg-white transition-transform"
      :class="[sizeConfig.knob, modelValue ? sizeConfig.on : 'translate-x-0']"
    />
  </button>
</template>
