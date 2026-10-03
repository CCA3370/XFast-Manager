<template>
  <div
    data-testid="health-page"
    class="doctor-view flex h-full flex-col overflow-hidden px-6 pt-3 pb-6 text-gray-900 dark:text-white"
  >
    <header class="mb-3 flex flex-none items-center justify-between gap-4">
      <div class="min-w-0">
        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
          {{ t('doctor.center.heading') }}
        </h1>
        <div
          v-if="run?.completedAt"
          class="mt-0.5 flex min-w-0 flex-wrap items-center gap-x-2 text-xs text-gray-500 dark:text-gray-400"
        >
          <span>{{ t('doctor.center.status.lastChecked', { time: formatDateTime(run.completedAt) }) }}</span>
          <span aria-hidden="true">·</span>
          <span>{{ t('doctor.center.status.duration', { duration: formatDuration(run.durationMs) }) }}</span>
          <span v-if="store.selectedRunId" aria-hidden="true">·</span>
          <span v-if="store.selectedRunId">{{ t('doctor.center.status.viewingHistory') }}</span>
        </div>
      </div>

      <div v-if="appStore.xplanePath" class="flex flex-none items-center gap-2">
        <button
          v-if="store.isRunning"
          type="button"
          data-testid="cancel-health-scan"
          class="inline-flex items-center gap-2 rounded-md border border-red-200 bg-white px-3 py-1.5 text-xs font-semibold text-red-700 transition-colors hover:bg-red-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500/40 dark:border-red-900/70 dark:bg-gray-900 dark:text-red-300 dark:hover:bg-red-950/30"
          @click="store.cancelRun()"
        >
          <span class="h-2 w-2 rounded-sm bg-current" aria-hidden="true" />
          {{ t('doctor.center.cancel') }}
        </button>
        <template v-else>
          <button
            type="button"
            data-testid="quick-health-scan"
            :title="t('doctor.center.quickHint')"
            :disabled="store.isBusy"
            class="rounded-md border border-gray-300 bg-white px-3 py-1.5 text-xs font-semibold text-gray-700 transition-colors hover:bg-gray-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-200 dark:hover:bg-gray-800"
            @click="runDiagnostics('quick')"
          >
            {{ t('doctor.center.quickCheck') }}
          </button>
          <button
            type="button"
            data-testid="full-health-scan"
            :title="t('doctor.center.fullHint')"
            :disabled="store.isBusy"
            class="rounded-md bg-blue-600 px-3 py-1.5 text-xs font-semibold text-white transition-colors hover:bg-blue-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 disabled:cursor-not-allowed disabled:opacity-50"
            @click="runDiagnostics('full')"
          >
            {{ t('doctor.center.fullCheck') }}
          </button>
        </template>
      </div>
    </header>

    <main
      class="min-h-0 flex-1 overflow-y-auto pr-1"
      :aria-busy="store.isBusy || store.isHistoryLoading"
    >
      <div
        v-if="store.error"
        role="alert"
        aria-live="assertive"
        class="mb-3 flex items-center justify-between gap-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-800 dark:border-red-900/60 dark:bg-red-950/30 dark:text-red-200"
      >
        <span>{{ errorMessage }}</span>
        <button
          type="button"
          class="rounded px-1 font-bold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500/50"
          :aria-label="t('common.close')"
          @click="store.error = null"
        >
          ×
        </button>
      </div>

      <section
        v-if="!appStore.xplanePath"
        class="flex min-h-[320px] items-center justify-center rounded-lg border border-dashed border-gray-300 bg-white p-8 text-center dark:border-gray-700 dark:bg-gray-900/40"
      >
        <div class="max-w-md">
          <h2 class="text-base font-semibold">{{ t('doctor.center.messages.noPathTitle') }}</h2>
          <p class="mt-2 text-sm leading-6 text-gray-500 dark:text-gray-400">
            {{ t('doctor.center.messages.noPathDescription') }}
          </p>
          <router-link
            to="/settings"
            class="mt-4 inline-flex rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40"
          >
            {{ t('doctor.center.messages.configurePath') }}
          </router-link>
        </div>
      </section>

      <template v-else>
        <section
          v-if="store.isHistoryLoading && !run && !store.isRunning"
          data-testid="health-history-loading"
          role="status"
          aria-live="polite"
          class="flex min-h-[240px] items-center justify-center rounded-lg border border-gray-200 bg-white text-sm text-gray-500 dark:border-gray-700 dark:bg-gray-900/40 dark:text-gray-400"
        >
          <span
            class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-blue-500 border-t-transparent"
            aria-hidden="true"
          />
          {{ t('common.loading') }}
        </section>

        <section
          v-if="run"
          data-testid="health-summary"
          aria-live="polite"
          class="mb-3 overflow-hidden rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900/45"
        >
          <div class="flex flex-col gap-3 px-3 py-2.5 lg:flex-row lg:items-center lg:justify-between">
            <div class="flex min-w-0 items-center gap-3">
              <span
                class="h-2.5 w-2.5 flex-none rounded-full"
                :class="severityDotClass(run.summary.severity, run.summary.completeness)"
                aria-hidden="true"
              />
              <div class="min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                  <h2 class="text-sm font-semibold text-gray-900 dark:text-white">
                    {{ healthTitle }}
                  </h2>
                  <span
                    class="rounded border px-1.5 py-0.5 text-[10px] font-semibold"
                    :class="modeBadgeClass"
                  >
                    {{
                      run.mode === 'quick'
                        ? t('doctor.center.history.quick')
                        : t('doctor.center.history.full')
                    }}
                  </span>
                  <span
                    v-if="store.xplaneRunning"
                    class="rounded bg-gray-100 px-1.5 py-0.5 text-[10px] font-medium text-gray-600 dark:bg-gray-800 dark:text-gray-300"
                  >
                    {{ t('doctor.center.checks.xplaneRunning') }}
                  </span>
                </div>
                <div
                  v-if="run.state === 'running'"
                  data-testid="health-scan-progress"
                  class="mt-1 text-[11px] text-gray-500 dark:text-gray-400"
                >
                  {{
                    t('doctor.center.progress', {
                      done: runtimeCompleted,
                      total: store.checkRuntime.length,
                    })
                  }}
                  <span class="px-1">·</span>
                  <span class="font-semibold text-gray-600 dark:text-gray-300">{{ runtimePercent }}%</span>
                  <span class="px-1">·</span>
                  {{
                    store.phase === 'network'
                      ? t('doctor.center.networkPhase')
                      : t('doctor.center.localPhase')
                  }}
                </div>
              </div>
            </div>

            <div v-if="run.state !== 'running'" class="flex flex-wrap items-center gap-1.5 text-[11px]">
              <span class="rounded bg-emerald-50 px-2 py-1 font-medium text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-300">
                {{ t('doctor.center.outcomes.pass') }} {{ run.summary.pass }}
              </span>
              <span v-if="run.summary.info" class="rounded bg-blue-50 px-2 py-1 font-medium text-blue-700 dark:bg-blue-950/30 dark:text-blue-300">
                {{ t('doctor.center.outcomes.info') }} {{ run.summary.info }}
              </span>
              <span v-if="run.summary.warning" class="rounded bg-amber-50 px-2 py-1 font-medium text-amber-700 dark:bg-amber-950/30 dark:text-amber-300">
                {{ t('doctor.center.outcomes.warning') }} {{ run.summary.warning }}
              </span>
              <span v-if="run.summary.critical" class="rounded bg-red-50 px-2 py-1 font-medium text-red-700 dark:bg-red-950/30 dark:text-red-300">
                {{ t('doctor.center.outcomes.critical') }} {{ run.summary.critical }}
              </span>
              <span v-if="run.summary.unavailable" class="rounded bg-gray-100 px-2 py-1 font-medium text-gray-600 dark:bg-gray-800 dark:text-gray-300">
                {{ t('doctor.center.outcomes.unavailable') }} {{ run.summary.unavailable }}
              </span>
              <span class="ml-1 font-semibold text-gray-600 dark:text-gray-300">
                {{ run.summary.coveragePercent }}%
              </span>
            </div>
          </div>

          <div class="h-1 bg-gray-100 dark:bg-gray-800">
            <div
              class="h-full transition-all duration-300"
              :class="run.state === 'running' ? 'bg-blue-600' : coverageBarClass"
              :style="{ width: `${run.state === 'running' ? runtimePercent : run.summary.coveragePercent}%` }"
            />
          </div>

          <div
            v-if="!store.isRunning && (displayedRunIsStale || run.summary.completeness !== 'complete')"
            class="flex flex-wrap gap-x-4 gap-y-1 border-t border-gray-100 px-3 py-2 text-[11px] text-gray-600 dark:border-gray-800 dark:text-gray-300"
          >
            <span v-if="displayedRunIsStale">
              <strong>{{ t('doctor.center.status.stale') }}</strong>
              — {{ t('doctor.center.status.staleDescription') }}
            </span>
            <span v-if="run.summary.completeness !== 'complete'">
              <strong>{{ t('doctor.center.status.incomplete') }}</strong>
              —
              {{
                run.summary.completeness === 'cancelled'
                  ? t('doctor.center.status.cancelledDescription')
                  : t('doctor.center.status.partialDescription')
              }}
            </span>
          </div>
        </section>

        <section
          v-if="!run && !store.isRunning && !store.isHistoryLoading"
          class="flex min-h-[280px] items-center justify-center rounded-lg border border-dashed border-gray-300 bg-white p-8 text-center dark:border-gray-700 dark:bg-gray-900/40"
        >
          <div class="max-w-md">
            <h2 class="text-base font-semibold">{{ t('doctor.center.messages.noResultTitle') }}</h2>
            <p class="mt-2 text-sm leading-6 text-gray-500 dark:text-gray-400">
              {{ t('doctor.center.messages.noResultDescription') }}
            </p>
            <div class="mt-4 flex justify-center gap-2">
              <button
                type="button"
                class="rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm font-medium hover:bg-gray-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 dark:border-gray-700 dark:bg-gray-900 dark:hover:bg-gray-800"
                @click="runDiagnostics('quick')"
              >
                {{ t('doctor.center.quickCheck') }}
              </button>
              <button
                type="button"
                class="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40"
                @click="runDiagnostics('full')"
              >
                {{ t('doctor.center.fullCheck') }}
              </button>
            </div>
          </div>
        </section>

        <div v-if="run" class="grid items-start gap-3 xl:grid-cols-[minmax(0,1fr)_360px]">
          <section
            data-testid="health-results-panel"
            class="min-w-0 overflow-hidden rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900/45"
          >
            <header class="flex min-h-11 items-center justify-between gap-3 border-b border-gray-200 px-3 py-2 dark:border-gray-700">
              <div class="flex min-w-0 items-center gap-2">
                <h2 class="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                  {{ t('doctor.center.status.currentResult') }}
                </h2>
                <span class="text-[11px] text-gray-400">
                  {{ run.summary.total }}
                </span>
              </div>

              <button
                v-if="safeFixCount > 0 && canRepair"
                type="button"
                :disabled="store.repairingAll || store.xplaneRunning"
                class="rounded-md border border-emerald-300 bg-emerald-50 px-2.5 py-1 text-[11px] font-semibold text-emerald-700 transition-colors hover:bg-emerald-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40 disabled:cursor-not-allowed disabled:opacity-50 dark:border-emerald-900 dark:bg-emerald-950/30 dark:text-emerald-300"
                @click="applyAllSafeFixes"
              >
                {{
                  store.repairingAll
                    ? t('doctor.center.remediation.fixingAll')
                    : `${t('doctor.center.remediation.fixAllSafe')} (${safeFixCount})`
                }}
              </button>
            </header>

            <div>
              <section
                v-for="group in checkGroups"
                :key="group.section"
                :aria-labelledby="`health-section-${group.section}`"
              >
                <div
                  class="flex h-8 items-center justify-between border-b border-gray-100 bg-gray-50/80 px-3 text-[11px] font-semibold text-gray-600 dark:border-gray-800 dark:bg-gray-900 dark:text-gray-300"
                >
                  <div class="flex min-w-0 items-center gap-2">
                    <svg
                      class="h-3.5 w-3.5 flex-none text-gray-400"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                      aria-hidden="true"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        :d="sectionIcon(group.section)"
                      />
                    </svg>
                    <span :id="`health-section-${group.section}`">
                      {{ sectionLabel(group.section) }}
                    </span>
                  </div>
                  <span class="font-normal text-gray-400">
                    {{ group.issueCount ? `${group.issueCount} / ` : '' }}{{ group.checks.length }}
                  </span>
                </div>

                <div class="divide-y divide-gray-100 dark:divide-gray-800">
                  <div
                    v-for="check in group.checks"
                    :key="`${run.id}-${check.id}`"
                    data-testid="health-check-row"
                    :data-check-id="check.id"
                    class="grid grid-cols-[minmax(0,1fr)_auto] items-stretch transition-colors"
                    :class="
                      selectedCheck?.id === check.id
                        ? 'bg-blue-50/70 dark:bg-blue-950/20'
                        : 'hover:bg-gray-50/70 dark:hover:bg-gray-800/35'
                    "
                  >
                    <button
                      type="button"
                      class="grid min-w-0 grid-cols-[12px_minmax(0,1fr)_auto] items-center gap-2.5 px-3 py-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-blue-500/40"
                      :aria-pressed="selectedCheck?.id === check.id"
                      @click="selectCheck(check)"
                    >
                      <span
                        class="h-2 w-2 rounded-full"
                        :class="severityDotClass(
                          check.outcome === 'critical'
                            ? 'critical'
                            : check.outcome === 'warning'
                              ? 'warning'
                              : check.outcome === 'info'
                                ? 'info'
                                : 'ok',
                          check.outcome === 'unavailable' || check.outcome === 'cancelled'
                            ? 'partial'
                            : 'complete',
                        )"
                        aria-hidden="true"
                      />
                      <div class="min-w-0">
                        <div class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5">
                          <span class="truncate text-xs font-medium text-gray-900 dark:text-gray-100">
                            {{ checkLabel(check) }}
                          </span>
                          <span
                            v-for="(value, key) in check.params"
                            :key="key"
                            class="max-w-[220px] truncate font-mono text-[10px] text-gray-400"
                          >
                            {{ formatParamKey(String(key)) }}={{ value }}
                          </span>
                        </div>
                      </div>
                      <div class="flex items-center gap-2">
                        <span
                          class="rounded px-1.5 py-0.5 text-[10px] font-semibold"
                          :class="outcomeBadgeClass(check.outcome)"
                        >
                          {{ outcomeLabel(check.outcome) }}
                        </span>
                        <span class="w-12 text-right font-mono text-[10px] tabular-nums text-gray-400">
                          {{ formatDuration(check.durationMs) }}
                        </span>
                      </div>
                    </button>

                    <div
                      v-if="check.remediation"
                      class="flex items-center border-l border-gray-100 px-2 dark:border-gray-800"
                    >
                      <button
                        type="button"
                        :disabled="
                          store.fixingId === check.id ||
                          store.isBusy ||
                          (check.remediation.kind === 'automatic' && !canRepair)
                        "
                        class="rounded-md border px-2 py-1 text-[10px] font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 disabled:cursor-not-allowed disabled:opacity-50"
                        :class="remediationButtonClass(check.remediation)"
                        @click="handleRemediation(check)"
                      >
                        {{
                          store.fixingId === check.id
                            ? t('doctor.fixes.applying')
                            : remediationLabel(check.remediation)
                        }}
                      </button>
                    </div>
                  </div>
                </div>
              </section>
            </div>
          </section>

          <aside class="min-w-0 space-y-3 xl:sticky xl:top-0">
            <section
              v-if="selectedCheck"
              data-testid="health-detail-panel"
              class="overflow-hidden rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900/45"
            >
              <header class="flex items-start justify-between gap-3 border-b border-gray-200 px-3 py-2.5 dark:border-gray-700">
                <div class="min-w-0">
                  <div class="text-[10px] font-semibold uppercase tracking-wide text-gray-400">
                    {{ sectionLabel(selectedCheck.section) }}
                  </div>
                  <h2 class="mt-0.5 text-sm font-semibold leading-5 text-gray-900 dark:text-white">
                    {{ checkLabel(selectedCheck) }}
                  </h2>
                </div>
                <span
                  class="flex-none rounded px-1.5 py-0.5 text-[10px] font-semibold"
                  :class="outcomeBadgeClass(selectedCheck.outcome)"
                >
                  {{ outcomeLabel(selectedCheck.outcome) }}
                </span>
              </header>

              <div class="space-y-3 px-3 py-3 text-xs">
                <p class="leading-5 text-gray-600 dark:text-gray-300">
                  {{ checkDescription(selectedCheck) }}
                </p>

                <p
                  v-if="checkSuggestion(selectedCheck)"
                  class="border-l-2 border-blue-400 pl-2.5 leading-5 text-gray-700 dark:text-gray-200"
                >
                  {{ checkSuggestion(selectedCheck) }}
                </p>

                <dl
                  v-if="selectedCheck.params && Object.keys(selectedCheck.params).length"
                  class="grid grid-cols-[minmax(90px,auto)_minmax(0,1fr)] gap-x-3 gap-y-1.5 border-t border-gray-100 pt-3 text-[11px] dark:border-gray-800"
                >
                  <template v-for="(value, key) in selectedCheck.params" :key="key">
                    <dt class="text-gray-400">{{ formatParamKey(String(key)) }}</dt>
                    <dd class="min-w-0 break-words text-right font-mono text-gray-700 dark:text-gray-200">
                      {{ value }}
                    </dd>
                  </template>
                </dl>

                <div
                  v-if="selectedCheck.evidence?.length"
                  class="border-t border-gray-100 pt-3 dark:border-gray-800"
                >
                  <div class="mb-1.5 flex items-center justify-between">
                    <span class="font-semibold text-gray-700 dark:text-gray-200">
                      {{ t('doctor.center.evidence.title') }}
                    </span>
                    <span class="text-[10px] text-gray-400">
                      {{ selectedCheck.evidence.length }}
                    </span>
                  </div>
                  <div class="max-h-52 space-y-1 overflow-y-auto rounded-md bg-gray-50 p-2 font-mono text-[10px] leading-4 text-gray-600 dark:bg-gray-950/60 dark:text-gray-300">
                    <div
                      v-for="(item, index) in selectedCheck.evidence"
                      :key="`${index}-${item.value}`"
                      class="break-all"
                    >
                      <span v-if="item.label" class="font-semibold">{{ item.label }}: </span>{{ item.value }}
                    </div>
                  </div>
                </div>

                <button
                  v-if="selectedCheck.remediation"
                  type="button"
                  :disabled="
                    store.fixingId === selectedCheck.id ||
                    store.isBusy ||
                    (selectedCheck.remediation.kind === 'automatic' && !canRepair)
                  "
                  class="w-full rounded-md border px-3 py-1.5 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 disabled:cursor-not-allowed disabled:opacity-50"
                  :class="remediationButtonClass(selectedCheck.remediation)"
                  @click="handleRemediation(selectedCheck)"
                >
                  {{
                    store.fixingId === selectedCheck.id
                      ? t('doctor.fixes.applying')
                      : remediationLabel(selectedCheck.remediation)
                  }}
                </button>
              </div>
            </section>

            <section
              v-if="run.system"
              class="rounded-lg border border-gray-200 bg-white px-3 py-2.5 dark:border-gray-700 dark:bg-gray-900/45"
            >
              <h2 class="text-xs font-semibold text-gray-700 dark:text-gray-200">
                {{ t('doctor.center.system.title') }}
              </h2>
              <dl class="mt-2 grid gap-y-1.5 text-[11px]">
                <div
                  v-for="item in systemItems"
                  :key="item.label"
                  class="grid grid-cols-[92px_minmax(0,1fr)] gap-2"
                >
                  <dt class="text-gray-400">{{ item.label }}</dt>
                  <dd class="min-w-0 truncate text-right text-gray-700 dark:text-gray-200" :title="item.value">
                    {{ item.value }}
                  </dd>
                </div>
              </dl>
            </section>

            <details class="group rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900/45">
              <summary class="flex cursor-pointer list-none items-center justify-between gap-3 px-3 py-2.5 text-xs font-semibold text-gray-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-blue-500/40 dark:text-gray-200">
                <span>{{ t('doctor.center.history.title') }}</span>
                <span class="flex items-center gap-2 text-[10px] font-normal text-gray-400">
                  {{ store.history.length }}/20
                  <svg class="h-3 w-3 transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 9l6 6 6-6" />
                  </svg>
                </span>
              </summary>
              <div class="border-t border-gray-100 px-2 py-2 dark:border-gray-800">
                <button
                  v-if="store.selectedRunId"
                  type="button"
                  :disabled="store.isBusy"
                  class="mb-2 w-full rounded-md border border-blue-200 bg-blue-50 px-2 py-1.5 text-[11px] font-semibold text-blue-700 dark:border-blue-900 dark:bg-blue-950/30 dark:text-blue-200"
                  @click="store.selectHistoryRun(null)"
                >
                  {{ t('doctor.center.history.backToCurrent') }}
                </button>
                <div v-if="store.isHistoryLoading" role="status" class="py-3 text-center text-[11px] text-gray-400">
                  {{ t('common.loading') }}
                </div>
                <div v-else-if="store.history.length" class="max-h-52 divide-y divide-gray-100 overflow-y-auto dark:divide-gray-800">
                  <button
                    v-for="item in store.history"
                    :key="item.id"
                    type="button"
                    :disabled="store.isBusy"
                    class="flex w-full items-center justify-between gap-3 px-1 py-2 text-left text-[11px] transition-colors hover:bg-gray-50 disabled:opacity-50 dark:hover:bg-gray-800/50"
                    @click="store.selectHistoryRun(item.id)"
                  >
                    <span class="min-w-0 truncate">
                      {{ formatDateTime(item.startedAt) }}
                    </span>
                    <span class="flex flex-none items-center gap-2 text-gray-400">
                      {{
                        item.mode === 'quick'
                          ? t('doctor.center.history.quick')
                          : t('doctor.center.history.full')
                      }}
                      <span
                        class="h-2 w-2 rounded-full"
                        :class="severityDotClass(item.summary.severity, item.summary.completeness)"
                        :title="historyStatusLabel(item.summary.severity, item.summary.completeness)"
                      />
                    </span>
                  </button>
                </div>
                <p v-else class="py-3 text-center text-[11px] text-gray-400">
                  {{ t('doctor.center.history.empty') }}
                </p>
              </div>
            </details>

            <details class="group rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900/45">
              <summary class="flex cursor-pointer list-none items-center justify-between gap-3 px-3 py-2.5 text-xs font-semibold text-gray-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-blue-500/40 dark:text-gray-200">
                <span>{{ t('doctor.center.export') }}</span>
                <svg class="h-3 w-3 text-gray-400 transition-transform group-open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 9l6 6 6-6" />
                </svg>
              </summary>
              <div class="space-y-2 border-t border-gray-100 px-3 py-2.5 dark:border-gray-800">
                <div class="flex items-center justify-between gap-3 text-[11px]">
                  <span :class="includeSensitive ? 'text-amber-700 dark:text-amber-300' : 'text-gray-500 dark:text-gray-400'">
                    {{
                      includeSensitive
                        ? t('doctor.center.privacy.sensitive')
                        : t('doctor.center.privacy.redacted')
                    }}
                  </span>
                  <button
                    type="button"
                    class="flex-none font-semibold text-blue-600 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 dark:text-blue-400"
                    @click="toggleSensitivePaths"
                  >
                    {{
                      includeSensitive
                        ? t('doctor.center.excludeSensitive')
                        : t('doctor.center.includeSensitive')
                    }}
                  </button>
                </div>
                <div class="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    :disabled="store.isBusy"
                    class="rounded-md border border-gray-300 px-2 py-1.5 text-[11px] font-semibold hover:bg-gray-50 disabled:opacity-50 dark:border-gray-700 dark:hover:bg-gray-800"
                    @click="exportReport('markdown')"
                  >
                    {{ t('doctor.center.exportMarkdown') }}
                  </button>
                  <button
                    type="button"
                    :disabled="store.isBusy"
                    class="rounded-md border border-gray-300 px-2 py-1.5 text-[11px] font-semibold hover:bg-gray-50 disabled:opacity-50 dark:border-gray-700 dark:hover:bg-gray-800"
                    @click="exportReport('json')"
                  >
                    {{ t('doctor.center.exportJson') }}
                  </button>
                </div>
              </div>
            </details>
          </aside>
        </div>
      </template>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { save } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useDoctorStore } from '@/stores/doctor'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { logError } from '@/services/logger'
import { DOCTOR_SECTION_ORDER, isDoctorRunStale } from '@/utils/doctor'
import { formatDoctorDuration } from '@/utils/doctorPresentation'
import type { DoctorReportFormat } from '@/utils/doctorReport'
import type {
  DoctorCheckOutcome,
  DoctorCheckResult,
  DoctorRemediation,
  DoctorRunMode,
  DoctorSection,
} from '@/types/doctor'

const { t, te, locale } = useI18n()
const router = useRouter()
const appStore = useAppStore()
const store = useDoctorStore()
const modalStore = useModalStore()
const toastStore = useToastStore()
const includeSensitive = ref(false)
const selectedCheckId = ref<string | null>(null)

const CHECK_NAME_KEYS: Record<string, string> = {
  'installation.structure': 'installationStructure',
  'installation.xplane_running': 'xplaneRunning',
  'storage.xplane_space': 'xplaneSpace',
  'installation.location': 'installationLocation',
  'installation.readonly': 'readonlyFiles',
  'system.injectors': 'graphicsInjectors',
  'scenery.competing_organizer': 'competingOrganizer',
  'system.inventory': 'systemInventory',
  'performance.available_memory': 'availableMemory',
  'storage.app_data_space': 'appDataSpace',
  'xfast.app_data': 'appDataWritable',
  'xfast.database': 'database',
  'xfast.scenery_index': 'sceneryIndex',
  'stability.log_available': 'logAvailable',
  'stability.last_session': 'lastSession',
  'stability.crash_dump': 'crashDump',
  'system.beta_build': 'betaBuild',
  'scenery.global_airports': 'globalAirports',
  'scenery.load_order': 'loadOrder',
  'scenery.dependencies': 'sceneryDependencies',
  'scenery.overlaps': 'sceneryOverlaps',
  'scenery.duplicate_airports': 'duplicateAirports',
  'scenery.flatten': 'flattenOverrides',
  'navdata.custom_data': 'customNavdata',
  'navdata.cycles': 'navdataCycles',
  'navdata.cifp': 'cifp',
  'navdata.integrity': 'navdataIntegrity',
  'navdata.consistency': 'navdataConsistency',
  'xfast.recent_failures': 'recentFailures',
  'storage.cleanable': 'cleanableFiles',
  'addons.inventory': 'addonInventory',
  'addons.platform': 'addonPlatform',
  'updates.addons': 'addonUpdates',
  'updates.gateway': 'gatewayUpdates',
  'updates.app': 'appUpdate',
  'updates.csl': 'cslUpdates',
}

const LEGACY_TEXT_ROOTS: Record<string, string> = {
  'installation.structure': 'doctor.checks.integrity.missing_core_dirs',
  'installation.xplane_running': 'doctor.checks.integrity.xplane_running',
  'storage.xplane_space': 'doctor.checks.disk.low_space',
  'installation.location': 'doctor.checks.integrity.program_files',
  'installation.readonly': 'doctor.checks.integrity.readonly_files',
  'system.injectors': 'doctor.checks.environment.injectors',
  'scenery.competing_organizer': 'doctor.checks.scenery.competing_organizer',
  'stability.last_session': 'doctor.checks.crashes.last_session_crashed',
  'system.beta_build': 'doctor.checks.environment.beta_build',
  'scenery.global_airports': 'doctor.checks.scenery.global_airports_disabled',
  'scenery.load_order': 'doctor.checks.scenery.needs_sort',
  'scenery.dependencies': 'doctor.checks.scenery.missing_libraries',
  'scenery.overlaps': 'doctor.checks.scenery.duplicate_tiles',
  'scenery.duplicate_airports': 'doctor.checks.scenery.duplicate_airports',
  'scenery.flatten': 'doctor.checks.scenery.flatten_drift',
  'navdata.custom_data': 'doctor.checks.navdata.no_custom_data',
  'navdata.cifp': 'doctor.checks.navdata.cifp_missing',
  'navdata.integrity': 'doctor.checks.navdata.earth_dat_missing',
  'navdata.consistency': 'doctor.checks.navdata.cycle_mismatch',
  'xfast.recent_failures': 'doctor.checks.integrity.recent_failures',
  'storage.cleanable': 'doctor.checks.disk.cleanable_caches',
  'updates.addons': 'doctor.checks.updates.addons',
  'updates.gateway': 'doctor.checks.updates.gateway',
  'updates.app': 'doctor.checks.updates.app',
}

const SECTION_ICONS: Record<DoctorSection, string> = {
  installation: 'M4 7h16M6 3h12a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V5a2 2 0 012-2z',
  stability: 'M4 12h3l2-5 3 10 3-7 2 2h3',
  scenery: 'M3 20l6-16 4 10 2-5 6 11H3z',
  addons: 'M8 3v3m8-3v3M5 9h14v4a7 7 0 01-14 0V9z',
  navdata: 'M12 21a9 9 0 100-18 9 9 0 000 18zm0-4l2-6-6 2 4 4z',
  performance:
    'M12 3v2m6.36.64l-1.42 1.42M21 12h-2M5.05 5.05L6.46 6.46M3 12h2m7 9a7 7 0 100-14 7 7 0 000 14z',
  storage:
    'M4 7c0 2.2 3.58 4 8 4s8-1.8 8-4-3.58-4-8-4-8 1.8-8 4zm0 0v10c0 2.2 3.58 4 8 4s8-1.8 8-4V7',
  system: 'M9.75 3h4.5l.75 3H9l.75-3zM5 8h14v11H5V8zm3 4h8',
  xfast: 'M13 2L3 14h8l-1 8 11-13h-8V2z',
  updates: 'M4 4v6h6M20 20v-6h-6M5.6 15a7 7 0 0012.8 1M18.4 9A7 7 0 005.6 8',
}

const run = computed(() => store.displayedRun)
const displayedRunIsStale = computed(() => isDoctorRunStale(run.value))
const canRepair = computed(() => store.canRepairCurrentRun)
const errorMessage = computed(() => {
  if (store.error === 'xplane_running') return t('doctor.center.messages.xplaneRunning')
  if (store.error === 'xplane_path_missing') return t('doctor.center.messages.noPathDescription')
  return t('doctor.center.messages.fixFailed')
})
const safeFixCount = computed(
  () =>
    run.value?.checks.filter(
      (check) => check.remediation?.kind === 'automatic' && check.remediation.risk === 'safe',
    ).length ?? 0,
)
const checkGroups = computed(() =>
  DOCTOR_SECTION_ORDER.flatMap((section) => {
    const checks = run.value?.checks.filter((check) => check.section === section) ?? []
    if (!checks.length) return []
    return [
      {
        section,
        checks,
        issueCount: checks.filter((check) =>
          ['info', 'warning', 'critical', 'unavailable'].includes(check.outcome),
        ).length,
      },
    ]
  }),
)
const selectedCheck = computed(() => {
  const checks = run.value?.checks ?? []
  if (!checks.length) return null
  const explicit = selectedCheckId.value
    ? checks.find((check) => check.id === selectedCheckId.value)
    : null
  if (explicit) return explicit
  return (
    checks.find((check) => ['critical', 'warning', 'unavailable', 'info'].includes(check.outcome)) ??
    checks[0]
  )
})
const runtimeCompleted = computed(
  () =>
    store.checkRuntime.filter((runtime) => ['completed', 'cancelled'].includes(runtime.state))
      .length,
)
const runtimePercent = computed(() =>
  store.checkRuntime.length
    ? Math.round((runtimeCompleted.value / store.checkRuntime.length) * 100)
    : 0,
)
const healthTitle = computed(() => {
  if (!run.value) return ''
  if (run.value.state === 'running') {
    return store.phase === 'network'
      ? t('doctor.center.networkPhase')
      : t('doctor.center.localPhase')
  }
  if (run.value.summary.severity === 'critical') return t('doctor.center.status.critical')
  if (run.value.summary.severity === 'warning') return t('doctor.center.status.attention')
  if (run.value.summary.severity === 'info') return t('doctor.center.status.informational')
  if (run.value.summary.completeness !== 'complete') return t('doctor.center.status.incomplete')
  return t('doctor.center.status.allClear')
})
const systemItems = computed(() => {
  const system = run.value?.system
  if (!system) return []
  return [
    {
      label: t('doctor.center.system.os'),
      value: [system.os, system.osVersion, system.architecture].filter(Boolean).join(' · '),
    },
    {
      label: t('doctor.center.system.cpu'),
      value: `${system.cpuModel || t('common.unknown')} · ${system.logicalCores}`,
    },
    {
      label: t('doctor.center.system.memory'),
      value: `${formatBytes(system.availableMemoryBytes)} / ${formatBytes(system.totalMemoryBytes)}`,
    },
    {
      label: t('doctor.center.system.gpu'),
      value: [system.gpuModel, system.gpuDriver].filter(Boolean).join(' · ') || t('common.unknown'),
    },
    {
      label: t('doctor.center.system.xplane'),
      value: system.xplaneVersionRaw || t('common.unknown'),
    },
    {
      label: t('doctor.center.system.xplaneDisk'),
      value: formatDisk(system.xplaneFreeBytes, system.xplaneTotalBytes),
    },
    {
      label: t('doctor.center.system.appDataDisk'),
      value: formatDisk(system.appDataFreeBytes, system.appDataTotalBytes),
    },
  ]
})

watch(
  () => appStore.xplanePath,
  async (path) => {
    await store.loadHistory()
    if (path) await store.runAutomaticQuickCheck()
  },
  { immediate: true },
)

watch(
  () => (run.value ? `${run.value.id}:${run.value.completedAt ?? 'running'}` : null),
  () => {
    includeSensitive.value = false
  },
)

watch(
  () => run.value?.id ?? null,
  () => {
    selectedCheckId.value = null
  },
)

async function runDiagnostics(mode: DoctorRunMode) {
  store.selectHistoryRun(null)
  await store.runDiagnostics(mode)
}

function selectCheck(check: DoctorCheckResult) {
  selectedCheckId.value = check.id
}

function sectionLabel(section: DoctorSection): string {
  return t(`doctor.center.sections.${section}`)
}

function sectionIcon(section: DoctorSection): string {
  return SECTION_ICONS[section]
}

function checkLabel(check: DoctorCheckResult): string {
  const key = CHECK_NAME_KEYS[check.id]
  if (key) return t(`doctor.center.checks.${key}`, check.params ?? {})
  if (check.id.startsWith('log.')) {
    const category = check.id.slice(4)
    const logKey = `doctor.checks.log.${category}.title`
    if (te(logKey)) return t(logKey, check.params ?? {})
  }
  if (check.id.startsWith('diagnostic.')) {
    return t('doctor.center.checks.diagnosticProbe', {
      probe: check.id.slice('diagnostic.'.length),
    })
  }
  return t('doctor.center.checks.generic', { id: check.id })
}

function legacyTextRoot(check: DoctorCheckResult): string | null {
  if (check.id.startsWith('log.')) {
    const root = `doctor.checks.log.${check.id.slice(4)}`
    return te(`${root}.description`) ? root : null
  }
  return LEGACY_TEXT_ROOTS[check.id] ?? null
}

function checkDescription(check: DoctorCheckResult): string {
  const root = legacyTextRoot(check)
  if (root && check.outcome !== 'pass' && te(`${root}.description`)) {
    return t(`${root}.description`, check.params ?? {})
  }
  return t(`doctor.center.checkDescriptions.${check.outcome}`)
}

function checkSuggestion(check: DoctorCheckResult): string {
  const root = legacyTextRoot(check)
  if (root && check.outcome !== 'pass' && te(`${root}.suggestion`)) {
    return t(`${root}.suggestion`, check.params ?? {})
  }
  if (check.remediation?.kind === 'automatic')
    return t('doctor.center.remediation.automaticAvailable')
  if (check.outcome === 'critical' || check.outcome === 'warning')
    return t('doctor.center.remediation.manualReview')
  return ''
}

function outcomeLabel(outcome: DoctorCheckOutcome): string {
  return t(`doctor.center.outcomes.${outcome}`)
}

function formatDateTime(timestamp: number): string {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(
    timestamp,
  )
}

const formatDuration = formatDoctorDuration

function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes)) return t('common.unknown')
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`
}

function formatDisk(free: number | null, total: number | null): string {
  if (free === null || total === null) return t('common.unknown')
  return `${formatBytes(free)} / ${formatBytes(total)}`
}

function formatParamKey(key: string): string {
  return key.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/_/g, ' ')
}

function severityDotClass(severity: string, completeness: string): string {
  if (severity === 'critical') return 'bg-red-500'
  if (severity === 'warning') return 'bg-amber-500'
  if (severity === 'info') return 'bg-blue-500'
  if (completeness !== 'complete') return 'bg-gray-400'
  return 'bg-emerald-500'
}

function historyStatusLabel(severity: string, completeness: string): string {
  if (severity === 'critical') return t('doctor.center.status.critical')
  if (severity === 'warning') return t('doctor.center.status.attention')
  if (severity === 'info') return t('doctor.center.status.informational')
  if (completeness !== 'complete') return t('doctor.center.status.incomplete')
  return t('doctor.center.status.allClear')
}

function outcomeBadgeClass(outcome: DoctorCheckOutcome): string {
  return {
    pass: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200',
    info: 'bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-200',
    warning: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-200',
    critical: 'bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-200',
    unavailable: 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-200',
    notApplicable: 'bg-gray-100 text-gray-500 dark:bg-gray-700/60 dark:text-gray-400',
    cancelled: 'bg-gray-100 text-gray-500 dark:bg-gray-700/60 dark:text-gray-400',
  }[outcome]
}



function remediationButtonClass(remediation: DoctorRemediation): string {
  if (remediation.kind !== 'automatic')
    return 'border-gray-300 bg-white text-gray-700 hover:bg-gray-50 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700'
  if (remediation.risk === 'safe')
    return 'border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-200'
  return 'border-amber-200 bg-amber-50 text-amber-800 hover:bg-amber-100 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-200'
}

function remediationLabel(remediation: DoctorRemediation): string {
  if (remediation.kind === 'navigate') return t('doctor.center.remediation.open')
  if (remediation.kind === 'guide') return t('doctor.center.remediation.guide')
  return t('doctor.center.remediation.apply')
}

async function handleRemediation(check: DoctorCheckResult) {
  const remediation = check.remediation
  if (!remediation) return
  if (remediation.kind === 'navigate' && remediation.route) {
    await router.push(remediation.route)
    return
  }
  if (remediation.kind === 'guide') {
    if (remediation.route) await router.push(remediation.route)
    else toastStore.info(t('doctor.center.remediation.manualReview'))
    return
  }
  if (!canRepair.value) {
    toastStore.info(t('doctor.center.messages.historyOnly'))
    return
  }
  if (remediation.risk !== 'safe') {
    modalStore.showConfirm({
      title: t('doctor.center.remediation.confirmTitle'),
      message: t('doctor.center.remediation.confirmBody'),
      confirmText: t('doctor.center.remediation.apply'),
      cancelText: t('common.cancel'),
      type: remediation.risk === 'destructive' ? 'danger' : 'warning',
      onConfirm: () => void applyRepair(check),
      onCancel: () => {},
    })
    return
  }
  await applyRepair(check)
}

async function applyRepair(check: DoctorCheckResult) {
  const applied = await store.applyRemediation(check)
  if (applied) toastStore.success(t('doctor.center.messages.fixSuccess'))
  else
    toastStore.error(
      store.error === 'xplane_running'
        ? t('doctor.center.messages.xplaneRunning')
        : t('doctor.center.messages.fixFailed'),
    )
}

async function applyAllSafeFixes() {
  const result = await store.applyAllSafeFixes()
  if (store.error === 'xplane_running') {
    toastStore.error(t('doctor.center.messages.xplaneRunning'))
    return
  }
  const message = t('doctor.center.messages.batchResult', {
    applied: result.applied,
    failed: result.failed,
  })
  if (result.failed) toastStore.error(message)
  else toastStore.success(message)
}

function toggleSensitivePaths() {
  if (includeSensitive.value) {
    includeSensitive.value = false
    return
  }
  modalStore.showConfirm({
    title: t('doctor.center.privacy.confirmTitle'),
    message: t('doctor.center.privacy.confirmBody'),
    confirmText: t('doctor.center.includeSensitive'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => {
      includeSensitive.value = true
    },
    onCancel: () => {},
  })
}

async function exportReport(format: DoctorReportFormat) {
  if (!run.value) return
  try {
    const extension = format === 'markdown' ? 'md' : 'json'
    const selected = await save({
      defaultPath: `xfast-health-${new Date(run.value.startedAt).toISOString().slice(0, 10)}.${extension}`,
      filters: [
        {
          name: format === 'markdown' ? 'Markdown' : 'JSON',
          extensions: [extension],
        },
      ],
    })
    if (!selected || Array.isArray(selected)) return
    await store.exportReport(selected, format, includeSensitive.value, {
      checkLabel,
      sectionLabel: (check) => sectionLabel(check.section),
    })
    toastStore.success(t('doctor.center.messages.exportSuccess'))
  } catch (reason) {
    logError(`Health report export failed: ${reason}`, 'doctor')
    toastStore.error(t('doctor.center.messages.exportFailed'))
  }
}

const coverageBarClass = computed(() => {
  if (run.value?.summary.completeness !== 'complete') return 'bg-gray-500'
  if (run.value?.summary.severity === 'critical') return 'bg-red-500'
  if (run.value?.summary.severity === 'warning') return 'bg-amber-500'
  if (run.value?.summary.severity === 'info') return 'bg-blue-500'
  return 'bg-emerald-500'
})

const modeBadgeClass = computed(() =>
  run.value?.mode === 'full'
    ? 'border-violet-200 bg-violet-50 text-violet-700 dark:border-violet-900 dark:bg-violet-950/40 dark:text-violet-200'
    : 'border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-900 dark:bg-blue-950/40 dark:text-blue-200',
)
</script>
