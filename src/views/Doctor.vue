<template>
  <div class="min-h-full bg-slate-50 text-slate-950 dark:bg-slate-950 dark:text-slate-100">
    <header
      class="border-b border-slate-200/80 bg-white/90 px-5 py-5 backdrop-blur-xl dark:border-slate-800 dark:bg-slate-900/90 lg:px-8"
    >
      <div
        class="mx-auto flex max-w-[1500px] flex-col gap-5 xl:flex-row xl:items-center xl:justify-between"
      >
        <div class="min-w-0">
          <div class="flex items-center gap-3">
            <div
              class="flex h-11 w-11 flex-none items-center justify-center rounded-2xl bg-blue-600 text-white shadow-lg shadow-blue-600/20"
            >
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 12h3l2-5 3 10 3-7 2 2h3M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78L12 21.23l8.84-8.84a5.5 5.5 0 000-7.78z"
                />
              </svg>
            </div>
            <div class="min-w-0">
              <h1 class="text-2xl font-bold tracking-tight text-slate-950 dark:text-white">
                {{ t('doctor.center.heading') }}
              </h1>
              <p class="mt-1 max-w-3xl text-sm leading-5 text-slate-600 dark:text-slate-400">
                {{ t('doctor.center.intro') }}
              </p>
            </div>
          </div>
        </div>

        <div v-if="appStore.xplanePath" class="flex flex-wrap items-center gap-2">
          <button
            v-if="store.isRunning"
            class="inline-flex items-center gap-2 rounded-xl border border-slate-300 bg-white px-4 py-2.5 text-sm font-semibold text-slate-700 shadow-sm transition hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700"
            @click="store.cancelRun()"
          >
            <span class="h-2.5 w-2.5 rounded-sm bg-current" />
            {{ t('doctor.center.cancel') }}
          </button>
          <template v-else>
            <button
              class="group rounded-xl border border-slate-300 bg-white px-4 py-2.5 text-left shadow-sm transition hover:border-blue-400 hover:bg-blue-50/60 dark:border-slate-700 dark:bg-slate-800 dark:hover:border-blue-500 dark:hover:bg-blue-950/30"
              @click="runDiagnostics('quick')"
            >
              <span class="block text-sm font-semibold text-slate-900 dark:text-white">
                {{ t('doctor.center.quickCheck') }}
              </span>
              <span class="mt-0.5 block text-[11px] text-slate-500 dark:text-slate-400">
                {{ t('doctor.center.quickHint') }}
              </span>
            </button>
            <button
              class="group rounded-xl bg-blue-600 px-4 py-2.5 text-left text-white shadow-lg shadow-blue-600/20 transition hover:bg-blue-700"
              @click="runDiagnostics('full')"
            >
              <span class="block text-sm font-semibold">{{ t('doctor.center.fullCheck') }}</span>
              <span class="mt-0.5 block text-[11px] text-blue-100">
                {{ t('doctor.center.fullHint') }}
              </span>
            </button>
          </template>
        </div>
      </div>
    </header>

    <main class="mx-auto max-w-[1500px] space-y-5 px-5 py-6 lg:px-8">
      <div
        v-if="store.error"
        class="flex items-start justify-between gap-4 rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800 dark:border-red-900/60 dark:bg-red-950/40 dark:text-red-200"
      >
        <span>{{
          store.error === 'xplane_running' ? t('doctor.center.messages.xplaneRunning') : store.error
        }}</span>
        <button class="font-bold" :aria-label="t('common.close')" @click="store.error = null">
          ×
        </button>
      </div>

      <section
        v-if="!appStore.xplanePath"
        class="flex min-h-[520px] items-center justify-center rounded-3xl border border-dashed border-slate-300 bg-white p-8 text-center shadow-sm dark:border-slate-700 dark:bg-slate-900"
      >
        <div class="max-w-lg">
          <div
            class="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-300"
          >
            <svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.7"
                d="M9 12h6m-3-3v6m9-3a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          </div>
          <h2 class="mt-5 text-xl font-bold">{{ t('doctor.center.messages.noPathTitle') }}</h2>
          <p class="mt-2 text-sm leading-6 text-slate-600 dark:text-slate-400">
            {{ t('doctor.center.messages.noPathDescription') }}
          </p>
          <router-link
            to="/settings"
            class="mt-6 inline-flex rounded-xl bg-blue-600 px-5 py-2.5 text-sm font-semibold text-white transition hover:bg-blue-700"
          >
            {{ t('doctor.center.messages.configurePath') }}
          </router-link>
        </div>
      </section>

      <template v-else>
        <div
          class="flex items-center gap-2 rounded-xl border border-blue-100 bg-blue-50/70 px-4 py-2.5 text-xs text-blue-800 dark:border-blue-900/50 dark:bg-blue-950/30 dark:text-blue-200"
        >
          <svg class="h-4 w-4 flex-none" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          {{ t('doctor.center.automaticQuick') }}
        </div>

        <section
          v-if="run"
          class="overflow-hidden rounded-3xl border shadow-sm"
          :class="summaryCardClass"
        >
          <div class="grid gap-6 p-5 lg:grid-cols-[minmax(280px,0.9fr)_minmax(360px,1.2fr)] lg:p-6">
            <div class="flex items-start gap-4">
              <div
                class="flex h-14 w-14 flex-none items-center justify-center rounded-2xl"
                :class="summaryIconClass"
              >
                <svg
                  v-if="run.summary.severity === 'ok' && run.summary.completeness === 'complete'"
                  class="h-7 w-7"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2.4"
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                <svg v-else class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
              </div>
              <div class="min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                  <h2 class="text-xl font-bold" :class="summaryTextClass">{{ healthTitle }}</h2>
                  <span
                    class="rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide"
                    :class="modeBadgeClass"
                  >
                    {{
                      run.mode === 'quick'
                        ? t('doctor.center.history.quick')
                        : t('doctor.center.history.full')
                    }}
                  </span>
                  <span
                    v-if="store.selectedRunId"
                    class="rounded-full bg-slate-200 px-2 py-0.5 text-[10px] font-semibold text-slate-700 dark:bg-slate-700 dark:text-slate-200"
                  >
                    {{ t('doctor.center.status.viewingHistory') }}
                  </span>
                </div>
                <p v-if="run.completedAt" class="mt-2 text-xs text-slate-600 dark:text-slate-400">
                  {{
                    t('doctor.center.status.lastChecked', { time: formatDateTime(run.completedAt) })
                  }}
                  <span class="px-1">·</span>
                  {{
                    t('doctor.center.status.duration', { duration: formatDuration(run.durationMs) })
                  }}
                </p>
                <p v-else class="mt-2 text-xs font-medium text-blue-700 dark:text-blue-300">
                  {{
                    store.phase === 'network'
                      ? t('doctor.center.networkPhase')
                      : t('doctor.center.localPhase')
                  }}
                </p>
              </div>
            </div>

            <div class="space-y-4">
              <div>
                <div
                  class="mb-2 flex items-center justify-between text-xs font-medium text-slate-700 dark:text-slate-300"
                >
                  <span>{{ t('doctor.center.status.coverage') }}</span>
                  <span>{{ run.summary.coveragePercent }}%</span>
                </div>
                <div class="h-2.5 overflow-hidden rounded-full bg-slate-200/80 dark:bg-slate-700">
                  <div
                    class="h-full rounded-full transition-all duration-500"
                    :class="coverageBarClass"
                    :style="{ width: `${run.summary.coveragePercent}%` }"
                  />
                </div>
                <p class="mt-2 text-[11px] text-slate-500 dark:text-slate-400">
                  {{
                    t('doctor.center.status.checksCovered', {
                      covered: run.summary.covered,
                      eligible: run.summary.eligible,
                    })
                  }}
                </p>
              </div>

              <div class="grid grid-cols-3 gap-2 sm:grid-cols-6">
                <div
                  v-for="stat in summaryStats"
                  :key="stat.key"
                  class="rounded-xl border border-white/60 bg-white/60 px-2 py-2 text-center dark:border-slate-700/60 dark:bg-slate-900/40"
                >
                  <div class="text-lg font-bold" :class="stat.className">{{ stat.value }}</div>
                  <div class="truncate text-[10px] text-slate-500 dark:text-slate-400">
                    {{ stat.label }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        <div
          v-if="run && displayedRunIsStale && !store.isRunning"
          class="flex items-start gap-3 rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-200"
        >
          <svg
            class="mt-0.5 h-5 w-5 flex-none"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <div>
            <strong>{{ t('doctor.center.status.stale') }}</strong>
            <p class="mt-0.5 text-xs opacity-90">
              {{ t('doctor.center.status.staleDescription') }}
            </p>
          </div>
        </div>

        <div
          v-if="run && run.summary.completeness !== 'complete' && !store.isRunning"
          class="flex items-start gap-3 rounded-2xl border border-slate-300 bg-slate-100 px-4 py-3 text-sm text-slate-800 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200"
        >
          <svg
            class="mt-0.5 h-5 w-5 flex-none"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M8 9h8m-8 4h6m7-1a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <div>
            <strong>{{ t('doctor.center.status.incomplete') }}</strong>
            <p class="mt-0.5 text-xs opacity-90">
              {{
                run.summary.completeness === 'cancelled'
                  ? t('doctor.center.status.cancelledDescription')
                  : t('doctor.center.status.partialDescription')
              }}
            </p>
          </div>
        </div>

        <section
          v-if="store.isRunning"
          class="rounded-2xl border border-blue-200 bg-white p-5 shadow-sm dark:border-blue-900/60 dark:bg-slate-900"
        >
          <div class="flex items-center justify-between gap-4">
            <div>
              <h2 class="font-semibold text-slate-900 dark:text-white">
                {{
                  store.phase === 'network'
                    ? t('doctor.center.networkPhase')
                    : t('doctor.center.localPhase')
                }}
              </h2>
              <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
                {{
                  t('doctor.center.progress', {
                    done: runtimeCompleted,
                    total: store.checkRuntime.length,
                  })
                }}
              </p>
            </div>
            <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">
              {{ runtimePercent }}%
            </div>
          </div>
          <div class="mt-4 h-2 overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800">
            <div
              class="h-full rounded-full bg-blue-600 transition-all duration-300"
              :style="{ width: `${runtimePercent}%` }"
            />
          </div>
          <div class="mt-4 flex flex-wrap gap-2">
            <span
              v-for="runtime in store.checkRuntime"
              :key="runtime.id"
              class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px]"
              :class="runtimeClass(runtime.state)"
            >
              <span class="h-1.5 w-1.5 rounded-full" :class="runtimeDotClass(runtime.state)" />
              {{ sectionLabel(runtime.section) }}
            </span>
          </div>
        </section>

        <section
          v-if="!run && !store.isRunning"
          class="flex min-h-[420px] items-center justify-center rounded-3xl border border-dashed border-slate-300 bg-white p-8 text-center shadow-sm dark:border-slate-700 dark:bg-slate-900"
        >
          <div class="max-w-lg">
            <div
              class="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl bg-blue-50 text-blue-600 dark:bg-blue-950/50 dark:text-blue-300"
            >
              <svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="1.7"
                  d="M9 12h6m-3-3v6m8 3.5A9 9 0 113.5 6M18 3v5h-5"
                />
              </svg>
            </div>
            <h2 class="mt-5 text-xl font-bold">{{ t('doctor.center.messages.noResultTitle') }}</h2>
            <p class="mt-2 text-sm leading-6 text-slate-600 dark:text-slate-400">
              {{ t('doctor.center.messages.noResultDescription') }}
            </p>
            <div class="mt-6 flex justify-center gap-2">
              <button
                class="rounded-xl border border-slate-300 bg-white px-4 py-2.5 text-sm font-semibold hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700"
                @click="runDiagnostics('quick')"
              >
                {{ t('doctor.center.quickCheck') }}
              </button>
              <button
                class="rounded-xl bg-blue-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-blue-700"
                @click="runDiagnostics('full')"
              >
                {{ t('doctor.center.fullCheck') }}
              </button>
            </div>
          </div>
        </section>

        <div v-if="run" class="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_330px]">
          <div class="min-w-0 space-y-4">
            <div
              v-if="safeFixCount > 0 && canRepair"
              class="flex flex-col gap-3 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 dark:border-emerald-900/60 dark:bg-emerald-950/30 sm:flex-row sm:items-center sm:justify-between"
            >
              <div>
                <div class="text-sm font-semibold text-emerald-900 dark:text-emerald-100">
                  {{ t('doctor.center.remediation.safe') }}
                </div>
                <p class="mt-0.5 text-xs text-emerald-800/80 dark:text-emerald-200/80">
                  {{ safeFixCount }} · {{ t('doctor.center.remediation.automaticAvailable') }}
                </p>
              </div>
              <button
                :disabled="store.repairingAll || store.xplaneRunning"
                class="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-emerald-700 disabled:cursor-not-allowed disabled:opacity-50"
                @click="applyAllSafeFixes"
              >
                {{
                  store.repairingAll
                    ? t('doctor.center.remediation.fixingAll')
                    : t('doctor.center.remediation.fixAllSafe')
                }}
              </button>
            </div>

            <article
              v-for="group in checkGroups"
              :key="group.section"
              class="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900"
            >
              <header
                class="flex items-center justify-between border-b border-slate-200 px-4 py-3 dark:border-slate-800"
              >
                <div class="flex items-center gap-2.5">
                  <span
                    class="flex h-8 w-8 items-center justify-center rounded-lg bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300"
                    ><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        :d="sectionIcon(group.section)"
                      /></svg
                  ></span>
                  <h2 class="font-semibold">{{ sectionLabel(group.section) }}</h2>
                </div>
                <div class="flex items-center gap-2 text-[11px] text-slate-500 dark:text-slate-400">
                  <span
                    v-if="group.issueCount"
                    class="rounded-full bg-amber-100 px-2 py-0.5 font-semibold text-amber-800 dark:bg-amber-950 dark:text-amber-200"
                    >{{ group.issueCount }}</span
                  ><span>{{ group.checks.length }}</span>
                </div>
              </header>

              <div class="divide-y divide-slate-100 dark:divide-slate-800">
                <div
                  v-for="check in group.checks"
                  :key="`${run.id}-${check.id}`"
                  class="p-4"
                  :class="checkRowClass(check.outcome)"
                >
                  <div class="flex items-start gap-3">
                    <span
                      class="mt-0.5 flex h-7 w-7 flex-none items-center justify-center rounded-lg"
                      :class="outcomeIconClass(check.outcome)"
                    >
                      <svg
                        v-if="check.outcome === 'pass'"
                        class="h-4 w-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2.5"
                          d="M5 13l4 4L19 7"
                        />
                      </svg>
                      <svg
                        v-else-if="check.outcome === 'cancelled'"
                        class="h-4 w-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M6 6l12 12M18 6L6 18"
                        />
                      </svg>
                      <svg
                        v-else
                        class="h-4 w-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M12 9v2m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                        />
                      </svg>
                    </span>

                    <div class="min-w-0 flex-1">
                      <div
                        class="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between"
                      >
                        <div class="min-w-0">
                          <div class="flex flex-wrap items-center gap-2">
                            <h3 class="text-sm font-semibold text-slate-900 dark:text-white">
                              {{ checkLabel(check) }}
                            </h3>
                            <span
                              class="rounded-full px-2 py-0.5 text-[10px] font-bold"
                              :class="outcomeBadgeClass(check.outcome)"
                              >{{ outcomeLabel(check.outcome) }}</span
                            >
                            <span class="text-[10px] text-slate-400">{{
                              formatDuration(check.durationMs)
                            }}</span>
                          </div>
                          <p class="mt-1 text-xs leading-5 text-slate-600 dark:text-slate-400">
                            {{ checkDescription(check) }}
                          </p>
                          <p
                            v-if="checkSuggestion(check)"
                            class="mt-1.5 text-xs font-medium text-blue-700 dark:text-blue-300"
                          >
                            {{ checkSuggestion(check) }}
                          </p>
                        </div>

                        <button
                          v-if="check.remediation"
                          :disabled="
                            store.fixingId === check.id ||
                            store.isRunning ||
                            (check.remediation.kind === 'automatic' && !canRepair)
                          "
                          class="flex-none rounded-lg border px-3 py-1.5 text-xs font-semibold transition disabled:cursor-not-allowed disabled:opacity-50"
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

                      <div
                        v-if="check.params && Object.keys(check.params).length"
                        class="mt-2 flex flex-wrap gap-1.5"
                      >
                        <span
                          v-for="(value, key) in check.params"
                          :key="key"
                          class="rounded-md bg-slate-100 px-2 py-1 font-mono text-[10px] text-slate-600 dark:bg-slate-800 dark:text-slate-300"
                          >{{ formatParamKey(String(key)) }}: {{ value }}</span
                        >
                      </div>

                      <details v-if="check.evidence?.length" class="group mt-3">
                        <summary
                          class="inline-flex cursor-pointer list-none items-center gap-1.5 text-xs font-medium text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200"
                        >
                          <svg
                            class="h-3.5 w-3.5 transition group-open:rotate-90"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                          >
                            <path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M9 5l7 7-7 7"
                            />
                          </svg>
                          {{ t('doctor.center.evidence.show') }} ({{ check.evidence.length }})
                        </summary>
                        <div
                          class="mt-2 max-h-56 space-y-1 overflow-y-auto rounded-xl border border-slate-200 bg-slate-50 p-3 font-mono text-[11px] leading-5 text-slate-700 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-300"
                        >
                          <div
                            v-for="(item, index) in check.evidence"
                            :key="`${index}-${item.value}`"
                            class="break-all"
                          >
                            <span v-if="item.label" class="font-semibold">{{ item.label }}: </span
                            >{{ item.value }}
                          </div>
                        </div>
                      </details>
                    </div>
                  </div>
                </div>
              </div>
            </article>
          </div>

          <aside class="space-y-4 xl:sticky xl:top-4">
            <section
              class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900"
            >
              <div class="flex items-center justify-between">
                <h2 class="text-sm font-semibold">{{ t('doctor.center.export') }}</h2>
                <svg
                  class="h-4 w-4 text-slate-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 10v6m0 0l-3-3m3 3l3-3m2 7H7a2 2 0 01-2-2V6a2 2 0 012-2h4l2 2h4a2 2 0 012 2v10a2 2 0 01-2 2z"
                  />
                </svg>
              </div>
              <div
                class="mt-3 rounded-xl border px-3 py-2.5 text-xs"
                :class="
                  includeSensitive
                    ? 'border-amber-200 bg-amber-50 text-amber-900 dark:border-amber-900 dark:bg-amber-950/30 dark:text-amber-200'
                    : 'border-emerald-200 bg-emerald-50 text-emerald-900 dark:border-emerald-900 dark:bg-emerald-950/30 dark:text-emerald-200'
                "
              >
                <div class="font-semibold">{{ t('doctor.center.privacy.title') }}</div>
                <p class="mt-1 leading-4">
                  {{
                    includeSensitive
                      ? t('doctor.center.privacy.sensitive')
                      : t('doctor.center.privacy.redacted')
                  }}
                </p>
                <button class="mt-2 underline underline-offset-2" @click="toggleSensitivePaths">
                  {{
                    includeSensitive
                      ? t('doctor.center.excludeSensitive')
                      : t('doctor.center.includeSensitive')
                  }}
                </button>
              </div>
              <div class="mt-3 grid grid-cols-2 gap-2">
                <button
                  class="rounded-xl border border-slate-300 px-3 py-2 text-xs font-semibold hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800"
                  @click="exportReport('markdown')"
                >
                  {{ t('doctor.center.exportMarkdown') }}
                </button>
                <button
                  class="rounded-xl border border-slate-300 px-3 py-2 text-xs font-semibold hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800"
                  @click="exportReport('json')"
                >
                  {{ t('doctor.center.exportJson') }}
                </button>
              </div>
            </section>

            <section
              class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900"
            >
              <div class="flex items-center justify-between gap-2">
                <h2 class="text-sm font-semibold">{{ t('doctor.center.history.title') }}</h2>
                <span
                  class="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-semibold text-slate-500 dark:bg-slate-800 dark:text-slate-300"
                  >{{ store.history.length }}/20</span
                >
              </div>
              <p class="mt-1 text-[11px] leading-4 text-slate-500 dark:text-slate-400">
                {{ t('doctor.center.history.description') }}
              </p>
              <button
                v-if="store.selectedRunId"
                class="mt-3 w-full rounded-lg border border-blue-200 bg-blue-50 px-3 py-2 text-xs font-semibold text-blue-700 dark:border-blue-900 dark:bg-blue-950/40 dark:text-blue-200"
                @click="store.selectHistoryRun(null)"
              >
                {{ t('doctor.center.history.backToCurrent') }}
              </button>
              <div
                v-if="store.history.length"
                class="mt-3 max-h-72 space-y-1.5 overflow-y-auto pr-1"
              >
                <button
                  v-for="item in store.history"
                  :key="item.id"
                  class="w-full rounded-xl border px-3 py-2.5 text-left transition"
                  :class="
                    store.displayedRun?.id === item.id
                      ? 'border-blue-300 bg-blue-50 dark:border-blue-800 dark:bg-blue-950/30'
                      : 'border-slate-200 hover:bg-slate-50 dark:border-slate-800 dark:hover:bg-slate-800'
                  "
                  @click="store.selectHistoryRun(item.id)"
                >
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-xs font-semibold">{{ formatDateTime(item.startedAt) }}</span
                    ><span
                      class="h-2 w-2 rounded-full"
                      :class="severityDotClass(item.summary.severity, item.summary.completeness)"
                    />
                  </div>
                  <div
                    class="mt-1 flex items-center justify-between text-[10px] text-slate-500 dark:text-slate-400"
                  >
                    <span>{{
                      item.mode === 'quick'
                        ? t('doctor.center.history.quick')
                        : t('doctor.center.history.full')
                    }}</span
                    ><span>{{ item.summary.coveragePercent }}%</span>
                  </div>
                </button>
              </div>
              <p v-else class="mt-4 text-center text-xs text-slate-500">
                {{ t('doctor.center.history.empty') }}
              </p>
            </section>

            <section
              v-if="run.system"
              class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900"
            >
              <h2 class="text-sm font-semibold">{{ t('doctor.center.system.title') }}</h2>
              <dl class="mt-3 space-y-2.5 text-xs">
                <div
                  v-for="item in systemItems"
                  :key="item.label"
                  class="grid grid-cols-[100px_minmax(0,1fr)] gap-2"
                >
                  <dt class="text-slate-500 dark:text-slate-400">{{ item.label }}</dt>
                  <dd
                    class="min-w-0 break-words text-right font-medium text-slate-800 dark:text-slate-200"
                  >
                    {{ item.value }}
                  </dd>
                </div>
              </dl>
            </section>
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
import { DOCTOR_SECTION_ORDER, isDoctorRunStale } from '@/utils/doctor'
import type { DoctorReportFormat } from '@/utils/doctorReport'
import type {
  DoctorCheckOutcome,
  DoctorCheckResult,
  DoctorCheckRuntimeState,
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
const canRepair = computed(() => store.currentRunIsLive && !store.selectedRunId && !store.isRunning)
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
  if (run.value.summary.severity === 'critical') return t('doctor.center.status.critical')
  if (run.value.summary.severity === 'warning') return t('doctor.center.status.attention')
  if (run.value.summary.severity === 'info') return t('doctor.center.status.informational')
  if (run.value.summary.completeness !== 'complete') return t('doctor.center.status.incomplete')
  return t('doctor.center.status.allClear')
})
const summaryStats = computed(() => {
  if (!run.value) return []
  return [
    {
      key: 'pass',
      value: run.value.summary.pass,
      label: t('doctor.center.outcomes.pass'),
      className: 'text-emerald-600 dark:text-emerald-400',
    },
    {
      key: 'info',
      value: run.value.summary.info,
      label: t('doctor.center.outcomes.info'),
      className: 'text-blue-600 dark:text-blue-400',
    },
    {
      key: 'warning',
      value: run.value.summary.warning,
      label: t('doctor.center.outcomes.warning'),
      className: 'text-amber-600 dark:text-amber-400',
    },
    {
      key: 'critical',
      value: run.value.summary.critical,
      label: t('doctor.center.outcomes.critical'),
      className: 'text-red-600 dark:text-red-400',
    },
    {
      key: 'unavailable',
      value: run.value.summary.unavailable,
      label: t('doctor.center.outcomes.unavailable'),
      className: 'text-slate-600 dark:text-slate-300',
    },
    {
      key: 'na',
      value: run.value.summary.notApplicable,
      label: t('doctor.center.outcomes.notApplicable'),
      className: 'text-slate-400',
    },
  ]
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

async function runDiagnostics(mode: DoctorRunMode) {
  store.selectHistoryRun(null)
  await store.runDiagnostics(mode)
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

function formatDuration(milliseconds: number): string {
  if (milliseconds < 1000) return `${milliseconds} ms`
  if (milliseconds < 60_000) return `${(milliseconds / 1000).toFixed(1)} s`
  return `${Math.floor(milliseconds / 60_000)}m ${Math.round((milliseconds % 60_000) / 1000)}s`
}

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
  if (completeness !== 'complete') return 'bg-slate-400'
  return 'bg-emerald-500'
}

function runtimeClass(state: DoctorCheckRuntimeState): string {
  if (state === 'running')
    return 'border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-900 dark:bg-blue-950/40 dark:text-blue-200'
  if (state === 'completed')
    return 'border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-200'
  if (state === 'cancelled')
    return 'border-slate-300 bg-slate-100 text-slate-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300'
  return 'border-slate-200 bg-white text-slate-400 dark:border-slate-800 dark:bg-slate-900'
}

function runtimeDotClass(state: DoctorCheckRuntimeState): string {
  if (state === 'running') return 'animate-pulse bg-blue-500'
  if (state === 'completed') return 'bg-emerald-500'
  return 'bg-slate-400'
}

function outcomeBadgeClass(outcome: DoctorCheckOutcome): string {
  return {
    pass: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200',
    info: 'bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-200',
    warning: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-200',
    critical: 'bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-200',
    unavailable: 'bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200',
    notApplicable: 'bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400',
    cancelled: 'bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400',
  }[outcome]
}

function outcomeIconClass(outcome: DoctorCheckOutcome): string {
  return outcomeBadgeClass(outcome)
}

function checkRowClass(outcome: DoctorCheckOutcome): string {
  if (outcome === 'critical') return 'bg-red-50/40 dark:bg-red-950/10'
  if (outcome === 'warning') return 'bg-amber-50/30 dark:bg-amber-950/10'
  if (outcome === 'unavailable') return 'bg-slate-50 dark:bg-slate-950/30'
  return ''
}

function remediationButtonClass(remediation: DoctorRemediation): string {
  if (remediation.kind !== 'automatic')
    return 'border-slate-300 bg-white text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700'
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
  const message = t('doctor.center.messages.batchResult', result)
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
  try {
    await store.exportReport(selected, format, includeSensitive.value, {
      checkLabel,
      sectionLabel: (check) => sectionLabel(check.section),
    })
    toastStore.success(t('doctor.center.messages.exportSuccess'))
  } catch (reason) {
    toastStore.error(`${t('doctor.center.messages.exportFailed')} ${String(reason)}`)
  }
}

const summaryCardClass = computed(() => {
  const severity = run.value?.summary.severity
  if (severity === 'critical')
    return 'border-red-200 bg-gradient-to-br from-red-50 to-white dark:border-red-900/60 dark:from-red-950/30 dark:to-slate-900'
  if (severity === 'warning')
    return 'border-amber-200 bg-gradient-to-br from-amber-50 to-white dark:border-amber-900/60 dark:from-amber-950/30 dark:to-slate-900'
  if (severity === 'info')
    return 'border-blue-200 bg-gradient-to-br from-blue-50 to-white dark:border-blue-900/60 dark:from-blue-950/30 dark:to-slate-900'
  if (run.value?.summary.completeness !== 'complete')
    return 'border-slate-300 bg-gradient-to-br from-slate-100 to-white dark:border-slate-700 dark:from-slate-800 dark:to-slate-900'
  return 'border-emerald-200 bg-gradient-to-br from-emerald-50 to-white dark:border-emerald-900/60 dark:from-emerald-950/30 dark:to-slate-900'
})
const summaryIconClass = computed(() => {
  const severity = run.value?.summary.severity
  if (severity === 'critical') return 'bg-red-100 text-red-700 dark:bg-red-950 dark:text-red-300'
  if (severity === 'warning')
    return 'bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300'
  if (severity === 'info') return 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300'
  if (run.value?.summary.completeness !== 'complete')
    return 'bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200'
  return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300'
})
const summaryTextClass = computed(() => {
  const severity = run.value?.summary.severity
  if (severity === 'critical') return 'text-red-900 dark:text-red-100'
  if (severity === 'warning') return 'text-amber-900 dark:text-amber-100'
  if (severity === 'info') return 'text-blue-900 dark:text-blue-100'
  return 'text-slate-900 dark:text-white'
})
const coverageBarClass = computed(() => {
  if (run.value?.summary.completeness !== 'complete') return 'bg-slate-500'
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
