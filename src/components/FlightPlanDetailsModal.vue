<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import FlightPlanCharts from './FlightPlanCharts.vue'
import { enrichPlan, legs, type PlanDetails, type NavLookup } from '@/services/flightPlanDetails'
const props = defineProps<{ name: string; root: string; plan: PlanDetails }>()
const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()
const label = (key: string) => t(`flightPlanDetails.${key}`)
const dialog = ref<HTMLDialogElement | null>(null)
const plan = ref(props.plan)
const tab = ref('route')
const procedureFields = computed(() =>
  Object.fromEntries(
    Object.entries(plan.value.fields).filter(
      ([key]) => !['ADEP', 'ADES', 'DEP', 'DES', 'CYCLE'].includes(key),
    ),
  ),
)
const busy = ref(false)
const lookup = ref<NavLookup | null>(null)
const error = ref('')
let disposed = false
const routeLegs = computed(() => legs(plan.value.points))
const known = computed(() => routeLegs.value.flatMap((l) => (l.nm === null ? [] : [l.nm])))
const total = computed(() => known.value.reduce((a, b) => a + b, 0))
const stats = computed(() =>
  known.value.length
    ? {
        longest: Math.max(...known.value),
        shortest: Math.min(...known.value),
        average: total.value / known.value.length,
      }
    : null,
)
async function navigation() {
  busy.value = true
  error.value = ''
  try {
    const result = await invoke<NavLookup>('lookup_flight_plan_navigation', {
      xplanePath: props.root,
      identifiers: [...new Set(plan.value.points.map((p) => p.id))],
    })
    if (disposed) return
    lookup.value = result
    plan.value = enrichPlan(props.plan, result)
  } catch {
    if (!disposed) error.value = label('navFailed')
  } finally {
    busy.value = false
  }
}
onMounted(() => {
  dialog.value?.showModal()
  if (
    plan.value.points.some((p) => [2, 3, 11].includes(p.kind) && (p.lat === null || p.lon === null))
  )
    void navigation()
})
onBeforeUnmount(() => {
  disposed = true
  dialog.value?.close()
})
</script>
<template>
  <Teleport to="body">
    <dialog
      ref="dialog"
      aria-labelledby="flight-plan-detail-title"
      class="flight-detail m-auto w-[min(1100px,96vw)] max-w-none max-h-[92vh] rounded-xl border border-gray-300 bg-white p-0 text-gray-900 shadow-xl dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100"
      @cancel.prevent="emit('close')"
    >
      <div class="flex max-h-[92vh] flex-col">
        <header
          class="flex shrink-0 items-start justify-between gap-3 border-b border-gray-400/20 px-5 py-4"
        >
          <div class="min-w-0">
            <h2 id="flight-plan-detail-title" class="break-all text-lg font-semibold">
              {{ name }}
            </h2>
            <p class="mt-1 text-sm text-gray-500">
              <template v-if="plan.fields.CYCLE">AIRAC {{ plan.fields.CYCLE }} · </template>
              {{ plan.points.length }} {{ label('waypoints') }}
            </p>
          </div>
          <button
            class="rounded-md border border-gray-400/30 px-3 py-1.5 text-sm"
            @click="emit('close')"
          >
            {{ t('common.close') }}
          </button>
        </header>
        <nav
          class="flex shrink-0 gap-1 overflow-x-auto border-b border-gray-400/20 px-5"
          :aria-label="label('details')"
        >
          <button
            v-for="item in ['route', 'legs', 'waypoints', 'raw']"
            :key="item"
            class="whitespace-nowrap border-b-2 px-3 py-3 text-sm"
            :class="
              tab === item ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-500'
            "
            :aria-pressed="tab === item"
            @click="tab = item"
          >
            {{ label(item) }}
          </button>
        </nav>
        <div class="min-h-0 overflow-y-auto p-5 space-y-5">
          <template v-if="tab === 'route'">
            <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
              <div
                v-for="(value, key) in {
                  departure: plan.fields.ADEP || plan.fields.DEP || plan.points[0]?.id || '—',
                  destination: plan.fields.ADES || plan.fields.DES || plan.points.at(-1)?.id || '—',
                  distance: total.toFixed(1) + ' NM',
                  waypoints: plan.points.length,
                }"
                :key="key"
                class="rounded-lg bg-gray-500/5 px-3 py-2"
              >
                <p class="text-xs text-gray-500">{{ label(key) }}</p>
                <p class="mt-1 font-semibold tabular-nums">{{ value }}</p>
              </div>
            </div>
            <p v-if="known.length !== routeLegs.length" class="text-sm text-amber-600">
              {{ label('partial') }}
            </p>
            <dl
              v-if="Object.keys(procedureFields).length"
              class="flex flex-wrap gap-x-5 gap-y-2 text-sm"
            >
              <div v-for="(value, key) in procedureFields" :key="key" class="flex gap-2">
                <dt class="text-gray-500">{{ key }}</dt>
                <dd class="font-mono">{{ value }}</dd>
              </div>
            </dl>
            <FlightPlanCharts :points="plan.points" />
            <div v-if="busy || lookup || error" class="border-t border-gray-400/20 pt-4 space-y-2">
              <p v-if="busy" class="text-sm text-gray-500">{{ t('common.loading') }}</p>
              <p v-if="error" role="alert" class="select-text text-sm text-red-600">{{ error }}</p>
              <template v-if="lookup">
                <p v-for="warning in lookup.warnings" :key="warning" class="text-xs text-amber-600">
                  {{ warning.startsWith('unreadable:') ? warning : label(warning) }}
                </p>
              </template>
            </div>
          </template>
          <template v-else-if="tab === 'legs'">
            <div v-if="stats" class="flex flex-wrap gap-5 text-sm">
              <p v-for="(value, key) in stats" :key="key">
                <span class="text-gray-500">{{ label(key) }}</span> {{ value.toFixed(1) }} NM
              </p>
            </div>
            <div class="overflow-x-auto">
              <table class="w-full whitespace-nowrap text-left text-sm">
                <thead class="text-xs text-gray-500">
                  <tr>
                    <th>#</th>
                    <th>{{ label('from') }}</th>
                    <th>{{ label('to') }}</th>
                    <th>{{ label('distance') }}</th>
                    <th>{{ label('cumulative') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(leg, i) in routeLegs" :key="i" class="border-t border-gray-400/20">
                    <td>{{ i + 1 }}</td>
                    <td class="font-mono">{{ leg.from }}</td>
                    <td class="font-mono">{{ leg.to }}</td>
                    <td>
                      {{ leg.nm?.toFixed(1) ?? '—' }} NM
                      <div
                        class="mt-1 h-1 rounded bg-blue-500/60"
                        :style="{
                          width:
                            (leg.nm === null
                              ? 0
                              : (leg.nm / Math.max(stats?.longest ?? 1, 1)) * 100) + '%',
                        }"
                      />
                    </td>
                    <td>{{ leg.cumulative?.toFixed(1) ?? '—' }} NM</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>
          <template v-else-if="tab === 'waypoints'">
            <h3 class="font-semibold">{{ label('waypoints') }}</h3>
            <div class="overflow-x-auto">
              <table class="w-full whitespace-nowrap text-left text-sm">
                <thead class="text-xs text-gray-500">
                  <tr>
                    <th>#</th>
                    <th>{{ label('waypoints') }}</th>
                    <th>{{ label('via') }}</th>
                    <th>{{ label('altitude') }}</th>
                    <th>{{ label('coordinates') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(point, i) in plan.points"
                    :key="i"
                    class="border-t border-gray-400/20"
                  >
                    <td>{{ i + 1 }}</td>
                    <td>
                      <span class="font-mono">{{ point.id }}</span>
                      <p v-if="point.name" class="text-xs text-gray-500">{{ point.name }}</p>
                      <p v-if="point.source" class="text-xs text-gray-500">{{ point.source }}</p>
                    </td>
                    <td>{{ point.via }}</td>
                    <td>
                      {{ point.altitude === null ? '—' : point.altitude.toLocaleString() + ' ft' }}
                    </td>
                    <td class="font-mono text-xs">
                      {{ point.lat?.toFixed(5) ?? '—' }}, {{ point.lon?.toFixed(5) ?? '—' }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>
          <pre
            v-else
            class="select-text overflow-x-auto rounded-lg bg-gray-500/5 p-4 font-mono text-xs leading-6"
            tabindex="0"
            >{{ plan.raw }}</pre
          >
          <details
            v-if="plan.issues.length"
            class="rounded-lg border border-amber-500/30 p-3 text-sm"
          >
            <summary class="cursor-pointer text-amber-600">
              {{ label('issues') }} · {{ plan.issues.length }}
            </summary>
            <p v-for="(issue, i) in plan.issues" :key="i" class="mt-2">
              {{ label(issue.split(':')[0] === 'coordinates' ? 'row' : issue.split(':')[0]!)
              }}{{ issue.includes(':') ? ' · ' + issue.split(':')[1] : '' }}
            </p>
          </details>
        </div>
      </div>
    </dialog>
    <!-- End inspector -->
  </Teleport>
</template>
<style scoped>
.flight-detail::backdrop {
  background: rgb(0 0 0 / 45%);
}
th,
td {
  padding: 0.65rem 0.6rem;
}
</style>
