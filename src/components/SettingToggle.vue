<template>
  <div
    class="flex items-start justify-between gap-4 rounded-lg px-3 py-2.5 transition-colors hover:bg-gray-50 dark:hover:bg-gray-800/50"
  >
    <div class="min-w-0">
      <p class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ label }}</p>
      <p v-if="description" class="mt-0.5 text-xs leading-relaxed text-gray-500 dark:text-gray-400">
        {{ description }}
      </p>
    </div>
    <ToggleSwitch
      class="mt-0.5 shrink-0"
      :aria-label="label"
      :model-value="modelValue"
      :disabled="disabled"
      @update:model-value="emit('update:modelValue', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import ToggleSwitch from '@/components/ToggleSwitch.vue'

defineProps<{ label: string; description?: string; modelValue: boolean; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [enabled: boolean] }>()
</script>
