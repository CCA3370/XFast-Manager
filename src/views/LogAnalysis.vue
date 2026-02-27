<template>
  <div class="log-analysis-view h-full flex flex-col p-5">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4 flex-shrink-0">
      <div>
        <h2 class="text-xl font-bold text-gray-900 dark:text-white">
          <AnimatedText>{{ $t('logAnalysis.title') }}</AnimatedText>
        </h2>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
          {{ $t('logAnalysis.subtitle') }}
        </p>
      </div>
      <div class="flex items-center space-x-2">
        <button
          v-if="result"
          class="flex items-center space-x-1.5 px-3 py-1.5 text-sm font-medium text-gray-600 dark:text-gray-300 bg-white/80 dark:bg-gray-800/40 border border-gray-200 dark:border-white/10 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
          @click="openLog"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
            />
          </svg>
          <span>{{ $t('logAnalysis.openLog') }}</span>
        </button>
        <button
          :disabled="!appStore.xplanePath || loading"
          class="flex items-center space-x-1.5 px-4 py-1.5 text-sm font-medium rounded-lg transition-colors"
          :class="
            appStore.xplanePath && !loading
              ? 'bg-blue-600 hover:bg-blue-700 text-white'
              : 'bg-gray-200 dark:bg-gray-700 text-gray-400 dark:text-gray-500 cursor-not-allowed'
          "
          @click="analyze(true)"
        >
          <svg
            v-if="loading"
            class="w-4 h-4 animate-spin"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
          <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
            />
          </svg>
          <span>{{ loading ? $t('logAnalysis.analyzing') : $t('logAnalysis.analyze') }}</span>
        </button>
      </div>
    </div>

    <!-- Scrollable content -->
    <div class="flex-1 overflow-y-auto space-y-3 pb-4">
      <!-- No path configured -->
      <div
        v-if="!appStore.xplanePath"
        class="flex flex-col items-center justify-center py-16 text-center"
      >
        <div
          class="w-14 h-14 rounded-2xl bg-gray-100 dark:bg-gray-800/60 flex items-center justify-center mb-4"
        >
          <svg
            class="w-7 h-7 text-gray-400 dark:text-gray-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        </div>
        <p class="text-sm text-gray-500 dark:text-gray-400 max-w-xs">
          {{ $t('logAnalysis.noPath') }}
        </p>
        <router-link
          to="/settings"
          class="mt-3 text-sm text-blue-600 dark:text-blue-400 hover:underline"
        >
          {{ $t('common.settings') }}
        </router-link>
      </div>

      <!-- Error state -->
      <div
        v-else-if="error"
        class="bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-xl p-4"
      >
        <div class="flex items-start space-x-3">
          <svg
            class="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <p class="text-sm text-red-700 dark:text-red-400">{{ error }}</p>
        </div>
      </div>

      <!-- Results -->
      <template v-else-if="result">
        <!-- Not X-Plane log warning -->
        <div
          v-if="!result.is_xplane_log"
          class="bg-yellow-50 dark:bg-yellow-500/10 border border-yellow-200 dark:border-yellow-500/20 rounded-xl p-4"
        >
          <div class="flex items-center space-x-2">
            <svg
              class="w-4 h-4 text-yellow-600 dark:text-yellow-400 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <span class="text-sm text-yellow-700 dark:text-yellow-400">{{
              $t('logAnalysis.notXplaneLog')
            }}</span>
          </div>
        </div>

        <!-- Crash banner -->
        <div
          v-if="result.crash_detected"
          class="bg-red-600 dark:bg-red-700 rounded-xl p-4 text-white"
        >
          <div class="flex items-start space-x-3">
            <svg
              class="w-5 h-5 flex-shrink-0 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <div class="flex-1 min-w-0">
              <p class="font-semibold text-sm">{{ $t('logAnalysis.crashDetected') }}</p>
              <!-- Crash context (E/ lines around crash marker) -->
              <details v-if="crashContextLines.length > 0" class="mt-2">
                <summary
                  class="text-xs text-white/70 cursor-pointer hover:text-white/90 select-none"
                >
                  {{ $t('logAnalysis.crashContext') }}
                </summary>
                <div
                  class="mt-1.5 text-xs font-mono text-white/80 bg-black/20 rounded p-2 leading-relaxed overflow-x-auto max-h-48 overflow-y-auto"
                >
                  <div v-for="(line, idx) in crashContextLines" :key="idx" class="break-words">
                    {{ line }}
                  </div>
                </div>
              </details>
            </div>
          </div>
        </div>

        <!-- Deep Crash Analysis -->
        <template v-if="result.crash_detected">
          <!-- Loading state -->
          <div
            v-if="crashAnalysisLoading"
            class="bg-white/80 dark:bg-gray-800/40 border border-gray-200 dark:border-white/5 rounded-xl p-6 flex items-center justify-center space-x-3"
          >
            <svg
              class="w-5 h-5 animate-spin text-red-500"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
            <span class="text-sm text-gray-600 dark:text-gray-300">{{
              $t('logAnalysis.crashAnalysis.loading')
            }}</span>
          </div>

          <!-- Crash analysis error -->
          <div
            v-else-if="crashAnalysisError"
            class="bg-yellow-50 dark:bg-yellow-500/10 border border-yellow-200 dark:border-yellow-500/20 rounded-xl p-3"
          >
            <div class="flex items-center space-x-2">
              <svg
                class="w-4 h-4 text-yellow-600 dark:text-yellow-400 flex-shrink-0"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
              <span class="text-xs text-yellow-700 dark:text-yellow-400">{{
                crashAnalysisError
              }}</span>
            </div>
          </div>

          <!-- Crash causes -->
          <template v-else-if="crashAnalysis">
            <!-- Parse warning -->
            <div
              v-if="!crashAnalysis.parse_success"
              class="bg-yellow-50 dark:bg-yellow-500/10 border border-yellow-200 dark:border-yellow-500/20 rounded-xl p-3"
            >
              <span class="text-xs text-yellow-700 dark:text-yellow-400">{{
                $t('logAnalysis.crashAnalysis.parseWarning')
              }}</span>
            </div>

            <!-- Crash causes header -->
            <div
              v-if="crashAnalysis.crash_causes.length > 0"
              class="bg-white/80 dark:bg-gray-800/40 border border-gray-200 dark:border-white/5 rounded-xl p-4"
            >
              <h3
                class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3"
              >
                {{ $t('logAnalysis.crashAnalysis.title') }}
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
                {{ $t('logAnalysis.crashAnalysis.subtitle') }}
              </p>

              <!-- Cause cards -->
              <div class="space-y-3">
                <div
                  v-for="cause in crashAnalysis.crash_causes"
                  :key="cause.cause_key"
                  class="rounded-lg border p-3"
                  :class="
                    cause.score >= 60
                      ? 'border-red-200 dark:border-red-500/20 bg-red-50/50 dark:bg-red-500/5'
                      : cause.score >= 30
                        ? 'border-yellow-200 dark:border-yellow-500/20 bg-yellow-50/50 dark:bg-yellow-500/5'
                        : 'border-gray-200 dark:border-white/10 bg-gray-50/50 dark:bg-gray-800/20'
                  "
                >
                  <!-- Score bar + name -->
                  <div class="flex items-center justify-between mb-2">
                    <div class="flex items-center space-x-2 min-w-0">
                      <span class="text-sm font-semibold text-gray-900 dark:text-white">
                        {{
                          $t(
                            `logAnalysis.crashAnalysis.causes.${cause.cause_key}.name`,
                            cause.cause_key,
                          )
                        }}
                      </span>
                      <span
                        v-if="cause.blamed_module"
                        class="text-xs text-gray-500 dark:text-gray-400 truncate"
                      >
                        {{
                          $t('logAnalysis.crashAnalysis.technical.blamedModule', {
                            module: cause.blamed_module,
                          })
                        }}
                      </span>
                    </div>
                    <span
                      class="text-sm font-bold flex-shrink-0 ml-2"
                      :class="
                        cause.score >= 60
                          ? 'text-red-600 dark:text-red-400'
                          : cause.score >= 30
                            ? 'text-yellow-600 dark:text-yellow-400'
                            : 'text-gray-600 dark:text-gray-400'
                      "
                    >
                      {{ cause.score.toFixed(1) }}%
                    </span>
                  </div>

                  <!-- Progress bar -->
                  <div class="w-full h-1.5 rounded-full bg-gray-200 dark:bg-gray-700 mb-2">
                    <div
                      class="h-full rounded-full transition-all"
                      :class="
                        cause.score >= 60
                          ? 'bg-red-500'
                          : cause.score >= 30
                            ? 'bg-yellow-400'
                            : 'bg-gray-400'
                      "
                      :style="{ width: `${Math.min(cause.score, 100)}%` }"
                    ></div>
                  </div>

                  <!-- Description -->
                  <p class="text-xs text-gray-600 dark:text-gray-400 mb-1">
                    {{
                      $t(
                        `logAnalysis.crashAnalysis.causes.${cause.cause_key}.description`,
                        '',
                      )
                    }}
                  </p>

                  <!-- Evidence (expandable) -->
                  <details v-if="cause.evidence.length > 0" class="mt-1">
                    <summary
                      class="text-xs text-gray-400 dark:text-gray-500 cursor-pointer hover:text-gray-600 dark:hover:text-gray-300 select-none"
                    >
                      {{ cause.evidence.length }} evidence point{{
                        cause.evidence.length > 1 ? 's' : ''
                      }}
                    </summary>
                    <ul
                      class="mt-1 text-xs text-gray-500 dark:text-gray-400 space-y-0.5 list-disc list-inside"
                    >
                      <li v-for="(ev, idx) in cause.evidence" :key="idx">
                        {{ $t(`logAnalysis.crashAnalysis.evidence.${ev}`, ev) }}
                      </li>
                    </ul>
                  </details>
                </div>
              </div>
            </div>

            <!-- Technical details (collapsible) -->
            <details
              v-if="crashAnalysis.exception || crashAnalysis.crash_stack.length > 0"
              class="bg-white/80 dark:bg-gray-800/40 border border-gray-200 dark:border-white/5 rounded-xl overflow-hidden"
            >
              <summary
                class="px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/30 select-none"
              >
                {{ $t('logAnalysis.crashAnalysis.technical.title') }}
              </summary>
              <div class="px-4 pb-4 space-y-4">
                <!-- Exception info -->
                <div v-if="crashAnalysis.exception" class="space-y-1">
                  <h4 class="text-xs font-medium text-gray-700 dark:text-gray-300">
                    {{ $t('logAnalysis.crashAnalysis.technical.exception') }}
                  </h4>
                  <div
                    class="text-xs font-mono text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-black/20 rounded p-2 space-y-0.5"
                  >
                    <div>
                      {{ $t('logAnalysis.crashAnalysis.technical.exceptionType') }}:
                      {{ crashAnalysis.exception.exception_type }}
                    </div>
                    <div>
                      {{ $t('logAnalysis.crashAnalysis.technical.exceptionCode') }}:
                      {{ crashAnalysis.exception.exception_code }}
                    </div>
                    <div>
                      {{ $t('logAnalysis.crashAnalysis.technical.crashAddress') }}:
                      {{ crashAnalysis.exception.crash_address }}
                    </div>
                    <div v-if="crashAnalysis.exception.crash_module">
                      {{ $t('logAnalysis.crashAnalysis.technical.crashModule') }}:
                      {{ crashAnalysis.exception.crash_module }}
                    </div>
                    <div v-if="crashAnalysis.exception.crash_module_offset">
                      {{ $t('logAnalysis.crashAnalysis.technical.crashOffset') }}:
                      {{ crashAnalysis.exception.crash_module_offset }}
                    </div>
                    <div
                      v-if="crashAnalysis.exception.exception_flags !== null"
                      class="pt-1 border-t border-gray-200 dark:border-gray-700"
                    >
                      {{ $t('logAnalysis.crashAnalysis.technical.exceptionFlags') }}:
                      0x{{ crashAnalysis.exception.exception_flags.toString(16).toUpperCase() }}
                      <span
                        v-if="
                          crashAnalysis.exception.exception_type
                            .toUpperCase()
                            .includes('0X40000015') ||
                          crashAnalysis.exception.exception_type
                            .toUpperCase()
                            .includes('FATAL_APP_EXIT')
                        "
                        class="ml-2 text-amber-600 dark:text-amber-400"
                      >
                        ({{ $t('logAnalysis.crashAnalysis.technical.controlledExit') }})
                      </span>
                    </div>
                  </div>
                </div>

                <!-- Stack trace -->
                <div v-if="crashAnalysis.crash_stack.length > 0" class="space-y-1">
                  <h4 class="text-xs font-medium text-gray-700 dark:text-gray-300">
                    {{ $t('logAnalysis.crashAnalysis.technical.stackTrace') }}
                  </h4>
                  <div
                    class="text-xs font-mono bg-gray-50 dark:bg-black/20 rounded p-2 overflow-x-auto"
                  >
                    <table class="w-full">
                      <thead>
                        <tr class="text-gray-400 dark:text-gray-500">
                          <th class="text-left pr-3 pb-1">
                            {{ $t('logAnalysis.crashAnalysis.technical.frameIndex') }}
                          </th>
                          <th class="text-left pr-3 pb-1">
                            {{ $t('logAnalysis.crashAnalysis.technical.frameModule') }}
                          </th>
                          <th class="text-left pr-3 pb-1">
                            {{ $t('logAnalysis.crashAnalysis.technical.frameOffset') }}
                          </th>
                        </tr>
                      </thead>
                      <tbody class="text-gray-500 dark:text-gray-400">
                        <tr
                          v-for="frame in crashAnalysis.crash_stack.slice(0, 10)"
                          :key="frame.frame_index"
                        >
                          <td class="pr-3 py-0.5">{{ frame.frame_index }}</td>
                          <td class="pr-3 py-0.5 truncate max-w-[200px]">
                            {{ frame.module_name || '???' }}
                          </td>
                          <td class="pr-3 py-0.5">{{ frame.offset }}</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>

                <!-- Loaded plugins -->
                <div v-if="crashAnalysis.loaded_plugins.length > 0" class="space-y-1">
                  <h4 class="text-xs font-medium text-gray-700 dark:text-gray-300">
                    {{ $t('logAnalysis.crashAnalysis.technical.loadedPlugins') }}
                    ({{ crashAnalysis.loaded_plugins.length }})
                  </h4>
                  <div
                    class="text-xs font-mono text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-black/20 rounded p-2 max-h-60 overflow-y-auto"
                  >
                    <div
                      v-for="(plugin, idx) in crashAnalysis.loaded_plugins"
                      :key="idx"
                      class="py-0.5 break-all"
                      :title="plugin"
                    >
                      {{ plugin }}
                    </div>
                  </div>
                </div>

                <!-- Report file info -->
                <div class="space-y-1">
                  <h4 class="text-xs font-medium text-gray-700 dark:text-gray-300">
                    {{ $t('logAnalysis.crashAnalysis.technical.reportFile') }}
                  </h4>
                  <div
                    class="text-xs font-mono text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-black/20 rounded p-2 space-y-0.5"
                  >
                    <div>
                      {{ $t('logAnalysis.crashAnalysis.technical.fileName') }}:
                      {{ crashAnalysis.report_info.file_name }}
                    </div>
                    <div>
                      {{ $t('logAnalysis.crashAnalysis.technical.fileSize') }}:
                      {{ formatFileSize(crashAnalysis.report_info.file_size) }}
                    </div>
                  </div>
                </div>
              </div>
            </details>
          </template>
        </template>

        <!-- No crash status -->
        <div
          v-else
          class="bg-green-50 dark:bg-green-500/10 border border-green-200 dark:border-green-500/20 rounded-xl px-4 py-2.5 flex items-center space-x-2"
        >
          <svg
            class="w-4 h-4 text-green-600 dark:text-green-400 flex-shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <span class="text-sm text-green-700 dark:text-green-400">{{
            $t('logAnalysis.noCrash')
          }}</span>
        </div>

        <!-- System Info -->
        <div
          v-if="hasSystemInfo"
          class="bg-white/80 dark:bg-gray-800/40 border border-gray-200 dark:border-white/5 rounded-xl p-4"
        >
          <h3
            class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3"
          >
            {{ $t('logAnalysis.systemInfo') }}
          </h3>
          <div class="grid grid-cols-3 gap-3">
            <div v-if="result.system_info.xplane_version">
              <div class="text-xs text-gray-500 dark:text-gray-400">
                {{ $t('logAnalysis.xplaneVersion') }}
              </div>
              <div class="text-sm font-medium text-gray-900 dark:text-white font-mono mt-0.5">
                {{ result.system_info.xplane_version }}
              </div>
            </div>
            <div v-if="result.system_info.gpu_model">
              <div class="text-xs text-gray-500 dark:text-gray-400">
                {{ $t('logAnalysis.gpuModel') }}
              </div>
              <div class="text-sm font-medium text-gray-900 dark:text-white mt-0.5 truncate">
                {{ result.system_info.gpu_model }}
              </div>
            </div>
            <div v-if="result.system_info.gpu_driver">
              <div class="text-xs text-gray-500 dark:text-gray-400">
                {{ $t('logAnalysis.gpuDriver') }}
              </div>
              <div class="text-sm font-medium text-gray-900 dark:text-white font-mono mt-0.5">
                {{ result.system_info.gpu_driver }}
              </div>
            </div>
          </div>
        </div>

        <!-- Summary row -->
        <div class="grid grid-cols-3 gap-3">
          <div
            class="bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-xl p-3 text-center"
          >
            <div class="text-2xl font-bold text-red-700 dark:text-red-400">
              {{ result.total_high }}
            </div>
            <div class="text-xs text-red-600 dark:text-red-500 mt-0.5">
              {{ $t('logAnalysis.high') }}
            </div>
          </div>
          <div
            class="bg-yellow-50 dark:bg-yellow-500/10 border border-yellow-200 dark:border-yellow-500/20 rounded-xl p-3 text-center"
          >
            <div class="text-2xl font-bold text-yellow-700 dark:text-yellow-400">
              {{ result.total_medium }}
            </div>
            <div class="text-xs text-yellow-600 dark:text-yellow-500 mt-0.5">
              {{ $t('logAnalysis.medium') }}
            </div>
          </div>
          <div
            class="bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/20 rounded-xl p-3 text-center"
          >
            <div class="text-2xl font-bold text-blue-700 dark:text-blue-400">
              {{ result.total_low }}
            </div>
            <div class="text-xs text-blue-600 dark:text-blue-500 mt-0.5">
              {{ $t('logAnalysis.low') }}
            </div>
          </div>
        </div>

        <!-- No issues state -->
        <div
          v-if="result.issues.length === 0"
          class="flex flex-col items-center justify-center py-10 text-center"
        >
          <p class="text-sm text-gray-500 dark:text-gray-400">{{ $t('logAnalysis.noIssues') }}</p>
        </div>

        <!-- Issues list -->
        <div v-else class="space-y-2">
          <div
            v-for="issue in processedIssues"
            :key="issue.category"
            class="bg-white/80 dark:bg-gray-800/40 border rounded-xl overflow-hidden"
            :class="severityBorderClass(issue.severity)"
          >
            <!-- Issue header -->
            <div class="px-4 py-3 flex items-start space-x-3">
              <!-- Severity dot -->
              <span
                class="w-2 h-2 rounded-full flex-shrink-0 mt-1.5"
                :class="severityDotClass(issue.severity)"
              ></span>

              <div class="flex-1 min-w-0">
                <!-- Name + severity badge -->
                <div class="flex items-center space-x-2 flex-wrap gap-y-1">
                  <span class="text-sm font-semibold text-gray-900 dark:text-white">
                    {{ categoryName(issue.category) }}
                  </span>
                  <span
                    class="text-xs px-1.5 py-0.5 rounded font-medium"
                    :class="severityBadgeClass(issue.severity)"
                  >
                    {{ $t(`logAnalysis.${issue.severity}`) }}
                  </span>
                  <!-- Line count badge -->
                  <span class="text-xs text-gray-400 dark:text-gray-500">
                    {{ formatLineNumbers(issue.line_numbers) }}
                  </span>
                </div>

                <!-- Description -->
                <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 leading-relaxed">
                  {{ categoryDescription(issue.category) }}
                </p>

                <!-- Suggestion -->
                <div
                  class="mt-2 flex items-start space-x-1.5 text-xs text-blue-700 dark:text-blue-300"
                >
                  <svg
                    class="w-3.5 h-3.5 flex-shrink-0 mt-0.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  <span class="leading-relaxed">{{ categorySuggestion(issue.category) }}</span>
                </div>

                <!-- Sample block (collapsed by default) -->
                <details v-if="issue.sample_line" class="mt-2">
                  <summary
                    class="text-xs text-gray-400 dark:text-gray-500 cursor-pointer hover:text-gray-600 dark:hover:text-gray-300 select-none"
                  >
                    {{ $t('logAnalysis.lineNumbers', { nums: issue.line_numbers[0] }) }}
                  </summary>
                  <div
                    class="mt-1 text-xs font-mono text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-black/20 rounded p-2 leading-relaxed overflow-x-auto"
                  >
                    <div v-for="(line, idx) in issue.sampleLines" :key="idx" class="break-words">
                      {{ line }}
                    </div>
                  </div>
                </details>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- Initial state -->
      <div
        v-else-if="appStore.xplanePath && !loading"
        class="flex flex-col items-center justify-center py-16 text-center"
      >
        <div
          class="w-14 h-14 rounded-2xl bg-blue-50 dark:bg-blue-500/10 flex items-center justify-center mb-4"
        >
          <svg
            class="w-7 h-7 text-blue-500 dark:text-blue-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        </div>
        <p class="text-sm text-gray-500 dark:text-gray-400">
          {{ $t('logAnalysis.subtitle') }}
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import AnimatedText from '@/components/AnimatedText.vue'

const { t } = useI18n()
const appStore = useAppStore()

interface SystemInfo {
  xplane_version: string | null
  gpu_model: string | null
  gpu_driver: string | null
}

interface LogIssue {
  category: string
  severity: string
  line_numbers: number[]
  sample_line: string
}

interface RenderLogIssue extends LogIssue {
  sampleLines: string[]
}

interface XPlaneLogAnalysis {
  log_path: string
  is_xplane_log: boolean
  crash_detected: boolean
  crash_info: string | null
  issues: LogIssue[]
  system_info: SystemInfo
  total_high: number
  total_medium: number
  total_low: number
}

interface CrashReportInfo {
  dmp_path: string
  file_name: string
  timestamp: number
  file_size: number
}

interface CrashExceptionInfo {
  exception_type: string
  exception_code: string
  crash_address: string
  crash_module: string | null
  crash_module_offset: string | null
  exception_flags: number | null
}

interface CrashStackFrame {
  frame_index: number
  module_name: string | null
  offset: string
  trust: string
}

interface CrashCause {
  cause_key: string
  score: number
  evidence: string[]
  blamed_module: string | null
}

interface DeepCrashAnalysis {
  report_info: CrashReportInfo
  exception: CrashExceptionInfo | null
  crash_stack: CrashStackFrame[]
  loaded_modules: unknown[]
  loaded_plugins: string[]
  crash_causes: CrashCause[]
  parse_success: boolean
  parse_warnings: string[]
}

const loading = ref(false)
const result = ref<XPlaneLogAnalysis | null>(null)
const error = ref<string | null>(null)
const crashAnalysis = ref<DeepCrashAnalysis | null>(null)
const crashAnalysisLoading = ref(false)
const crashAnalysisError = ref<string | null>(null)

const analysisCache = new Map<string, XPlaneLogAnalysis>()
const crashCache = new Map<string, DeepCrashAnalysis>()

const hasSystemInfo = computed(() => {
  const si = result.value?.system_info
  return si && (si.xplane_version || si.gpu_model || si.gpu_driver)
})

const processedIssues = computed<RenderLogIssue[]>(() => {
  return (result.value?.issues || []).map((issue) => ({
    ...issue,
    sampleLines: issue.sample_line ? issue.sample_line.split('\n') : [],
  }))
})

const crashContextLines = computed<string[]>(() => {
  const info = result.value?.crash_info
  if (!info) return []
  return info.split('\n')
})

async function analyze(force = false) {
  if (!appStore.xplanePath) return

  const cacheKey = appStore.xplanePath
  if (!force) {
    const cached = analysisCache.get(cacheKey)
    if (cached) {
      result.value = cached
      error.value = null
      // Restore crash cache too
      const cachedCrash = crashCache.get(cacheKey)
      if (cachedCrash) {
        crashAnalysis.value = cachedCrash
      }
      return
    }
  }

  loading.value = true
  error.value = null
  crashAnalysis.value = null
  crashAnalysisError.value = null

  try {
    const latest = await invoke<XPlaneLogAnalysis>('analyze_xplane_log', {
      xplanePath: appStore.xplanePath,
    })
    result.value = latest
    analysisCache.set(cacheKey, latest)

    // Auto-trigger crash analysis if crash detected
    if (latest.crash_detected) {
      crashAnalysisLoading.value = true
      try {
        const crashResult = await invoke<DeepCrashAnalysis | null>('analyze_crash_report', {
          xplanePath: appStore.xplanePath,
          logIssues: latest.issues,
          skipDateCheck: appStore.crashAnalysisIgnoreDateCheck,
        })
        if (crashResult) {
          crashAnalysis.value = crashResult
          crashCache.set(cacheKey, crashResult)
        }
      } catch (ce) {
        crashAnalysisError.value = String(ce)
      } finally {
        crashAnalysisLoading.value = false
      }
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  if (appStore.xplanePath) {
    requestAnimationFrame(() => {
      setTimeout(() => {
        void analyze(false)
      }, 0)
    })
  }
})

async function openLog() {
  if (!result.value?.log_path) return
  try {
    await invoke('open_url', { url: result.value.log_path })
  } catch {
    // ignore
  }
}

function categoryName(key: string): string {
  return t(`logAnalysis.categories.${key}.name`, key)
}

function categoryDescription(key: string): string {
  return t(`logAnalysis.categories.${key}.description`, '')
}

function categorySuggestion(key: string): string {
  return t(`logAnalysis.categories.${key}.suggestion`, '')
}

function formatLineNumbers(nums: number[]): string {
  if (nums.length === 0) return ''
  const shown = nums.slice(0, 3).join(', ')
  return nums.length > 3
    ? t('logAnalysis.lineNumbers', { nums: `${shown} (+${nums.length - 3})` })
    : t('logAnalysis.lineNumbers', { nums: shown })
}

function severityDotClass(severity: string): string {
  return severity === 'high'
    ? 'bg-red-500'
    : severity === 'medium'
      ? 'bg-yellow-400'
      : 'bg-blue-400'
}

function severityBorderClass(severity: string): string {
  return severity === 'high'
    ? 'border-red-200 dark:border-red-500/20'
    : severity === 'medium'
      ? 'border-yellow-200 dark:border-yellow-500/20'
      : 'border-blue-200 dark:border-blue-500/20'
}

function severityBadgeClass(severity: string): string {
  return severity === 'high'
    ? 'bg-red-100 dark:bg-red-500/20 text-red-700 dark:text-red-400'
    : severity === 'medium'
      ? 'bg-yellow-100 dark:bg-yellow-500/20 text-yellow-700 dark:text-yellow-400'
      : 'bg-blue-100 dark:bg-blue-500/20 text-blue-700 dark:text-blue-400'
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>
