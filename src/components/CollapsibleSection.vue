<template>
  <details
    class="group rounded-lg border border-gray-200 bg-gray-50/70 p-3 transition-colors open:border-gray-300 dark:border-gray-700 dark:bg-gray-900/30 dark:open:border-gray-600"
    @toggle="emit('toggle', $event)"
  >
    <summary
      class="flex cursor-pointer list-none select-none items-center gap-2 text-sm font-medium text-gray-700 transition-colors hover:text-gray-950 focus-visible:outline-none dark:text-gray-300 dark:hover:text-white [&::-webkit-details-marker]:hidden [&::marker]:hidden"
    >
      <svg
        class="h-4 w-4 shrink-0 text-gray-400 transition-transform duration-200 group-open:rotate-90 dark:text-gray-500"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        aria-hidden="true"
      >
        <path stroke-linecap="round" stroke-linejoin="round" d="m9 18 6-6-6-6" />
      </svg>
      <slot name="title">{{ title }}</slot>
    </summary>
    <div class="mt-3 border-t border-gray-200 pt-3 dark:border-gray-700">
      <slot />
    </div>
  </details>
</template>

<script setup lang="ts">
defineProps<{ title?: string }>()
const emit = defineEmits<{ toggle: [event: Event] }>()
</script>
