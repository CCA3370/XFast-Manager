<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { getCurrentAiracCycle, getNavdataCycleStatus } from '@/utils/airac'
import { readPlan, legs, type PlanDetails } from '@/services/flightPlanDetails'
import FlightPlanDetailsModal from '@/components/FlightPlanDetailsModal.vue'

interface FlightPlan {
  name: string
  size: number
  modified: number
}
const { t } = useI18n()
const app = useAppStore()
const modal = useModalStore()
const plans = ref<FlightPlan[]>([])
const expanded = ref<string | null>(null)
const selected = ref<string | null>(null)
const details = ref<Record<string, PlanDetails>>({})
const loading = ref<string | null>(null)
async function inspect(name: string, full = false) {
  if (!full && expanded.value === name) {
    expanded.value = null
    return
  }
  const request = ++inspection
  const current = generation
  const root = app.xplanePath
  if (!details.value[name]) {
    loading.value = name
    try {
      const result = await readPlan(root, name)
      if (current !== generation) return
      details.value[name] = result
    } catch (e) {
      if (current === generation) errors.value = [String(e)]
      return
    } finally {
      if (current === generation) loading.value = null
    }
  }
  if (request !== inspection) return
  if (full) selected.value = name
  else expanded.value = name
}
function total(plan: PlanDetails) {
  const route = legs(plan.points)
  return route.length && route.every((l) => l.nm !== null)
    ? route.reduce((sum, l) => sum + (l.nm ?? 0), 0).toFixed(1) + ' NM'
    : '—'
}
const query = ref('')
const busy = ref(false)
const errors = ref<string[]>([])
let generation = 0
let inspection = 0
const departure = ref(''),
  arrival = ref(''),
  cycleFilter = ref('')
const index = ref<Record<string, { departure: string; arrival: string; cycle: string }>>({})
const indexing = ref(false)
const now = ref(new Date())
const filtered = computed(() =>
  plans.value.filter((plan) => {
    const meta = index.value[plan.name]
    return (
      plan.name.toLowerCase().includes(query.value.toLowerCase()) &&
      (!departure.value ||
        meta?.departure.toUpperCase().includes(departure.value.trim().toUpperCase())) &&
      (!arrival.value ||
        meta?.arrival.toUpperCase().includes(arrival.value.trim().toUpperCase())) &&
      (!cycleFilter.value ||
        (getNavdataCycleStatus(meta?.cycle, now.value) === 'future'
          ? 'unknown'
          : (getNavdataCycleStatus(meta?.cycle, now.value) ?? 'unknown')) === cycleFilter.value)
    )
  }),
)
async function buildIndex(root: string, list: FlightPlan[], current: number) {
  indexing.value = true
  for (let offset = 0; offset < list.length; offset += 4) {
    if (current !== generation) return
    const batch = await Promise.all(
      list.slice(offset, offset + 4).map(async (plan) => {
        try {
          const parsed = await readPlan(root, plan.name)
          return [
            plan.name,
            {
              departure: parsed.fields.ADEP || parsed.fields.DEP || parsed.points[0]?.id || '',
              arrival: parsed.fields.ADES || parsed.fields.DES || parsed.points.at(-1)?.id || '',
              cycle: parsed.fields.CYCLE || '',
            },
          ] as const
        } catch {
          return [plan.name, { departure: '', arrival: '', cycle: '' }] as const
        }
      }),
    )
    if (current !== generation) return
    index.value = { ...index.value, ...Object.fromEntries(batch) }
  }
  if (current === generation) {
    indexing.value = false
  }
}
async function refresh() {
  inspection++
  const current = ++generation
  const root = app.xplanePath
  plans.value = []
  details.value = {}
  index.value = {}
  indexing.value = false
  now.value = new Date()
  expanded.value = selected.value = loading.value = null
  errors.value = []
  if (!root) return
  try {
    const result = await invoke<FlightPlan[]>('list_flight_plans', { xplanePath: root })
    if (current === generation) {
      plans.value = result
      void buildIndex(root, result, current)
    }
  } catch (e) {
    if (current === generation) errors.value = [String(e)]
  }
}
function remove(plan: FlightPlan) {
  const root = app.xplanePath
  modal.showConfirm({
    title: t('common.delete'),
    message: t('flightPlans.confirmDelete', { name: plan.name }),
    confirmText: t('common.delete'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onCancel: () => {},
    onConfirm: async () => {
      if (busy.value) return
      busy.value = true
      errors.value = []
      try {
        await invoke('delete_flight_plan', { xplanePath: root, name: plan.name })
        await refresh()
      } catch (e) {
        errors.value = [String(e)]
      } finally {
        busy.value = false
      }
    },
  })
}
watch(
  () => app.xplanePath,
  () => {
    errors.value = []
    departure.value = arrival.value = cycleFilter.value = ''
    void refresh()
  },
)
onMounted(() => void refresh())
onBeforeUnmount(() => {
  generation++
})
</script>

<template>
  <div class="p-6 space-y-4 text-gray-900 dark:text-gray-100">
    <header class="flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0">
        <h1 class="text-2xl font-bold">{{ t('flightPlans.title') }}</h1>
        <p class="mt-1 text-sm text-gray-500">{{ t('flightPlans.description') }}</p>
      </div>
    </header>
    <p v-if="!app.xplanePath">{{ t('flightPlans.selectPath') }}</p>
    <template v-else>
      <div
        v-if="errors.length"
        role="alert"
        class="select-text rounded-lg bg-red-500/10 p-3 text-red-600 break-words"
      >
        <p v-for="(error, i) in errors" :key="i">{{ error }}</p>
      </div>
      <div class="plan-filter-bar" data-testid="plan-filters">
        <input
          v-model="query"
          :placeholder="t('flightPlans.search')"
          :aria-label="t('flightPlans.search')"
          class="min-w-0 rounded-lg border border-gray-400/30 bg-transparent p-2"
        />
        <input
          v-model="departure"
          data-testid="departure-filter"
          :aria-label="t('flightPlanDetails.departure')"
          :placeholder="t('flightPlanDetails.from') + ' ICAO'"
          spellcheck="false"
          autocapitalize="characters"
          class="uppercase"
        />
        <input
          v-model="arrival"
          data-testid="arrival-filter"
          :aria-label="t('flightPlanDetails.destination')"
          :placeholder="t('flightPlanDetails.to') + ' ICAO'"
          spellcheck="false"
          autocapitalize="characters"
          class="uppercase"
        />
        <select
          v-model="cycleFilter"
          :disabled="indexing"
          :aria-label="'AIRAC ' + getCurrentAiracCycle(now)"
          class="min-w-0 rounded-lg border border-gray-400/30 bg-white p-2 text-sm dark:bg-gray-900"
        >
          <option value="">{{ t('flightPlanDetails.allCycles') }}</option>
          <option v-for="state in ['current', 'outdated', 'unknown']" :key="state" :value="state">
            {{ t('flightPlanDetails.' + (state === 'unknown' ? 'unknownCycle' : state)) }}
          </option>
        </select>
        <button
          :disabled="busy"
          class="rounded-lg border border-gray-400/30 px-4 disabled:opacity-50"
          @click="refresh"
        >
          {{ t('flightPlans.refresh') }}
        </button>
      </div>
      <p v-if="!filtered.length" class="py-8 text-center text-gray-500">
        {{ t('flightPlans.empty') }}
      </p>
      <ul class="divide-y divide-gray-400/20">
        <li v-for="plan in filtered" :key="plan.name" class="py-3">
          <div class="flex items-center gap-3">
            <button
              :aria-expanded="expanded === plan.name"
              :disabled="loading === plan.name"
              class="min-w-0 flex-1 text-left rounded-md px-2 py-2 hover:bg-gray-500/5"
              @click="inspect(plan.name)"
            >
              <div class="min-w-0 flex-1">
                <p class="break-words font-medium">
                  {{ expanded === plan.name ? '▾' : '▸' }} {{ plan.name }}
                  <span
                    v-if="getNavdataCycleStatus(index[plan.name]?.cycle, now) === 'future'"
                    class="ml-2 text-xs font-normal text-amber-600"
                    >{{ t('flightPlanDetails.future') }}</span
                  >
                </p>
                <p class="text-xs text-gray-500">
                  {{ (plan.size / 1024).toFixed(1) }} KB ·
                  {{ new Date(plan.modified * 1000).toLocaleString() }}
                </p>
              </div>
            </button>
            <button
              class="rounded-lg border border-gray-400/30 px-3 py-2 text-sm"
              :disabled="loading === plan.name"
              @click="inspect(plan.name, true)"
            >
              {{ t('flightPlanDetails.details') }}
            </button>
            <button
              data-testid="delete-plan"
              :disabled="busy"
              class="rounded-lg px-3 py-2 text-red-500 hover:bg-red-500/10 disabled:opacity-50"
              @click="remove(plan)"
            >
              {{ t('common.delete') }}
            </button>
          </div>
          <div
            v-if="expanded === plan.name && details[plan.name]"
            class="mt-2 grid grid-cols-2 gap-3 rounded-lg bg-gray-500/5 p-4 text-sm sm:grid-cols-4"
          >
            <div>
              <p class="text-xs text-gray-500">{{ t('flightPlanDetails.departure') }}</p>
              <p class="mt-1 font-mono">
                {{
                  details[plan.name]!.fields.ADEP ||
                  details[plan.name]!.fields.DEP ||
                  details[plan.name]!.points[0]?.id ||
                  '—'
                }}
              </p>
            </div>
            <div>
              <p class="text-xs text-gray-500">{{ t('flightPlanDetails.destination') }}</p>
              <p class="mt-1 font-mono">
                {{
                  details[plan.name]!.fields.ADES ||
                  details[plan.name]!.fields.DES ||
                  details[plan.name]!.points.at(-1)?.id ||
                  '—'
                }}
              </p>
            </div>
            <div>
              <p class="text-xs text-gray-500">{{ t('flightPlanDetails.waypoints') }}</p>
              <p class="mt-1">
                {{ details[plan.name]!.points.length }} · {{ total(details[plan.name]!) }}
              </p>
            </div>
            <div>
              <p class="text-xs text-gray-500">AIRAC</p>
              <p class="mt-1">
                {{ details[plan.name]!.fields.CYCLE || '—' }}
              </p>
            </div>
          </div>
        </li>
      </ul>
    </template>
    <FlightPlanDetailsModal
      v-if="selected && details[selected]"
      :key="selected"
      :name="selected"
      :root="app.xplanePath"
      :plan="details[selected]!"
      @close="selected = null"
    />
  </div>
</template>

<style scoped>
.plan-filter-bar {
  --filter-surface: #fff;
  --filter-text: #1f2937;
  --filter-border: #cbd5e1;
  --filter-placeholder: #64748b;
  --filter-scheme: light;
  display: grid;
  grid-template-columns:
    minmax(120px, 1fr) repeat(2, clamp(100px, 11vw, 150px)) clamp(110px, 12vw, 150px)
    76px;
  gap: 0.5rem;
  padding: 0.5rem;
  border: 1px solid rgb(148 163 184 / 25%);
  border-radius: 0.75rem;
  background: #f1f5f9;
}
:global(.dark .plan-filter-bar) {
  --filter-surface: #111827;
  --filter-text: #f3f4f6;
  --filter-border: #374151;
  --filter-placeholder: #9ca3af;
  --filter-scheme: dark;
  background: #192230;
}
.plan-filter-bar > input,
.plan-filter-bar > select,
.plan-filter-bar > button {
  box-sizing: border-box;
  height: 42px;
  min-width: 0;
  border: 1px solid var(--filter-border);
  border-radius: 0.5rem;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  line-height: 1.25rem;
  color: var(--filter-text);
  background: var(--filter-surface);
  color-scheme: var(--filter-scheme);
}
.plan-filter-bar > input::placeholder {
  color: var(--filter-placeholder);
  opacity: 1;
}
.plan-filter-bar > select {
  appearance: none;
  padding-right: 2.25rem;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 16 16'%3E%3Cpath d='m4 6 4 4 4-4' fill='none' stroke='%2364748b' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 0.75rem center;
  background-size: 1rem 1rem;
}
:global(.dark .plan-filter-bar > select) {
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 16 16'%3E%3Cpath d='m4 6 4 4 4-4' fill='none' stroke='%239ca3af' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
}
.plan-filter-bar > button {
  font-weight: 600;
  min-width: 76px;
  white-space: nowrap;
}
.plan-filter-bar > button:hover {
  border-color: #3b82f6;
}
.plan-filter-bar > :focus-visible {
  outline: 2px solid rgb(59 130 246 / 35%);
  outline-offset: 1px;
}
</style>
