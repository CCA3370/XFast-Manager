<template>
  <Teleport to="body">
    <dialog
      v-if="conflict"
      ref="dialog"
      data-testid="flight-plan-conflict"
      aria-labelledby="flight-plan-conflict-title"
      aria-describedby="flight-plan-conflict-description"
      class="m-auto w-full max-w-md rounded-xl border border-gray-200 bg-white p-0 text-gray-900 shadow-xl dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100"
      @cancel.prevent="choose('cancel')"
      @click="onBackdrop"
    >
      <div class="space-y-4 p-5">
        <h2 id="flight-plan-conflict-title" class="text-base font-semibold">
          {{ t('flightPlans.conflictTitle') }}
        </h2>
        <p class="break-all rounded-md bg-gray-50 px-3 py-2 font-mono text-sm dark:bg-gray-800">
          {{ conflict.name }}
        </p>
        <p
          id="flight-plan-conflict-description"
          class="text-sm leading-6 text-gray-600 dark:text-gray-300"
        >
          {{ t('flightPlans.conflictDescription') }}
        </p>
        <label
          class="flex cursor-pointer items-center gap-2 text-sm text-gray-600 dark:text-gray-300"
        >
          <input
            v-model="remember"
            data-testid="remember-flight-plan-choice"
            type="checkbox"
            class="h-4 w-4 accent-blue-600"
          />
          {{ t('flightPlans.rememberChoice') }}
        </label>
      </div>
      <footer
        class="flex items-center justify-end gap-2 border-t border-gray-100 px-5 py-3 dark:border-gray-800"
      >
        <button
          type="button"
          data-testid="cancel-flight-plan"
          class="mr-auto rounded-md border border-gray-300 px-3 py-2 text-sm dark:border-gray-700"
          @click="choose('cancel')"
        >
          {{ t('common.cancel') }}
        </button>
        <button
          ref="renameButton"
          type="button"
          data-testid="rename-flight-plan"
          class="rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40"
          @click="choose('rename')"
        >
          {{ t('flightPlans.rename') }}
        </button>
        <button
          type="button"
          data-testid="replace-flight-plan"
          class="rounded-md border border-red-300 px-3 py-2 text-sm font-medium text-red-600 hover:bg-red-50 dark:border-red-900 dark:text-red-300 dark:hover:bg-red-950"
          @click="choose('replace')"
        >
          {{ t('flightPlans.replace') }}
        </button>
      </footer>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { FlightPlanConflict, FlightPlanConflictDecision } from '@/types/flightPlans'

const props = defineProps<{ conflict: FlightPlanConflict | null }>()
const emit = defineEmits<{ resolve: [decision: FlightPlanConflictDecision] }>()
const { t } = useI18n()
const dialog = ref<HTMLDialogElement | null>(null)
const renameButton = ref<HTMLButtonElement | null>(null)
const remember = ref(false)

watch(
  () => props.conflict,
  async (value) => {
    remember.value = false
    if (value) {
      await nextTick()
      if (props.conflict !== value || !dialog.value) return
      if (!dialog.value.open) dialog.value.showModal()
      renameButton.value?.focus()
    } else dialog.value?.close()
  },
  { immediate: true },
)

function choose(action: FlightPlanConflictDecision['action']) {
  emit('resolve', { action, remember: action !== 'cancel' && remember.value })
}

function onBackdrop(event: MouseEvent) {
  const element = dialog.value
  if (!element || event.target !== element) return
  const bounds = element.getBoundingClientRect()
  if (
    event.clientX < bounds.left ||
    event.clientX > bounds.right ||
    event.clientY < bounds.top ||
    event.clientY > bounds.bottom
  )
    choose('cancel')
}

onBeforeUnmount(() => dialog.value?.close())
</script>

<style scoped>
dialog::backdrop {
  background: rgb(0 0 0 / 45%);
}
</style>
