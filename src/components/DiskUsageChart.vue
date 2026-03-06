<template>
  <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`">
    <!-- Background circle -->
    <circle
      :cx="center"
      :cy="center"
      :r="radius"
      fill="none"
      :stroke="isDark ? '#374151' : '#f3f4f6'"
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
      :stroke-dashoffset="-seg.offset"
      stroke-linecap="butt"
      class="transition-all duration-500"
      @mouseenter="hovered = i"
      @mouseleave="hovered = -1"
    />
    <!-- Center text -->
    <text
      :x="center"
      :y="center - 6"
      text-anchor="middle"
      :fill="isDark ? '#e5e7eb' : '#111827'"
      font-size="14"
      font-weight="bold"
    >
      {{ formatSize(totalBytes) }}
    </text>
    <text
      :x="center"
      :y="center + 12"
      text-anchor="middle"
      :fill="isDark ? '#9ca3af' : '#6b7280'"
      font-size="10"
    >
      {{ hovered >= 0 ? categories[hovered].name : $t('diskUsage.total') }}
    </text>
    <text
      v-if="hovered >= 0"
      :x="center"
      :y="center + 26"
      text-anchor="middle"
      :fill="isDark ? '#9ca3af' : '#6b7280'"
      font-size="10"
    >
      {{ formatSize(categories[hovered].bytes) }}
      ({{ ((categories[hovered].bytes / totalBytes) * 100).toFixed(1) }}%)
    </text>
  </svg>
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
const size = computed(() => props.size ?? 200)
const strokeWidth = 24
const center = computed(() => size.value / 2)
const radius = computed(() => (size.value - strokeWidth) / 2)
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
  return `${(bytes / 1073741824).toFixed(2)} GB`
}
</script>
