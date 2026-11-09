<template>
  <div class="space-y-2 rounded-lg border border-gray-200 p-3 dark:border-gray-700">
    <label
      for="flight-plan-conflict-action"
      class="text-sm font-medium text-gray-700 dark:text-gray-200"
      >{{ t('flightPlans.conflictPreference') }}</label
    >
    <select
      id="flight-plan-conflict-action"
      v-model="action"
      :disabled="saving"
      class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
      @change="save"
    >
      <option value="ask">{{ t('flightPlans.askEveryTime') }}</option>
      <option value="replace">{{ t('flightPlans.replace') }}</option>
      <option value="rename">{{ t('flightPlans.rename') }}</option>
    </select>
    <p v-if="failed" role="alert" class="text-xs text-red-600">
      {{ t('flightPlans.preferenceNotSaved') }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
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
onMounted(async () => {
  try {
    action.value = saved = (await loadFlightPlanConflictAction()) ?? 'ask'
  } catch {
    failed.value = true
  } finally {
    saving.value = false
  }
})
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
