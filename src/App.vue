<!-- Main App Component -->
<template>
  <div
    class="app-container transition-colors duration-300 text-gray-900 dark:text-gray-100 font-sans selection:bg-blue-500/30"
  >
    <!-- Navbar -->
    <nav class="fixed top-0 left-0 w-full z-50 transition-all duration-300">
      <div
        class="absolute inset-0 bg-white/70 dark:bg-gray-900/70 backdrop-blur-xl border-b border-gray-200/50 dark:border-white/5 shadow-sm dark:shadow-2xl transition-colors duration-300"
      ></div>

      <div class="relative w-full px-4 sm:px-6">
        <div class="w-full">
          <!-- Primary row -->
          <div ref="primaryNavViewport" class="flex h-10 items-center gap-3">
            <div v-if="!isOnboardingRoute" class="min-w-0 flex-1 overflow-hidden">
              <div class="flex items-center gap-1 whitespace-nowrap">
                <template v-for="item in primaryVisibleNavItems" :key="`primary-${item.id}`">
                  <div v-if="item.id === 'management'" class="relative flex items-center">
                    <router-link
                      :to="item.to"
                      class="relative px-2.5 py-1.5 rounded-lg group overflow-hidden transition-all duration-300"
                      :class="navLinkClass(item)"
                    >
                      <div
                        class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                        :class="navLinkBackgroundClass(item)"
                      ></div>
                      <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path
                            v-for="(path, index) in item.iconPaths"
                            :key="`${item.id}-icon-${index}`"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            :d="path"
                          ></path>
                        </svg>
                        <AnimatedText>{{ item.label }}</AnimatedText>
                      </span>
                    </router-link>
                    <div
                      v-if="store.sceneryManagerHintVisible && store.sceneryManagerHintMessageKey"
                      class="absolute left-1/2 top-full -translate-x-1/2 mt-2 z-50"
                    >
                      <div
                        class="relative min-w-[240px] max-w-[340px] w-max bg-cyan-50 dark:bg-cyan-900/60 border border-cyan-200 dark:border-cyan-700 text-cyan-900 dark:text-cyan-100 text-xs px-3 py-2 rounded-lg shadow-lg flex items-start gap-2"
                      >
                        <div
                          class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-cyan-50 dark:bg-cyan-900/60 border-l border-t border-cyan-200 dark:border-cyan-700 rotate-45"
                        ></div>
                        <span class="leading-4">{{ $t(store.sceneryManagerHintMessageKey) }}</span>
                        <button
                          class="ml-1 text-cyan-700/80 dark:text-cyan-200/80 hover:text-cyan-900 dark:hover:text-white"
                          @click="store.dismissSceneryManagerHint()"
                        >
                          <svg
                            class="w-3.5 h-3.5"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                          >
                            <path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M6 18L18 6M6 6l12 12"
                            />
                          </svg>
                        </button>
                      </div>
                    </div>
                  </div>

                  <router-link
                    v-else
                    :ref="item.id === 'log-analysis' ? setLogAnalysisLinkRef : undefined"
                    :to="item.to"
                    class="relative px-2.5 py-1.5 rounded-lg group overflow-hidden transition-all duration-300"
                    :class="navLinkClass(item)"
                  >
                    <div
                      class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                      :class="navLinkBackgroundClass(item)"
                    ></div>
                    <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path
                          v-for="(path, index) in item.iconPaths"
                          :key="`${item.id}-icon-${index}`"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          :d="path"
                        ></path>
                      </svg>
                      <AnimatedText>{{ item.label }}</AnimatedText>
                    </span>
                  </router-link>
                </template>

                <button
                  v-if="hasOverflowNav"
                  class="relative px-2.5 py-1.5 rounded-lg group overflow-hidden transition-all duration-300 text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white"
                  :title="moreNavLabel"
                  @click="navExpanded = !navExpanded"
                >
                  <div
                    class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                    :class="
                      navExpanded
                        ? 'scale-x-100 opacity-100'
                        : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'
                    "
                  ></div>
                  <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                    <svg
                      class="w-4 h-4 transition-transform duration-300"
                      :class="navExpanded ? 'rotate-180' : ''"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M19 9l-7 7-7-7"
                      />
                    </svg>
                    <AnimatedText>{{ moreNavLabel }}</AnimatedText>
                  </span>
                </button>
              </div>
            </div>

            <div ref="primaryNavActions" class="ml-auto flex flex-none items-center">
              <div
                v-if="!isOnboardingRoute"
                class="mx-[clamp(0.75rem,2vw,1.5rem)] h-5 w-px bg-gray-200 dark:bg-white/10 transition-colors"
              ></div>

              <div class="flex items-center gap-1">
                <button
                  class="relative p-1.5 rounded-lg group overflow-hidden transition-all duration-300"
                  :class="
                    $route.path === '/feedback'
                      ? 'text-blue-600 dark:text-white'
                      : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
                  "
                  :title="$t('feedback.navTitle')"
                  @click="handleFeedbackClick"
                >
                  <div
                    class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-center"
                    :class="
                      $route.path === '/feedback'
                        ? 'scale-100 opacity-100'
                        : 'scale-0 opacity-0 group-hover:scale-100 group-hover:opacity-50'
                    "
                  ></div>
                  <span class="relative flex items-center z-10">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M8 10h8M8 14h5M7 3h10a2 2 0 012 2v14l-4-2-4 2-4-2-4 2V5a2 2 0 012-2z"
                      />
                    </svg>
                  </span>
                </button>
                <!-- Sponsor button (Chinese locale only) -->
                <button
                  v-if="locale === 'zh'"
                  class="relative p-1.5 rounded-lg group overflow-hidden transition-all duration-300 text-gray-600 dark:text-gray-400 hover:text-pink-500 dark:hover:text-pink-400"
                  :title="$t('sponsor.title')"
                  @click="showSponsor = true"
                >
                  <div
                    class="absolute inset-0 bg-pink-50 dark:bg-pink-500/10 rounded-lg transition-all duration-300 transform origin-center scale-0 opacity-0 group-hover:scale-100 group-hover:opacity-50"
                  ></div>
                  <span class="relative flex items-center z-10">
                    <svg class="w-4 h-4 sponsor-heartbeat" fill="currentColor" viewBox="0 0 24 24">
                      <path
                        d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"
                      />
                    </svg>
                  </span>
                </button>
                <!-- Always on top button -->
                <button
                  class="relative p-1.5 rounded-lg group overflow-hidden transition-all duration-300"
                  :class="
                    isAlwaysOnTop
                      ? 'text-blue-600 dark:text-blue-400'
                      : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
                  "
                  :title="isAlwaysOnTop ? 'Unpin window' : 'Pin window on top'"
                  @click="toggleAlwaysOnTop"
                >
                  <div
                    class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-center"
                    :class="
                      isAlwaysOnTop
                        ? 'scale-100 opacity-100'
                        : 'scale-0 opacity-0 group-hover:scale-100 group-hover:opacity-50'
                    "
                  ></div>
                  <span class="relative flex items-center z-10">
                    <!-- Pin icon -->
                    <svg
                      class="w-4 h-4"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                      stroke-width="2"
                    >
                      <path
                        v-if="isAlwaysOnTop"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
                        fill="currentColor"
                      />
                      <path
                        v-else
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
                      />
                    </svg>
                  </span>
                </button>
                <ThemeSwitcher />
                <LanguageSwitcher />
              </div>
            </div>
          </div>
        </div>
        <div
          v-if="!isOnboardingRoute && hasOverflowNav"
          class="overflow-hidden transition-all duration-300 ease-in-out"
          :style="{ maxHeight: navExpanded ? `${overflowNavHeight}px` : '0px' }"
        >
          <div
            ref="overflowNavContent"
            class="flex flex-wrap items-start content-start gap-1 whitespace-nowrap py-1"
          >
            <template v-for="item in overflowNavItems" :key="`overflow-${item.id}`">
              <div v-if="item.id === 'management'" class="relative flex items-center">
                <router-link
                  :to="item.to"
                  class="relative px-2.5 py-1.5 rounded-lg group overflow-hidden transition-all duration-300"
                  :class="navLinkClass(item)"
                  @click="handleOverflowNavClick"
                >
                  <div
                    class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                    :class="navLinkBackgroundClass(item)"
                  ></div>
                  <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        v-for="(path, index) in item.iconPaths"
                        :key="`${item.id}-overflow-icon-${index}`"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        :d="path"
                      ></path>
                    </svg>
                    <AnimatedText>{{ item.label }}</AnimatedText>
                  </span>
                </router-link>
                <div
                  v-if="store.sceneryManagerHintVisible && store.sceneryManagerHintMessageKey"
                  class="absolute left-1/2 top-full -translate-x-1/2 mt-2 z-50"
                >
                  <div
                    class="relative min-w-[240px] max-w-[340px] w-max bg-cyan-50 dark:bg-cyan-900/60 border border-cyan-200 dark:border-cyan-700 text-cyan-900 dark:text-cyan-100 text-xs px-3 py-2 rounded-lg shadow-lg flex items-start gap-2"
                  >
                    <div
                      class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-cyan-50 dark:bg-cyan-900/60 border-l border-t border-cyan-200 dark:border-cyan-700 rotate-45"
                    ></div>
                    <span class="leading-4">{{ $t(store.sceneryManagerHintMessageKey) }}</span>
                    <button
                      class="ml-1 text-cyan-700/80 dark:text-cyan-200/80 hover:text-cyan-900 dark:hover:text-white"
                      @click="store.dismissSceneryManagerHint()"
                    >
                      <svg
                        class="w-3.5 h-3.5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M6 18L18 6M6 6l12 12"
                        />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>

              <router-link
                v-else
                :ref="item.id === 'log-analysis' ? setLogAnalysisLinkRef : undefined"
                :to="item.to"
                class="relative px-2.5 py-1.5 rounded-lg group overflow-hidden transition-all duration-300"
                :class="navLinkClass(item)"
                @click="handleOverflowNavClick"
              >
                <div
                  class="absolute inset-0 bg-blue-50 dark:bg-white/10 rounded-lg transition-all duration-300 transform origin-left"
                  :class="navLinkBackgroundClass(item)"
                ></div>
                <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      v-for="(path, index) in item.iconPaths"
                      :key="`${item.id}-overflow-icon-${index}`"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      :d="path"
                    ></path>
                  </svg>
                  <AnimatedText>{{ item.label }}</AnimatedText>
                </span>
              </router-link>
            </template>
          </div>
        </div>

        <div class="nav-measurements" aria-hidden="true">
          <div class="flex items-center gap-1 whitespace-nowrap">
            <div
              v-for="item in allNavItems"
              :key="`measure-${item.id}`"
              :ref="(el) => setNavMeasureRef(item.id, el)"
            >
              <div class="relative px-2.5 py-1.5 rounded-lg text-gray-600 dark:text-gray-400">
                <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      v-for="(path, index) in item.iconPaths"
                      :key="`${item.id}-measure-icon-${index}`"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      :d="path"
                    ></path>
                  </svg>
                  <span>{{ item.label }}</span>
                </span>
              </div>
            </div>

            <div :ref="(el) => setNavMeasureRef('more', el)">
              <div class="relative px-2.5 py-1.5 rounded-lg text-gray-600 dark:text-gray-400">
                <span class="relative flex items-center space-x-1 text-sm font-medium z-10">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 9l-7 7-7-7"
                    />
                  </svg>
                  <span>{{ moreNavLabel }}</span>
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </nav>

    <!-- Main Content -->
    <main
      :class="[
        'main-content',
        'flex-1',
        'min-h-0',
        'overflow-hidden',
        { 'hide-scrollbar': $route.path === '/' },
      ]"
      :style="{ paddingTop: mainContentPaddingTop }"
    >
      <div class="h-full overflow-y-auto">
        <router-view v-slot="{ Component }">
          <transition :name="transitionName" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </div>
    </main>

    <!-- Global Components -->
    <ToastNotification />
    <ErrorModal />
    <ConfirmModal />
    <ContextMenu />
    <SponsorModal :show="showSponsor" @close="showSponsor = false" />
    <IssueUpdateModal />
    <UpdateChangelogModal />
    <FeedbackModal
      v-model:show="feedbackStore.showSubmitModal"
      @dirty-change="handleFeedbackDirtyChange"
      @submitted="handleFeedbackSubmitted"
    />
    <AddonUpdateDrawer
      v-model:show="addonDrawerVisible"
      :tasks="addonUpdateDrawerStore.tasks"
      :active-task-key="addonUpdateDrawerStore.activeTaskKey"
      @select-task="addonUpdateDrawerStore.selectTask"
      @updated="handleGlobalAddonUpdated"
    />
    <CommandPalette ref="commandPaletteRef" />

    <!-- Log Analysis First-time Hint -->
    <Teleport to="body">
      <Transition name="hint-fade">
        <div
          v-if="store.logAnalysisHintVisible"
          class="fixed z-50 pointer-events-none"
          :style="{
            top: hintPosition.top + 'px',
            left: hintPosition.left + 'px',
            transform: 'translateX(-50%)',
          }"
        >
          <!-- Arrow pointing up -->
          <div class="flex flex-col items-center">
            <svg
              width="16"
              height="10"
              viewBox="0 0 16 10"
              class="text-blue-600 dark:text-blue-500 flex-shrink-0"
            >
              <path d="M8 0 L16 10 L0 10 Z" fill="currentColor" />
            </svg>
            <!-- Bubble -->
            <div
              class="pointer-events-auto bg-blue-600 dark:bg-blue-500 text-white rounded-xl px-4 py-3 shadow-xl max-w-[200px] text-center"
            >
              <p class="text-xs font-medium leading-relaxed">{{ $t('logAnalysis.hintText') }}</p>
              <button
                class="mt-2 text-xs text-blue-100 hover:text-white underline underline-offset-2 transition-colors"
                @click="store.dismissLogAnalysisHint()"
              >
                {{ $t('common.gotIt') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { useUpdateStore } from '@/stores/update'
import { useSceneryStore } from '@/stores/scenery'
import { useModalStore } from '@/stores/modal'
import { useIssueTrackerStore } from '@/stores/issueTracker'
import { useFeedbackStore } from '@/stores/feedback'
import { useManagementStore } from '@/stores/management'
import { useAddonUpdateDrawerStore } from '@/stores/addonUpdateDrawer'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { syncLocaleToBackend } from '@/i18n'
import { logBasic, logDebug, logError } from '@/services/logger'
import ToastNotification from '@/components/ToastNotification.vue'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'
import ThemeSwitcher from '@/components/ThemeSwitcher.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import ErrorModal from '@/components/ErrorModal.vue'
import ConfirmModal from '@/components/ConfirmModal.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import SponsorModal from '@/components/SponsorModal.vue'
import IssueUpdateModal from '@/components/IssueUpdateModal.vue'
import UpdateChangelogModal from '@/components/UpdateChangelogModal.vue'
import FeedbackModal from '@/components/FeedbackModal.vue'
import AddonUpdateDrawer from '@/components/AddonUpdateDrawer.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import { registerShortcut } from '@/composables/useKeyboardShortcuts'

const { t, locale } = useI18n()
const store = useAppStore()
const updateStore = useUpdateStore()
const sceneryStore = useSceneryStore()
const modalStore = useModalStore()
const issueTrackerStore = useIssueTrackerStore()
const feedbackStore = useFeedbackStore()
const managementStore = useManagementStore()
const addonUpdateDrawerStore = useAddonUpdateDrawerStore()
const router = useRouter()
const route = useRoute()
const isOnboardingRoute = computed(() => route.path === '/onboarding')
const commandPaletteRef = ref<InstanceType<typeof CommandPalette> | null>(null)
const addonDrawerVisible = computed({
  get: () => addonUpdateDrawerStore.show,
  set: (visible: boolean) => {
    if (visible) {
      addonUpdateDrawerStore.show = true
      return
    }
    addonUpdateDrawerStore.clearTasks()
  },
})

// Always on top state
const isAlwaysOnTop = ref(false)
const showSponsor = ref(false)

// Nav expand state
type NavId =
  | 'home'
  | 'management'
  | 'screenshots'
  | 'log-analysis'
  | 'activity'
  | 'disk-usage'
  | 'presets'
  | 'csl'
  | 'settings'
type NavMeasureId = NavId | 'more'
type ElementRefTarget = Element | { $el?: Element | null } | null

interface NavItem {
  id: NavId
  to: string
  label: string
  active: boolean
  iconPaths: string[]
}

const NAV_ORDER: NavId[] = [
  'home',
  'management',
  'screenshots',
  'log-analysis',
  'activity',
  'disk-usage',
  'presets',
  'csl',
  'settings',
]

const NAV_GAP = 4
const PRIMARY_NAV_HEIGHT_PX = 40

const NAV_ICON_PATHS: Record<NavId, string[]> = {
  home: ['M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4'],
  management: [
    'M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10',
  ],
  screenshots: [
    'M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z',
    'M15 13a3 3 0 11-6 0 3 3 0 016 0z',
  ],
  'log-analysis': [
    'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
  ],
  activity: ['M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z'],
  'disk-usage': [
    'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4',
  ],
  presets: [
    'M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4',
  ],
  csl: ['M12 4l7 4-7 4-7-4 7-4zm7 4v8l-7 4m7-12l-7 4m-7-4v8l7 4m-7-12l7 4'],
  settings: [
    'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z',
    'M15 12a3 3 0 11-6 0 3 3 0 016 0z',
  ],
}

const navExpanded = ref(false)
const primaryNavViewport = ref<HTMLElement | null>(null)
const primaryNavActions = ref<HTMLElement | null>(null)
const overflowNavContent = ref<HTMLElement | null>(null)
const primaryVisibleNavIds = ref<NavId[]>([])
const overflowNavIds = ref<NavId[]>([])
const navMeasureRefs: Partial<Record<NavMeasureId, HTMLElement | null>> = {}
let navLayoutObserver: ResizeObserver | null = null
let navLayoutFrame = 0
let overflowNavHeightFrame = 0
let hintPositionFrame = 0
const overflowNavHeight = ref(PRIMARY_NAV_HEIGHT_PX)
const moreNavLabel = computed(() =>
  navExpanded.value ? t('common.collapseNav') : t('common.expandNav'),
)
const primaryVisibleNavIdSet = computed(() => new Set(primaryVisibleNavIds.value))
const overflowNavIdSet = computed(() => new Set(overflowNavIds.value))
const hasOverflowNav = computed(() => overflowNavIds.value.length > 0)
const allNavItems = computed<NavItem[]>(() => [
  {
    id: 'home',
    to: '/',
    label: t('common.home'),
    active: route.path === '/',
    iconPaths: NAV_ICON_PATHS.home,
  },
  {
    id: 'management',
    to: '/management',
    label: t('management.navTitle'),
    active: route.path.startsWith('/management'),
    iconPaths: NAV_ICON_PATHS.management,
  },
  {
    id: 'screenshots',
    to: '/screenshots',
    label: t('screenshot.navTitle'),
    active: route.path === '/screenshots',
    iconPaths: NAV_ICON_PATHS.screenshots,
  },
  {
    id: 'log-analysis',
    to: '/log-analysis',
    label: t('logAnalysis.navTitle'),
    active: route.path === '/log-analysis',
    iconPaths: NAV_ICON_PATHS['log-analysis'],
  },
  {
    id: 'activity',
    to: '/activity',
    label: t('activityLog.navTitle'),
    active: route.path === '/activity',
    iconPaths: NAV_ICON_PATHS.activity,
  },
  {
    id: 'disk-usage',
    to: '/disk-usage',
    label: t('diskUsage.navTitle'),
    active: route.path === '/disk-usage',
    iconPaths: NAV_ICON_PATHS['disk-usage'],
  },
  {
    id: 'presets',
    to: '/presets',
    label: t('presets.navTitle'),
    active: route.path === '/presets',
    iconPaths: NAV_ICON_PATHS.presets,
  },
  {
    id: 'csl',
    to: '/csl',
    label: t('csl.navTitle'),
    active: route.path === '/csl',
    iconPaths: NAV_ICON_PATHS.csl,
  },
  {
    id: 'settings',
    to: '/settings',
    label: t('common.settings'),
    active: route.path === '/settings',
    iconPaths: NAV_ICON_PATHS.settings,
  },
])
const primaryVisibleNavItems = computed(() =>
  allNavItems.value.filter((item) => primaryVisibleNavIdSet.value.has(item.id)),
)
const overflowNavItems = computed(() =>
  allNavItems.value.filter((item) => overflowNavIdSet.value.has(item.id)),
)
const mainContentPaddingTop = computed(() => {
  if (!navExpanded.value || !hasOverflowNav.value) {
    return `${PRIMARY_NAV_HEIGHT_PX}px`
  }

  return `${PRIMARY_NAV_HEIGHT_PX + overflowNavHeight.value}px`
})

// Log analysis hint
const logAnalysisLink = ref<HTMLElement | null>(null)
const hintPosition = ref({ top: 0, left: 0 })

function resolveHTMLElement(target: ElementRefTarget) {
  if (target instanceof HTMLElement) {
    return target
  }

  const element = target && '$el' in target ? target.$el : null
  return element instanceof HTMLElement ? element : null
}

function navLinkClass(item: NavItem) {
  return item.active
    ? 'text-blue-600 dark:text-white'
    : 'text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-white'
}

function navLinkBackgroundClass(item: NavItem) {
  return item.active
    ? 'scale-x-100 opacity-100'
    : 'scale-x-0 opacity-0 group-hover:scale-x-100 group-hover:opacity-50'
}

function setLogAnalysisLinkRef(target: ElementRefTarget) {
  logAnalysisLink.value = resolveHTMLElement(target)
}

function setNavMeasureRef(id: NavMeasureId, target: Element | null) {
  navMeasureRefs[id] = target instanceof HTMLElement ? target : null
}

function getNavMeasureWidth(id: NavMeasureId) {
  const width = navMeasureRefs[id]?.getBoundingClientRect().width ?? 0
  return Math.ceil(width)
}

function updatePrimaryNavLayout() {
  if (isOnboardingRoute.value) {
    primaryVisibleNavIds.value = []
    overflowNavIds.value = []
    return
  }

  const viewportWidth = primaryNavViewport.value?.clientWidth ?? 0
  const actionsWidth = Math.ceil(primaryNavActions.value?.getBoundingClientRect().width ?? 0)
  const moreWidth = getNavMeasureWidth('more')
  const itemWidths = NAV_ORDER.map((id) => ({ id, width: getNavMeasureWidth(id) }))

  if (viewportWidth <= 0 || moreWidth <= 0 || itemWidths.some((item) => item.width <= 0)) {
    return
  }

  const availableWidth = Math.max(0, viewportWidth - actionsWidth)
  let usedWidth = 0
  let visibleCount = 0

  for (let index = 0; index < itemWidths.length; index += 1) {
    const itemWidth = itemWidths[index].width
    const nextUsedWidth = usedWidth + (visibleCount > 0 ? NAV_GAP : 0) + itemWidth
    const hasRemainingItems = index < itemWidths.length - 1
    const reservedMoreWidth = hasRemainingItems ? NAV_GAP + moreWidth : 0

    if (nextUsedWidth + reservedMoreWidth > availableWidth) {
      break
    }

    usedWidth = nextUsedWidth
    visibleCount += 1
  }

  primaryVisibleNavIds.value = NAV_ORDER.slice(0, visibleCount)
  overflowNavIds.value = NAV_ORDER.slice(visibleCount)
}

function schedulePrimaryNavLayout() {
  if (navLayoutFrame) {
    cancelAnimationFrame(navLayoutFrame)
  }

  navLayoutFrame = requestAnimationFrame(() => {
    navLayoutFrame = 0
    updatePrimaryNavLayout()
    scheduleOverflowNavHeightUpdate()
  })
}

function updateOverflowNavHeight() {
  if (!hasOverflowNav.value) {
    overflowNavHeight.value = 0
    return
  }

  const height = Math.ceil(overflowNavContent.value?.getBoundingClientRect().height ?? 0)
  overflowNavHeight.value = Math.max(PRIMARY_NAV_HEIGHT_PX, height)
}

function scheduleOverflowNavHeightUpdate() {
  if (overflowNavHeightFrame) {
    cancelAnimationFrame(overflowNavHeightFrame)
  }

  overflowNavHeightFrame = requestAnimationFrame(() => {
    overflowNavHeightFrame = 0
    updateOverflowNavHeight()
  })
}

function updateHintPosition() {
  if (!store.logAnalysisHintVisible) {
    return
  }

  const rect = logAnalysisLink.value?.getBoundingClientRect()
  if (!rect) {
    return
  }

  hintPosition.value = {
    top: rect.bottom + 10,
    left: rect.left + rect.width / 2,
  }
}

function scheduleHintPositionUpdate() {
  if (hintPositionFrame) {
    cancelAnimationFrame(hintPositionFrame)
  }

  hintPositionFrame = requestAnimationFrame(() => {
    hintPositionFrame = 0
    updateHintPosition()
  })
}

function reconnectNavLayoutObserver() {
  navLayoutObserver?.disconnect()

  if (typeof ResizeObserver === 'undefined') {
    schedulePrimaryNavLayout()
    scheduleHintPositionUpdate()
    return
  }

  navLayoutObserver = new ResizeObserver(() => {
    schedulePrimaryNavLayout()
    scheduleOverflowNavHeightUpdate()
    scheduleHintPositionUpdate()
  })
  ;[primaryNavViewport.value, primaryNavActions.value, overflowNavContent.value].forEach(
    (element) => {
      if (element) {
        navLayoutObserver?.observe(element)
      }
    },
  )
}

function handleOverflowNavClick() {
  navExpanded.value = false
}

async function toggleAlwaysOnTop() {
  try {
    const appWindow = getCurrentWindow()
    isAlwaysOnTop.value = !isAlwaysOnTop.value
    await appWindow.setAlwaysOnTop(isAlwaysOnTop.value)
  } catch (error) {
    logError(`Failed to toggle always on top: ${error}`, 'app')
  }
}

// Route order for determining transition direction
const routeOrder: Record<string, number> = {
  '/': 0,
  '/management': 1,
  '/management/liveries': 1,
  '/presets': 1.5,
  '/screenshots': 2,
  '/log-analysis': 3,
  '/activity': 3.5,
  '/disk-usage': 4,
  '/feedback': 5,
  '/settings': 6,
  '/onboarding': -1,
}

// Track transition direction based on route navigation
const transitionName = ref('page-right')

// Watch route changes to determine transition direction
watch(
  () => route.path,
  (newPath, oldPath) => {
    const newOrder = routeOrder[newPath] ?? 0
    const oldOrder = routeOrder[oldPath] ?? 0
    // Going to higher index (right in nav) = slide left, going to lower index = slide right
    transitionName.value = newOrder > oldOrder ? 'page-left' : 'page-right'
  },
)

watch(
  () => feedbackStore.showSubmitModal,
  (visible) => {
    if (!visible) {
      feedbackStore.setModalDirty(false)
    }
  },
)

watch(
  [() => route.fullPath, () => locale.value, () => navExpanded.value],
  () => {
    schedulePrimaryNavLayout()
    scheduleOverflowNavHeightUpdate()
    scheduleHintPositionUpdate()
  },
  { flush: 'post' },
)

watch(
  hasOverflowNav,
  (hasOverflow) => {
    if (!hasOverflow && navExpanded.value) {
      navExpanded.value = false
    }
    reconnectNavLayoutObserver()
    scheduleOverflowNavHeightUpdate()
  },
  { flush: 'post' },
)

watch(
  () => store.logAnalysisHintVisible,
  () => {
    scheduleHintPositionUpdate()
  },
  { flush: 'post' },
)

function handleFeedbackDirtyChange(isDirty: boolean) {
  feedbackStore.setModalDirty(isDirty)
}

async function handleFeedbackSubmitted(issueNumber: number) {
  feedbackStore.setModalDirty(false)
  await router.push({
    path: '/feedback',
    query: {
      issue: String(issueNumber),
    },
  })
}

async function handleFeedbackClick() {
  await issueTrackerStore.initStore()
  if (issueTrackerStore.hasSubmittedFeedback) {
    await router.push('/feedback')
    return
  }

  feedbackStore.openSubmitModal()
}

async function handleGlobalAddonUpdated() {
  const refreshJobs: Promise<unknown>[] = [
    managementStore.loadAircraft(),
    managementStore.loadPlugins(),
  ]

  if (route.path.startsWith('/management') && String(route.query.tab || '') === 'scenery') {
    refreshJobs.push(sceneryStore.loadData())
  }

  await Promise.allSettled(refreshJobs)
}

onMounted(async () => {
  reconnectNavLayoutObserver()
  schedulePrimaryNavLayout()
  scheduleOverflowNavHeightUpdate()
  scheduleHintPositionUpdate()

  // Log app startup (basic level - always logged)
  logBasic(t('log.appStarted'), 'app')
  logDebug('Loading app store and initializing', 'app')

  // Non-blocking: load X-Plane path
  store
    .loadXplanePath()
    .then(() => {
      logDebug(`X-Plane path loaded: ${store.xplanePath || '(not set)'}`, 'app')
    })
    .catch((e) => {
      logError(`Failed to load X-Plane path: ${e}`, 'app')
    })

  logDebug(`Log level: ${store.logLevel}`, 'app')

  // Detect platform and context menu status at startup (once)
  try {
    logDebug('Detecting platform...', 'app')
    const platform = await invoke<string>('get_platform')
    store.isWindows = platform === 'windows'
    logDebug(`Platform detected: ${platform}`, 'app')

    // Check context menu registration status (Windows only)
    if (store.isWindows) {
      store.isContextMenuRegistered = await invoke<boolean>('is_context_menu_registered')
      logDebug(`Context menu registered: ${store.isContextMenuRegistered}`, 'app')

      // Sync context menu paths if registered (handles exe relocation)
      if (store.isContextMenuRegistered) {
        try {
          const updated = await invoke<boolean>('sync_context_menu_paths')
          if (updated) {
            logDebug('Context menu paths synced to current location', 'app')
          }
        } catch (error) {
          logError(`Failed to sync context menu paths: ${error}`, 'app')
        }
      }
    }
  } catch (error) {
    logError(`Failed to detect platform: ${error}`, 'app')
  }

  // Non-blocking sync locale to backend (moved from i18n module top-level)
  syncLocaleToBackend()

  // Register keyboard shortcuts
  registerShortcut({
    id: 'command-palette-open',
    keys: 'ctrl+shift+p',
    label: t('commandPalette.title'),
    category: t('commandPalette.categoryGlobal'),
    action: () => commandPaletteRef.value?.toggle(),
  })
  registerShortcut({
    id: 'nav-install',
    keys: 'ctrl+1',
    label: t('common.home'),
    category: t('commandPalette.categoryNav'),
    action: () => router.push('/'),
  })
  registerShortcut({
    id: 'nav-management',
    keys: 'ctrl+2',
    label: t('management.navTitle'),
    category: t('commandPalette.categoryNav'),
    action: () => router.push('/management'),
  })
  registerShortcut({
    id: 'nav-screenshots',
    keys: 'ctrl+3',
    label: t('screenshot.navTitle'),
    category: t('commandPalette.categoryNav'),
    action: () => router.push('/screenshots'),
  })
  registerShortcut({
    id: 'nav-log-analysis',
    keys: 'ctrl+4',
    label: t('logAnalysis.navTitle'),
    category: t('commandPalette.categoryNav'),
    action: () => router.push('/log-analysis'),
  })
  registerShortcut({
    id: 'nav-map',
    keys: 'ctrl+5',
    label: 'Map',
    category: t('commandPalette.categoryNav'),
    action: () => router.push('/map'),
  })
  registerShortcut({
    id: 'nav-settings',
    keys: 'ctrl+,',
    label: t('common.settings'),
    category: t('commandPalette.categoryNav'),
    action: () => router.push('/settings'),
  })
  registerShortcut({
    id: 'command-palette-close',
    keys: 'escape',
    label: t('commandPalette.close'),
    category: t('commandPalette.categoryGlobal'),
    action: () => commandPaletteRef.value?.close(),
    when: () => commandPaletteRef.value?.visible ?? false,
  })

  // Check for updates (non-blocking, delayed to avoid affecting startup performance)
  setTimeout(() => {
    updateStore.checkAndShowPostUpdateChangelog()
  }, 1500)

  setTimeout(() => {
    if (updateStore.autoCheckEnabled) {
      logDebug('Auto-checking for updates...', 'app')
      updateStore.checkForUpdates(false)
    }
  }, 3000) // 3 second delay

  // Check tracked issue updates in the background
  setTimeout(() => {
    issueTrackerStore.checkAllTrackedIssues()
  }, 5000) // 5 second delay

  // Disable context menu and devtools shortcuts in production
  if (import.meta.env.MODE === 'production') {
    // Disable right-click context menu
    document.addEventListener('contextmenu', (e) => {
      e.preventDefault()
      return false
    })

    // Disable F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+U (devtools shortcuts)
    // Note: Ctrl+Shift+P is reserved for command palette — not blocked
    document.addEventListener('keydown', (e) => {
      // F12
      if (e.key === 'F12') {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+I (Inspector)
      if (e.ctrlKey && e.shiftKey && e.key === 'I') {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+J (Console)
      if (e.ctrlKey && e.shiftKey && e.key === 'J') {
        e.preventDefault()
        return false
      }
      // Ctrl+Shift+C (Element picker)
      if (e.ctrlKey && e.shiftKey && e.key === 'C') {
        e.preventDefault()
        return false
      }
      // Ctrl+U (View source)
      if (e.ctrlKey && e.key === 'u') {
        e.preventDefault()
        return false
      }
    })
  }

  // Listen for cli-args events from Rust (emitted by single-instance plugin for subsequent launches)
  try {
    logDebug('Setting up CLI args listener...', 'app')
    await listen<string[]>('cli-args', async (event) => {
      logDebug(`CLI args event received: ${event.payload.join(', ')}`, 'app')
      logBasic(t('log.launchedWithArgs'), 'app')
      if (event.payload && event.payload.length > 0) {
        // Use batch processing to handle multiple file selections
        // (Windows launches separate instances for each file)
        const fileArgs = store.addCliArgsToBatch(event.payload)
        if (fileArgs.length > 0) {
          await router.push('/')
        }
      }
    })
  } catch (error) {
    logError(`Failed to setup CLI args listener: ${error}`, 'app')
  }

  // On first launch, the cli-args event from setup() fires before this listener is ready,
  // so we also poll for CLI args to handle the cold-start case
  try {
    logDebug('Getting CLI args from first launch...', 'app')
    const args = await invoke<string[]>('get_cli_args')
    if (args && args.length > 0) {
      logDebug(`CLI args from first launch: ${args.join(', ')}`, 'app')
      logBasic(t('log.launchedWithArgs'), 'app')
      const fileArgs = store.addCliArgsToBatch(args)
      if (fileArgs.length > 0) {
        await router.push('/')
      }
    }
  } catch (error) {
    logError(`Failed to get CLI args on startup: ${error}`, 'app')
  }

  // Set up window close confirmation for active operations
  try {
    logDebug('Setting up window close handler...', 'app')
    const appWindow = getCurrentWindow()
    await appWindow.onCloseRequested(async (event) => {
      // Check conditions in priority order
      if (store.isInstalling) {
        // Installation in progress - highest priority warning
        event.preventDefault()
        modalStore.showConfirm({
          title: t('modal.installInProgressTitle'),
          message: t('modal.installInProgressMessage'),
          warning: t('modal.installInProgressWarning'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'danger',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      } else if (managementStore.isExecutingUpdate) {
        // Addon update installation in progress
        event.preventDefault()
        modalStore.showConfirm({
          title: t('management.updateCloseRunningTitle'),
          message: t('management.updateCloseRunningMessage'),
          warning: t('management.updateCloseRunningWarning'),
          confirmText: t('management.updateCloseRunningConfirm'),
          cancelText: t('management.updateCloseRunningCancel'),
          type: 'warning',
          onConfirm: async () => {
            try {
              await invoke('cancel_installation')
            } catch {
              // Ignore cancellation errors; window close will terminate remaining work.
            } finally {
              await appWindow.destroy()
            }
          },
          onCancel: () => {},
        })
      } else if (updateStore.isDownloading) {
        // App update download in progress
        event.preventDefault()
        modalStore.showConfirm({
          title: t('update.downloadInProgressTitle'),
          message: t('update.downloadInProgressMessage'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'warning',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      } else if (store.isLibraryLinkSubmitting) {
        // Library link submission in progress
        event.preventDefault()
        modalStore.showError(t('sceneryManager.submissionInProgressCloseBlocked'))
      } else if (feedbackStore.showSubmitModal && feedbackStore.feedbackModalDirty) {
        event.preventDefault()
        modalStore.showConfirm({
          title: t('feedback.discardTitle'),
          message: t('feedback.closeAppDiscardMessage'),
          warning: t('feedback.discardWarning'),
          confirmText: t('feedback.discardConfirm'),
          cancelText: t('feedback.discardCancel'),
          type: 'warning',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      } else if (store.isConfirmationOpen) {
        // Confirmation modal is open - pending installation
        event.preventDefault()
        modalStore.showConfirm({
          title: t('modal.confirmationOpenTitle'),
          message: t('modal.confirmationOpenMessage'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'warning',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      } else if (sceneryStore.hasChanges) {
        // Unsaved scenery changes
        event.preventDefault()
        modalStore.showConfirm({
          title: t('modal.unsavedSceneryChangesTitle'),
          message: t('modal.unsavedSceneryChangesMessage'),
          warning: t('modal.unsavedSceneryChangesWarning'),
          confirmText: t('modal.closeAnyway'),
          cancelText: t('modal.goBack'),
          type: 'warning',
          onConfirm: async () => await appWindow.destroy(),
          onCancel: () => {},
        })
      }
      // If no conditions match, allow the window to close normally
    })
  } catch (error) {
    logError(`Failed to setup window close listener: ${error}`, 'app')
  }

  logDebug('App.vue onMounted completed', 'app')
})

onBeforeUnmount(() => {
  navLayoutObserver?.disconnect()
  if (navLayoutFrame) {
    cancelAnimationFrame(navLayoutFrame)
  }
  if (overflowNavHeightFrame) {
    cancelAnimationFrame(overflowNavHeightFrame)
  }
  if (hintPositionFrame) {
    cancelAnimationFrame(hintPositionFrame)
  }
})
</script>

<style scoped>
.app-container {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: linear-gradient(135deg, var(--app-bg-from), var(--app-bg-via), var(--app-bg-to));
  background-color: var(--app-bg-from);
}

.hint-fade-enter-active {
  transition:
    opacity 0.3s ease,
    transform 0.3s ease;
}
.hint-fade-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}
.hint-fade-enter-from,
.hint-fade-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-6px);
}

nav {
  transform: translateZ(0);
  will-change: transform;
  backface-visibility: hidden;
}

.nav-measurements {
  position: absolute;
  top: 0;
  left: -9999px;
  height: 0;
  overflow: hidden;
  visibility: hidden;
  pointer-events: none;
}

.main-content {
  flex: 1;
  min-height: 0;
  transition: padding-top 0.3s ease;
  /* Do not reserve scrollbar gutter globally; we will control visual scrollbar per-route */
  scrollbar-gutter: auto;
  /* Allow inner container to manage the actual scrolling */
}

/* Ensure the immediate child scroll container always shows a vertical scrollbar area
   so the scrollbar won't appear/disappear during route transitions. */
.main-content > div {
  height: 100%;
  overflow-y: auto;
}

/* Hide visual scrollbar for Home page while keeping scroll functionality */
.hide-scrollbar > div {
  overflow-y: auto;
  /* Firefox */
  scrollbar-width: none;
}
.hide-scrollbar > div::-webkit-scrollbar {
  width: 0;
  height: 0;
  display: none;
}

/* When .no-scrollbar is applied (Settings route) completely disable scrolling
   and remove any reserved scrollbar gutter so there's no scrollbar area on all platforms. */
.no-scrollbar {
  /* Reset any reserved gutter from the global setting */
  scrollbar-gutter: auto;
}
.no-scrollbar > div {
  /* Completely disable scrolling and hide scrollbars visually */
  overflow: hidden !important;
  /* Firefox */
  scrollbar-width: none;
}
.no-scrollbar > div::-webkit-scrollbar {
  /* Chromium-based */
  width: 0;
  height: 0;
  display: none;
}

/* Page transitions - left direction (going to higher index route) */
.page-left-enter-active,
.page-left-leave-active,
.page-right-enter-active,
.page-right-leave-active {
  transition: all 0.2s ease;
  will-change: transform, opacity;
  backface-visibility: hidden;
}

/* Going left (e.g., Home -> Management -> Settings) */
.page-left-enter-from {
  opacity: 0;
  transform: translateX(15px);
}

.page-left-leave-to {
  opacity: 0;
  transform: translateX(-15px);
}

/* Going right (e.g., Settings -> Management -> Home) */
.page-right-enter-from {
  opacity: 0;
  transform: translateX(-15px);
}

.page-right-leave-to {
  opacity: 0;
  transform: translateX(15px);
}

/* Navigation animations */
.nav-link {
  position: relative;
  overflow: hidden;
}

.nav-link::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
  transition: left 0.5s;
}

.nav-link:hover::before {
  left: 100%;
}

/* Sponsor heart: intermittent red pulse every 4s */
@keyframes heartbeat {
  0%,
  100% {
    transform: scale(1);
    color: inherit;
  }
  6% {
    transform: scale(1.25);
    color: #ef4444;
  }
  12% {
    transform: scale(1);
    color: #ef4444;
  }
  16% {
    transform: scale(1.2);
    color: #ef4444;
  }
  22% {
    transform: scale(1);
    color: inherit;
  }
}

.sponsor-heartbeat {
  animation: heartbeat 10s ease-in-out infinite;
}
</style>
