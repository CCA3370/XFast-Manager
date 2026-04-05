<template>
  <div class="flex items-center gap-4">
    <!-- Chart -->
    <div class="relative shrink-0" :style="{ width: size + 'px', height: size + 'px' }">
      <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`" class="rotate-[-90deg]">
        <!-- Background circle -->
        <circle
          :cx="center"
          :cy="center"
          :r="radius"
          fill="none"
          :stroke="isDark ? '#1f2937' : '#f1f5f9'"
          :stroke-width="strokeWidth"
        />
        <!-- Segments -->
        <circle
          v-for="(seg, i) in segments"
          :key="i"
          :cx="center"
          :cy="center"
          :r="radius"
          fill="none"
          :stroke="seg.color"
          :stroke-width="strokeWidth"
          :stroke-dasharray="`${seg.length} ${circumference - seg.length}`"
          :stroke-dashoffset="-(seg.offset || 0)"
          stroke-linecap="round"
          class="transition-all duration-700 ease-out"
          @mouseenter="hovered = i"
          @mouseleave="hovered = -1"
        />
      </svg>
      <!-- Center text -->
      <div class="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
        <span
          class="font-bold text-gray-900 dark:text-white leading-tight"
          :class="size < 150 ? 'text-sm' : 'text-lg'"
        >
          {{ hovered >= 0 ? formatSize(categories[hovered].bytes) : formatSize(totalBytes) }}
        </span>
        <span class="text-[10px] text-gray-500 dark:text-gray-400 uppercase tracking-tighter">
          {{ hovered >= 0 ? categories[hovered].name : $t('diskUsage.total') }}
        </span>
      </div>
    </div>

    <!-- Legend (Always visible in this version) -->
    <div class="flex-1 min-w-0 space-y-1.5">
      <div
        v-for="(cat, i) in categories.slice(0, 5)"
        :key="i"
        class="flex items-center justify-between text-[10px] group cursor-default"
        @mouseenter="hovered = i"
        @mouseleave="hovered = -1"
      >
        <div class="flex items-center gap-1.5 min-w-0">
          <div
            class="w-1.5 h-1.5 rounded-full shrink-0"
            :style="{ backgroundColor: cat.color }"
          ></div>
          <span
            class="truncate text-gray-600 dark:text-gray-400 font-medium group-hover:text-blue-500 transition-colors"
          >
            {{ cat.name }}
          </span>
        </div>
        <span class="text-gray-400 dark:text-gray-500 tabular-nums">
          {{ ((cat.bytes / totalBytes) * 100).toFixed(1) }}%
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

useI18n()

const props = defineProps<{
  categories: { name: string; bytes: number; color: string }[]
  totalBytes: number
  size?: number
  isDark?: boolean
}>()

const hovered = ref(-1)
const size = computed(() => props.size ?? 160)
const strokeWidth = computed(() => (size.value < 150 ? 10 : 14))
const center = computed(() => size.value / 2)
const radius = computed(() => (size.value - strokeWidth.value) / 2)
const circumference = computed(() => 2 * Math.PI * radius.value)

interface Segment {
  color: string
  length: number
  offset: number
}

const segments = computed<Segment[]>(() => {
  if (props.totalBytes === 0) return []
  const segs: Segment[] = []
  let offset = 0
  for (const cat of props.categories) {
    const ratio = cat.bytes / props.totalBytes
    const length = ratio * circumference.value
    segs.push({ color: cat.color, length, offset })
    offset += length
  }
  return segs
})

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(1)} GB`
}
</script>
