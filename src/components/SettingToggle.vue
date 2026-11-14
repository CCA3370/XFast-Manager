<template>
  <div class="flex items-start justify-between gap-4 py-2">
    <div class="min-w-0">
      <p class="text-sm font-medium">{{ label }}</p>
      <p v-if="description" class="mt-0.5 text-sm text-gray-600 dark:text-gray-400">
        {{ description }}
      </p>
    </div>
    <ToggleSwitch
      class="mt-0.5 focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2"
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
