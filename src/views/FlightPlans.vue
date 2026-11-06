<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
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
const filtered = computed(() =>
  plans.value.filter((p) => p.name.toLowerCase().includes(query.value.toLowerCase())),
)
async function refresh() {
  inspection++
  const current = ++generation
  const root = app.xplanePath
  plans.value = []
  details.value = {}
  expanded.value = selected.value = loading.value = null
  errors.value = []
  if (!root) return
  try {
    const result = await invoke<FlightPlan[]>('list_flight_plans', { xplanePath: root })
    if (current === generation) plans.value = result
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
    void refresh()
  },
)
onMounted(() => void refresh())
onBeforeUnmount(() => {
  generation++
})
</script>

<template>
  <div class="p-6 space-y-4">
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
      <div class="flex gap-3">
        <input
          v-model="query"
          :placeholder="t('flightPlans.search')"
          :aria-label="t('flightPlans.search')"
          class="flex-1 rounded-lg border border-gray-400/30 bg-transparent p-2"
        />
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
              <p class="text-xs text-gray-500">AIRAC / FMS</p>
              <p class="mt-1">
                {{ details[plan.name]!.fields.CYCLE || '—' }} /
                {{ details[plan.name]!.version || '—' }}
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
