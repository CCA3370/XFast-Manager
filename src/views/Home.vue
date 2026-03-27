<template>
  <div class="home-view h-full flex flex-col p-3 animate-fade-in relative overflow-hidden select-none">
    <div class="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none z-0 opacity-0 dark:opacity-100 transition-opacity duration-500">
      <div class="absolute top-1/4 left-1/4 w-40 h-40 bg-blue-500/5 rounded-full blur-3xl"></div>
      <div class="absolute bottom-1/4 right-1/4 w-56 h-56 bg-purple-500/5 rounded-full blur-3xl"></div>
    </div>

    <div class="w-full z-10 flex flex-col flex-1 min-h-0 gap-2 overflow-y-auto pr-1 custom-scrollbar">
      <UpdateBanner :visible="updateStore.showUpdateBanner" :update-info="updateStore.updateInfo" :is-downloading="updateStore.isDownloading" :download-progress="updateStore.downloadProgress" :update-phase="updateStore.updatePhase" :update-error="updateStore.updateError" class="scale-90 origin-top mb-[-8px]" @view-release="updateStore.openReleaseUrl" @dismiss="updateStore.dismissUpdate" @update="updateStore.performUpdate" @retry="updateStore.performUpdate" />

      <div class="grid grid-cols-4 gap-2 pb-2">
        <div v-for="s in stats" :key="s.label" class="bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-xl p-2.5 shadow-sm transition-all">
          <div class="flex justify-between items-start">
            <div class="p-1.5 rounded-lg" :class="s.bg"><component :is="s.icon" class="w-4 h-4" :class="s.color" /></div>
            <div v-if="s.badge > 0" class="px-1 py-0.5 bg-blue-500 text-white text-[8px] font-bold rounded-full animate-pulse">{{ s.badge }}</div>
          </div>
          <div class="mt-1.5">
            <div class="text-lg font-black text-gray-900 dark:text-white leading-none tabular-nums">{{ s.value }}</div>
            <div class="text-[9px] text-gray-500 dark:text-gray-400 font-bold uppercase tracking-tighter mt-0.5">{{ $t(s.label) }}</div>
          </div>
        </div>

        <div class="col-span-2 bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-xl p-3 shadow-sm flex flex-col justify-center min-h-[120px]">
          <DiskUsageChart :categories="diskCategories" :total-bytes="diskUsageStore.report?.totalBytes || 0" :size="100" :is-dark="store.theme === 'dark'" />
        </div>

        <div class="col-span-2 bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-xl p-3 shadow-sm">
          <div class="grid grid-cols-3 gap-1.5 h-full">
            <button v-for="a in actions" :key="a.name" class="flex flex-col items-center justify-center gap-1 p-1.5 rounded-lg border border-white/20 dark:border-gray-700/30 hover:bg-white/40 dark:hover:bg-gray-700/40 transition-all group" @click="a.fn">
              <component :is="a.icon" class="w-4 h-4 text-gray-500 group-hover:text-blue-500 transition-colors" />
              <span class="text-[8px] font-bold uppercase text-gray-600 dark:text-gray-400 text-center leading-tight">{{ a.name }}</span>
            </button>
          </div>
        </div>

        <div class="col-span-4 bg-white/40 dark:bg-gray-800/40 backdrop-blur-xl border border-white/20 dark:border-gray-700/30 rounded-xl p-3 shadow-sm">
          <div class="flex justify-between items-center mb-2">
            <h3 class="text-[10px] font-black text-gray-900 dark:text-white uppercase tracking-widest">{{ $t('dashboard.recentActivity') }}</h3>
            <button class="text-[9px] font-black text-blue-500 hover:text-blue-600 uppercase" @click="router.push('/activity')">{{ $t('dashboard.viewAll') }}</button>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
            <div v-for="e in activityLogStore.entries.slice(0, 4)" :key="e.id" class="flex items-center gap-2 p-1.5 rounded-lg bg-white/50 dark:bg-gray-700/30 border border-white/20 transition-all">
              <div class="w-6 h-6 rounded-full flex items-center justify-center shrink-0" :class="e.success ? 'bg-emerald-100 text-emerald-600' : 'bg-red-100 text-red-600'">
                <component :is="e.operation.includes('Install') ? IconAircraft : e.operation.includes('Delete') ? IconNavdata : IconPlugin" class="w-3.5 h-3.5" />
              </div>
              <div class="min-w-0 flex-1">
                <div class="text-[10px] font-bold text-gray-900 dark:text-white truncate leading-tight">{{ e.itemName || e.operation }}</div>
                <div class="text-[8px] text-gray-400 truncate uppercase">{{ e.operation }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <transition name="fade">
      <div v-if="isDragging && !store.isInstalling" class="fixed inset-0 z-[100] flex items-center justify-center bg-blue-500/10 backdrop-blur-md transition-all duration-300 pointer-events-none">
        <div class="p-8 rounded-[2rem] bg-white/90 dark:bg-gray-800/90 shadow-2xl border-4 border-dashed border-blue-500 flex flex-col items-center gap-4 animate-bounce-in">
          <div class="w-16 h-16 rounded-full bg-blue-500 flex items-center justify-center text-white shadow-lg"><svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M12 4v16m8-8H4" /></svg></div>
          <h2 class="text-xl font-black text-gray-900 dark:text-white uppercase tracking-wider">{{ $t('dashboard.dropToInstall') }}</h2>
        </div>
      </div>
    </transition>

    <transition name="fade" mode="out-in">
      <AnalyzingOverlay v-if="store.isAnalyzing" key="analyzing" />
      <InstallProgressOverlay v-else-if="store.isInstalling || store.showCompletion" key="installing" :percentage="progressStore.formatted.percentage" :task-name="progressStore.formatted.taskName" :processed-m-b="progressStore.formatted.processedMB" :total-m-b="progressStore.formatted.totalMB" :task-progress="progressStore.formatted.taskProgress" :tasks="store.installingTasks" :current-task-index="progressStore.progress?.currentTaskIndex ?? 0" :current-task-percentage="progressStore.formatted.currentTaskPercentage" :current-task-processed-m-b="progressStore.formatted.currentTaskProcessedMB" :current-task-total-m-b="progressStore.formatted.currentTaskTotalMB" :is-complete="store.showCompletion" :install-result="store.installResult" :active-tasks="progressStore.activeTasks" :completed-task-count="progressStore.completedTaskCount" :completed-task-ids="progressStore.completedTaskIds" @skip="handleSkipTask" @cancel="handleCancelInstallation" @confirm="handleCompletionConfirm" />
    </transition>

    <ConfirmationModal v-if="showConfirmation" @close="showConfirmation = false" @confirm="handleInstall" />
    <PasswordModal v-if="showPasswordModal" :archive-paths="passwordRequiredPaths" :error-message="passwordErrorMessage" @confirm="handlePasswordSubmit" @cancel="handlePasswordCancel" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed, markRaw } from 'vue'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
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
import { open } from '@tauri-apps/plugin-dialog'
import ConfirmationModal from '@/components/ConfirmationModal.vue'
import PasswordModal from '@/components/PasswordModal.vue'
import AnimatedText from '@/components/AnimatedText.vue'
import UpdateBanner from '@/components/UpdateBanner.vue'
import InstallProgressOverlay from '@/components/InstallProgressOverlay.vue'
import AnalyzingOverlay from '@/components/AnalyzingOverlay.vue'
import DiskUsageChart from '@/components/DiskUsageChart.vue'
import { AddonType, getErrorMessage } from '@/types'
import { logError, logDebug } from '@/services/logger'
import { setTrackedTimeout } from '@/utils/timeout'

// --- Icons (Minimal & High Performance) ---
const IconAircraft = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" /></svg>' })
const IconPlugin = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" /></svg>' })
const IconScenery = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>' })
const IconNavdata = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>' })
const IconFolder = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg>' })
const IconMap = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 01-1.447-.894L15 9m0 8V9" /></svg>' })
const IconPaint = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" /></svg>' })
const IconPlay = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>' })
const IconFix = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>' })
const IconAnalysis = markRaw({ template: '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>' })

const { t } = useI18n(); const router = useRouter(); const store = useAppStore(); const toast = useToastStore(); const modal = useModalStore()
const updateStore = useUpdateStore(); const progressStore = useProgressStore(); const sceneryStore = useSceneryStore(); const managementStore = useManagementStore()
const activityLogStore = useActivityLogStore(); const diskUsageStore = useDiskUsageStore(); const lockStore = useLockStore()

const isDragging = ref(false); const showConfirmation = ref(false); const showLaunchConfirmDialog = ref(false)
const isLaunchingXPlane = ref(false); const isXPlaneRunning = ref(false); let xplaneCheckInterval: any = null
const showPasswordModal = ref(false); const passwordRequiredPaths = ref([]); const pendingAnalysisPaths = ref([]); const collectedPasswords = ref({}); const passwordErrorMessage = ref('')

const stats = computed(() => [
  { label: 'dashboard.stats.aircraft', value: managementStore.aircraftTotalCount, badge: managementStore.aircraftUpdateCount, icon: IconAircraft, bg: 'bg-blue-100 dark:bg-blue-500/20', color: 'text-blue-600 dark:text-blue-400' },
  { label: 'dashboard.stats.plugins', value: managementStore.pluginsTotalCount, badge: managementStore.pluginsUpdateCount, icon: IconPlugin, bg: 'bg-purple-100 dark:bg-purple-500/20', color: 'text-purple-600 dark:text-purple-400' },
  { label: 'dashboard.stats.scenery', value: sceneryStore.totalCount, badge: 0, icon: IconScenery, bg: 'bg-emerald-100 dark:bg-emerald-500/20', color: 'text-emerald-600 dark:text-emerald-400' },
  { label: 'dashboard.stats.navdata', value: managementStore.navdataTotalCount, badge: 0, icon: IconNavdata, bg: 'bg-pink-100 dark:bg-pink-500/20', color: 'text-pink-600 dark:text-pink-400' }
])

const actions = [
  { name: 'Launch XP', icon: IconPlay, fn: handleLaunchXPlane },
  { name: 'XP Folder', icon: IconFolder, fn: () => handleOpenFolder('') },
  { name: 'Scenery Dir', icon: IconMap, fn: () => handleOpenFolder('Custom Scenery') },
  { name: 'Fix Scenery', icon: IconFix, fn: handleFixScenery },
  { name: 'Liveries', icon: IconPaint, fn: () => router.push('/liveries') },
  { name: 'Log Anal', icon: IconAnalysis, fn: () => router.push('/log-analysis') }
]

const diskCategories = computed(() => {
  if (!diskUsageStore.report) return []
  const colors = ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ef4444']
  return diskUsageStore.report.categories.map((c, i) => ({ name: t(`addonType.${c.category}`), bytes: c.totalBytes, color: colors[i % colors.length] }))
})

const activeTimeoutIds = new Set<any>()
let unlistenDragDrop: any = null; let unlistenProgress: any = null

onMounted(async () => {
  const checkXP = async () => { try { isXPlaneRunning.value = await invoke('is_xplane_running') } catch (e) {} }
  checkXP(); xplaneCheckInterval = setInterval(checkXP, 3000)
  if (store.xplanePath) { managementStore.loadAircraft(); managementStore.loadPlugins(); managementStore.loadNavdata(); sceneryStore.loadData(); activityLogStore.loadRecent(); diskUsageStore.scan() }
  try {
    unlistenDragDrop = await getCurrentWebviewWindow().onDragDropEvent((e: any) => {
      if (store.isInstalling) return
      if (e.payload.type === 'over') isDragging.value = true
      else if (e.payload.type === 'leave') isDragging.value = false
      else if (e.payload.type === 'drop') { isDragging.value = false; if (e.payload.paths?.length > 0) analyzeFiles(e.payload.paths) }
    })
  } catch (e) {}
  unlistenProgress = await listen('install-progress', (e: any) => progressStore.update(e.payload))
})

onBeforeUnmount(() => { clearInterval(xplaneCheckInterval); if (unlistenDragDrop) unlistenDragDrop(); if (unlistenProgress) unlistenProgress(); activeTimeoutIds.forEach(id => clearTimeout(id)) })

async function analyzeFiles(paths: string[], ps?: any) {
  if (!store.xplanePath) return; store.isAnalyzing = true
  try {
    const res: any = await invoke('analyze_addons', { paths, xplanePath: store.xplanePath, passwords: ps || null, verificationPreferences: store.verificationPreferences })
    if (res.passwordRequired?.length > 0) { pendingAnalysisPaths.value = paths; passwordRequiredPaths.value = res.passwordRequired; showPasswordModal.value = true; return }
    if (res.tasks?.length > 0) { store.setCurrentTasks(res.tasks); showConfirmation.value = true }
  } catch (e) { modal.showError(getErrorMessage(e)) } finally { store.isAnalyzing = false }
}

async function handleInstall() {
  showConfirmation.value = false; const enabled = store.currentTasks.filter(t => store.getTaskEnabled(t.id))
  if (enabled.length === 0) return; store.isInstalling = true
  try {
    const res: any = await invoke('install_addons', { tasks: store.getTasksWithOverwrite().filter(t => store.getTaskEnabled(t.id)), atomicInstallEnabled: store.atomicInstallEnabled, xplanePath: store.xplanePath, deleteSourceAfterInstall: store.deleteSourceAfterInstall, autoSortScenery: store.autoSortScenery, lockedSceneryFolderNames: [], parallelEnabled: store.parallelInstallEnabled, maxParallel: store.maxParallelTasks })
    store.setInstallResult(res); progressStore.setPercentage(100)
    setTimeout(() => { store.isInstalling = false; progressStore.reset(); diskUsageStore.scan() }, 200)
  } catch (e) { modal.showError(getErrorMessage(e)); store.isInstalling = false }
}

async function handleLaunchXPlane() { await launchXPlane() }
async function launchXPlane() { try { await invoke('launch_xplane', { xplanePath: store.xplanePath, args: null }) } catch (e) {} }
async function handleOpenFolder(sub: string) { try { await invoke('open_path_in_explorer', { path: sub ? `${store.xplanePath}/${sub}` : store.xplanePath }) } catch (e) {} }
async function handleFixScenery() { try { await invoke('sort_scenery_packs', { xplanePath: store.xplanePath }); toast.success(t('settings.scenerySorted')) } catch (e) {} }
function cancelLaunchDialog() { showLaunchConfirmDialog.value = false }
function handleCompletionConfirm() { store.clearInstallResult(); store.clearTasks() }
function formatRelativeTime(ts: number): string { return 'RECENT' }
async function handleSkipTask() { await invoke('skip_current_task') }
async function handleCancelInstallation() { await invoke('cancel_installation') }
function showConfirmDialog(opts: any): Promise<boolean> { return new Promise(r => modal.showConfirm({ ...opts, onConfirm: () => r(true), onCancel: () => r(false) })) }
async function handlePasswordSubmit(ps: any) { analyzeFiles(pendingAnalysisPaths.value, ps); showPasswordModal.value = false }
function handlePasswordCancel() { showPasswordModal.value = false }
</script>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(156, 163, 175, 0.2); border-radius: 4px; }
@keyframes fade-in { from { opacity: 0; transform: translateY(10px); } to { opacity: 1; transform: translateY(0); } }
.animate-fade-in { animation: fade-in 0.4s ease-out forwards; }
.fade-enter-active, .fade-leave-active { transition: opacity 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
