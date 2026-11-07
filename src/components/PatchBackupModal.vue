<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { listPatchBackups, revertPatch } from '@/services/patch-api'
import { getErrorMessage, type PatchBackupSession } from '@/types'

const props = defineProps<{ xplanePath: string; aircraftFolder: string; aircraftName: string }>()
const emit = defineEmits<{ close: []; restored: [count: number] }>()
const { t, locale } = useI18n()
const dialog = ref<HTMLDialogElement | null>(null)
const sessions = ref<PatchBackupSession[]>([])
const selectedId = ref('')
const selected = computed(() => sessions.value.find((session) => session.id === selectedId.value))
const size = computed(
  () => selected.value?.files.reduce((sum, file) => sum + file.sizeBytes, 0) ?? 0,
)
const loading = ref(true)
const confirming = ref(false)
const restoring = ref(false)
const error = ref('')
let disposed = false

onMounted(async () => {
  dialog.value?.showModal()
  try {
    const backups = await listPatchBackups(props.xplanePath, props.aircraftFolder)
    if (disposed) return
    sessions.value = backups
    selectedId.value = (backups.find((backup) => !backup.error) ?? backups[0])?.id ?? ''
  } catch (cause) {
    if (!disposed) error.value = getErrorMessage(cause)
  } finally {
    if (!disposed) loading.value = false
  }
})

onBeforeUnmount(() => {
  disposed = true
  dialog.value?.close()
})

const dateFormat = computed(
  () => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }),
)
const sizeFormat = computed(() => new Intl.NumberFormat(locale.value, { maximumFractionDigits: 1 }))

function dateLabel(timestamp: number | null) {
  return timestamp === null ? t('common.unknown') : dateFormat.value.format(timestamp)
}

function sizeLabel(bytes: number) {
  let unit = 0
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  while (bytes >= 1024 && unit < units.length - 1) {
    bytes /= 1024
    unit++
  }
  return `${sizeFormat.value.format(bytes)} ${units[unit]}`
}

function close() {
  if (!restoring.value) emit('close')
}

function resetConfirmation() {
  confirming.value = false
  error.value = ''
}

async function confirmRestore() {
  const session = selected.value
  if (!session || session.error || restoring.value || !confirming.value) return
  restoring.value = true
  error.value = ''
  try {
    const count = await revertPatch(props.xplanePath, props.aircraftFolder, session.id)
    if (!disposed) emit('restored', count)
  } catch (cause) {
    if (!disposed) error.value = getErrorMessage(cause)
  } finally {
    if (!disposed) {
      restoring.value = false
      confirming.value = false
    }
  }
}
</script>

<template>
  <Teleport to="body">
    <dialog
      ref="dialog"
      aria-labelledby="patch-backup-title"
      class="patch-backup m-auto w-[min(40rem,calc(100vw-2rem))] max-w-none overflow-hidden rounded-xl border border-gray-200 bg-white p-0 text-gray-900 shadow-xl dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100"
      @cancel.prevent="close"
    >
      <div class="flex max-h-[88vh] flex-col">
        <header
          class="flex shrink-0 items-start justify-between gap-4 border-b border-gray-400/20 px-5 py-4"
        >
          <div class="min-w-0">
            <h2 id="patch-backup-title" class="text-lg font-semibold">
              {{ t('patch.backupTitle') }}
            </h2>
            <p class="mt-1 break-words text-sm text-gray-500 dark:text-gray-400">
              {{ aircraftName }}
            </p>
          </div>
          <button
            type="button"
            class="shrink-0 rounded-md border border-gray-400/30 px-3 py-1.5 text-sm hover:bg-gray-400/10 disabled:opacity-50"
            :disabled="restoring"
            @click="close"
          >
            {{ t('common.close') }}
          </button>
        </header>

        <div class="min-h-0 space-y-4 overflow-y-auto px-5 py-4">
          <p
            v-if="loading"
            role="status"
            class="py-6 text-center text-sm text-gray-500 dark:text-gray-400"
          >
            {{ t('common.loading') }}
          </p>
          <p
            v-else-if="!sessions.length && !error"
            class="py-6 text-center text-sm text-gray-500 dark:text-gray-400"
          >
            {{ t('patch.noBackups') }}
          </p>
          <template v-else-if="selected">
            <div>
              <label for="patch-backup-session" class="mb-1.5 block text-sm font-medium">
                {{ t('patch.backupTitle') }}
              </label>
              <select
                id="patch-backup-session"
                v-model="selectedId"
                :disabled="restoring"
                class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-800"
                @change="resetConfirmation"
              >
                <option v-for="session in sessions" :key="session.id" :value="session.id">
                  {{ session.name }} · {{ dateLabel(session.timestamp)
                  }}{{ session.error ? ` · ${t('patch.backupUnavailable')}` : '' }}
                </option>
              </select>
            </div>
            <div
              v-if="selected.error"
              class="rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-sm"
              role="status"
            >
              <p class="font-medium text-amber-800 dark:text-amber-300">
                {{ t('patch.backupUnavailable') }}
              </p>
              <p class="mt-1 break-words text-gray-600 dark:text-gray-300">{{ selected.error }}</p>
            </div>
            <section v-else aria-labelledby="patch-backup-files">
              <h3 id="patch-backup-files" class="mb-2 text-sm font-semibold">
                {{ t('patch.backupFiles', { count: selected.files.length }) }} ·
                {{ sizeLabel(size) }}
              </h3>
              <ul
                class="divide-y divide-gray-400/20 rounded-lg border border-gray-400/20 bg-gray-50 dark:bg-gray-800/60"
              >
                <li
                  v-for="file in selected.files"
                  :key="file.path"
                  class="flex items-start justify-between gap-3 px-3 py-2 text-sm"
                >
                  <span class="min-w-0 break-all font-mono text-xs select-text">{{
                    file.path
                  }}</span>
                  <span class="shrink-0 text-xs text-gray-500 dark:text-gray-400">{{
                    sizeLabel(file.sizeBytes)
                  }}</span>
                </li>
              </ul>
            </section>
          </template>
          <p
            v-if="error"
            role="alert"
            class="break-words rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-700 dark:text-red-300"
          >
            {{ error }}
          </p>
        </div>

        <footer
          v-if="selected && !selected.error"
          class="shrink-0 border-t border-gray-400/20 px-5 py-4"
        >
          <p
            v-if="confirming"
            role="status"
            class="mb-3 break-words text-sm text-amber-800 dark:text-amber-300"
          >
            {{ t('patch.restoreConfirm', { aircraft: aircraftName, backup: selected.name }) }}
          </p>
          <div class="flex flex-wrap justify-end gap-2">
            <button
              v-if="confirming"
              type="button"
              class="rounded-md border border-gray-400/30 px-3 py-1.5 text-sm hover:bg-gray-400/10 disabled:opacity-50"
              :disabled="restoring"
              @click="resetConfirmation"
            >
              {{ t('common.cancel') }}
            </button>
            <button
              v-if="confirming"
              type="button"
              class="rounded-md bg-amber-700 px-3 py-1.5 text-sm font-medium text-white hover:bg-amber-800 disabled:opacity-50"
              :disabled="restoring"
              @click="confirmRestore"
            >
              {{ restoring ? t('patch.restoring') : t('common.confirm') }}
            </button>
            <button
              v-else
              type="button"
              class="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700"
              @click="confirming = true"
            >
              {{ t('management.restoreBackup') }}
            </button>
          </div>
        </footer>
      </div>
    </dialog>
  </Teleport>
</template>

<style scoped>
.patch-backup::backdrop {
  background: rgb(0 0 0 / 45%);
  backdrop-filter: blur(3px);
}
</style>
