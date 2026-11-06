<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'

interface FlightPlan {
  name: string
  size: number
  modified: number
}
const { t } = useI18n()
const app = useAppStore()
const modal = useModalStore()
const plans = ref<FlightPlan[]>([])
const query = ref('')
const busy = ref(false)
const errors = ref<string[]>([])
let generation = 0
const filtered = computed(() =>
  plans.value.filter((p) => p.name.toLowerCase().includes(query.value.toLowerCase())),
)
async function refresh() {
  const current = ++generation
  const root = app.xplanePath
  plans.value = []
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
        class="rounded-lg bg-red-500/10 p-3 text-red-600 break-words"
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
        <li v-for="plan in filtered" :key="plan.name" class="flex items-center gap-4 py-3">
          <div class="min-w-0 flex-1">
            <p class="break-words font-medium">{{ plan.name }}</p>
            <p class="text-xs text-gray-500">
              {{ (plan.size / 1024).toFixed(1) }} KB ·
              {{ new Date(plan.modified * 1000).toLocaleString() }}
            </p>
          </div>
          <button
            :disabled="busy"
            class="rounded-lg px-3 py-2 text-red-500 hover:bg-red-500/10 disabled:opacity-50"
            @click="remove(plan)"
          >
            {{ t('common.delete') }}
          </button>
        </li>
      </ul>
    </template>
  </div>
</template>
