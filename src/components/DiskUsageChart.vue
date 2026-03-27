<template>
  <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`" class="overflow-visible">
    <!-- Radar Grids (Only in radar mode) -->
    <g v-if="radar" class="opacity-20 pointer-events-none text-aviation-amber">
      <circle :cx="center" :cy="center" :r="radius + strokeWidth/2 + 4" fill="none" stroke="currentColor" stroke-width="1" stroke-dasharray="2 4" />
      <circle :cx="center" :cy="center" :r="radius - strokeWidth/2 - 4" fill="none" stroke="currentColor" stroke-width="1" stroke-dasharray="2 4" />
      <line :x1="center" :y1="0" :x2="center" :y2="size" stroke="currentColor" stroke-width="0.5" />
      <line :x1="0" :y1="center" :x2="size" :y2="center" stroke="currentColor" stroke-width="0.5" />
    </g>

    <!-- Background circle -->
    <circle
      :cx="center"
      :cy="center"
      :r="radius"
      fill="none"
      :stroke="isDark ? '#1f2937' : '#f3f4f6'"
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
      class="transition-all duration-500 cursor-pointer"
      :class="{ 'opacity-40': hovered >= 0 && hovered !== i }"
      @mouseenter="hovered = i"
      @mouseleave="hovered = -1"
    />
    <!-- Center text -->
    <text
      :x="center"
      :y="center - 4"
      text-anchor="middle"
      :fill="isDark ? (radar ? '#ffb000' : '#e5e7eb') : '#111827'"
      class="font-mono"
      :font-size="radar ? 12 : 14"
      font-weight="bold"
    >
      {{ formatSize(totalBytes) }}
    </text>
    <text
      :x="center"
      :y="center + (radar ? 10 : 12)"
      text-anchor="middle"
      :fill="isDark ? (radar ? '#ffb000' : '#9ca3af') : '#6b7280'"
      class="font-mono uppercase tracking-tighter"
      :font-size="radar ? 8 : 10"
    >
      {{ hovered >= 0 ? categories[hovered].name : (radar ? 'TOTAL TEL' : $t('diskUsage.total')) }}
    </text>
  </svg>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  categories: { name: string; bytes: number; color: string }[]
  totalBytes: number
  size?: number
  isDark?: boolean
  radar?: boolean
}>()

const hovered = ref(-1)
const size = computed(() => props.size ?? 200)
const strokeWidth = computed(() => props.radar ? 12 : 24)
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
  return `${(bytes / 1073741824).toFixed(2)} GB`
}
</script>
