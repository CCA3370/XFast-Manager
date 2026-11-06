<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import countries from '@/assets/geography/countries.json'
import { legs, type Waypoint } from '@/services/flightPlanDetails'
const props = defineProps<{ points: Waypoint[] }>()
const { t } = useI18n()
const label = (key: string) => t(`flightPlanDetails.${key}`)
const zoom = ref(1)
const offset = ref({ x: 0, y: 0 })
let drag: { x: number; y: number; ox: number; oy: number } | null = null
const segments = computed(() => legs(props.points))
const positioned = computed(() => {
  let previous: number | null = null
  return props.points.map((p, i) => {
    if (p.lat === null || p.lon === null) return null
    let lon = p.lon
    if (previous !== null) {
      while (lon - previous > 180) lon -= 360
      while (lon - previous < -180) lon += 360
    }
    previous = lon
    return { x: lon, y: -p.lat, id: p.id, i }
  })
})
const projection = computed(() => {
  const all = positioned.value.filter((p) => p !== null)
  const xs = all.map((p) => p.x),
    ys = all.map((p) => p.y)
  if (!all.length) return { cx: 0, cy: 0, factor: 1, scale: 1 }
  const factor = Math.max(
    0.05,
    Math.cos((((Math.min(...ys) + Math.max(...ys)) / 2) * Math.PI) / 180),
  )
  return {
    cx: (Math.min(...xs) + Math.max(...xs)) / 2,
    cy: (Math.min(...ys) + Math.max(...ys)) / 2,
    factor,
    scale: Math.min(
      680 / Math.max(0.1, (Math.max(...xs) - Math.min(...xs)) * factor),
      240 / Math.max(0.1, Math.max(...ys) - Math.min(...ys)),
    ),
  }
})
function project(lon: number, lat: number) {
  const { cx, cy, factor, scale } = projection.value
  return { x: 400 + (lon - cx) * factor * scale, y: 160 + (-lat - cy) * scale }
}
const projected = computed(() =>
  positioned.value.map((p) => (p ? { ...p, ...project(p.x, -p.y) } : null)),
)
interface Country {
  name: string
  type: string
  coordinates: number[][][] | number[][][][]
}
const basemap = computed(() => {
  const center = Math.round(projection.value.cx / 360) * 360
  return (countries as Country[]).flatMap((country) => {
    const polygons =
      country.type === 'Polygon'
        ? [country.coordinates as number[][][]]
        : (country.coordinates as number[][][][])
    return [-360, 0, 360].map((shift) => ({
      name: country.name,
      path: polygons
        .map((polygon) =>
          polygon
            .map(
              (ring) =>
                ring
                  .map((p, i) => {
                    const q = project(p[0]! + center + shift, p[1]!)
                    return `${i ? 'L' : 'M'}${q.x.toFixed(2)},${q.y.toFixed(2)}`
                  })
                  .join(' ') + 'Z',
            )
            .join(' '),
        )
        .join(' '),
    }))
  })
})
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
const routeLines = computed(() =>
  projected.value.flatMap((p, i) =>
    p && projected.value[i - 1] ? [{ a: projected.value[i - 1]!, b: p }] : [],
  ),
)
const profileLines = computed(() =>
  profile.value.flatMap((p, i) =>
    p && profile.value[i - 1] ? [{ a: profile.value[i - 1]!, b: p }] : [],
  ),
)
function down(e: PointerEvent) {
  const svg = e.currentTarget as SVGSVGElement
  svg.setPointerCapture(e.pointerId)
  drag = { x: e.clientX, y: e.clientY, ox: offset.value.x, oy: offset.value.y }
}
function move(e: PointerEvent) {
  if (!drag) return
  const bounds = (e.currentTarget as SVGSVGElement).getBoundingClientRect()
  offset.value = {
    x: drag.ox + ((e.clientX - drag.x) * 800) / bounds.width,
    y: drag.oy + ((e.clientY - drag.y) * 320) / bounds.height,
  }
}
function fit() {
  zoom.value = 1
  offset.value = { x: 0, y: 0 }
}
</script>
<template>
  <section class="space-y-2">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <h3 class="font-semibold">{{ label('route') }}</h3>
      <div class="flex gap-1 text-sm">
        <button
          class="chart-button"
          :aria-label="label('zoomOut')"
          @click="zoom = Math.max(0.5, zoom / 1.4)"
          v-text="'−'"
        /><button
          class="chart-button"
          :aria-label="label('zoomIn')"
          @click="zoom = Math.min(12, zoom * 1.4)"
          v-text="'+'"
        /><button class="chart-button" @click="fit">{{ label('fit') }}</button>
      </div>
    </div>
    <svg
      v-if="projected.some(Boolean)"
      viewBox="0 0 800 320"
      class="w-full touch-none cursor-grab rounded-lg border border-gray-400/20 bg-slate-50 text-blue-600 dark:bg-gray-950 dark:text-blue-400"
      role="img"
      :aria-label="label('route')"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="drag = null"
      @pointercancel="drag = null"
    >
      <defs>
        <pattern id="flight-route-grid" width="40" height="40" patternUnits="userSpaceOnUse">
          <path d="M 40 0 L 0 0 0 40" fill="none" stroke="currentColor" opacity=".09" />
        </pattern>
      </defs>
      <rect width="800" height="320" class="ocean" />
      <rect width="800" height="320" fill="url(#flight-route-grid)" />
      <g
        :transform="`translate(${400 + offset.x} ${160 + offset.y}) scale(${zoom}) translate(-400 -160)`"
      >
        <path
          v-for="(country, i) in basemap"
          :key="i"
          :d="country.path"
          class="land"
          fill-rule="evenodd"
          vector-effect="non-scaling-stroke"
        >
          <title>{{ country.name }}</title>
        </path>
        <line
          v-for="(line, i) in routeLines"
          :key="i"
          :x1="line.a.x"
          :y1="line.a.y"
          :x2="line.b.x"
          :y2="line.b.y"
          stroke="currentColor"
          stroke-width="2"
          vector-effect="non-scaling-stroke"
        />
        <template v-for="(point, i) in projected" :key="i">
          <g v-if="point">
            <circle :cx="point.x" :cy="point.y" r="3" fill="currentColor">
              <title>{{ i + 1 }} · {{ point.id }}</title>
            </circle>
            <text
              v-if="projected.length < 35 || i === 0 || i === projected.length - 1"
              :x="point.x + 6"
              :y="point.y - 7"
              font-size="11"
              fill="currentColor"
            >
              {{ point.id }}
            </text>
          </g>
        </template>
      </g>
      <text x="788" y="306" text-anchor="end" font-size="9" fill="currentColor">
        Natural Earth · 1:110m
      </text>
      <text x="16" y="24" font-size="12" fill="currentColor">N ↑</text>
    </svg>
    <p v-else class="py-6 text-sm text-gray-500">{{ label('empty') }}</p>
    <p class="text-xs leading-5 text-gray-500">{{ label('scope') }}</p>
  </section>
  <section class="space-y-2">
    <h3 class="font-semibold">{{ label('profile') }}</h3>
    <svg
      v-if="profile.some(Boolean)"
      viewBox="0 0 800 180"
      class="w-full rounded-lg border border-gray-400/20 text-blue-600 dark:text-blue-400"
      role="img"
      :aria-label="label('profile')"
    >
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
      />
      <template v-for="(point, i) in profile" :key="i">
        <g v-if="point">
          <circle :cx="point.x" :cy="point.y" r="3" fill="currentColor">
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
    <p class="text-xs text-gray-500">{{ label('altHint') }}</p>
  </section>
</template>
<style scoped>
svg,
svg * {
  user-select: none;
  -webkit-user-select: none;
}
.ocean {
  fill: #e6eff5;
}
.land {
  fill: #f3f2e9;
  stroke: #a3b2ab;
  stroke-width: 0.65;
}
:global(.dark) .ocean {
  fill: #162736;
}
:global(.dark) .land {
  fill: #293a39;
  stroke: #536768;
}

.chart-button {
  border: 1px solid #94a3b84d;
  border-radius: 0.375rem;
  padding: 0.2rem 0.65rem;
}
</style>
