<template>
  <div class="settings-view h-full flex flex-col p-5 overflow-hidden">
    <!-- Header -->
    <div class="mb-4 flex-shrink-0">
      <h2 class="text-2xl font-bold text-gray-900 dark:text-white"><AnimatedText>{{ $t('common.settings') }}</AnimatedText></h2>
    </div>

    <!-- Scrollable Content Area -->
    <div class="flex-1 overflow-y-auto space-y-4 pr-1">
      
      <!-- 1. X-Plane Path (Compact) -->
      <section class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300">
        <div class="p-4 space-y-3">
          <div class="flex items-center space-x-3">
            <div class="w-8 h-8 bg-blue-100 dark:bg-blue-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-blue-600 dark:text-blue-400">
              <!-- Folder icon -->
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
              </svg>
            </div>
            <div class="flex-1">
              <h3 class="text-sm font-semibold text-gray-900 dark:text-white"><AnimatedText>{{ $t('settings.xplanePath') }}</AnimatedText></h3>
            </div>
          </div>

          <div class="relative pb-2">
            <div class="flex items-center bg-gray-50 dark:bg-gray-900/50 border rounded-lg overflow-hidden focus-within:border-blue-500 dark:focus-within:border-blue-500 transition-colors duration-200"
              :class="pathError ? 'border-red-500 dark:border-red-500' : 'border-gray-200 dark:border-gray-700/50'"
            >
              <input
                v-model="xplanePathInput"
                type="text"
                placeholder="C:\X-Plane 12"
                class="flex-1 px-4 py-2.5 bg-transparent border-none text-sm text-gray-900 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-600 focus:outline-none focus:ring-0"
              />
              <button
                type="button"
                @click.stop.prevent="selectFolder"
                class="px-4 py-1.5 m-1 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-xs font-medium rounded-md transition-colors duration-200 flex items-center space-x-1.5 flex-shrink-0 border border-gray-300 dark:border-gray-600"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"></path>
                </svg>
                <span><AnimatedText>{{ $t('common.browse') }}</AnimatedText></span>
              </button>
            </div>
            <!-- Error message with absolute positioning -->
            <transition name="fade">
              <div v-if="pathError" class="absolute left-0 top-full mt-1 text-xs text-red-500 dark:text-red-400 flex items-center space-x-1">
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
                <span>{{ pathError }}</span>
              </div>
            </transition>
          </div>
          
          <!-- Auto-save status -->
          <div class="h-4 flex items-center justify-end px-1">
            <transition name="fade">
              <div v-if="saveStatus" class="flex items-center text-[10px] font-medium space-x-1" :class="saveStatus === 'saved' ? 'text-emerald-500' : 'text-gray-400'">
                <svg v-if="saveStatus === 'saved'" class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                </svg>
                <svg v-else class="w-3 h-3 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                </svg>
                <span>{{ saveStatus === 'saved' ? $t('settings.saved') : $t('settings.saving') }}</span>
              </div>
            </transition>
          </div>
        </div>
      </section>

      <!-- 2. Grid for Windows Integration & Preferences -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">

        <!-- Windows Integration (Left Column, Windows only) -->
        <transition name="slide-up">
          <section v-if="isWindows" class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300">
            <div class="p-4">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                  <div class="w-8 h-8 bg-gray-100 dark:bg-gray-700 rounded-lg flex items-center justify-center flex-shrink-0 text-gray-600 dark:text-gray-300">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
                    </svg>
                  </div>
                  <div>
                    <h3 class="text-sm font-semibold text-gray-900 dark:text-white"><AnimatedText>{{ $t('settings.windowsIntegration') }}</AnimatedText></h3>
                    <p class="text-xs text-gray-500 dark:text-gray-400"><AnimatedText>{{ $t('settings.windowsIntegrationDesc') }}</AnimatedText></p>
                  </div>
                </div>

                <button
                  @click="toggleContextMenu"
                  :disabled="isProcessing"
                  class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-300 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1 focus:ring-offset-white dark:focus:ring-offset-gray-900"
                  :class="isContextRegistered ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-700'"
                >
                  <span
                    class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-300 shadow-sm"
                    :class="isContextRegistered ? 'translate-x-4.5' : 'translate-x-0.5'"
                  />
                </button>
              </div>
            </div>
          </section>
        </transition>

        <!-- Placeholder for non-Windows (to maintain grid layout) -->
        <div v-if="!isWindows"></div>

        <!-- Installation Preferences (Right Column or Full Width on non-Windows) -->
        <section class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300" :class="{ 'md:col-span-2': !isWindows }">
          <div
            class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
            @click="preferencesExpanded = !preferencesExpanded"
          >
            <div class="flex items-center space-x-3">
              <div class="w-8 h-8 bg-green-100 dark:bg-green-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-green-600 dark:text-green-400">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"></path>
                </svg>
              </div>
              <div>
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white"><AnimatedText>{{ $t('settings.installPreferences') }}</AnimatedText></h3>
                <p class="text-xs text-gray-500 dark:text-gray-400"><AnimatedText>{{ $t('settings.installPreferencesDesc') }}</AnimatedText></p>
              </div>
            </div>

            <!-- Expand/Collapse indicator -->
            <svg
              class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
              :class="{ 'rotate-180': preferencesExpanded }"
              fill="none" stroke="currentColor" viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
            </svg>
          </div>

          <!-- Collapsible content -->
          <transition name="collapse">
            <div v-if="preferencesExpanded" class="px-4 pb-4 space-y-3">
              <!-- Master Toggle -->
              <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5">
                <span class="text-xs font-medium text-gray-700 dark:text-gray-300"><AnimatedText>{{ $t('settings.toggleAll') }}</AnimatedText></span>
                <button
                  @click="toggleAllPreferences"
                  class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                  :class="allPreferencesEnabled ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                >
                  <span
                    class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                    :class="allPreferencesEnabled ? 'translate-x-4.5' : 'translate-x-0.5'"
                  />
                </button>
              </div>

              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <div v-for="type in addonTypes" :key="type" class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5">
                  <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2" :title="getTypeName(type)">
                    <AnimatedText>{{ getTypeName(type) }}</AnimatedText>
                  </span>
                  <button
                    @click="store.togglePreference(type)"
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                    :class="store.installPreferences[type] ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="store.installPreferences[type] ? 'translate-x-3.5' : 'translate-x-0.5'"
                    />
                  </button>
                </div>
              </div>
            </div>
          </transition>
        </section>
      </div>

      <!-- 3. Aircraft Backup Configuration -->
      <section class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300">
          <div
            class="p-4 flex items-center justify-between cursor-pointer hover:bg-gray-50/50 dark:hover:bg-gray-700/20 transition-colors rounded-t-xl"
            @click="backupExpanded = !backupExpanded"
          >
            <div class="flex items-center space-x-3">
              <div class="w-8 h-8 bg-blue-100 dark:bg-blue-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-blue-600 dark:text-blue-400">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"></path>
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

            <!-- Expand/Collapse indicator -->
            <svg
              class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
              :class="{ 'rotate-180': backupExpanded }"
              fill="none" stroke="currentColor" viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
            </svg>
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
                  <div v-for="(pattern, index) in configPatterns" :key="index"
                       class="flex items-center gap-2 p-2 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5">
                    <input
                      v-model="configPatterns[index]"
                      type="text"
                      class="flex-1 px-2 py-1 text-xs bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded text-gray-900 dark:text-gray-200 focus:outline-none focus:ring-1 focus:ring-blue-500"
                      placeholder="*_prefs.txt"
                    >
                    <button
                      @click="removePattern(index)"
                      class="p-1 text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 rounded transition-colors"
                      :title="$t('common.delete')"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                      </svg>
                    </button>
                  </div>
                </div>

                <button
                  @click="addPattern"
                  class="w-full px-3 py-1.5 text-xs bg-blue-50 dark:bg-blue-500/10 hover:bg-blue-100 dark:hover:bg-blue-500/20 text-blue-600 dark:text-blue-400 rounded-lg transition-colors border border-blue-200 dark:border-blue-500/20 flex items-center justify-center gap-1"
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
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

      <!-- 4. Logs Section (Collapsible) -->
      <section class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300">
        <!-- Header (clickable to expand/collapse) -->
        <div
          class="p-4 flex items-center justify-between cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-700/30 rounded-xl transition-colors"
          @click="toggleLogsExpanded"
        >
          <div class="flex items-center space-x-3">
            <div class="w-8 h-8 bg-amber-100 dark:bg-amber-500/10 rounded-lg flex items-center justify-center flex-shrink-0 text-amber-600 dark:text-amber-400">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-semibold text-gray-900 dark:text-white"><AnimatedText>{{ $t('settings.logs') }}</AnimatedText></h3>
              <p class="text-xs text-gray-500 dark:text-gray-400"><AnimatedText>{{ $t('settings.logLevelDesc') }}</AnimatedText></p>
            </div>
          </div>

          <!-- Expand/Collapse indicator -->
          <svg
            class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
            :class="{ 'rotate-180': logsExpanded }"
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
          </svg>
        </div>

        <!-- Collapsible content -->
        <transition name="collapse">
          <div v-if="logsExpanded" class="px-4 pb-4 space-y-3">
            <!-- Log Level Selector -->
            <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5">
              <div class="flex-1">
                <label class="text-xs font-medium text-gray-700 dark:text-gray-300"><AnimatedText>{{ $t('settings.logLevel') }}</AnimatedText></label>
              </div>
              <div class="flex items-center space-x-2">
                <button
                  v-for="level in ['basic', 'full', 'debug']"
                  :key="level"
                  @click.stop="store.setLogLevel(level)"
                  class="px-3 py-1 text-xs rounded-md transition-all duration-200 border"
                  :class="store.logLevel === level
                    ? 'bg-blue-500 text-white border-blue-500 shadow-sm'
                    : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700'"
                >
                  <AnimatedText>{{ $t(`settings.logLevel${level.charAt(0).toUpperCase() + level.slice(1)}`) }}</AnimatedText>
                </button>
              </div>
            </div>

            <!-- Action buttons -->
            <div class="flex items-center justify-end space-x-2">
              <button
                @click.stop="refreshLogs"
                class="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700/50 hover:bg-gray-200 dark:hover:bg-gray-600/50 text-gray-700 dark:text-gray-300 rounded-md transition-colors border border-transparent dark:border-white/5"
              >
                <AnimatedText>{{ $t('settings.refreshLogs') }}</AnimatedText>
              </button>
              <button
                @click.stop="handleOpenLogFolder"
                class="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700/50 hover:bg-gray-200 dark:hover:bg-gray-600/50 text-gray-700 dark:text-gray-300 rounded-md transition-colors border border-transparent dark:border-white/5"
              >
                <AnimatedText>{{ $t('settings.openLogFolder') }}</AnimatedText>
              </button>
              <button
                @click.stop="handleCopyLogs"
                class="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700/50 hover:bg-gray-200 dark:hover:bg-gray-600/50 text-gray-700 dark:text-gray-300 rounded-md transition-colors border border-transparent dark:border-white/5"
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
              >{{ log }}</div>
            </div>

            <!-- Log path -->
            <div class="text-[10px] text-gray-400 dark:text-gray-500 truncate" :title="logPath">
              {{ logPath }}
            </div>
          </div>
        </transition>
      </section>

      <!-- 4. About -->
      <section class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md transition-colors duration-300">
        <div class="p-4 flex items-center space-x-4">
          <div class="w-12 h-12 rounded-xl shadow-lg transform rotate-3 flex-shrink-0 overflow-hidden">
            <img src="/icon.png" alt="XFastInstall" class="w-full h-full object-cover" />
          </div>
          <div>
            <h3 class="text-base font-bold text-gray-900 dark:text-white">XFastInstall</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">
              v0.1.0 • © 2026
            </p>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import { useAppStore } from '@/stores/app'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import AnimatedText from '@/components/AnimatedText.vue'
import { AddonType } from '@/types'
import { logger } from '@/services/logger'

const { t } = useI18n()
const store = useAppStore()
const toast = useToastStore()
const modal = useModalStore()

const xplanePathInput = ref('')
const isWindows = ref(false)
const isContextRegistered = ref(false)
const isProcessing = ref(false)
const saveStatus = ref<'saving' | 'saved' | null>(null)
const pathError = ref<string | null>(null)
let saveTimeout: ReturnType<typeof setTimeout> | null = null

// Logs state
const recentLogs = ref<string[]>([])
const logPath = ref('')
const logsExpanded = ref(false)
const logContainer = ref<HTMLElement | null>(null)

// Config patterns state
const configPatterns = ref<string[]>([])
const backupExpanded = ref(false)
const preferencesExpanded = ref(false) // Default collapsed

const addonTypes = [AddonType.Aircraft, AddonType.Scenery, AddonType.SceneryLibrary, AddonType.Plugin, AddonType.Navdata]

// Scroll log container to bottom
function scrollLogsToBottom() {
  // Use setTimeout to ensure DOM is fully rendered and transition is complete
  setTimeout(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  }, 100) // Wait 100ms for transition
}

// Master toggle computed
const allPreferencesEnabled = computed(() => {
  return addonTypes.every(type => store.installPreferences[type])
})

function toggleAllPreferences() {
  const newValue = !allPreferencesEnabled.value
  addonTypes.forEach(type => {
    if (store.installPreferences[type] !== newValue) {
      store.togglePreference(type)
    }
  })
}

function getTypeName(type: AddonType): string {
  switch (type) {
    case AddonType.Aircraft: return t('settings.typeAircraft')
    case AddonType.Scenery: return t('settings.typeScenery')
    case AddonType.SceneryLibrary: return t('settings.typeSceneryLibrary')
    case AddonType.Plugin: return t('settings.typePlugin')
    case AddonType.Navdata: return t('settings.typeNavdata')
    default: return type
  }
}

onMounted(async () => {
  xplanePathInput.value = store.xplanePath

  try {
    const platform = await invoke<string>('get_platform')
    isWindows.value = platform === 'windows'

    // Check if context menu is already registered (Windows only)
    if (isWindows.value) {
      isContextRegistered.value = await invoke<boolean>('is_context_menu_registered')
    }
  } catch (error) {
    console.error('Failed to get platform:', error)
  }

  // Load logs
  await refreshLogs()
  logPath.value = await logger.getLogPath()

  // Load config patterns
  configPatterns.value = [...store.getConfigFilePatterns()]
})

// Cleanup timers on component unmount to prevent memory leaks
onBeforeUnmount(() => {
  if (saveTimeout) {
    clearTimeout(saveTimeout)
    saveTimeout = null
  }
})

// Auto-save logic with path validation
watch(xplanePathInput, async (newValue) => {
  if (saveTimeout) clearTimeout(saveTimeout)

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
        console.error('Failed to validate path:', error)
      }
    }

    saveTimeout = setTimeout(() => {
      store.setXplanePath(newValue)
      saveStatus.value = 'saved'
      setTimeout(() => {
        saveStatus.value = null
      }, 2000)
    }, 800) // 800ms debounce
  }
})

// Watch config patterns and save to store
watch(configPatterns, (newPatterns) => {
  // Filter out empty patterns
  const filtered = newPatterns.filter(p => p.trim() !== '')
  store.setConfigFilePatterns(filtered)
}, { deep: true })

// Add a new pattern
function addPattern() {
  configPatterns.value.push('')
}

// Remove a pattern by index
function removePattern(index: number) {
  configPatterns.value.splice(index, 1)
}

async function selectFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.selectXplaneFolder')
    })
    
    if (selected) {
      xplanePathInput.value = selected as string
      // Immediate save on selection
      if (saveTimeout) clearTimeout(saveTimeout)
      store.setXplanePath(xplanePathInput.value)
      saveStatus.value = 'saved'
      setTimeout(() => { saveStatus.value = null }, 2000)
    }
  } catch (error) {
    console.error('Failed to open folder dialog:', error)
    modal.showError(t('common.error') + ': ' + String(error))
  }
}

async function toggleContextMenu() {
  if (isProcessing.value) return
  isProcessing.value = true
  try {
    if (!isContextRegistered.value) {
      await invoke('register_context_menu')
      toast.success(t('settings.contextMenuRegistered'))
      isContextRegistered.value = true
    } else {
      await invoke('unregister_context_menu')
      toast.success(t('settings.contextMenuUnregistered'))
      isContextRegistered.value = false
    }
  } catch (error) {
    modal.showError(t('common.error') + ': ' + String(error))
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
    modal.showError(t('common.error') + ': ' + String(error))
  }
}

async function handleCopyLogs() {
  const success = await logger.copyLogsToClipboard()
  if (success) {
    toast.success(t('settings.logsCopied'))
  } else {
    toast.error(t('common.error'))
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