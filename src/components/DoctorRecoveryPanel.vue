<template>
  <details
    class="group border-t border-gray-100 dark:border-gray-800"
    data-testid="health-recovery"
  >
    <summary
      class="cursor-pointer px-4 py-3 text-sm font-semibold text-gray-700 dark:text-gray-200"
    >
      {{ copy.title }}
      <span v-if="store.backups.length" class="ml-1 text-gray-400">{{ store.backups.length }}</span>
    </summary>
    <div class="space-y-3 border-t border-gray-100 px-4 py-3 dark:border-gray-800">
      <div class="flex flex-wrap gap-2">
        <button
          v-for="action in actions"
          :key="action.id"
          type="button"
          :data-testid="`health-repair-${action.id}`"
          :disabled="!store.canRepairCurrentRun || !action.available || store.isBusy"
          :title="copy[`${action.id}Body`]"
          class="rounded-md border border-gray-300 px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40 dark:border-gray-700 dark:text-gray-200 dark:hover:bg-gray-800"
          @click="confirmRepair(action.id)"
        >
          {{ copy[action.id] }}
        </button>
      </div>
      <p class="text-xs leading-5 text-gray-500 dark:text-gray-400">{{ copy.backupNote }}</p>
      <p v-if="store.backupError" role="alert" class="text-xs text-red-600">
        {{ copy.backupError }}
      </p>
      <ul
        v-else-if="store.backups.length"
        class="max-h-52 divide-y divide-gray-100 overflow-y-auto dark:divide-gray-800"
      >
        <li
          v-for="backup in store.backups"
          :key="backup.id"
          class="flex items-center justify-between gap-3 py-2"
        >
          <div class="min-w-0">
            <p class="truncate text-xs font-medium text-gray-700 dark:text-gray-200">
              {{ actionLabel(backup.action) }}
            </p>
            <p class="text-xs tabular-nums text-gray-500">
              {{ new Date(backup.createdAt).toLocaleString(locale) }}
            </p>
          </div>
          <button
            type="button"
            :data-testid="`health-restore-${backup.id}`"
            :disabled="!store.canRepairCurrentRun || store.isBusy"
            class="rounded-md border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40 dark:border-gray-700 dark:text-gray-200 dark:hover:bg-gray-800"
            @click="confirmRestore(backup.id)"
          >
            {{ copy.restore }}
          </button>
        </li>
      </ul>
      <p v-else class="text-xs text-gray-400">{{ copy.empty }}</p>
    </div>
  </details>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDoctorStore } from '@/stores/doctor'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { HEALTH_RECOVERY_COPY } from '@/utils/doctorFindings'

type RepairAction = 'disable_injectors' | 'rebuild_shader_cache' | 'reset_preferences'
const { t, locale } = useI18n()
const store = useDoctorStore()
const modal = useModalStore()
const toast = useToastStore()
const copy = computed(() => HEALTH_RECOVERY_COPY[locale.value === 'zh' ? 'zh' : 'en'])
const actions = computed<{ id: RepairAction; available: boolean }[]>(() => [
  {
    id: 'disable_injectors',
    available:
      store.displayedRun?.checks.some(
        (check) => check.id === 'system.injectors' && check.outcome === 'warning',
      ) ?? false,
  },
  { id: 'rebuild_shader_cache', available: store.inspectionReport?.shaderCachePresent ?? false },
  { id: 'reset_preferences', available: store.inspectionReport?.preferencesPresent ?? false },
])

function actionLabel(action: string): string {
  return action in copy.value ? copy.value[action as RepairAction] : action
}

function confirmRepair(action: RepairAction) {
  modal.showConfirm({
    title: copy.value[action],
    message: copy.value[`${action}Body`],
    confirmText: copy.value.confirm,
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => void perform(action),
    onCancel: () => {},
  })
}

function confirmRestore(id: string) {
  modal.showConfirm({
    title: copy.value.restore,
    message: copy.value.restoreBody,
    confirmText: copy.value.restore,
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => void perform('restore_health_backup', id),
    onCancel: () => {},
  })
}

async function perform(action: string, backupId?: string) {
  if (await store.runMaintenance(action, backupId))
    toast.success(t('doctor.center.messages.fixSuccess'))
  else
    toast.error(
      t(
        store.error === 'xplane_running'
          ? 'doctor.center.messages.xplaneRunning'
          : 'doctor.center.messages.fixFailed',
      ),
    )
}
</script>
