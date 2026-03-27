<template>
  <div class="home-view h-full flex flex-col p-4 animate-cockpit-power-on relative overflow-hidden select-none bg-aviation-dark text-aviation-green font-mono">
    <!-- CRT Overlay Effect -->
    <div class="absolute inset-0 pointer-events-none z-50 opacity-[0.03] bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.25)_50%),linear-gradient(90deg,rgba(255,0,0,0.06),rgba(0,255,0,0.02),rgba(0,0,255,0.06))] bg-[length:100%_2px,3px_100%]"></div>
    
    <!-- Top Status Bar (PFD Style) -->
    <div class="flex-shrink-0 h-10 bezel-container mb-3 flex items-center px-4 justify-between bg-aviation-panel border-b-aviation-cyan/30">
      <div class="flex items-center gap-6">
        <div class="text-[10px] uppercase tracking-tighter opacity-70">Flight Systems Manager</div>
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full" :class="isXPlaneRunning ? 'bg-aviation-green shadow-[0_0_8px_#00ff41]' : 'bg-gray-700'"></div>
          <span class="text-xs font-black uppercase tracking-widest">{{ isXPlaneRunning ? 'XP-CONNECTED' : 'XP-DISCONNECTED' }}</span>
        </div>
      </div>

      <!-- Center Warning Banner -->
      <div v-if="updateStore.showUpdateBanner" class="bg-aviation-amber/20 border border-aviation-amber px-4 py-0.5 rounded animate-pulse">
        <span class="text-[10px] font-black text-aviation-amber uppercase">System Update Available: v{{ updateStore.updateInfo?.latestVersion }}</span>
      </div>
      <div v-else-if="!store.xplanePath" class="bg-aviation-red/20 border border-aviation-red px-4 py-0.5 rounded">
        <span class="text-[10px] font-black text-aviation-red uppercase underline">Critical: X-Plane Path Not Configured</span>
      </div>

      <div class="text-[10px] uppercase opacity-70">
        {{ new Date().toLocaleTimeString([], { hour12: false }) }} Z
      </div>
    </div>

    <!-- Main Instrument Panel Grid -->
    <div class="flex-1 min-h-0 grid grid-cols-12 gap-3 overflow-hidden">
<!-- Left Column: Engine Gauges (Stats) -->
      <div class="col-span-3 flex flex-col gap-3 h-full">
        <div class="bezel-container flex-1 p-4 lcd-screen flex flex-col justify-between">
          <div class="text-[10px] font-black uppercase text-aviation-cyan mb-4 border-b border-aviation-cyan/20 pb-1">Resources Inventory</div>
          
          <div class="space-y-6">
            <!-- Aircraft Gauge -->
            <div class="flex items-center justify-between group">
              <div class="relative w-12 h-12">
                <svg class="w-full h-full -rotate-90">
                  <circle cx="24" cy="24" r="20" fill="none" stroke="currentColor" stroke-width="2" class="text-gray-800" />
                  <circle
cx="24" cy="24" r="20" fill="none" stroke="currentColor" stroke-width="2" 
                    stroke-dasharray="125.6" 
                    :stroke-dashoffset="125.6 * (1 - Math.min(managementStore.aircraftTotalCount / 100, 1))"
                    class="text-aviation-green transition-all duration-1000" />
                </svg>
                <div class="absolute inset-0 flex items-center justify-center text-[10px] font-black">{{ managementStore.aircraftTotalCount }}</div>
              </div>
              <div class="flex-1 ml-3">
                <div class="text-[9px] uppercase opacity-60">Aircraft</div>
                <div class="text-xs font-black group-hover:text-white transition-colors uppercase">Fleet Active</div>
              </div>
            </div>

            <!-- Plugins Gauge -->
            <div class="flex items-center justify-between group">
              <div class="relative w-12 h-12">
                <svg class="w-full h-full -rotate-90">
                  <circle cx="24" cy="24" r="20" fill="none" stroke="currentColor" stroke-width="2" class="text-gray-800" />
                  <circle
cx="24" cy="24" r="20" fill="none" stroke="currentColor" stroke-width="2" 
                    stroke-dasharray="125.6" 
                    :stroke-dashoffset="125.6 * (1 - Math.min(managementStore.pluginsTotalCount / 50, 1))"
                    class="text-aviation-cyan transition-all duration-1000" />
                </svg>
                <div class="absolute inset-0 flex items-center justify-center text-[10px] font-black">{{ managementStore.pluginsTotalCount }}</div>
              </div>
              <div class="flex-1 ml-3">
                <div class="text-[9px] uppercase opacity-60">Plugins</div>
                <div class="text-xs font-black group-hover:text-white transition-colors uppercase">Modules Loaded</div>
              </div>
            </div>

            <!-- Scenery Tape -->
            <div class="space-y-1">
              <div class="flex justify-between text-[9px] uppercase">
                <span>Scenery Packs</span>
                <span class="text-aviation-green">{{ sceneryStore.totalCount }}</span>
              </div>
              <div class="h-2 bg-gray-900 border border-aviation-green/20 overflow-hidden">
                <div
class="h-full bg-aviation-green transition-all duration-1000 shadow-[0_0_10px_#00ff41]" 
                  :style="{ width: `${Math.min(sceneryStore.totalCount / 5, 100)}%` }"></div>
              </div>
            </div>

            <!-- Navdata Tape -->
            <div class="space-y-1">
              <div class="flex justify-between text-[9px] uppercase">
                <span>Navigation Data</span>
                <span class="text-aviation-amber">{{ managementStore.navdataTotalCount }}</span>
              </div>
              <div class="h-2 bg-gray-900 border border-aviation-amber/20 overflow-hidden">
                <div
class="h-full bg-aviation-amber transition-all duration-1000 shadow-[0_0_10px_#ffb000]" 
                  :style="{ width: `${Math.min(managementStore.navdataTotalCount * 10, 100)}%` }"></div>
              </div>
            </div>
          </div>

          <!-- Bottom Summary -->
          <div class="mt-auto pt-4 border-t border-aviation-green/10 text-[9px] space-y-1">
            <div class="flex justify-between uppercase">
              <span class="opacity-50">Status:</span>
              <span class="text-aviation-green">Nominal</span>
            </div>
            <div class="flex justify-between uppercase">
              <span class="opacity-50">Ready:</span>
              <span>100.0%</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Center Column: MFD (Main Controls) -->
      <div class="col-span-6 flex flex-col gap-3">
        <!-- Master Display -->
        <div
class="bezel-container flex-1 lcd-screen relative flex flex-col items-center justify-center overflow-hidden p-6"
          :class="{'animate-pulse border-aviation-cyan shadow-[inset_0_0_30px_rgba(0,243,255,0.1)]': store.isAnalyzing}"
          @click="handleDropZoneClick"
        >
          <!-- Background Radar Effect -->
          <div class="absolute inset-0 pointer-events-none opacity-10">
            <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] border border-aviation-green rounded-full"></div>
            <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[300px] h-[300px] border border-aviation-green rounded-full opacity-50"></div>
            <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[100px] h-[100px] border border-aviation-green rounded-full opacity-20"></div>
            <div class="absolute top-1/2 left-1/2 w-1/2 h-px bg-aviation-green origin-left animate-radar-sweep"></div>
          </div>

          <!-- Master Start Button -->
          <button 
            :disabled="isLaunchingXPlane || isXPlaneRunning"
            class="relative z-10 w-40 h-40 rounded-full border-4 border-aviation-panel shadow-[0_0_20px_rgba(0,0,0,0.5),inset_0_2px_4px_rgba(255,255,255,0.1)] transition-all active:scale-95 group overflow-hidden"
            :class="isXPlaneRunning ? 'cursor-default' : 'hover:shadow-[0_0_40px_rgba(0,243,255,0.3)]'"
            @click.stop="handleLaunchXPlane"
          >
            <div class="absolute inset-0 bg-gradient-to-b from-gray-700 to-aviation-panel"></div>
            <div class="absolute inset-1 rounded-full border-2 border-dashed border-aviation-cyan/30 opacity-50 group-hover:animate-spin-slow"></div>
            
            <div class="relative flex flex-col items-center justify-center h-full gap-2">
              <svg v-if="isLaunchingXPlane" class="w-10 h-10 animate-spin text-aviation-cyan" viewBox="0 0 24 24" fill="none">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <svg
v-else class="w-12 h-12 transition-transform duration-500" 
                :class="[isXPlaneRunning ? 'text-aviation-green scale-110' : 'text-aviation-cyan group-hover:scale-110']"
                fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path v-if="isXPlaneRunning" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              <div class="text-[10px] font-black uppercase tracking-[0.2em]" :class="isXPlaneRunning ? 'text-aviation-green' : 'text-aviation-cyan'">
                {{ isXPlaneRunning ? 'ONLINE' : 'MASTER START' }}
              </div>
            </div>
          </button>

          <!-- Drop Hint -->
          <div class="mt-8 text-center relative z-10 transition-opacity duration-500" :class="{'opacity-30': isLaunchingXPlane}">
            <div class="text-sm font-black uppercase tracking-widest text-aviation-green mb-1">{{ $t('dashboard.dropToInstall') }}</div>
            <div class="text-[9px] uppercase opacity-50">{{ $t('home.supportedFormats') }}</div>
          </div>

          <!-- Bottom Quick Actions CDU Style -->
          <div class="absolute bottom-4 left-4 right-4 grid grid-cols-4 gap-2">
            <button class="aviation-button group" @click.stop="handleFixScenery">
              <span class="group-hover:text-aviation-green transition-colors">Fix Scen</span>
            </button>
            <button class="aviation-button group" @click.stop="router.push('/log-analysis')">
              <span class="group-hover:text-aviation-amber transition-colors">Log Anal</span>
            </button>
            <button class="aviation-button group" @click.stop="router.push('/activity')">
              <span class="group-hover:text-aviation-cyan transition-colors">Activity</span>
            </button>
            <button class="aviation-button group" @click.stop="router.push('/management')">
              <span class="group-hover:text-white transition-colors">Manage</span>
            </button>
          </div>
        </div>
      </div>

      <!-- Right Column: Telemetry & Status -->
      <div class="col-span-3 flex flex-col gap-3">
        <!-- Storage Telemetry -->
        <div class="bezel-container h-1/2 p-4 lcd-screen flex flex-col">
          <div class="text-[10px] font-black uppercase text-aviation-amber mb-4 border-b border-aviation-amber/20 pb-1">Storage Telemetry</div>
          
          <div class="flex-1 flex flex-col items-center justify-center">
            <div v-if="diskUsageStore.isScanning" class="animate-pulse flex flex-col items-center">
              <div class="text-[10px] uppercase mb-2">Scanning sectors...</div>
              <div class="w-32 h-1 bg-gray-900 overflow-hidden">
                <div class="h-full bg-aviation-amber animate-loading-bar"></div>
              </div>
            </div>
            <div v-else-if="diskUsageStore.report" class="relative group">
              <DiskUsageChart
                :categories="diskCategories"
                :total-bytes="diskUsageStore.report.totalBytes"
                :size="140"
                is-dark
                radar
              />
              <!-- Raw readout overlay on hover -->
              <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity bg-aviation-dark/80 backdrop-blur-sm pointer-events-none">
                <div class="text-[8px] text-aviation-amber space-y-1">
                  <div v-for="cat in diskUsageStore.report.categories.slice(0, 4)" :key="cat.category" class="flex justify-between w-24">
                    <span>{{ cat.category.toUpperCase() }}:</span>
                    <span>{{ formatSize(cat.totalBytes) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Annunciator Panel -->
        <div class="bezel-container flex-1 p-4 bg-aviation-panel flex flex-col">
          <div class="text-[10px] font-black uppercase text-gray-500 mb-3">Annunciator Panel</div>
          <div class="grid grid-cols-2 gap-1.5 flex-1">
            <div class="annunciator-lamp" :class="store.xplanePath ? 'lamp-green' : 'lamp-red'">Path</div>
            <div class="annunciator-lamp" :class="isXPlaneRunning ? 'lamp-amber' : 'lamp-off'">Running</div>
            <div class="annunciator-lamp" :class="store.isContextMenuRegistered ? 'lamp-green' : 'lamp-off'">Shell</div>
            <div class="annunciator-lamp" :class="updateStore.showUpdateBanner ? 'lamp-amber' : 'lamp-off'">Update</div>
            <div class="annunciator-lamp" :class="sceneryStore.hasChanges ? 'lamp-amber' : 'lamp-off'">Scenery</div>
            <div class="annunciator-lamp" :class="sceneryStore.duplicatesCount > 0 ? 'lamp-red' : 'lamp-off'">Dupl</div>
            <div class="annunciator-lamp" :class="managementStore.aircraftUpdateCount > 0 ? 'lamp-amber' : 'lamp-off'">Fleet</div>
            <div class="annunciator-lamp lamp-green">Sys OK</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Data Link (Activity Stream) -->
    <div class="h-24 bezel-container mt-3 p-3 lcd-screen flex gap-4">
      <div class="flex-shrink-0 flex flex-col justify-center border-r border-aviation-green/20 pr-4">
        <div class="text-[10px] font-black text-aviation-green uppercase tracking-widest">Data Link</div>
        <div class="text-[8px] opacity-50 uppercase">Satellite Comms</div>
      </div>
      <div class="flex-1 overflow-y-auto custom-scrollbar text-[10px] leading-tight space-y-1 py-1">
        <div v-for="entry in activityLogStore.entries.slice(0, 10)" :key="entry.id" class="flex items-start gap-3 group">
          <span class="opacity-40 whitespace-nowrap">[{{ formatRelativeTime(entry.timestamp) }}]</span>
          <span :class="entry.success ? 'text-aviation-green' : 'text-aviation-red'" class="uppercase">
            &gt; {{ entry.operation }}: {{ entry.itemName || entry.itemType }}
          </span>
          <span class="opacity-0 group-hover:opacity-100 transition-opacity text-gray-600">-- SECURED</span>
        </div>
        <div v-if="activityLogStore.entries.length === 0" class="opacity-20 animate-pulse">Waiting for system telemetry...</div>
      </div>
    </div>

    <!-- Drag Overlay (Full Cockpit Alert) -->
    <transition name="fade">
      <div
        v-if="isDragging && !store.isInstalling"
        class="fixed inset-0 z-[100] flex flex-col items-center justify-center bg-aviation-cyan/10 backdrop-blur-md pointer-events-none"
      >
        <div class="bezel-container p-12 bg-aviation-dark/90 border-4 border-aviation-cyan animate-cockpit-alert">
           <div class="w-24 h-24 rounded-full border-4 border-aviation-cyan flex items-center justify-center text-aviation-cyan mb-6 shadow-[0_0_30px_#00f3ff]">
             <svg class="w-12 h-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
               <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M12 4v16m8-8H4" />
             </svg>
           </div>
           <div class="text-center">
             <h2 class="text-3xl font-black text-aviation-cyan tracking-[0.2em] uppercase mb-2">Ready to Load</h2>
             <p class="text-aviation-cyan/60 font-black uppercase text-xs">Awaiting data injection</p>
           </div>
        </div>
      </div>
    </transition>

    <!-- Progress Overlays (Retain existing functional components) -->
    <transition name="fade" mode="out-in">
      <AnalyzingOverlay v-if="store.isAnalyzing" key="analyzing" />

      <InstallProgressOverlay
        v-else-if="store.isInstalling || store.showCompletion"
        key="installing"
        :percentage="progressStore.formatted.percentage"
        :task-name="progressStore.formatted.taskName"
        :processed-m-b="progressStore.formatted.processedMB"
        :total-m-b="progressStore.formatted.totalMB"
        :task-progress="progressStore.formatted.taskProgress"
        :tasks="store.installingTasks"
        :current-task-index="progressStore.progress?.currentTaskIndex ?? 0"
        :current-task-percentage="progressStore.formatted.currentTaskPercentage"
        :current-task-processed-m-b="progressStore.formatted.currentTaskProcessedMB"
        :current-task-total-m-b="progressStore.formatted.currentTaskTotalMB"
        :is-complete="store.showCompletion"
        :install-result="store.installResult"
        :active-tasks="progressStore.activeTasks"
        :completed-task-count="progressStore.completedTaskCount"
        :completed-task-ids="progressStore.completedTaskIds"
        @skip="handleSkipTask"
        @cancel="handleCancelInstallation"
        @confirm="handleCompletionConfirm"
      />
    </transition>

    <ConfirmationModal
      v-if="showConfirmation"
      @close="showConfirmation = false"
      @confirm="handleInstall"
    />
    <PasswordModal
      v-if="showPasswordModal"
      :archive-paths="passwordRequiredPaths"
      :error-message="passwordErrorMessage"
      @confirm="handlePasswordSubmit"
      @cancel="handlePasswordCancel"
    />

    <!-- Launch X-Plane Confirmation Dialog -->
    <transition name="fade">
      <div v-if="showLaunchConfirmDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm" @click.self="cancelLaunchDialog">
        <div class="bezel-container max-w-md w-full p-6 animate-cockpit-alert border-aviation-amber">
          <div class="flex items-center gap-4 mb-6 text-aviation-amber">
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
            <h3 class="text-xl font-black uppercase tracking-wider">{{ $t('home.launchConfirmTitle') }}</h3>
          </div>
          <p class="text-xs uppercase leading-relaxed text-aviation-amber/80 mb-8">{{ $t('home.launchConfirmMessage') }}</p>
          <div class="flex gap-3 justify-end">
            <button class="aviation-button border-gray-700" @click="cancelLaunchDialog">{{ $t('common.cancel') }}</button>
            <button class="aviation-button border-aviation-cyan text-aviation-cyan" @click="launchXPlane">{{ $t('home.launchDirectly') }}</button>
            <button class="aviation-button bg-aviation-amber border-aviation-amber text-black" @click="applyAndLaunch">{{ $t('home.applyAndLaunch') }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore, type ConfirmOptions } from '@/stores/modal'
import { useProgressStore } from '@/stores/progress'
import { useUpdateStore } from '@/stores/update'
import { useSceneryStore } from '@/stores/scenery'
import { useManagementStore } from '@/stores/management'
import { useActivityLogStore } from '@/stores/activityLog'
import { useDiskUsageStore } from '@/stores/diskUsage'
import { useLockStore } from '@/stores/lock'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import ConfirmationModal from '@/components/ConfirmationModal.vue'
import PasswordModal from '@/components/PasswordModal.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import UpdateBanner from '@/components/UpdateBanner.vue'
import InstallProgressOverlay from '@/components/InstallProgressOverlay.vue'
import AnalyzingOverlay from '@/components/AnalyzingOverlay.vue'
import DiskUsageChart from '@/components/DiskUsageChart.vue'
import type { AnalysisResult, InstallProgress, InstallResult } from '@/types'
import { AddonType } from '@/types'
import { getErrorMessage } from '@/types'
import { logOperation, logError, logDebug, logBasic } from '@/services/logger'
import { setTrackedTimeout } from '@/utils/timeout'

const { t } = useI18n()
const router = useRouter()

const store = useAppStore()
const toast = useToastStore()
const modal = useModalStore()
const updateStore = useUpdateStore()
const progressStore = useProgressStore()
const sceneryStore = useSceneryStore()
const managementStore = useManagementStore()
const activityLogStore = useActivityLogStore()
const diskUsageStore = useDiskUsageStore()
const lockStore = useLockStore()

const isDragging = ref(false)
const showConfirmation = ref(false)
const showLaunchConfirmDialog = ref(false)
const isLaunchingXPlane = ref(false)
const isXPlaneRunning = ref(false)
let xplaneCheckInterval: number | null = null
const debugDropFlash = ref(false)

// Dashboard data loading state
const isInitialLoading = ref(true)

// Sync confirmation modal state with store for exit confirmation
watch(showConfirmation, (value) => {
  store.setConfirmationOpen(value)
})

// Password modal state
const showPasswordModal = ref(false)
const passwordRequiredPaths = ref<string[]>([])
const pendingAnalysisPaths = ref<string[]>([])
const collectedPasswords = ref<Record<string, string>>({})
const passwordRetryCount = ref(0)
const passwordErrorMessage = ref('')
const MAX_PASSWORD_RETRIES = 3

// Password rate limiting
const passwordAttemptTimestamps = ref<number[]>([])
const MIN_PASSWORD_ATTEMPT_DELAY_MS = 1000 
const PASSWORD_RATE_LIMIT_WINDOW_MS = 10000 
const DEBUG_DROP_FLASH_DURATION_MS = 800 
const DROP_ZONE_CLICK_SUPPRESS_AFTER_FOCUS_MS = 350 
const COMPLETION_ANIMATION_DELAY_MS = 100 
const suppressDropZoneClickUntil = ref(0)
const windowWasBlurred = ref(!document.hasFocus())

const DISK_CATEGORY_KEY_MAP: Record<string, string> = {
  aircraft: 'diskUsage.categoryAircraft',
  plugin: 'diskUsage.categoryPlugins',
  plugins: 'diskUsage.categoryPlugins',
  scenery: 'diskUsage.categoryScenery',
  navdata: 'diskUsage.categoryNavdata',
  screenshot: 'diskUsage.categoryScreenshots',
  screenshots: 'diskUsage.categoryScreenshots',
}

function diskCategoryLabel(category: string): string {
  const key = DISK_CATEGORY_KEY_MAP[category.trim().toLowerCase()]
  return key ? t(key) : category
}

// Disk usage categories for the widget - use aviation colors
const diskCategories = computed(() => {
  if (!diskUsageStore.report) return []
  const colors = ['#00ff41', '#00f3ff', '#ffb000', '#ff3e3e', '#8b5cf6', '#6b7280']
  return diskUsageStore.report.categories.map((cat, i) => ({
    name: diskCategoryLabel(cat.category),
    bytes: cat.totalBytes,
    color: colors[i % colors.length],
  }))
})

// Timer tracking for cleanup on unmount to prevent memory leaks
const activeTimeoutIds = new Set<ReturnType<typeof setTimeout>>()

// Tauri drag-drop event unsubscribe function
let unlistenDragDrop: UnlistenFn | null = null
let unlistenProgress: UnlistenFn | null = null
let unlistenDeletionSkipped: UnlistenFn | null = null

// Watch for pending CLI args changes
watch(
  () => store.pendingCliArgs,
  async (args) => {
    if (args && args.length > 0) {
      if (store.isAnalyzeInProgress || store.isAnalyzing || store.isInstalling) {
        logDebug('Analysis in progress, re-queueing args for later', 'app')
        store.addCliArgsToBatch(args)
        store.clearPendingCliArgs()
        return
      }

      store.isAnalyzeInProgress = true
      logDebug(`Processing pending CLI args from watcher: ${args.join(', ')}`, 'app')
      const argsCopy = [...args]
      store.clearPendingCliArgs()
      try {
        await analyzeFiles(argsCopy)
      } catch (error) {
        logError(`Failed to process CLI args: ${error}`, 'app')
        modal.showError(getErrorMessage(error))
      } finally {
        store.isAnalyzeInProgress = false
      }
    }
  },
)

// Global listeners for drag/drop visual feedback
function onWindowDragOver(e: DragEvent) {
  e.preventDefault()
  if (store.isInstalling) return
  isDragging.value = true
}

function onWindowDragLeave(e: DragEvent) {
  if (store.isInstalling) return
  if (!e.relatedTarget) isDragging.value = false
}

function onWindowDrop(e: DragEvent) {
  e.preventDefault()
  if (store.isInstalling) return
  isDragging.value = false
  debugDropFlash.value = true
  setTrackedTimeout(
    () => (debugDropFlash.value = false),
    DEBUG_DROP_FLASH_DURATION_MS,
    activeTimeoutIds,
  )
}

function onWindowFocus() {
  if (windowWasBlurred.value) {
    suppressDropZoneClickUntil.value = Date.now() + DROP_ZONE_CLICK_SUPPRESS_AFTER_FOCUS_MS
  }
  windowWasBlurred.value = false
}

function onWindowBlur() {
  windowWasBlurred.value = true
  suppressDropZoneClickUntil.value = 0
}

async function handleDropZoneClick() {
  if (store.isInstalling || store.isAnalyzing || store.showCompletion) return

  if (!document.hasFocus() || Date.now() < suppressDropZoneClickUntil.value) {
    logDebug('Ignoring drop-zone click while window is inactive/just activated', 'drag-drop')
    return
  }

  try {
    const selected = await open({
      multiple: true,
      title: t('home.dropFilesHere'),
      filters: [
        {
          name: 'Addon Files',
          extensions: ['zip', '7z', 'rar', 'lua'],
        },
        {
          name: 'All Files',
          extensions: ['*'],
        },
      ],
    })

    const paths = Array.isArray(selected) ? selected : selected ? [selected] : []
    if (paths.length === 0) return

    await analyzeFiles(paths)
  } catch (error) {
    logError(`Failed to open file picker: ${error}`, 'drag-drop')
    modal.showError(getErrorMessage(error))
  }
}

function formatRelativeTime(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)

  if (seconds < 60) return `${seconds}S`
  if (minutes < 60) return `${minutes}M`
  return `${hours}H`
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0B'
  const k = 1024
  const sizes = ['B', 'K', 'M', 'G', 'T']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + sizes[i]
}

onMounted(async () => {
  window.addEventListener('dragover', onWindowDragOver)
  window.addEventListener('dragleave', onWindowDragLeave)
  window.addEventListener('drop', onWindowDrop)
  window.addEventListener('focus', onWindowFocus)
  window.addEventListener('blur', onWindowBlur)

  // Initialize dashboard data
  if (store.xplanePath) {
    Promise.all([
      managementStore.loadAircraft(),
      managementStore.loadPlugins(),
      managementStore.loadNavdata(),
      sceneryStore.loadData(),
      activityLogStore.loadRecent(),
      diskUsageStore.scan(),
    ]).finally(() => {
      isInitialLoading.value = false
    })
  } else {
    isInitialLoading.value = false
  }

  // Use Tauri 2's native drag-drop event
  try {
    const webview = getCurrentWebviewWindow()
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      if (store.isInstalling) return

      if (event.payload.type === 'over') {
        isDragging.value = true
      } else if (event.payload.type === 'leave') {
        isDragging.value = false
      } else if (event.payload.type === 'drop') {
        isDragging.value = false
        debugDropFlash.value = true
        setTrackedTimeout(
          () => (debugDropFlash.value = false),
          DEBUG_DROP_FLASH_DURATION_MS,
          activeTimeoutIds,
        )

        if (store.showCompletion) {
          store.clearInstallResult()
        }

        const paths = event.payload.paths
        if (paths && paths.length > 0) {
          try {
            await analyzeFiles(paths)
          } catch (error) {
            logError(`Failed to analyze dropped files: ${error}`, 'drag-drop')
            modal.showError(getErrorMessage(error))
          }
        }
      }
    })
  } catch (error) {
    logError(`Failed to setup Tauri drag-drop listener: ${error}`, 'drag-drop')
  }

  // Listen for installation progress events
  try {
    unlistenProgress = await listen<InstallProgress>('install-progress', (event) => {
      progressStore.update(event.payload)
    })
  } catch (error) {
    logError(`Failed to setup progress listener: ${error}`, 'install')
  }

  // Listen for source deletion skipped events
  try {
    unlistenDeletionSkipped = await listen<string>('source-deletion-skipped', (event) => {
      const path = event.payload
      toast.info(t('home.sourceDeletionSkipped', { path }))
    })
  } catch (error) {
    logError(`Failed to setup source deletion skipped listener: ${error}`, 'install')
  }

  // Check if X-Plane is running
  const checkXPlaneRunning = async () => {
    try {
      isXPlaneRunning.value = await invoke<boolean>('is_xplane_running')
    } catch (error) {
      logDebug(`Failed to check X-Plane running status: ${error}`, 'app')
    }
  }
  await checkXPlaneRunning()
  xplaneCheckInterval = window.setInterval(checkXPlaneRunning, 3000)
})

onBeforeUnmount(() => {
  window.removeEventListener('dragover', onWindowDragOver)
  window.removeEventListener('dragleave', onWindowDragLeave)
  window.removeEventListener('drop', onWindowDrop)
  window.removeEventListener('focus', onWindowFocus)
  window.removeEventListener('blur', onWindowBlur)

  activeTimeoutIds.forEach((id) => clearTimeout(id))
  activeTimeoutIds.clear()

  if (unlistenDragDrop) unlistenDragDrop()
  if (unlistenProgress) unlistenProgress()
  if (unlistenDeletionSkipped) unlistenDeletionSkipped()
  if (xplaneCheckInterval !== null) clearInterval(xplaneCheckInterval)
})

async function analyzeFiles(paths: string[], passwords?: Record<string, string>) {
  logOperation(t('log.filesDropped'), t('log.fileCount', { count: paths.length }))

  if (!passwords || Object.keys(passwords).length === 0) {
    passwordRetryCount.value = 0
  }

  if (!store.xplanePath) {
    toast.warning(t('home.pathNotSet'))
    return
  }

  store.isAnalyzing = true
  try {
    const result = await invoke<AnalysisResult>('analyze_addons', {
      paths,
      xplanePath: store.xplanePath,
      passwords: passwords || null,
      verificationPreferences: store.verificationPreferences,
    })

    const nestedRequiredPaths = result.nestedPasswordRequired ? Object.keys(result.nestedPasswordRequired) : []
    const allRequiredPaths = [...(result.passwordRequired || []), ...nestedRequiredPaths]

    if (allRequiredPaths.length > 0) {
      pendingAnalysisPaths.value = paths
      passwordRequiredPaths.value = allRequiredPaths
      if (passwords) collectedPasswords.value = { ...passwords }
      showPasswordModal.value = true
      store.isAnalyzing = false
      return
    }

    if (result.errors.length > 0) {
      const passwordErrors = result.errors.filter(err => err.includes('Wrong password') || err.toLowerCase().includes('wrong password'))
      if (passwordErrors.length > 0 && passwords && Object.keys(passwords).length > 0) {
        passwordRetryCount.value++
        if (passwordRetryCount.value >= MAX_PASSWORD_RETRIES) {
          modal.showError(t('password.maxRetries') + '\n\n' + result.errors.join('\n'))
          resetPasswordState()
          store.isAnalyzing = false
          return
        }
        const wrongPasswordPaths = extractWrongPasswordPaths(passwordErrors)
        if (wrongPasswordPaths.length > 0) passwordRequiredPaths.value = wrongPasswordPaths
        passwordErrorMessage.value = t('password.wrongPassword')
        showPasswordModal.value = true
        store.isAnalyzing = false
        return
      }

      if (result.tasks.length > 0) {
        toast.warning(`${t('home.partialAnalysisWarning', { count: result.errors.length })} ${t('home.partialAnalysisHint')}`)
      } else {
        modal.showError(result.errors.join('\n'))
        return
      }
    }

    if (result.tasks.length > 0) {
      const allowedTasks = result.tasks.filter((task) => {
        const effectiveType = task.type === AddonType.LuaScript ? AddonType.Plugin : task.type
        return store.installPreferences[effectiveType]
      })
      const ignoredCount = result.tasks.length - allowedTasks.length

      if (ignoredCount > 0) toast.info(t('home.ignoredTasks', { count: ignoredCount }))

      if (allowedTasks.length > 0) {
        if (showConfirmation.value) {
          const addedCount = store.appendTasks(allowedTasks)
          if (addedCount > 0) toast.success(t('home.tasksAppended', { count: addedCount }))
          else toast.info(t('home.duplicateTasksIgnored'))
        } else {
          store.setCurrentTasks(allowedTasks)
          showConfirmation.value = true
        }
        resetPasswordState()
      } else if (ignoredCount > 0) {
        toast.warning(t('home.allIgnored'))
      } else {
        toast.warning(t('home.noValidAddons'))
      }
    } else {
      toast.warning(t('home.noValidAddons'))
    }
  } catch (error) {
    logError(`${t('log.analysisFailed')}: ${error}`, 'analysis')
    modal.showError(t('home.failedToAnalyze') + ': ' + getErrorMessage(error))
  } finally {
    store.isAnalyzing = false
  }
}

async function handlePasswordSubmit(passwords: Record<string, string>) {
  const now = Date.now()
  const recentAttempts = passwordAttemptTimestamps.value.filter(t => now - t < PASSWORD_RATE_LIMIT_WINDOW_MS)
  if (recentAttempts.length > 0) {
    const lastAttempt = Math.max(...recentAttempts)
    if (now - lastAttempt < MIN_PASSWORD_ATTEMPT_DELAY_MS) {
      toast.warning(t('password.tooFast'))
      return
    }
  }
  passwordAttemptTimestamps.value.push(now)
  showPasswordModal.value = false
  passwordErrorMessage.value = ''
  const allPasswords = { ...collectedPasswords.value, ...passwords }
  await analyzeFiles(pendingAnalysisPaths.value, allPasswords)
}

async function handlePasswordCancel() {
  showPasswordModal.value = false
  const nonPasswordPaths = pendingAnalysisPaths.value.filter(p => !passwordRequiredPaths.value.includes(p))
  resetPasswordState()
  if (nonPasswordPaths.length > 0) await analyzeFiles(nonPasswordPaths)
}

function extractWrongPasswordPaths(errors: string[]): string[] {
  const paths: string[] = []
  for (const err of errors) {
    const match = err.match(/Wrong password for archive:\s*(.+)$/i)
    if (match && match[1]) paths.push(match[1].trim())
  }
  return paths.length > 0 ? paths : passwordRequiredPaths.value
}

function resetPasswordState() {
  pendingAnalysisPaths.value = []
  passwordRequiredPaths.value = []
  collectedPasswords.value = {}
  passwordRetryCount.value = 0
  passwordErrorMessage.value = ''
}

async function handleInstall() {
  showConfirmation.value = false
  const enabledTasks = store.currentTasks.filter((task) => store.getTaskEnabled(task.id))
  if (enabledTasks.length === 0) {
    toast.warning(t('home.noTasksEnabled'))
    return
  }

  store.setInstallingTasks(enabledTasks)
  store.isInstalling = true

  try {
    if (!lockStore.isInitialized) await lockStore.initStore()
    const allTasksWithSettings = store.getTasksWithOverwrite()
    const tasksWithOverwrite = allTasksWithSettings.filter((task) => store.getTaskEnabled(task.id))

    const result = await invoke<InstallResult>('install_addons', {
      tasks: tasksWithOverwrite,
      atomicInstallEnabled: store.atomicInstallEnabled,
      xplanePath: store.xplanePath,
      deleteSourceAfterInstall: store.deleteSourceAfterInstall,
      autoSortScenery: store.autoSortScenery,
      lockedSceneryFolderNames: lockStore.getLockedItems('scenery'),
      parallelEnabled: store.parallelInstallEnabled,
      maxParallel: store.maxParallelTasks,
    })

    progressStore.setPercentage(100)
    store.setInstallResult(result)
    setTrackedTimeout(
      () => {
        store.isInstalling = false
        progressStore.reset()
        // Refresh dashboard data after install
        activityLogStore.loadRecent()
        managementStore.loadAircraft()
        managementStore.loadPlugins()
        sceneryStore.loadData()
        diskUsageStore.scan()
      },
      COMPLETION_ANIMATION_DELAY_MS,
      activeTimeoutIds,
    )
  } catch (error) {
    logError(`${t('log.installationFailed')}: ${error}`, 'installation')
    modal.showError(t('home.installationFailed') + ': ' + getErrorMessage(error))
    store.isInstalling = false
    progressStore.reset()
  }
}

async function handleSkipTask() {
  const confirmed = await showConfirmDialog({
    title: t('taskControl.skipConfirmTitle'),
    message: t('taskControl.skipConfirmMessage'),
    warning: t('taskControl.skipWarningClean'),
    confirmText: t('taskControl.confirmSkip'),
    cancelText: t('common.cancel'),
    type: 'warning',
  })
  if (confirmed) {
    try {
      await invoke('skip_current_task')
      toast.info(t('taskControl.taskSkipped'))
    } catch (error) {
      modal.showError(getErrorMessage(error))
    }
  }
}

async function handleCancelInstallation() {
  const confirmed = await showConfirmDialog({
    title: t('taskControl.cancelConfirmTitle'),
    message: t('taskControl.cancelConfirmMessage'),
    warning: t('taskControl.cancelWarningClean'),
    confirmText: t('taskControl.confirmCancel'),
    cancelText: t('common.cancel'),
    type: 'danger',
  })
  if (confirmed) {
    try {
      await invoke('cancel_installation')
      toast.info(t('taskControl.tasksCancelled'))
    } catch (error) {
      modal.showError(getErrorMessage(error))
    }
  }
}

function showConfirmDialog(
  options: Omit<ConfirmOptions, 'onConfirm' | 'onCancel'>,
): Promise<boolean> {
  return new Promise((resolve) => {
    modal.showConfirm({
      ...options,
      onConfirm: () => resolve(true),
      onCancel: () => resolve(false),
    })
  })
}

async function handleLaunchXPlane() {
  if (isLaunchingXPlane.value || isXPlaneRunning.value) return
  await sceneryStore.loadData()
  if (sceneryStore.hasChanges) showLaunchConfirmDialog.value = true
  else await launchXPlane()
}

async function launchXPlane() {
  if (isLaunchingXPlane.value || isXPlaneRunning.value) return
  isLaunchingXPlane.value = true
  try {
    const args = store.xplaneLaunchArgs ? store.xplaneLaunchArgs.split(/\s+/).filter(Boolean) : []
    await invoke('launch_xplane', {
      xplanePath: store.xplanePath,
      args: args.length > 0 ? args : null,
    })
    await new Promise((resolve) => setTimeout(resolve, 5000))
    showLaunchConfirmDialog.value = false
  } catch (error) {
    modal.showError(t('home.launchFailed') + ': ' + getErrorMessage(error))
  } finally {
    isLaunchingXPlane.value = false
  }
}

async function applyAndLaunch() {
  if (isLaunchingXPlane.value) return
  isLaunchingXPlane.value = true
  try {
    await sceneryStore.applyChanges()
    isLaunchingXPlane.value = false
    await launchXPlane()
  } catch (error) {
    modal.showError(getErrorMessage(error))
    isLaunchingXPlane.value = false
  }
}

function cancelLaunchDialog() {
  showLaunchConfirmDialog.value = false
}

function handleCompletionConfirm() {
  store.clearInstallResult()
  store.clearTasks()
}

async function handleFixScenery() {
  try {
    toast.info(t('settings.sorting'))
    await invoke('sort_scenery_packs', { xplanePath: store.xplanePath })
    toast.success(t('settings.scenerySorted'))
    await sceneryStore.loadData()
  } catch (error) {
    modal.showError(t('settings.scenerySortFailed') + ': ' + getErrorMessage(error))
  }
}
</script>

<style scoped>
/* Custom Scrollbar for dashboard */
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(0, 255, 65, 0.05);
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(0, 255, 65, 0.2);
  border-radius: 2px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 255, 65, 0.4);
}

/* Animations */
@keyframes cockpit-power-on {
  0% { opacity: 0; filter: brightness(5) contrast(2); transform: scale(1.05); }
  100% { opacity: 1; filter: brightness(1) contrast(1); transform: scale(1); }
}

@keyframes radar-sweep {
  0% { transform: rotate(0deg); opacity: 0.5; }
  50% { opacity: 0.2; }
  100% { transform: rotate(360deg); opacity: 0.5; }
}

@keyframes loading-bar {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}

@keyframes cockpit-alert {
  0%, 100% { border-color: inherit; box-shadow: 0 0 10px rgba(0,0,0,0); }
  50% { border-color: #ff3e3e; box-shadow: 0 0 30px rgba(255,62,62,0.4); }
}

.animate-cockpit-power-on {
  animation: cockpit-power-on 0.8s cubic-bezier(0.2, 0, 0.2, 1) forwards;
}

.animate-radar-sweep {
  animation: radar-sweep 4s linear infinite;
}

.animate-loading-bar {
  animation: loading-bar 1.5s infinite;
}

.animate-cockpit-alert {
  animation: cockpit-alert 1s infinite;
}

.animate-spin-slow {
  animation: spin 8s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Fade transition */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
