<template>
  <div
    class="settings-view mx-auto flex h-full w-full max-w-5xl flex-col overflow-hidden p-4 sm:p-6"
  >
    <h1 class="mb-4 shrink-0 text-xl font-semibold text-gray-900 dark:text-gray-100">
      {{ t('common.settings') }}
    </h1>
    <nav
      :aria-label="t('common.settings')"
      class="mb-4 flex shrink-0 gap-1 overflow-x-auto rounded-xl border border-gray-200 bg-gray-50 p-1 dark:border-gray-700 dark:bg-gray-800/60"
    >
      <button
        v-for="group in groups"
        :key="group.id"
        type="button"
        class="relative rounded-lg px-3 py-1.5 text-sm font-medium transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40"
        :class="
          activeGroup === group.id
            ? 'bg-white text-blue-600 shadow-sm dark:bg-gray-700 dark:text-blue-400'
            : 'text-gray-500 hover:bg-white/60 hover:text-gray-900 dark:text-gray-400 dark:hover:bg-gray-700/50 dark:hover:text-gray-100'
        "
        :aria-current="activeGroup === group.id ? 'page' : undefined"
        :aria-controls="`settings-${group.id}`"
        @click="switchGroup(group.id)"
      >
        <span class="relative z-10">{{ t(group.title) }}</span>
      </button>
    </nav>

    <div class="min-h-0 flex-1 overflow-y-auto pb-4 pr-1">
      <Transition
        mode="out-in"
        enter-active-class="transition-all duration-200 ease-out"
        enter-from-class="opacity-0 translate-y-1"
        enter-to-class="opacity-100 translate-y-0"
        leave-active-class="transition-all duration-150 ease-in"
        leave-from-class="opacity-100 translate-y-0"
        leave-to-class="opacity-0 translate-y-1"
      >
        <section v-show="activeGroup === 'xplane'" id="settings-xplane" :class="panel">
          <XplaneDirectoryForm />
          <CollapsibleSection :title="t('settingsPanels.advanced')">
            <label
              for="launch-args"
              class="block text-sm font-medium text-gray-900 dark:text-gray-100"
            >
              {{ t('settings.xplaneLaunchArgs') }}
            </label>
            <input
              id="launch-args"
              v-model="launchArgsInput"
              type="text"
              :class="inputStyle"
              :placeholder="t('settings.xplaneLaunchArgsPlaceholder')"
              aria-describedby="launch-args-help"
              @blur="handleLaunchArgsBlur"
              @keydown.enter.prevent="handleLaunchArgsBlur"
            />
            <p id="launch-args-help" :class="helpStyle">{{ t('settings.xplaneLaunchArgsDesc') }}</p>
          </CollapsibleSection>
        </section>
      </Transition>

      <section v-show="activeGroup === 'installation'" id="settings-installation" :class="panel">
        <div>
          <h2 :class="sectionTitle">{{ t('settingsPanels.installTypes') }}</h2>
          <p :class="helpStyle">{{ t('settingsPanels.installTypesHelp') }}</p>
          <SettingToggle
            :label="t('settings.toggleAll')"
            :model-value="allPreferencesEnabled"
            @update:model-value="toggleAllPreferences"
          />
          <div class="grid grid-cols-1 gap-x-4 sm:grid-cols-2">
            <SettingToggle
              v-for="[type, label] in addonTypes"
              :key="type"
              :label="t(label)"
              :model-value="store.installPreferences[type]"
              @update:model-value="store.togglePreference(type)"
            />
          </div>
        </div>
        <div
          v-if="store.isWindows"
          class="space-y-1 border-t border-gray-200 pt-2 dark:border-gray-700"
        >
          <SettingToggle
            :label="t('settings.windowsIntegration')"
            :description="t('settings.windowsIntegrationDesc')"
            :model-value="store.isContextMenuRegistered"
            :disabled="isProcessing"
            @update:model-value="toggleContextMenu"
          />
          <SettingToggle
            :label="t('settings.atomicInstallTitle')"
            :description="t('settings.atomicInstallBenefit4')"
            :model-value="store.atomicInstallEnabled"
            @update:model-value="store.toggleAtomicInstall()"
          />
          <p :class="helpStyle">{{ t('settings.atomicInstallNote') }}</p>
        </div>

        <CollapsibleSection :title="t('settingsPanels.advanced')">
          <div class="space-y-4">
            <div>
              <h2 :class="sectionTitle">{{ t('settings.verificationPreferences') }}</h2>
              <p :class="helpStyle">{{ t('settings.verificationPreferencesDesc') }}</p>
              <div class="grid grid-cols-1 gap-x-4 sm:grid-cols-2">
                <SettingToggle
                  v-for="[type, label] in verificationTypes"
                  :key="type"
                  :label="t(label)"
                  :model-value="store.verificationPreferences[type]"
                  @update:model-value="store.toggleVerificationPreference(type)"
                />
              </div>
              <p :class="helpStyle">{{ t('settings.rarVerificationNote') }}</p>
            </div>
            <div class="border-t border-gray-200 pt-3 dark:border-gray-700">
              <h2 :class="sectionTitle">{{ t('settings.aircraftBackup') }}</h2>
              <p :class="helpStyle">{{ t('settings.aircraftBackupDesc') }}</p>
              <div class="mt-3 space-y-2">
                <p id="config-pattern-help" :class="helpStyle">
                  {{ t('settings.patternHelpText') }}
                </p>
                <div v-for="(_, index) in configPatterns" :key="index" class="space-y-1">
                  <div class="flex gap-2">
                    <input
                      v-model="configPatterns[index]"
                      type="text"
                      :class="inputStyle"
                      :aria-label="`${t('settings.configFilePatterns')} ${index + 1}`"
                      :aria-invalid="Boolean(patternErrors[index])"
                      :aria-describedby="
                        patternErrors[index]
                          ? `config-pattern-error-${index}`
                          : 'config-pattern-help'
                      "
                      @blur="handlePatternBlur"
                      @keydown.enter.prevent="handlePatternBlur"
                    />
                    <button
                      type="button"
                      :class="buttonStyle"
                      :aria-label="`${t('common.delete')} ${index + 1}`"
                      @click="removePattern(index)"
                    >
                      {{ t('common.delete') }}
                    </button>
                  </div>
                  <p
                    v-if="patternErrors[index]"
                    :id="`config-pattern-error-${index}`"
                    class="text-sm text-red-700 dark:text-red-400"
                  >
                    {{ t(patternErrors[index]!) }}
                  </p>
                </div>
                <div class="flex flex-wrap items-center gap-3">
                  <button type="button" :class="buttonStyle" @click="addPattern">
                    {{ t('settings.addPattern') }}
                  </button>
                  <p role="status" :class="helpStyle">
                    {{
                      patternSaveStatus
                        ? t(patternSaveStatus === 'saved' ? 'settings.saved' : 'settings.saving')
                        : ''
                    }}
                  </p>
                </div>
              </div>
            </div>
            <div v-if="store.isWindows" class="border-t border-gray-200 pt-3 dark:border-gray-700">
              <SettingToggle
                :label="t('settings.deleteSourceTitle')"
                :description="t('settingsPanels.deleteOriginalsWarning')"
                :model-value="store.deleteSourceAfterInstall"
                :disabled="savingDeleteSource"
                @update:model-value="confirmDeleteSource"
              />
            </div>
          </div>
        </CollapsibleSection>
      </section>

      <section v-show="activeGroup === 'scenery'" id="settings-scenery" :class="panel">
        <div class="space-y-2">
          <h2 :class="sectionTitle">{{ t('settingsPanels.scenery') }}</h2>
          <p :class="helpStyle">{{ t('sceneryManager.subtitle') }}</p>
        </div>
        <SettingToggle
          :label="t('settings.sceneryAutoSort')"
          :description="
            t(
              store.autoSortScenery
                ? 'settings.sceneryAutoSortDesc'
                : 'settingsPanels.sceneryManualHelp',
            )
          "
          :model-value="store.autoSortScenery"
          :disabled="savingAutoSort || isRebuilding || sceneryStore.isSaving"
          @update:model-value="confirmAutoSort"
        />
        <div class="flex flex-wrap items-center gap-2">
          <button
            v-if="!sceneryStore.needsDatabaseReset"
            type="button"
            :disabled="
              isRebuilding ||
              !store.xplanePath ||
              sceneryStore.isSaving ||
              sceneryStore.hasLocalChanges
            "
            :title="
              t(
                sceneryStore.indexExists
                  ? 'settings.rebuildIndexTooltip'
                  : 'settings.createIndexTooltip',
              )
            "
            :class="buttonStyle"
            @click="handleRebuildIndex"
          >
            {{
              t(
                isRebuilding
                  ? 'settings.rebuilding'
                  : sceneryStore.indexExists
                    ? 'settings.rebuildIndex'
                    : 'settings.createIndex',
              )
            }}
          </button>
          <RouterLink
            :to="{ path: '/management', query: { tab: 'scenery' } }"
            :class="primaryStyle"
          >
            {{ t('sceneryManager.openManager') }}
          </RouterLink>
        </div>
        <p v-if="!store.xplanePath" :class="helpStyle">
          {{ t('settings.sceneryAutoSortNeedPath') }}
        </p>
      </section>

      <section v-show="activeGroup === 'updates'" id="settings-updates" :class="panel">
        <SettingToggle
          :label="t('update.autoUpdateCheck')"
          :description="t('update.autoUpdateCheckDesc')"
          :model-value="updateStore.autoCheckEnabled"
          @update:model-value="updateStore.toggleAutoCheck()"
        />
        <SettingToggle
          :label="t('update.includePreRelease')"
          :description="t('update.includePreReleaseDesc')"
          :model-value="updateStore.includePreRelease"
          @update:model-value="updateStore.toggleIncludePreRelease()"
        />
        <div class="flex flex-wrap gap-2">
          <button
            type="button"
            :class="primaryStyle"
            :disabled="updateStore.checkInProgress"
            @click="updateStore.checkForUpdates(true)"
          >
            {{ t(updateStore.checkInProgress ? 'update.checking' : 'update.checkForUpdates') }}
          </button>
          <button
            type="button"
            :class="buttonStyle"
            @click="updateStore.viewCurrentVersionChangelog()"
          >
            {{ t('update.viewCurrentChangelog') }}
          </button>
        </div>
        <p v-if="updateStore.lastCheckTime" :class="helpStyle">
          {{ t('update.lastChecked') }} · {{ formatLastCheckTime(updateStore.lastCheckTime) }}
        </p>
      </section>

      <section v-show="activeGroup === 'performance'" id="settings-performance" :class="panel">
        <SettingToggle
          :label="t('settings.parallelInstallTitle')"
          :description="t('settings.parallelInstallDesc')"
          :model-value="store.parallelInstallEnabled"
          @update:model-value="store.toggleParallelInstall()"
        />
        <SettingToggle
          :label="t('settings.chunkedDownloadTitle')"
          :description="t('settings.chunkedDownloadDesc')"
          :model-value="chunkedDownloads"
          @update:model-value="
            managementStore.setAddonUpdateOptions({ chunkedDownloadEnabled: $event })
          "
        />
        <CollapsibleSection :title="t('settingsPanels.advanced')">
          <div class="grid gap-3 sm:grid-cols-3">
            <label class="block text-sm font-medium text-gray-900 dark:text-gray-100">
              {{ t('settings.maxParallelTasks') }}
              <div class="mt-1">
                <SelectDropdown
                  :model-value="store.maxParallelTasks"
                  :options="parallelTaskOptions"
                  :disabled="!store.parallelInstallEnabled"
                  :aria-label="t('settings.maxParallelTasks')"
                  @update:model-value="store.setMaxParallelTasks(Number($event))"
                />
              </div>
            </label>
            <label
              v-for="option in downloadLimits"
              :key="option.key"
              class="block text-sm font-medium text-gray-900 dark:text-gray-100"
            >
              {{ t(option.label) }}
              <div class="mt-1">
                <SelectDropdown
                  :model-value="managementStore.addonUpdateOptions[option.key] ?? option.fallback"
                  :options="downloadLimitOptions(option.max)"
                  :disabled="!chunkedDownloads"
                  :aria-label="t(option.label)"
                  @update:model-value="
                    managementStore.setAddonUpdateOptions({ [option.key]: Number($event) })
                  "
                />
              </div>
            </label>
          </div>
          <p class="mt-3" :class="helpStyle">{{ t('settings.chunkedDownloadNote') }}</p>
        </CollapsibleSection>
      </section>

      <section v-show="activeGroup === 'diagnostics'" id="settings-diagnostics" :class="panel">
        <SettingToggle
          :label="t('settings.crashAnalysisDmpEnabled')"
          :description="t('settings.crashAnalysisDmpEnabledDesc')"
          :model-value="store.crashAnalysisDmpEnabled"
          @update:model-value="store.toggleCrashAnalysisDmpEnabled()"
        />
        <CollapsibleSection :title="t('settingsPanels.advanced')">
          <SettingToggle
            :label="t('settings.crashAnalysisIgnoreDateCheck')"
            :description="t('settings.crashAnalysisIgnoreDateCheckDesc')"
            :model-value="store.crashAnalysisIgnoreDateCheck"
            :disabled="!store.crashAnalysisDmpEnabled"
            @update:model-value="store.toggleCrashAnalysisIgnoreDateCheck()"
          />
        </CollapsibleSection>
        <CollapsibleSection :title="t('settings.logs')" @toggle="handleLogDetails">
          <div class="space-y-3">
            <label class="block text-sm font-medium text-gray-900 dark:text-gray-100">
              {{ t('settings.logLevel') }}
              <div class="mt-1">
                <SelectDropdown
                  :model-value="store.logLevel"
                  :options="logLevelOptions"
                  :aria-label="t('settings.logLevel')"
                  @update:model-value="store.setLogLevel($event as LogLevel)"
                />
              </div>
            </label>
            <p :class="helpStyle">{{ t('settings.logLevelDesc') }}</p>
            <div class="flex flex-wrap gap-2">
              <button type="button" :class="buttonStyle" @click="refreshLogs">
                {{ t('settings.refreshLogs') }}
              </button>
              <button type="button" :class="buttonStyle" @click="handleOpenLogFolder">
                {{ t('settings.openLogFolder') }}
              </button>
              <button type="button" :class="buttonStyle" @click="handleCopyLogs">
                {{ t('settings.copyLogs') }}
              </button>
            </div>
            <div
              ref="logContainer"
              tabindex="0"
              :aria-label="t('settings.logs')"
              class="h-48 select-text overflow-y-auto rounded-lg bg-slate-800 p-3 font-mono text-xs ring-1 ring-slate-700"
            >
              <p v-if="recentLogs.length === 0" class="text-gray-400">{{ t('settings.noLogs') }}</p>
              <p
                v-for="(log, index) in recentLogs"
                :key="index"
                class="whitespace-pre-wrap break-all leading-relaxed"
                :class="getLogColorClass(log)"
              >
                {{ log }}
              </p>
            </div>
            <p class="break-all" :class="helpStyle">{{ logPath }}</p>
          </div>
        </CollapsibleSection>
      </section>

      <section v-show="activeGroup === 'about'" id="settings-about" :class="panel">
        <div class="flex items-center gap-4">
          <img
            src="/icon.png"
            alt=""
            class="h-12 w-12 rounded-xl shadow-sm ring-1 ring-gray-200 dark:ring-gray-600"
          />
          <div>
            <h2 :class="sectionTitle">XFast Manager</h2>
            <p :class="helpStyle">
              <span v-if="appVersion">v{{ appVersion }} · </span>© 2026 3370Tech
            </p>
          </div>
        </div>
        <div>
          <h3 :class="subTitle">{{ t('settings.development') }}</h3>
          <button type="button" :class="buttonStyle" class="mt-2" @click="openDeveloperProfile">
            CCA3370
          </button>
        </div>
        <div>
          <h3 :class="subTitle">{{ t('settings.testing') }}</h3>
          <ul class="mt-2 flex flex-wrap gap-x-4 gap-y-2 text-sm text-gray-600 dark:text-gray-400">
            <li v-for="name in testers" :key="name">{{ name }}</li>
          </ul>
        </div>
      </section>

      <div v-show="activeGroup === 'xplane'" class="mt-4">
        <FlightPlanImportPreference />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import CollapsibleSection from '@/components/CollapsibleSection.vue'
import FlightPlanImportPreference from '@/components/FlightPlanImportPreference.vue'
import SelectDropdown from '@/components/SelectDropdown.vue'
import SettingToggle from '@/components/SettingToggle.vue'
import XplaneDirectoryForm from '@/components/XplaneDirectoryForm.vue'
import { logger, logDebug, logError } from '@/services/logger'
import { useAppStore, type LogLevel } from '@/stores/app'
import { useManagementStore } from '@/stores/management'
import { useModalStore } from '@/stores/modal'
import { useSceneryStore } from '@/stores/scenery'
import { useToastStore } from '@/stores/toast'
import { useUpdateStore } from '@/stores/update'
import { AddonType, getErrorMessage } from '@/types'
import { validateGlobPattern } from '@/utils/validation'

const { t } = useI18n()
const store = useAppStore()
const managementStore = useManagementStore()
const toast = useToastStore()
const modal = useModalStore()
const updateStore = useUpdateStore()
const sceneryStore = useSceneryStore()

const groups = [
  { id: 'xplane', title: 'settingsPanels.xplane' },
  { id: 'installation', title: 'settingsPanels.installation' },
  { id: 'scenery', title: 'settingsPanels.scenery' },
  { id: 'updates', title: 'settingsPanels.updates' },
  { id: 'performance', title: 'settingsPanels.performance' },
  { id: 'diagnostics', title: 'settingsPanels.diagnostics' },
  { id: 'about', title: 'settings.about' },
] as const
const activeGroup = ref<(typeof groups)[number]['id']>('xplane')

function switchGroup(id: (typeof groups)[number]['id']) {
  activeGroup.value = id
}

const panel =
  'space-y-3 rounded-xl border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800 sm:p-5'
const sectionTitle = 'text-base font-semibold text-gray-900 dark:text-gray-100'
const subTitle = 'text-sm font-semibold text-gray-900 dark:text-gray-100'
const inputStyle =
  'mt-1 block w-full min-w-0 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 transition-all duration-150 placeholder:text-gray-400 hover:border-gray-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder:text-gray-500 dark:hover:border-gray-500 dark:focus:border-blue-400 dark:focus:ring-blue-400/20'
const helpStyle = 'text-xs text-gray-500 dark:text-gray-400'
const buttonStyle =
  'inline-flex items-center justify-center gap-1.5 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 transition-all duration-150 hover:border-gray-400 hover:bg-gray-50 hover:text-gray-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-200 dark:hover:border-gray-500 dark:hover:bg-gray-700 dark:hover:text-white'
const primaryStyle =
  'inline-flex items-center justify-center gap-1.5 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-all duration-150 hover:bg-blue-700 hover:shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-blue-600 dark:hover:bg-blue-500'

const launchArgsInput = ref(store.xplaneLaunchArgs)
const isProcessing = ref(false)
const savingDeleteSource = ref(false)
const appVersion = ref('')
const recentLogs = ref<string[]>([])
const logPath = ref('')
const logContainer = ref<HTMLElement | null>(null)
const configPatterns = ref<string[]>([...store.getConfigFilePatterns()])
const patternErrors = ref<Record<number, string>>({})
const patternSaveStatus = ref<'saving' | 'saved' | null>(null)
let patternStatusTimer: ReturnType<typeof setTimeout> | undefined

const addonTypes = [
  [AddonType.Aircraft, 'settings.typeAircraft'],
  [AddonType.Scenery, 'settings.typeScenery'],
  [AddonType.SceneryLibrary, 'settings.typeSceneryLibrary'],
  [AddonType.Plugin, 'settingsPanels.pluginWithLua'],
  [AddonType.Navdata, 'settings.typeNavdata'],
  [AddonType.Livery, 'settings.typeLivery'],
] as const

const verificationTypes = [
  ['zip', 'settings.verifyZip'],
  ['7z', 'settings.verify7z'],
  ['directory', 'settings.verifyDirectory'],
] as const

const downloadLimits = [
  { key: 'threadsPerTask', label: 'settings.threadsPerTask', fallback: 6, max: 32 },
  { key: 'totalThreads', label: 'settings.totalThreads', fallback: 32, max: 64 },
] as const
const chunkedDownloads = computed(
  () => managementStore.addonUpdateOptions.chunkedDownloadEnabled ?? true,
)

const parallelTaskOptions = computed(() =>
  Array.from({ length: 9 }, (_, i) => ({ value: i + 2, label: String(i + 2) })),
)

function downloadLimitOptions(max: number) {
  return Array.from({ length: max }, (_, i) => ({ value: i + 1, label: String(i + 1) }))
}

const testers = ['SINO1660', 'enenY', '🍊', 'Tong Wu', 'HTony03', 'CHN131']
const logLevels = [
  ['basic', 'settings.logLevelBasic'],
  ['full', 'settings.logLevelFull'],
  ['debug', 'settings.logLevelDebug'],
] as const

const logLevelOptions = computed(() =>
  logLevels.map(([value, label]) => ({ value, label: t(label) })),
)

onMounted(async () => {
  void managementStore.loadAddonUpdateOptions()
  try {
    appVersion.value = await invoke<string>('get_app_version')
  } catch (error) {
    logError(`Failed to get app version: ${error}`, 'settings')
  }
})

onBeforeUnmount(() => clearTimeout(patternStatusTimer))

const allPreferencesEnabled = computed(() =>
  addonTypes.every(([type]) => store.installPreferences[type]),
)

function toggleAllPreferences(enabled: boolean) {
  for (const [type] of addonTypes) {
    if (store.installPreferences[type] !== enabled) void store.togglePreference(type)
  }
}

async function handleLaunchArgsBlur() {
  const value = launchArgsInput.value.trim()
  if (value !== store.xplaneLaunchArgs) await store.setXplaneLaunchArgs(value)
}

function addPattern() {
  configPatterns.value.push('')
}

function removePattern(index: number) {
  configPatterns.value.splice(index, 1)
  void handlePatternBlur()
}

async function handlePatternBlur() {
  const errors: Record<number, string> = {}
  const validPatterns: string[] = []
  for (const [index, pattern] of configPatterns.value.entries()) {
    const trimmed = pattern.trim()
    if (!trimmed) continue
    const error = validateGlobPattern(trimmed)
    if (error) errors[index] = error
    else validPatterns.push(trimmed)
  }
  patternErrors.value = errors
  if (Object.keys(errors).length || (!validPatterns.length && configPatterns.value.length)) return

  clearTimeout(patternStatusTimer)
  patternSaveStatus.value = 'saving'
  try {
    await store.setConfigFilePatterns(validPatterns)
    patternSaveStatus.value = 'saved'
    patternStatusTimer = setTimeout(() => (patternSaveStatus.value = null), 2000)
  } catch (error) {
    patternSaveStatus.value = null
    modal.showError(error)
  }
}

function confirmDeleteSource(enabled: boolean) {
  if (!enabled) {
    void saveDeleteSource(false)
    return
  }
  modal.showConfirm({
    title: t('settings.deleteSourceTitle'),
    message: t('settingsPanels.deleteOriginalsWarning'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
    type: 'danger',
    onConfirm: () => void saveDeleteSource(true),
    onCancel: () => {},
  })
}

async function saveDeleteSource(enabled: boolean) {
  if (savingDeleteSource.value || store.deleteSourceAfterInstall === enabled) return
  savingDeleteSource.value = true
  try {
    await store.setDeleteSourceAfterInstall(enabled)
  } catch (error) {
    modal.showError(error)
  } finally {
    savingDeleteSource.value = false
  }
}

async function handleLogDetails(event: Event) {
  if (!(event.target as HTMLDetailsElement).open) return
  await refreshLogs()
  logPath.value = await logger.getLogPath()
}

async function refreshLogs() {
  recentLogs.value = await logger.getRecentLogs(50)
  await nextTick()
  if (logContainer.value) logContainer.value.scrollTop = logContainer.value.scrollHeight
}

async function toggleContextMenu() {
  if (isProcessing.value) return
  isProcessing.value = true
  try {
    if (!store.isContextMenuRegistered) {
      try {
        await invoke('register_context_menu')
        toast.success(t('settings.contextMenuRegistered'))
        store.isContextMenuRegistered = true
      } catch (error) {
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
      try {
        await invoke('unregister_context_menu')
        toast.success(t('settings.contextMenuUnregistered'))
        store.isContextMenuRegistered = false
      } catch (error) {
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

const savingAutoSort = ref(false)
const isRebuilding = ref(false)

function confirmAutoSort(enabled: boolean) {
  if (!enabled) {
    void saveAutoSort(false)
    return
  }
  modal.showConfirm({
    title: t('settings.sceneryAutoSort'),
    message: t('settingsPanels.enableSceneryHelp'),
    confirmText: t('common.confirm'),
    cancelText: t('common.cancel'),
    type: 'warning',
    onConfirm: () => void saveAutoSort(true),
    onCancel: () => {},
  })
}

async function saveAutoSort(enabled: boolean) {
  if (savingAutoSort.value) return
  savingAutoSort.value = true
  try {
    await store.setAutoSortScenery(enabled)
  } catch (error) {
    modal.showError(error)
  } finally {
    savingAutoSort.value = false
  }
}

async function handleRebuildIndex() {
  if (isRebuilding.value || !store.xplanePath) return
  isRebuilding.value = true
  try {
    await invoke('rebuild_scenery_index', { xplanePath: store.xplanePath })
    await sceneryStore.loadData()
    toast.success(t('settings.indexRebuilt'))
  } catch (e) {
    modal.showError(t('settings.indexRebuildFailed') + ': ' + String(e))
  } finally {
    isRebuilding.value = false
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
