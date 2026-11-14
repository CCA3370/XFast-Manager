<template>
  <div
    class="space-y-2 rounded-xl border border-gray-200 bg-white p-4 transition-all duration-150 dark:border-gray-700 dark:bg-gray-800"
  >
    <label
      for="flight-plan-conflict-action"
      class="block text-sm font-medium text-gray-900 dark:text-gray-100"
    >
      {{ t('flightPlans.conflictPreference') }}
    </label>
    <SelectDropdown
      :model-value="action"
      :options="options"
      :disabled="saving"
      :aria-label="t('flightPlans.conflictPreference')"
      @update:model-value="onSelectChange"
    />
    <p v-if="failed" role="alert" class="text-sm text-red-700 dark:text-red-400">
      {{ t('flightPlans.preferenceNotSaved') }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import SelectDropdown from '@/components/SelectDropdown.vue'
import {
  loadFlightPlanConflictAction,
  saveFlightPlanConflictAction,
} from '@/services/flightPlanImport'
import type { FlightPlanConflictAction } from '@/types/flightPlans'

const { t } = useI18n()
const action = ref<FlightPlanConflictAction | 'ask'>('ask')
const saving = ref(true)
const failed = ref(false)
let saved: FlightPlanConflictAction | 'ask' = 'ask'

const options = computed(() => [
  { value: 'ask', label: t('flightPlans.askEveryTime') },
  { value: 'replace', label: t('flightPlans.replace') },
  { value: 'rename', label: t('flightPlans.rename') },
])

onMounted(async () => {
  try {
    action.value = saved = (await loadFlightPlanConflictAction()) ?? 'ask'
  } catch {
    failed.value = true
  } finally {
    saving.value = false
  }
})

function onSelectChange(value: string | number) {
  action.value = value as FlightPlanConflictAction | 'ask'
  void save()
}

async function save() {
  saving.value = true
  failed.value = false
  try {
    await saveFlightPlanConflictAction(action.value === 'ask' ? null : action.value)
    saved = action.value
  } catch {
    action.value = saved
    failed.value = true
  } finally {
    saving.value = false
  }
}
</script>
