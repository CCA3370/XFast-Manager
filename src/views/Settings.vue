<template>
  <div class="settings-view h-full flex flex-col p-5 overflow-hidden">
    <!-- Scrollable Content Area -->
    <div class="flex-1 overflow-y-auto space-y-4 pr-1 pb-4">
      <!-- 1. X-Plane Path (Compact) -->
      <section
        class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
      >
        <div class="p-4 space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center space-x-3">
              <div
                class="w-8 h-8 bg-blue-100 dark:bg-blue-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-blue-600 dark:text-blue-400"
              >
                <!-- Folder icon -->
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                  ></path>
                </svg>
              </div>
              <div class="flex-1">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                  <AnimatedText>{{ $t('settings.xplanePath') }}</AnimatedText>
                </h3>
              </div>
            </div>

            <!-- Status indicators on the right side of title -->
            <div class="flex items-center space-x-2">
              <!-- Error indicator -->
              <transition name="fade">
                <div
                  v-if="pathError"
                  class="flex items-center text-[10px] font-medium space-x-1 text-red-500 dark:text-red-400"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                    ></path>
                  </svg>
                  <span>{{ pathError }}</span>
                </div>
              </transition>

              <!-- Save status -->
              <transition name="fade">
                <div
                  v-if="saveStatus"
                  class="flex items-center text-[10px] font-medium space-x-1"
                  :class="saveStatus === 'saved' ? 'text-emerald-500' : 'text-gray-400'"
                >
                  <svg
                    v-if="saveStatus === 'saved'"
                    class="w-3 h-3"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    ></path>
                  </svg>
                  <svg
                    v-else
                    class="w-3 h-3 animate-spin"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                    ></path>
                  </svg>
                  <span>{{
                    saveStatus === 'saved' ? $t('settings.saved') : $t('settings.saving')
                  }}</span>
                </div>
              </transition>
            </div>
          </div>

          <div class="relative">
            <div
              class="flex items-center bg-gray-50 dark:bg-gray-900/50 border rounded-lg overflow-hidden focus-within:border-blue-500 dark:focus-within:border-blue-500 transition-colors duration-200"
              :class="
                pathError
                  ? 'border-red-500 dark:border-red-500'
                  : 'border-gray-200 dark:border-gray-700/50'
              "
            >
              <input
                v-model="xplanePathInput"
                type="text"
                placeholder="C:\X-Plane 12"
                class="flex-1 px-4 py-2.5 bg-transparent border-none text-sm text-gray-900 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-600 focus:outline-none focus:ring-0"
                @blur="handlePathBlur"
              />
              <button
                type="button"
                class="px-4 py-1.5 m-1 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-xs font-medium rounded-md transition-colors duration-200 flex items-center space-x-1.5 flex-shrink-0 border border-gray-300 dark:border-gray-600"
                @click.stop.prevent="selectFolder"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"
                  ></path>
                </svg>
                <span><AnimatedText>{{ $t('common.browse') }}</AnimatedText></span>
              </button>
            </div>
          </div>

          <!-- Launch Arguments -->
          <div class="space-y-1.5">
            <label
              class="text-xs font-medium text-gray-500 dark:text-gray-400 flex items-center gap-1.5"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                ></path>
              </svg>
              <AnimatedText>{{ $t('settings.xplaneLaunchArgs') }}</AnimatedText>
            </label>
            <input
              v-model="launchArgsInput"
              type="text"
              :placeholder="$t('settings.xplaneLaunchArgsPlaceholder')"
              class="w-full px-4 py-2 bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-700/50 rounded-lg text-sm text-gray-900 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-600 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:focus:ring-blue-500 focus:border-blue-500 dark:focus:border-blue-500 transition-colors duration-200"
              @blur="handleLaunchArgsBlur"
            />
            <p class="text-[11px] text-gray-400 dark:text-gray-500">
              <AnimatedText>{{ $t('settings.xplaneLaunchArgsDesc') }}</AnimatedText>
            </p>
          </div>
        </div>
      </section>

      <!-- 2. Grid for Windows Integration & Preferences -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- Windows Integration (Left Column, Windows only) -->
        <transition name="slide-up">
          <section
            v-if="store.isWindows"
            class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
          >
            <div
              class="p-4 cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
              @click="windowsIntegrationExpanded = !windowsIntegrationExpanded"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3 flex-1">
                  <div
                    class="w-8 h-8 bg-gray-100 dark:bg-gray-700 rounded-lg flex items-center justify-center flex-shrink-0 text-gray-600 dark:text-gray-300"
                  >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
                      ></path>
                    </svg>
                  </div>
                  <div class="flex-1">
                    <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                      <AnimatedText>{{ $t('settings.windowsIntegration') }}</AnimatedText>
                    </h3>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                      <AnimatedText>{{ $t('settings.windowsIntegrationDesc') }}</AnimatedText>
                    </p>
                  </div>
                </div>

                <div class="flex items-center space-x-3">
                  <button
                    :disabled="isProcessing"
                    class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-300 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1 focus:ring-offset-white dark:focus:ring-offset-gray-900"
                    :class="
                      store.isContextMenuRegistered ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-700'
                    "
                    @click.stop="toggleContextMenu"
                  >
                    <span
                      class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-300 shadow-sm"
                      :class="store.isContextMenuRegistered ? 'translate-x-4.5' : 'translate-x-0.5'"
                    />
                  </button>

                  <!-- Expand/Collapse indicator -->
                  <svg
                    class="w-5 h-5 text-gray-400 transition-transform duration-200"
                    :class="{ 'rotate-180': windowsIntegrationExpanded }"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 9l-7 7-7-7"
                    ></path>
                  </svg>
                </div>
              </div>
            </div>

            <!-- Expanded content -->
            <transition name="expand">
              <div v-if="windowsIntegrationExpanded" class="px-4 pb-4 space-y-3">
                <div
                  class="bg-blue-50/50 dark:bg-blue-500/5 border border-blue-200 dark:border-blue-500/20 rounded-lg p-3 space-y-2"
                >
                  <h4 class="text-xs font-semibold text-blue-900 dark:text-blue-300">
                    <AnimatedText>{{ $t('settings.windowsIntegrationExplain') }}</AnimatedText>
                  </h4>
                  <ul class="text-xs text-blue-800 dark:text-blue-200 space-y-1.5">
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-blue-500 dark:text-blue-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.windowsIntegrationBenefit1')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-blue-500 dark:text-blue-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.windowsIntegrationBenefit2')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-blue-500 dark:text-blue-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.windowsIntegrationBenefit3')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-blue-500 dark:text-blue-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.windowsIntegrationBenefit4')
                      }}</AnimatedText></span>
                    </li>
                  </ul>
                </div>
              </div>
            </transition>
          </section>
        </transition>

        <!-- Atomic Installation Mode (Windows only) -->
        <transition name="slide-up">
          <section
            v-if="store.isWindows"
            class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
          >
            <div
              class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
              @click="atomicExpanded = !atomicExpanded"
            >
              <div class="flex items-center space-x-3 flex-1">
                <div
                  class="w-8 h-8 bg-indigo-100 dark:bg-indigo-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-indigo-600 dark:text-indigo-400"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
                    ></path>
                  </svg>
                </div>
                <div class="flex-1">
                  <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                    <AnimatedText>{{ $t('settings.atomicInstallTitle') }}</AnimatedText>
                  </h3>
                  <p class="text-xs text-gray-500 dark:text-gray-400">
                    <AnimatedText>{{ $t('settings.atomicInstallDesc') }}</AnimatedText>
                  </p>
                </div>
              </div>

              <!-- Toggle Switch -->
              <div class="flex items-center space-x-3">
                <button
                  class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                  :class="
                    store.atomicInstallEnabled ? 'bg-indigo-500' : 'bg-gray-300 dark:bg-gray-600'
                  "
                  @click.stop="store.toggleAtomicInstall()"
                >
                  <span
                    class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform"
                    :class="store.atomicInstallEnabled ? 'translate-x-4.5' : 'translate-x-0.5'"
                  ></span>
                </button>

                <!-- Expand/Collapse indicator -->
                <svg
                  class="w-5 h-5 text-gray-400 transition-transform duration-200"
                  :class="{ 'rotate-180': atomicExpanded }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 9l-7 7-7-7"
                  ></path>
                </svg>
              </div>
            </div>

            <!-- Expanded content -->
            <transition name="expand">
              <div v-if="atomicExpanded" class="px-4 pb-4 space-y-3">
                <div
                  class="bg-indigo-50/50 dark:bg-indigo-500/5 border border-indigo-200 dark:border-indigo-500/20 rounded-lg p-3 space-y-2"
                >
                  <h4 class="text-xs font-semibold text-indigo-900 dark:text-indigo-300">
                    <AnimatedText>{{ $t('settings.atomicInstallExplain') }}</AnimatedText>
                  </h4>
                  <ul class="text-xs text-indigo-800 dark:text-indigo-200 space-y-1.5">
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-indigo-500 dark:text-indigo-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.atomicInstallBenefit1')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-indigo-500 dark:text-indigo-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.atomicInstallBenefit2')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-indigo-500 dark:text-indigo-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.atomicInstallBenefit3')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-indigo-500 dark:text-indigo-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.atomicInstallBenefit4')
                      }}</AnimatedText></span>
                    </li>
                  </ul>
                </div>

                <p
                  class="text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-500/10 p-2 rounded-lg border border-amber-200 dark:border-amber-500/20"
                >
                  <AnimatedText>{{ $t('settings.atomicInstallNote') }}</AnimatedText>
                </p>
              </div>
            </transition>
          </section>
        </transition>

        <!-- Delete Source After Install (Windows only) -->
        <transition name="slide-up">
          <section
            v-if="store.isWindows"
            class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
          >
            <div
              class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
              @click="deleteSourceExpanded = !deleteSourceExpanded"
            >
              <div class="flex items-center space-x-3 flex-1">
                <div
                  class="w-8 h-8 bg-red-100 dark:bg-red-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-red-600 dark:text-red-400"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                    ></path>
                  </svg>
                </div>
                <div class="flex-1">
                  <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                    <AnimatedText>{{ $t('settings.deleteSourceTitle') }}</AnimatedText>
                  </h3>
                  <p class="text-xs text-gray-500 dark:text-gray-400">
                    <AnimatedText>{{ $t('settings.deleteSourceDesc') }}</AnimatedText>
                  </p>
                </div>
              </div>

              <!-- Toggle Switch -->
              <div class="flex items-center space-x-3">
                <button
                  class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                  :class="
                    store.deleteSourceAfterInstall ? 'bg-red-500' : 'bg-gray-300 dark:bg-gray-600'
                  "
                  @click.stop="store.toggleDeleteSourceAfterInstall()"
                >
                  <span
                    class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform"
                    :class="store.deleteSourceAfterInstall ? 'translate-x-4.5' : 'translate-x-0.5'"
                  ></span>
                </button>

                <!-- Expand/Collapse indicator -->
                <svg
                  class="w-5 h-5 text-gray-400 transition-transform duration-200"
                  :class="{ 'rotate-180': deleteSourceExpanded }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 9l-7 7-7-7"
                  ></path>
                </svg>
              </div>
            </div>

            <!-- Expanded content -->
            <transition name="expand">
              <div v-if="deleteSourceExpanded" class="px-4 pb-4 space-y-3">
                <div
                  class="bg-red-50/50 dark:bg-red-500/5 border border-red-200 dark:border-red-500/20 rounded-lg p-3 space-y-2"
                >
                  <h4 class="text-xs font-semibold text-red-900 dark:text-red-300">
                    <AnimatedText>{{ $t('settings.deleteSourceExplain') }}</AnimatedText>
                  </h4>
                  <ul class="text-xs text-red-800 dark:text-red-200 space-y-1.5">
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.deleteSourceBenefit1')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.deleteSourceBenefit2')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.deleteSourceBenefit3')
                      }}</AnimatedText></span>
                    </li>
                    <li class="flex items-start space-x-2">
                      <svg
                        class="w-4 h-4 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M5 13l4 4L19 7"
                        ></path>
                      </svg>
                      <span><AnimatedText>{{
                        $t('settings.deleteSourceBenefit4')
                      }}</AnimatedText></span>
                    </li>
                  </ul>
                </div>
              </div>
            </transition>
          </section>
        </transition>

        <!-- Placeholder for non-Windows (to maintain grid layout) -->
        <div v-if="!store.isWindows"></div>

        <!-- Scenery Auto-Sorting -->
        <section
          class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300 md:col-span-2"
        >
          <div
            class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
            @click="sceneryAutoSortExpanded = !sceneryAutoSortExpanded"
          >
            <div class="flex items-center space-x-3 flex-1">
              <div
                class="w-8 h-8 bg-cyan-100 dark:bg-cyan-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-cyan-600 dark:text-cyan-400"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12"
                  ></path>
                </svg>
              </div>
              <div class="flex-1">
                <h3
                  class="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2"
                >
                  <AnimatedText>{{ $t('settings.sceneryAutoSort') }}</AnimatedText>
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">
                  <AnimatedText>{{ $t('settings.sceneryAutoSortDesc') }}</AnimatedText>
                </p>
              </div>
            </div>

            <!-- Toggle Switch -->
            <div class="flex items-center space-x-3">
              <button
                class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                :class="store.autoSortScenery ? 'bg-cyan-500' : 'bg-gray-300 dark:bg-gray-600'"
                @click.stop="handleToggleAutoSortScenery"
              >
                <span
                  class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform"
                  :class="store.autoSortScenery ? 'translate-x-4.5' : 'translate-x-0.5'"
                ></span>
              </button>

              <!-- Expand/Collapse indicator -->
              <svg
                class="w-5 h-5 text-gray-400 transition-transform duration-200"
                :class="{ 'rotate-180': sceneryAutoSortExpanded }"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M19 9l-7 7-7-7"
                ></path>
              </svg>
            </div>
          </div>

          <!-- Expanded content -->
          <transition name="expand">
            <div v-if="sceneryAutoSortExpanded" class="px-4 pb-4 space-y-3">
              <!-- Explanation -->
              <div
                class="bg-cyan-50/50 dark:bg-cyan-500/5 border border-cyan-200 dark:border-cyan-500/20 rounded-lg p-3 space-y-2"
              >
                <h4 class="text-xs font-semibold text-cyan-900 dark:text-cyan-300">
                  <AnimatedText>{{ $t('settings.sceneryAutoSortExplain') }}</AnimatedText>
                </h4>
                <ul class="text-xs text-cyan-800 dark:text-cyan-200 space-y-1.5">
                  <li class="flex items-start space-x-2">
                    <svg
                      class="w-4 h-4 text-cyan-500 dark:text-cyan-400 flex-shrink-0 mt-0.5"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                      ></path>
                    </svg>
                    <span><AnimatedText>{{
                      $t('settings.sceneryAutoSortBenefit1')
                    }}</AnimatedText></span>
                  </li>
                  <li class="flex items-start space-x-2">
                    <svg
                      class="w-4 h-4 text-cyan-500 dark:text-cyan-400 flex-shrink-0 mt-0.5"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                      ></path>
                    </svg>
                    <span><AnimatedText>{{
                      $t('settings.sceneryAutoSortBenefit2')
                    }}</AnimatedText></span>
                  </li>
                  <li class="flex items-start space-x-2">
                    <svg
                      class="w-4 h-4 text-cyan-500 dark:text-cyan-400 flex-shrink-0 mt-0.5"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                      ></path>
                    </svg>
                    <span><AnimatedText>{{
                      $t('settings.sceneryAutoSortBenefit3')
                    }}</AnimatedText></span>
                  </li>
                </ul>
              </div>

              <!-- Rebuild Index Button -->
              <button
                :disabled="isRebuildingIndex || !store.xplanePath"
                class="w-full px-4 py-2 bg-gray-500 hover:bg-gray-600 disabled:bg-gray-400 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors flex items-center justify-center space-x-2"
                :title="
                  indexExists
                    ? $t('settings.rebuildIndexTooltip')
                    : $t('settings.createIndexTooltip')
                "
                @click="handleRebuildIndex"
              >
                <svg
                  v-if="!isRebuildingIndex"
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  ></path>
                </svg>
                <svg
                  v-else
                  class="w-4 h-4 animate-spin [animation-direction:reverse]"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  ></path>
                </svg>
                <span>
                  <AnimatedText>
                    {{
                      isRebuildingIndex
                        ? $t('settings.rebuilding')
                        : indexExists
                          ? $t('settings.rebuildIndex')
                          : $t('settings.createIndex')
                    }}
                  </AnimatedText>
                </span>
              </button>

              <!-- Note about X-Plane path -->
              <p
                v-if="!store.xplanePath"
                class="text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-500/10 p-2 rounded-lg border border-amber-200 dark:border-amber-500/20"
              >
                <AnimatedText>{{ $t('settings.sceneryAutoSortNeedPath') }}</AnimatedText>
              </p>
            </div>
          </transition>
        </section>

        <!-- Installation Preferences (Right Column or Full Width on non-Windows) -->
        <section
          class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
          :class="{ 'md:col-span-2': !store.isWindows }"
        >
          <div
            class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
            @click="preferencesExpanded = !preferencesExpanded"
          >
            <div class="flex items-center space-x-3">
              <div
                class="w-8 h-8 bg-green-100 dark:bg-green-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-green-600 dark:text-green-400"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"
                  ></path>
                </svg>
              </div>
              <div>
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                  <AnimatedText>{{ $t('settings.installPreferences') }}</AnimatedText>
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">
                  <AnimatedText>{{ $t('settings.installPreferencesDesc') }}</AnimatedText>
                </p>
              </div>
            </div>

            <!-- Expand/Collapse indicator -->
            <svg
              class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
              :class="{ 'rotate-180': preferencesExpanded }"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              ></path>
            </svg>
          </div>

          <!-- Collapsible content -->
          <transition name="collapse">
            <div v-if="preferencesExpanded" class="px-4 pb-4 space-y-3">
              <!-- Master Toggle -->
              <div
                class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5"
              >
                <span class="text-xs font-medium text-gray-700 dark:text-gray-300"><AnimatedText>{{ $t('settings.toggleAll') }}</AnimatedText></span>
                <button
                  class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                  :class="allPreferencesEnabled ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                  @click="toggleAllPreferences"
                >
                  <span
                    class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                    :class="allPreferencesEnabled ? 'translate-x-4.5' : 'translate-x-0.5'"
                  />
                </button>
              </div>

              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <div
                  v-for="type in addonTypes"
                  :key="type"
                  class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5"
                >
                  <span
                    class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2"
                    :title="getTypeName(type)"
                  >
                    <AnimatedText>{{ getTypeName(type) }}</AnimatedText>
                  </span>
                  <button
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                    :class="
                      store.installPreferences[type]
                        ? 'bg-green-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                    "
                    @click="store.togglePreference(type)"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="
                        store.installPreferences[type] ? 'translate-x-3.5' : 'translate-x-0.5'
                      "
                    />
                  </button>
                </div>
              </div>
            </div>
          </transition>
        </section>

        <!-- 2.5. Verification Preferences -->
        <section
          class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
          :class="{ 'md:col-span-2': !store.isWindows }"
        >
          <div
            class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
            @click="verificationExpanded = !verificationExpanded"
          >
            <div class="flex items-center space-x-3">
              <div
                class="w-8 h-8 bg-purple-100 dark:bg-purple-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-purple-600 dark:text-purple-400"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
                  ></path>
                </svg>
              </div>
              <div>
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                  <AnimatedText>{{ $t('settings.verificationPreferences') }}</AnimatedText>
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">
                  <AnimatedText>{{ $t('settings.verificationPreferencesDesc') }}</AnimatedText>
                </p>
              </div>
            </div>

            <!-- Expand/Collapse indicator -->
            <svg
              class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
              :class="{ 'rotate-180': verificationExpanded }"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              ></path>
            </svg>
          </div>

          <!-- Collapsible content -->
          <transition name="collapse">
            <div v-if="verificationExpanded" class="px-4 pb-4 space-y-3">
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <!-- ZIP -->
                <div
                  class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5"
                >
                  <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">
                    <AnimatedText>{{ $t('settings.verifyZip') }}</AnimatedText>
                  </span>
                  <button
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                    :class="
                      store.verificationPreferences['zip']
                        ? 'bg-purple-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                    "
                    @click="store.toggleVerificationPreference('zip')"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="
                        store.verificationPreferences['zip'] ? 'translate-x-3.5' : 'translate-x-0.5'
                      "
                    />
                  </button>
                </div>

                <!-- 7z -->
                <div
                  class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5"
                >
                  <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">
                    <AnimatedText>{{ $t('settings.verify7z') }}</AnimatedText>
                  </span>
                  <button
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                    :class="
                      store.verificationPreferences['7z']
                        ? 'bg-purple-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                    "
                    @click="store.toggleVerificationPreference('7z')"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="
                        store.verificationPreferences['7z'] ? 'translate-x-3.5' : 'translate-x-0.5'
                      "
                    />
                  </button>
                </div>

                <!-- RAR (with note) -->
                <div
                  class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5 opacity-60"
                >
                  <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">
                    <AnimatedText>{{ $t('settings.verifyRar') }}</AnimatedText>
                  </span>
                  <button
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                    :class="
                      store.verificationPreferences['rar']
                        ? 'bg-purple-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                    "
                    disabled
                    @click="store.toggleVerificationPreference('rar')"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="
                        store.verificationPreferences['rar'] ? 'translate-x-3.5' : 'translate-x-0.5'
                      "
                    />
                  </button>
                </div>

                <!-- Directory -->
                <div
                  class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5"
                >
                  <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">
                    <AnimatedText>{{ $t('settings.verifyDirectory') }}</AnimatedText>
                  </span>
                  <button
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                    :class="
                      store.verificationPreferences['directory']
                        ? 'bg-purple-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                    "
                    @click="store.toggleVerificationPreference('directory')"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="
                        store.verificationPreferences['directory']
                          ? 'translate-x-3.5'
                          : 'translate-x-0.5'
                      "
                    />
                  </button>
                </div>
              </div>

              <!-- RAR note -->
              <p
                class="text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-500/10 p-2 rounded-lg border border-amber-200 dark:border-amber-500/20"
              >
                <AnimatedText>{{ $t('settings.rarVerificationNote') }}</AnimatedText>
              </p>
            </div>
          </transition>
        </section>
      </div>

      <!-- 3. Aircraft Backup Configuration -->
      <section
        class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
      >
        <div
          class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
          @click="backupExpanded = !backupExpanded"
        >
          <div class="flex items-center space-x-3">
            <div
              class="w-8 h-8 bg-blue-100 dark:bg-blue-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-blue-600 dark:text-blue-400"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
                ></path>
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                <AnimatedText>{{ $t('settings.aircraftBackup') }}</AnimatedText>
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400">
                <AnimatedText>{{ $t('settings.aircraftBackupDesc') }}</AnimatedText>
              </p>
            </div>
          </div>

          <!-- Status indicators and expand/collapse -->
          <div class="flex items-center space-x-2">
            <!-- Save status -->
            <transition name="fade">
              <div
                v-if="patternSaveStatus"
                class="flex items-center text-[10px] font-medium space-x-1"
                :class="patternSaveStatus === 'saved' ? 'text-emerald-500' : 'text-gray-400'"
              >
                <svg
                  v-if="patternSaveStatus === 'saved'"
                  class="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 13l4 4L19 7"
                  ></path>
                </svg>
                <svg
                  v-else
                  class="w-3 h-3 animate-spin"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  ></path>
                </svg>
                <span>{{
                  patternSaveStatus === 'saved' ? $t('settings.saved') : $t('settings.saving')
                }}</span>
              </div>
            </transition>

            <!-- Expand/Collapse indicator -->
            <svg
              class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
              :class="{ 'rotate-180': backupExpanded }"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              ></path>
            </svg>
          </div>
        </div>

        <!-- Collapsible content -->
        <transition name="collapse">
          <div v-if="backupExpanded" class="px-4 pb-4 space-y-3">
            <!-- Config file patterns list -->
            <div class="space-y-2">
              <label class="text-xs font-medium text-gray-700 dark:text-gray-300">
                <AnimatedText>{{ $t('settings.configFilePatterns') }}</AnimatedText>
              </label>

              <div class="space-y-1.5">
                <div v-for="(_pattern, index) in configPatterns" :key="index">
                  <div
                    class="flex items-center gap-2 p-2 bg-gray-50 dark:bg-gray-900/30 rounded-lg border transition-colors"
                    :class="
                      patternErrors[index]
                        ? 'border-red-300 dark:border-red-500/50'
                        : 'border-gray-100 dark:border-white/5'
                    "
                  >
                    <input
                      v-model="configPatterns[index]"
                      type="text"
                      class="flex-1 px-2 py-1 text-xs bg-white dark:bg-gray-800 border rounded text-gray-900 dark:text-gray-200 focus:outline-none focus:ring-1 focus:ring-blue-500 transition-colors"
                      :class="
                        patternErrors[index]
                          ? 'border-red-300 dark:border-red-500'
                          : 'border-gray-200 dark:border-gray-700'
                      "
                      placeholder="*_prefs.txt"
                      @blur="handlePatternBlur"
                    />
                    <button
                      class="p-1 text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 rounded transition-colors"
                      :title="$t('common.delete')"
                      @click="removePattern(index)"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M6 18L18 6M6 6l12 12"
                        ></path>
                      </svg>
                    </button>
                  </div>
                  <p
                    v-if="patternErrors[index]"
                    class="mt-0.5 ml-2 text-xs text-red-500 dark:text-red-400"
                  >
                    {{ patternErrors[index] }}
                  </p>
                </div>
              </div>

              <button
                class="w-full px-3 py-1.5 text-xs bg-blue-50 dark:bg-blue-500/10 hover:bg-blue-100 dark:hover:bg-blue-500/20 text-blue-600 dark:text-blue-400 rounded-lg transition-colors border border-blue-200 dark:border-blue-500/20 flex items-center justify-center gap-1"
                @click="addPattern"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 4v16m8-8H4"
                  ></path>
                </svg>
                <AnimatedText>{{ $t('settings.addPattern') }}</AnimatedText>
              </button>

              <!-- Help text -->
              <p class="text-xs text-gray-500 dark:text-gray-400 leading-relaxed">
                <AnimatedText>{{ $t('settings.patternHelpText') }}</AnimatedText>
              </p>
            </div>
          </div>
        </transition>
      </section>

      <!--  4. Auto Update Check -->
      <section
        class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
      >
        <div
          class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
          @click="updateCheckExpanded = !updateCheckExpanded"
        >
          <div class="flex items-center space-x-3 flex-1">
            <div
              class="w-8 h-8 bg-green-100 dark:bg-green-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-green-600 dark:text-green-400"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                ></path>
              </svg>
            </div>
            <div class="flex-1">
              <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                <AnimatedText>{{ $t('update.autoUpdateCheck') }}</AnimatedText>
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400">
                <AnimatedText>{{ $t('update.autoUpdateCheckDesc') }}</AnimatedText>
              </p>
            </div>
          </div>

          <!-- Toggle Switch -->
          <div class="flex items-center space-x-3">
            <button
              class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
              :class="
                updateStore.autoCheckEnabled ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'
              "
              @click.stop="updateStore.toggleAutoCheck()"
            >
              <span
                class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform"
                :class="updateStore.autoCheckEnabled ? 'translate-x-4.5' : 'translate-x-0.5'"
              ></span>
            </button>

            <!-- Expand/Collapse indicator -->
            <svg
              class="w-5 h-5 text-gray-400 transition-transform duration-200"
              :class="{ 'rotate-180': updateCheckExpanded }"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              ></path>
            </svg>
          </div>
        </div>

        <!-- Expanded content -->
        <transition name="expand">
          <div v-if="updateCheckExpanded" class="px-4 pb-4 space-y-3">
            <!-- 说明 -->
            <div
              class="bg-green-50/50 dark:bg-green-500/5 border border-green-200 dark:border-green-500/20 rounded-lg p-3 space-y-2"
            >
              <h4 class="text-xs font-semibold text-green-900 dark:text-green-300">
                <AnimatedText>{{ $t('update.autoUpdateCheckExplain') }}</AnimatedText>
              </h4>
              <ul class="text-xs text-green-800 dark:text-green-200 space-y-1.5">
                <li class="flex items-start space-x-2">
                  <svg
                    class="w-4 h-4 text-green-500 dark:text-green-400 flex-shrink-0 mt-0.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    ></path>
                  </svg>
                  <span><AnimatedText>{{ $t('update.autoUpdateCheckBenefit1') }}</AnimatedText></span>
                </li>
                <li class="flex items-start space-x-2">
                  <svg
                    class="w-4 h-4 text-green-500 dark:text-green-400 flex-shrink-0 mt-0.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    ></path>
                  </svg>
                  <span><AnimatedText>{{ $t('update.autoUpdateCheckBenefit2') }}</AnimatedText></span>
                </li>
                <li class="flex items-start space-x-2">
                  <svg
                    class="w-4 h-4 text-green-500 dark:text-green-400 flex-shrink-0 mt-0.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    ></path>
                  </svg>
                  <span><AnimatedText>{{ $t('update.autoUpdateCheckBenefit3') }}</AnimatedText></span>
                </li>
              </ul>
            </div>

            <!-- Pre-Release 选项 -->
            <div
              class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5"
            >
              <div class="flex-1">
                <label class="text-xs font-medium text-gray-700 dark:text-gray-300">
                  <AnimatedText>{{ $t('update.includePreRelease') }}</AnimatedText>
                </label>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  <AnimatedText>{{ $t('update.includePreReleaseDesc') }}</AnimatedText>
                </p>
              </div>
              <button
                class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0 ml-3"
                :class="
                  updateStore.includePreRelease ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'
                "
                @click="updateStore.toggleIncludePreRelease()"
              >
                <span
                  class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                  :class="updateStore.includePreRelease ? 'translate-x-3.5' : 'translate-x-0.5'"
                />
              </button>
            </div>

            <!-- 手动检查按钮 -->
            <button
              :disabled="updateStore.checkInProgress"
              class="w-full px-4 py-2 bg-green-500 hover:bg-green-600 disabled:bg-gray-400 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors flex items-center justify-center space-x-2"
              @click="handleCheckUpdate"
            >
              <svg
                v-if="!updateStore.checkInProgress"
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                ></path>
              </svg>
              <svg
                v-else
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
                ></path>
              </svg>
              <span>
                <AnimatedText>{{
                  updateStore.checkInProgress ? $t('update.checking') : $t('update.checkForUpdates')
                }}</AnimatedText>
              </span>
            </button>

            <!-- 上次检查时间 -->
            <p v-if="updateStore.lastCheckTime" class="text-xs text-gray-400 text-center">
              {{ $t('update.lastChecked') }}: {{ formatLastCheckTime(updateStore.lastCheckTime) }}
            </p>
          </div>
        </transition>
      </section>

      <!-- 5. Logs Section (Collapsible) -->
      <section
        class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
      >
        <!-- Header (clickable to expand/collapse) -->
        <div
          class="p-4 flex items-center justify-between cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-700/30 rounded-xl transition-colors"
          @click="toggleLogsExpanded"
        >
          <div class="flex items-center space-x-3">
            <div
              class="w-8 h-8 bg-amber-100 dark:bg-amber-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-amber-600 dark:text-amber-400"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                ></path>
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                <AnimatedText>{{ $t('settings.logs') }}</AnimatedText>
              </h3>
              <p class="text-xs text-gray-500 dark:text-gray-400">
                <AnimatedText>{{ $t('settings.logLevelDesc') }}</AnimatedText>
              </p>
            </div>
          </div>

          <!-- Expand/Collapse indicator -->
          <svg
            class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
            :class="{ 'rotate-180': logsExpanded }"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 9l-7 7-7-7"
            ></path>
          </svg>
        </div>

        <!-- Collapsible content -->
        <transition name="collapse">
          <div v-if="logsExpanded" class="px-4 pb-4 space-y-3">
            <!-- Log Level Selector -->
            <div
              class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5"
            >
              <div class="flex-1">
                <label class="text-xs font-medium text-gray-700 dark:text-gray-300"><AnimatedText>{{ $t('settings.logLevel') }}</AnimatedText></label>
              </div>
              <div class="flex items-center space-x-2">
                <button
                  v-for="level in ['basic', 'full', 'debug'] as const"
                  :key="level"
                  class="px-3 py-1 text-xs rounded-md transition-all duration-200 border"
                  :class="
                    store.logLevel === level
                      ? 'bg-blue-500 text-white border-blue-500 shadow-sm'
                      : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700'
                  "
                  @click.stop="store.setLogLevel(level)"
                >
                  <AnimatedText>
                    {{
                      $t(`settings.logLevel${level.charAt(0).toUpperCase() + level.slice(1)}`)
                    }}
                  </AnimatedText>
                </button>
              </div>
            </div>

            <!-- Action buttons -->
            <div class="flex items-center justify-end space-x-2">
              <button
                class="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700/50 hover:bg-gray-200 dark:hover:bg-gray-600/50 text-gray-700 dark:text-gray-300 rounded-md transition-colors border border-transparent dark:border-white/5"
                @click.stop="refreshLogs"
              >
                <AnimatedText>{{ $t('settings.refreshLogs') }}</AnimatedText>
              </button>
              <button
                class="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700/50 hover:bg-gray-200 dark:hover:bg-gray-600/50 text-gray-700 dark:text-gray-300 rounded-md transition-colors border border-transparent dark:border-white/5"
                @click.stop="handleOpenLogFolder"
              >
                <AnimatedText>{{ $t('settings.openLogFolder') }}</AnimatedText>
              </button>
              <button
                class="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700/50 hover:bg-gray-200 dark:hover:bg-gray-600/50 text-gray-700 dark:text-gray-300 rounded-md transition-colors border border-transparent dark:border-white/5"
                @click.stop="handleCopyLogs"
              >
                <AnimatedText>{{ $t('settings.copyLogs') }}</AnimatedText>
              </button>
            </div>

            <!-- Log viewer -->
            <div
              ref="logContainer"
              class="h-48 overflow-y-auto bg-gray-900 rounded-lg p-3 font-mono text-xs scrollbar-thin"
            >
              <div v-if="recentLogs.length === 0" class="text-gray-500 text-center py-4">
                {{ $t('settings.noLogs') }}
              </div>
              <div
                v-for="(log, index) in recentLogs"
                :key="index"
                class="leading-relaxed whitespace-pre-wrap break-all"
                :class="getLogColorClass(log)"
              >
                {{ log }}
              </div>
            </div>

            <!-- Log path -->
            <div class="text-[10px] text-gray-400 dark:text-gray-500 truncate" :title="logPath">
              {{ logPath }}
            </div>
          </div>
        </transition>
      </section>

      <!-- 6. About -->
      <section
        class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300"
      >
        <div
          class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors"
          :class="aboutExpanded ? 'rounded-t-xl' : 'rounded-xl'"
          @click="aboutExpanded = !aboutExpanded"
        >
          <div class="flex items-center space-x-4">
            <div
              class="w-12 h-12 rounded-xl shadow-lg transform rotate-3 flex-shrink-0 overflow-hidden"
            >
              <img src="/icon.png" alt="XFast Manager" class="w-full h-full object-cover" />
            </div>
            <div>
              <h3 class="text-base font-bold text-gray-900 dark:text-white">XFast Manager</h3>
              <p class="text-xs text-gray-500 dark:text-gray-400">
                v{{ appVersion }} • © 2026 3370Tech
              </p>
            </div>
          </div>
          <svg
            class="w-5 h-5 text-gray-400 transition-transform duration-200"
            :class="{ 'rotate-180': aboutExpanded }"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 9l-7 7-7-7"
            ></path>
          </svg>
        </div>

        <!-- Contributors section -->
        <transition name="collapse">
          <div
            v-if="aboutExpanded"
            class="px-4 pb-4 border-t border-gray-100 dark:border-gray-700/50"
          >
            <div class="pt-4 space-y-3">
              <!-- Development -->
              <div>
                <h4
                  class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2"
                >
                  {{ $t('settings.development') }}
                </h4>
                <div class="flex flex-wrap gap-2">
                  <button
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-blue-100 dark:bg-blue-500/20 text-blue-700 dark:text-blue-400 hover:bg-blue-200 dark:hover:bg-blue-500/30 transition-colors cursor-pointer"
                    @click="openDeveloperProfile"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M12.316 3.051a1 1 0 01.633 1.265l-4 12a1 1 0 11-1.898-.632l4-12a1 1 0 011.265-.633zM5.707 6.293a1 1 0 010 1.414L3.414 10l2.293 2.293a1 1 0 11-1.414 1.414l-3-3a1 1 0 010-1.414l3-3a1 1 0 011.414 0zm8.586 0a1 1 0 011.414 0l3 3a1 1 0 010 1.414l-3 3a1 1 0 11-1.414-1.414L16.586 10l-2.293-2.293a1 1 0 010-1.414z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    CCA3370
                  </button>
                </div>
              </div>

              <!-- Testing -->
              <div>
                <h4
                  class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2"
                >
                  {{ $t('settings.testing') }}
                </h4>
                <div class="flex flex-wrap gap-2">
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 dark:bg-green-500/20 text-green-700 dark:text-green-400"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    SINO1660
                  </span>
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 dark:bg-green-500/20 text-green-700 dark:text-green-400"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    enenY
                  </span>
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-orange-100 dark:bg-orange-500/20 text-orange-700 dark:text-orange-400"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    🍊
                  </span>
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 dark:bg-green-500/20 text-green-700 dark:text-green-400"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    Tong Wu
                  </span>
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 dark:bg-green-500/20 text-green-700 dark:text-green-400"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    HTony03
                  </span>
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 dark:bg-green-500/20 text-green-700 dark:text-green-400"
                  >
                    <svg class="w-3.5 h-3.5 mr-1.5" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fill-rule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    CHN131
                  </span>
                </div>
              </div>
            </div>
          </div>
        </transition>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useAppStore } from '@/stores/app'
import { useSceneryStore } from '@/stores/scenery'
import { useUpdateStore } from '@/stores/update'
import { validateGlobPattern } from '@/utils/validation'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import AnimatedText from '@/components/AnimatedText.vue'
import { AddonType, getErrorMessage } from '@/types'
import { logger, logError, logDebug } from '@/services/logger'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'
import { setTrackedTimeout } from '@/utils/timeout'

const { t } = useI18n()
const store = useAppStore()
const sceneryStore = useSceneryStore()
const toast = useToastStore()
const modal = useModalStore()
const updateStore = useUpdateStore()

const xplanePathInput = ref('')
const launchArgsInput = ref('')
const isProcessing = ref(false)
const saveStatus = ref<'saving' | 'saved' | null>(null)
const pathError = ref<string | null>(null)
const appVersion = ref('0.1.1')
const updateCheckExpanded = ref(false)

// Timer tracking for cleanup on unmount to prevent memory leaks
// Using Set for O(1) add/delete operations instead of array
const activeTimeoutIds = new Set<ReturnType<typeof setTimeout>>()

// Debounce timer for path validation
let pathValidationDebounceTimer: ReturnType<typeof setTimeout> | null = null
const PATH_VALIDATION_DEBOUNCE_MS = 300
const SAVE_STATUS_DISPLAY_DURATION_MS = 2000 // Duration to show "Saved" status

// Logs state
const recentLogs = ref<string[]>([])
const logPath = ref('')
const logsExpanded = ref(false)
const logContainer = ref<HTMLElement | null>(null)

// Config patterns state
const configPatterns = ref<string[]>([])
const patternErrors = ref<Record<number, string>>({})
const backupExpanded = ref(false)
const preferencesExpanded = ref(false) // Default collapsed
const verificationExpanded = ref(false) // Default collapsed
const atomicExpanded = ref(false) // Default collapsed
const deleteSourceExpanded = ref(false) // Default collapsed
const windowsIntegrationExpanded = ref(false) // Default collapsed
const patternSaveStatus = ref<'saving' | 'saved' | null>(null)
const sceneryAutoSortExpanded = ref(false) // Default collapsed
const aboutExpanded = ref(false) // Default collapsed
const isRebuildingIndex = ref(false)
const indexExists = computed(() => sceneryStore.indexExists)

const addonTypes = [
  AddonType.Aircraft,
  AddonType.Scenery,
  AddonType.SceneryLibrary,
  AddonType.Plugin,
  AddonType.Navdata,
  AddonType.Livery,
]

onMounted(() => {
  sceneryStore.loadIndexStatus()
})

watch(
  () => store.xplanePath,
  () => {
    sceneryStore.loadIndexStatus()
  },
)

// Scroll log container to bottom
function scrollLogsToBottom() {
  // Use setTimeout to ensure DOM is fully rendered and transition is complete
  setTrackedTimeout(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  }, 100, activeTimeoutIds) // Wait 100ms for transition
}

// Master toggle computed
const allPreferencesEnabled = computed(() => {
  return addonTypes.every((type) => store.installPreferences[type])
})

function toggleAllPreferences() {
  const newValue = !allPreferencesEnabled.value
  addonTypes.forEach((type) => {
    if (store.installPreferences[type] !== newValue) {
      store.togglePreference(type)
    }
  })
}

async function handleToggleAutoSortScenery() {
  const wasEnabled = store.autoSortScenery
  store.toggleAutoSortScenery()

  if (!wasEnabled && store.autoSortScenery) {
    const shown = await getItem<string>(STORAGE_KEYS.SCENERY_AUTO_SORT_HINT_SHOWN)
    if (!shown) {
      store.showSceneryManagerHint('sceneryManager.hintFromSettings')
      await setItem(STORAGE_KEYS.SCENERY_AUTO_SORT_HINT_SHOWN, 'true')
    }
  }
}

function getTypeName(type: AddonType): string {
  switch (type) {
    case AddonType.Aircraft:
      return t('settings.typeAircraft')
    case AddonType.Scenery:
      return t('settings.typeScenery')
    case AddonType.SceneryLibrary:
      return t('settings.typeSceneryLibrary')
    case AddonType.Plugin:
      return t('settings.typePlugin')
    case AddonType.Navdata:
      return t('settings.typeNavdata')
    case AddonType.Livery:
      return t('settings.typeLivery')
    case AddonType.LuaScript:
      return t('settings.typeLuaScript')
    default:
      return type
  }
}

onMounted(async () => {
  xplanePathInput.value = store.xplanePath
  launchArgsInput.value = store.xplaneLaunchArgs

  try {
    // Get app version
    appVersion.value = await invoke<string>('get_app_version')
  } catch (error) {
    logError(`Failed to get app version: ${error}`, 'settings')
  }

  // Check and sync context menu registration status (Windows only)
  if (store.isWindows) {
    try {
      const actualStatus = await invoke<boolean>('is_context_menu_registered')
      // If stored status doesn't match actual status, update it
      if (store.isContextMenuRegistered !== actualStatus) {
        logDebug(
          `Context menu status mismatch: stored=${store.isContextMenuRegistered}, actual=${actualStatus}. Syncing...`,
          'settings',
        )
        store.isContextMenuRegistered = actualStatus
      }
    } catch (error) {
      logError(`Failed to check context menu status: ${error}`, 'settings')
    }
  }

  // Load logs
  await refreshLogs()
  logPath.value = await logger.getLogPath()

  // Load config patterns
  configPatterns.value = [...store.getConfigFilePatterns()]
})

// Cleanup timers on component unmount to prevent memory leaks
onBeforeUnmount(() => {
  // Clear all tracked timeouts
  activeTimeoutIds.forEach((id) => clearTimeout(id))
  activeTimeoutIds.clear()
  // Clear path validation debounce timer
  if (pathValidationDebounceTimer) {
    clearTimeout(pathValidationDebounceTimer)
    pathValidationDebounceTimer = null
  }
})

// Handle X-Plane path input blur - validate and save with debounce
function handlePathBlur() {
  // Clear existing debounce timer
  if (pathValidationDebounceTimer) {
    clearTimeout(pathValidationDebounceTimer)
  }

  // Debounce to prevent excessive API calls on rapid blur events
  pathValidationDebounceTimer = setTimeout(async () => {
    pathValidationDebounceTimer = null
    await validateAndSavePath()
  }, PATH_VALIDATION_DEBOUNCE_MS)
}

// Handle launch args input blur - save to store
function handleLaunchArgsBlur() {
  const newValue = launchArgsInput.value.trim()
  if (newValue !== store.xplaneLaunchArgs) {
    store.setXplaneLaunchArgs(newValue)
  }
}

// Internal function to validate and save path
async function validateAndSavePath() {
  const newValue = xplanePathInput.value

  // Clear previous error
  pathError.value = null

  // Only save if different from store
  if (newValue !== store.xplanePath) {
    saveStatus.value = 'saving'

    // If path is not empty, validate it
    if (newValue.trim() !== '') {
      try {
        const isValid = await invoke<boolean>('validate_xplane_path', { path: newValue })
        if (!isValid) {
          // Check if path exists first
          const exists = await invoke<boolean>('check_path_exists', { path: newValue })
          if (!exists) {
            pathError.value = t('settings.pathNotExist')
          } else {
            pathError.value = t('settings.notValidXplanePath')
          }
          saveStatus.value = null
          return
        }
      } catch (error) {
        logError(`Failed to validate path: ${error}`, 'settings')
      }
    }

    // Save the path
    store.setXplanePath(newValue)
    saveStatus.value = 'saved'
    setTrackedTimeout(() => {
      saveStatus.value = null
    }, SAVE_STATUS_DISPLAY_DURATION_MS, activeTimeoutIds)
  }
}

// Handle pattern input blur - validate and save
function handlePatternBlur() {
  // Validate and filter patterns
  const errors: Record<number, string> = {}
  const validPatterns: string[] = []

  configPatterns.value.forEach((pattern, index) => {
    const trimmed = pattern.trim()
    if (trimmed === '') return

    // Validate glob pattern - returns error key or null
    const errorKey = validateGlobPattern(trimmed)
    if (errorKey) {
      errors[index] = t(errorKey)
    } else {
      validPatterns.push(trimmed)
    }
  })

  patternErrors.value = errors

  // Save valid patterns
  if (validPatterns.length > 0 || configPatterns.value.length === 0) {
    patternSaveStatus.value = 'saving'

    // Save only the valid (non-empty) patterns
    store.setConfigFilePatterns(validPatterns)
    patternSaveStatus.value = 'saved'

    // Hide saved status after 2 seconds
    setTrackedTimeout(() => {
      patternSaveStatus.value = null
    }, SAVE_STATUS_DISPLAY_DURATION_MS, activeTimeoutIds)
  }
}

// Add a new pattern
function addPattern() {
  configPatterns.value.push('')
}

// Remove a pattern by index and save immediately
function removePattern(index: number) {
  configPatterns.value.splice(index, 1)

  // Save immediately after deletion
  patternSaveStatus.value = 'saving'

  // Validate and filter remaining patterns
  const validPatterns = configPatterns.value
    .map((p) => p.trim())
    .filter((p) => p !== '' && !validateGlobPattern(p))

  store.setConfigFilePatterns(validPatterns)
  patternSaveStatus.value = 'saved'

  setTrackedTimeout(() => {
    patternSaveStatus.value = null
  }, SAVE_STATUS_DISPLAY_DURATION_MS, activeTimeoutIds)
}

async function selectFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.selectXplaneFolder'),
    })

    // Type guard: with multiple: false, selected is string | null (not array)
    if (selected && typeof selected === 'string') {
      const selectedPath = selected

      // Validate path before saving
      pathError.value = null
      saveStatus.value = 'saving'

      try {
        const isValid = await invoke<boolean>('validate_xplane_path', { path: selectedPath })
        if (!isValid) {
          const exists = await invoke<boolean>('check_path_exists', { path: selectedPath })
          if (!exists) {
            pathError.value = t('settings.pathNotExist')
          } else {
            pathError.value = t('settings.notValidXplanePath')
          }
          saveStatus.value = null
          // Still update input to show what was selected
          xplanePathInput.value = selectedPath
          return
        }
      } catch (error) {
        logError(`Failed to validate path: ${error}`, 'settings')
      }

      // Path is valid, save it
      xplanePathInput.value = selectedPath
      store.setXplanePath(selectedPath)
      saveStatus.value = 'saved'
      setTrackedTimeout(() => {
        saveStatus.value = null
      }, SAVE_STATUS_DISPLAY_DURATION_MS, activeTimeoutIds)
    }
  } catch (error) {
    logError(`Failed to open folder dialog: ${error}`, 'settings')
    modal.showError(t('common.error') + ': ' + getErrorMessage(error))
  }
}

async function toggleContextMenu() {
  if (isProcessing.value) return
  isProcessing.value = true
  try {
    if (!store.isContextMenuRegistered) {
      // Register context menu
      try {
        await invoke('register_context_menu')
        toast.success(t('settings.contextMenuRegistered'))
        store.isContextMenuRegistered = true
      } catch (error) {
        // If already registered, just update the state
        const errorMsg = getErrorMessage(error).toLowerCase()
        if (errorMsg.includes('already') || errorMsg.includes('exist')) {
          logDebug('Context menu already registered, updating state', 'settings')
          store.isContextMenuRegistered = true
          toast.info(t('settings.contextMenuRegistered'))
        } else {
          throw error
        }
      }
    } else {
      // Unregister context menu
      try {
        await invoke('unregister_context_menu')
        toast.success(t('settings.contextMenuUnregistered'))
        store.isContextMenuRegistered = false
      } catch (error) {
        // If already unregistered, just update the state
        const errorMsg = getErrorMessage(error).toLowerCase()
        if (
          errorMsg.includes('not found') ||
          errorMsg.includes('not exist') ||
          errorMsg.includes('not registered')
        ) {
          logDebug('Context menu already unregistered, updating state', 'settings')
          store.isContextMenuRegistered = false
          toast.info(t('settings.contextMenuUnregistered'))
        } else {
          throw error
        }
      }
    }
  } catch (error) {
    modal.showError(t('common.error') + ': ' + getErrorMessage(error))
  } finally {
    isProcessing.value = false
  }
}

// Log functions
function toggleLogsExpanded() {
  logsExpanded.value = !logsExpanded.value
  if (logsExpanded.value) {
    refreshLogs()
  }
}

async function refreshLogs() {
  recentLogs.value = await logger.getRecentLogs(50)
  // Scroll to bottom after logs are loaded
  scrollLogsToBottom()
}

async function handleOpenLogFolder() {
  try {
    await logger.openLogFolder()
  } catch (error) {
    modal.showError(t('common.error') + ': ' + getErrorMessage(error))
  }
}

async function handleCopyLogs() {
  const success = await logger.copyLogsToClipboard()
  if (success) {
    toast.success(t('settings.logsCopied'))
  } else {
    modal.showError(t('common.error'))
  }
}

function getLogColorClass(log: string): string {
  if (log.includes('[ERROR]')) {
    return 'text-red-400'
  } else if (log.includes('[DEBUG]')) {
    return 'text-purple-400'
  } else if (log.includes('[user-action]')) {
    return 'text-blue-400'
  }
  return 'text-gray-300'
}

// Update check functions
async function handleCheckUpdate() {
  await updateStore.checkForUpdates(true)
}

// Scenery auto-sorting functions
async function handleRebuildIndex() {
  if (isRebuildingIndex.value || !store.xplanePath) return

  isRebuildingIndex.value = true
  try {
    await invoke('rebuild_scenery_index', { xplanePath: store.xplanePath })
    toast.success(t('settings.indexRebuilt'))
    await sceneryStore.loadIndexStatus()
  } catch (error) {
    logError(`Failed to rebuild scenery index: ${error}`, 'settings')
    modal.showError(t('settings.indexRebuildFailed') + ': ' + getErrorMessage(error))
  } finally {
    isRebuildingIndex.value = false
  }
}

// About section functions
async function openDeveloperProfile() {
  try {
    await invoke('open_url', { url: 'https://forums.x-plane.org/profile/1288218-3370/' })
  } catch (error) {
    logError(`Failed to open URL: ${error}`, 'settings')
  }
}

function formatLastCheckTime(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes < 1) {
    return t('update.justNow')
  } else if (minutes < 60) {
    return t('update.minutesAgo', { minutes })
  } else if (hours < 24) {
    return t('update.hoursAgo', { hours })
  } else {
    return t('update.daysAgo', { days })
  }
}
</script>

<style scoped>
/* Collapse transition */
.collapse-enter-active,
.collapse-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.collapse-enter-from,
.collapse-leave-to {
  opacity: 0;
  max-height: 0;
  padding-top: 0;
  padding-bottom: 0;
}

.collapse-enter-to,
.collapse-leave-from {
  opacity: 1;
  max-height: 400px;
}
</style>
