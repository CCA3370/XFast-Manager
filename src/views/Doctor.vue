<template>
  <div class="flex h-full flex-col overflow-hidden bg-neutral-50 dark:bg-neutral-900">
    <!-- Header -->
    <div
      class="flex items-center justify-between border-b border-neutral-200 bg-white px-6 py-4 dark:border-neutral-700 dark:bg-neutral-800"
    >
      <div>
        <h1 class="text-2xl font-semibold text-neutral-900 dark:text-white">
          {{ t('doctor.title') }}
        </h1>
        <p class="mt-1 text-sm text-neutral-600 dark:text-neutral-400">
          {{ t('doctor.subtitle') }}
        </p>
      </div>
      <div class="flex items-center gap-3">
        <button
          v-if="store.lastRun"
          class="text-sm text-neutral-600 dark:text-neutral-400"
          disabled
        >
          {{ t('doctor.lastRun', { time: formatRelativeTime(store.lastRun) }) }}
        </button>
        <button
          :disabled="store.isRunning"
          class="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
          @click="store.runDiagnostics()"
        >
          {{
            store.isRunning
              ? t('doctor.running')
              : store.lastRun
                ? t('doctor.rerun')
                : t('doctor.runDiagnostics')
          }}
        </button>
      </div>
    </div>

    <!-- Error banner -->
    <div
      v-if="store.error"
      class="mx-6 mt-4 rounded-lg bg-red-50 p-4 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-400"
    >
      {{ store.error }}
    </div>

    <!-- Overall health summary -->
    <div v-if="store.lastRun && !store.isRunning" class="mx-6 mt-6">
      <div class="flex items-center gap-4 rounded-lg border p-4" :class="healthCardClass">
        <div
          class="flex h-12 w-12 items-center justify-center rounded-full"
          :class="healthIconClass"
        >
          <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              v-if="store.overallSeverity === 'ok'"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 13l4 4L19 7"
            />
            <path
              v-else
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
        </div>
        <div class="flex-1">
          <div class="text-lg font-semibold" :class="healthTextClass">{{ healthLabel }}</div>
          <div class="mt-1 flex gap-4 text-sm text-neutral-600 dark:text-neutral-400">
            <span v-if="store.counts.critical > 0" class="font-medium text-red-600 dark:text-red-400">
              {{ store.counts.critical }} {{ t('doctor.severity.critical') }}
            </span>
            <span v-if="store.counts.warning > 0" class="font-medium text-amber-600 dark:text-amber-400">
              {{ store.counts.warning }} {{ t('doctor.severity.warning') }}
            </span>
            <span v-if="store.counts.info > 0">{{ store.counts.info }} {{ t('doctor.severity.info') }}</span>
            <span v-if="store.counts.total === 0">{{ t('doctor.healthOk') }}</span>
          </div>
        </div>
        <!-- System info chips -->
        <div
          v-if="store.systemInfo"
          class="hidden flex-col items-end gap-1 text-xs text-neutral-500 dark:text-neutral-400 md:flex"
        >
          <span v-if="store.systemInfo.xplaneVersionRaw" class="font-mono">
            {{ store.systemInfo.xplaneVersionRaw }}
          </span>
          <span v-if="store.systemInfo.gpuModel">{{ store.systemInfo.gpuModel }}</span>
        </div>
      </div>
    </div>

    <!-- X-Plane running warning -->
    <div
      v-if="store.xplaneRunning && store.lastRun"
      class="mx-6 mt-3 rounded-lg bg-amber-50 px-4 py-2 text-sm text-amber-800 dark:bg-amber-900/20 dark:text-amber-400"
    >
      {{ t('doctor.xplaneRunning') }}
    </div>

    <!-- Findings by section -->
    <div v-if="store.lastRun && !store.isRunning" class="flex-1 overflow-y-auto px-6 py-6">
      <div v-if="store.findingsBySection.length === 0" class="py-12 text-center">
        <svg class="mx-auto h-12 w-12 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <p class="mt-4 text-lg font-medium text-neutral-900 dark:text-white">{{ t('doctor.healthOk') }}</p>
      </div>

      <div v-else class="space-y-6">
        <div
          v-for="group in store.findingsBySection"
          :key="group.section"
          class="overflow-hidden rounded-lg border border-neutral-200 bg-white dark:border-neutral-700 dark:bg-neutral-800"
        >
          <div class="border-b border-neutral-200 px-4 py-3 dark:border-neutral-700">
            <h2 class="text-base font-semibold text-neutral-900 dark:text-white">
              {{ t(`doctor.sections.${group.section}`) }}
              <span class="ml-2 text-sm font-normal text-neutral-500">({{ group.findings.length }})</span>
            </h2>
          </div>

          <div class="divide-y divide-neutral-200 dark:divide-neutral-700">
            <div
              v-for="finding in group.findings"
              :key="finding.id"
              class="p-4"
              :class="findingBgClass(finding.severity)"
            >
              <div class="flex items-start gap-3">
                <span
                  class="mt-1.5 h-2.5 w-2.5 flex-shrink-0 rounded-full"
                  :class="severityDotClass(finding.severity)"
                />
                <div class="min-w-0 flex-1">
                  <div class="flex items-start justify-between gap-3">
                    <div class="flex-1">
                      <h3 class="text-sm font-semibold text-neutral-900 dark:text-white">
                        {{ tc(finding, 'title') }}
                      </h3>
                      <p class="mt-1 text-sm text-neutral-700 dark:text-neutral-300">
                        {{ tc(finding, 'description') }}
                      </p>
                      <div class="mt-2 flex items-start gap-1.5 text-sm text-blue-700 dark:text-blue-300">
                        <svg class="mt-0.5 h-3.5 w-3.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                        <span>{{ tc(finding, 'suggestion') }}</span>
                      </div>

                      <details
                        v-if="finding.detail && finding.detail.length > 0"
                        class="mt-3"
                      >
                        <summary class="cursor-pointer select-none text-xs text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300">
                          {{ t('doctor.fixes.viewDetails') }} ({{ finding.detail.length }})
                        </summary>
                        <div
                          class="mt-2 max-h-48 overflow-y-auto rounded border border-neutral-200 bg-neutral-50 p-3 font-mono text-xs text-neutral-700 dark:border-neutral-600 dark:bg-neutral-900 dark:text-neutral-300"
                        >
                          <div v-for="(line, i) in finding.detail" :key="i" class="break-all">{{ line }}</div>
                        </div>
                      </details>
                    </div>

                    <div v-if="hasAction(finding)" class="flex flex-shrink-0 flex-col gap-2">
                      <button
                        v-if="finding.fix && finding.fix.tier !== 'none'"
                        :disabled="store.fixingId === finding.id || (store.xplaneRunning && isFsFix(finding))"
                        class="whitespace-nowrap rounded-lg px-3 py-1.5 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50"
                        :class="fixButtonClass(finding.fix.tier)"
                        @click="applyFix(finding)"
                      >
                        {{ store.fixingId === finding.id ? t('doctor.fixes.applying') : fixLabel(finding) }}
                      </button>
                      <button
                        v-if="finding.route"
                        class="whitespace-nowrap rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-xs font-medium text-neutral-700 transition-colors hover:bg-neutral-50 dark:border-neutral-600 dark:bg-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-600"
                        @click="goToRoute(finding)"
                      >
                        {{ t('doctor.fixes.viewDetails') }}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="store.isRunning" class="flex flex-1 items-center justify-center">
      <div class="text-center">
        <div class="mx-auto h-12 w-12 animate-spin rounded-full border-4 border-neutral-200 border-t-blue-600 dark:border-neutral-700" />
        <p class="mt-4 text-sm font-medium text-neutral-700 dark:text-neutral-300">{{ t('doctor.running') }}</p>
        <p class="mt-2 text-xs text-neutral-500 dark:text-neutral-400">{{ phaseLabel }}</p>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="!store.lastRun && !store.isRunning" class="flex flex-1 items-center justify-center">
      <div class="max-w-md text-center">
        <svg class="mx-auto h-16 w-16 text-neutral-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78l8.84 8.84 8.84-8.84a5.5 5.5 0 000-7.78z" />
        </svg>
        <p class="mt-4 text-lg font-medium text-neutral-900 dark:text-white">{{ t('doctor.title') }}</p>
        <p class="mt-2 text-sm text-neutral-600 dark:text-neutral-400">{{ t('doctor.subtitle') }}</p>
        <button
          v-if="appStore.xplanePath"
          class="mt-6 rounded-lg bg-blue-600 px-6 py-2.5 text-sm font-medium text-white transition-colors hover:bg-blue-700"
          @click="store.runDiagnostics()"
        >
          {{ t('doctor.runDiagnostics') }}
        </button>
        <router-link
          v-else
          to="/settings"
          class="mt-6 inline-block rounded-lg bg-blue-600 px-6 py-2.5 text-sm font-medium text-white transition-colors hover:bg-blue-700"
        >
          {{ t('common.settings') }}
        </router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { useDoctorStore, type DoctorFinding, type DoctorSeverity } from '@/stores/doctor'

const { t } = useI18n()
const router = useRouter()
const store = useDoctorStore()
const appStore = useAppStore()
const modalStore = useModalStore()
const toast = useToastStore()

onMounted(() => {
  // Auto-run on open when a path is configured and we haven't run yet this session.
  if (appStore.xplanePath && !store.lastRun && !store.isRunning) {
    void store.runDiagnostics()
  }
})

// Translate a check string with a graceful fallback (returns '' if key missing).
function tc(finding: DoctorFinding, field: 'title' | 'description' | 'suggestion'): string {
  const key = `doctor.checks.${finding.id}.${field}`
  const result = t(key, finding.params || {})
  // vue-i18n returns the key itself when missing; hide that.
  return result === key ? '' : result
}

const phaseLabel = computed(() => {
  if (store.phase === 'network') return t('doctor.sections.updates')
  return ''
})

const healthLabel = computed(() => {
  switch (store.overallSeverity) {
    case 'critical':
      return t('doctor.healthCritical')
    case 'warning':
      return t('doctor.healthWarning')
    case 'info':
      return t('doctor.healthInfo')
    default:
      return t('doctor.healthOk')
  }
})

const healthCardClass = computed(() => {
  switch (store.overallSeverity) {
    case 'critical':
      return 'border-red-200 bg-red-50 dark:border-red-900/40 dark:bg-red-900/10'
    case 'warning':
      return 'border-amber-200 bg-amber-50 dark:border-amber-900/40 dark:bg-amber-900/10'
    case 'info':
      return 'border-blue-200 bg-blue-50 dark:border-blue-900/40 dark:bg-blue-900/10'
    default:
      return 'border-green-200 bg-green-50 dark:border-green-900/40 dark:bg-green-900/10'
  }
})

const healthIconClass = computed(() => {
  switch (store.overallSeverity) {
    case 'critical':
      return 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400'
    case 'warning':
      return 'bg-amber-100 text-amber-600 dark:bg-amber-900/30 dark:text-amber-400'
    case 'info':
      return 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
    default:
      return 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400'
  }
})

const healthTextClass = computed(() => {
  switch (store.overallSeverity) {
    case 'critical':
      return 'text-red-800 dark:text-red-300'
    case 'warning':
      return 'text-amber-800 dark:text-amber-300'
    case 'info':
      return 'text-blue-800 dark:text-blue-300'
    default:
      return 'text-green-800 dark:text-green-300'
  }
})

function severityDotClass(severity: DoctorSeverity): string {
  switch (severity) {
    case 'critical':
      return 'bg-red-500'
    case 'warning':
      return 'bg-amber-500'
    case 'info':
      return 'bg-blue-500'
    default:
      return 'bg-green-500'
  }
}

function findingBgClass(severity: DoctorSeverity): string {
  return severity === 'critical' ? 'bg-red-50/40 dark:bg-red-900/5' : ''
}

function fixButtonClass(tier: string): string {
  if (tier === 'destructive') {
    return 'bg-red-600 text-white hover:bg-red-700'
  }
  if (tier === 'confirm') {
    return 'bg-amber-600 text-white hover:bg-amber-700'
  }
  return 'bg-blue-600 text-white hover:bg-blue-700'
}

function hasAction(finding: DoctorFinding): boolean {
  return Boolean((finding.fix && finding.fix.tier !== 'none') || finding.route)
}

function isFsFix(finding: DoctorFinding): boolean {
  if (!finding.fix) return false
  return ['sort_scenery', 'enable_global_airports', 'apply_flatten'].includes(finding.fix.id)
}

function fixLabel(finding: DoctorFinding): string {
  const key = `doctor.checks.${finding.id}.fix`
  const localized = t(key, finding.fix?.params || {})
  if (localized !== key) return localized
  return t('doctor.fixes.apply')
}

function goToRoute(finding: DoctorFinding) {
  if (finding.route) void router.push(finding.route)
}

async function applyFix(finding: DoctorFinding) {
  if (!finding.fix) return

  // Guidance-only / route fixes (e.g. cleanup, app update) just navigate.
  if (finding.fix.id === 'open_cleanup' || finding.fix.id === 'open_app_update') {
    if (finding.route) {
      void router.push(finding.route)
    } else if (finding.fix.id === 'open_app_update') {
      // App update lives in the global update banner; surface a hint.
      toast.info(t('doctor.checks.updates.app.suggestion'))
    }
    return
  }

  // Confirm / destructive tiers require explicit confirmation.
  if (finding.fix.tier === 'confirm' || finding.fix.tier === 'destructive') {
    modalStore.showConfirm({
      title: tc(finding, 'title'),
      message: tc(finding, 'suggestion'),
      confirmText: t('doctor.fixes.apply'),
      cancelText: t('common.cancel'),
      type: finding.fix.tier === 'destructive' ? 'danger' : 'warning',
      onConfirm: () => {
        void runFix(finding)
      },
      onCancel: () => {},
    })
    return
  }

  // Safe tier: apply immediately.
  await runFix(finding)
}

async function runFix(finding: DoctorFinding) {
  const ok = await store.applyFix(finding)
  if (ok) {
    toast.success(t('common.success'))
  } else if (store.error === 'xplane_running') {
    toast.error(t('doctor.xplaneRunning'))
    store.error = null
  } else if (store.error) {
    modalStore.showError(store.error)
    store.error = null
  }
}

function formatRelativeTime(ts: number): string {
  const diff = Math.floor((Date.now() - ts) / 1000)
  if (diff < 60) return t('activityLog.justNow')
  if (diff < 3600) return t('activityLog.minutesAgo', { n: Math.floor(diff / 60) })
  if (diff < 86400) return t('activityLog.hoursAgo', { n: Math.floor(diff / 3600) })
  return t('activityLog.daysAgo', { n: Math.floor(diff / 86400) })
}
</script>
