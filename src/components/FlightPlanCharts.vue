<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FlightPlanRouteMap from './FlightPlanRouteMap.vue'
import { legs, type Waypoint } from '@/services/flightPlanDetails'
const props = defineProps<{ points: Waypoint[] }>()
const { t } = useI18n()
const label = (key: string) => t(`flightPlanDetails.${key}`)
const segments = computed(() => legs(props.points))
const profile = computed(() => {
  const total = segments.value.at(-1)?.cumulative
  if (!total) return []
  const altitudes = props.points.flatMap((p) => (p.altitude === null ? [] : [p.altitude]))
  const low = Math.min(0, ...altitudes),
    high = Math.max(1000, ...altitudes)
  return props.points.map((p, i) => {
    const x = i === 0 ? 0 : segments.value[i - 1]?.cumulative
    if (x === null || x === undefined || p.altitude === null) return null
    return {
      x: 50 + (x / total) * 700,
      y: 145 - ((p.altitude - low) / (high - low)) * 120,
      id: p.id,
      altitude: p.altitude,
    }
  })
})
const altitudeLabels = computed(() => {
  const labels: { x: number; y: number; altitude: number }[] = []
  for (let i = 0; i < profile.value.length; i++) {
    const first = profile.value[i]
    if (!first) continue
    let last = first
    while (i + 1 < profile.value.length && profile.value[i + 1]?.altitude === first.altitude)
      last = profile.value[++i]!
    labels.push({ x: (first.x + last.x) / 2, y: first.y, altitude: first.altitude })
  }
  return labels
})
const profileAreas = computed(() => {
  const areas: string[] = []
  let group: { x: number; y: number }[] = []
  const flush = () => {
    if (group.length > 1)
      areas.push(
        `M${group[0]!.x},145 ` +
          group.map((p) => `L${p.x},${p.y}`).join(' ') +
          ` L${group.at(-1)!.x},145 Z`,
      )
    group = []
  }
  for (const point of profile.value) {
    if (point) group.push(point)
    else flush()
  }
  flush()
  return areas
})
const profileLines = computed(() =>
  profile.value.flatMap((p, i) =>
    p && profile.value[i - 1] ? [{ a: profile.value[i - 1]!, b: p }] : [],
  ),
)
</script>
<template>
  <FlightPlanRouteMap :points="points" />
  <section class="space-y-2">
    <h3 class="font-semibold">{{ label('profile') }}</h3>
    <svg
      v-if="profile.some(Boolean)"
      viewBox="0 0 800 180"
      class="w-full rounded-lg border border-gray-400/20 text-blue-600 dark:text-blue-400"
      role="img"
      :aria-label="label('profile')"
    >
      <defs>
        <linearGradient id="flight-altitude-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="#3b82f6" stop-opacity=".22" />
          <stop offset="100%" stop-color="#3b82f6" stop-opacity=".02" />
        </linearGradient>
      </defs>
      <path
        v-for="y in [25, 65, 105]"
        :key="y"
        :d="`M50 ${y} H760`"
        stroke="currentColor"
        stroke-opacity=".08"
      />
      <path
        v-for="(area, i) in profileAreas"
        :key="i"
        :d="area"
        fill="url(#flight-altitude-fill)"
      />
      <path d="M50 20 V145 H760" fill="none" stroke="currentColor" opacity=".3" />
      <line
        v-for="(line, i) in profileLines"
        :key="i"
        :x1="line.a.x"
        :y1="line.a.y"
        :x2="line.b.x"
        :y2="line.b.y"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
      />
      <template v-for="(point, i) in profile" :key="i">
        <g v-if="point">
          <circle
            :cx="point.x"
            :cy="point.y"
            r="3"
            fill="currentColor"
            stroke="white"
            stroke-width="1"
          >
            <title>{{ point.id }} · {{ point.altitude }} ft</title>
          </circle>
        </g>
      </template>
      <text
        v-for="(point, i) in altitudeLabels"
        :key="i"
        data-altitude-label
        :x="point.x"
        :y="point.y - 7"
        text-anchor="middle"
        font-size="10"
        fill="currentColor"
      >
        {{ point.altitude.toLocaleString() }} ft
      </text>
      <text x="50" y="170" font-size="11" fill="currentColor">0 NM</text>
      <text x="750" y="170" text-anchor="end" font-size="11" fill="currentColor">
        {{ segments.at(-1)?.cumulative?.toFixed(1) }} NM
      </text>
    </svg>
    <p v-else class="py-4 text-sm text-gray-500">{{ label('unknown') }}</p>
  </section>
</template>
<style scoped>
svg,
svg * {
  user-select: none;
  -webkit-user-select: none;
}
</style>
