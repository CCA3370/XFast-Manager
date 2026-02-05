import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { AddonType, type InstallTask, type InstallResult } from '@/types'
import { useLockStore } from './lock'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'

export type LogLevel = 'basic' | 'full' | 'debug'

/** Per-task state for installation management */
export interface TaskState {
  /** Whether the task is enabled for installation */
  enabled: boolean
  /** Whether to overwrite existing files on conflict */
  overwrite: boolean
  /** Whether large size warning has been confirmed */
  sizeConfirmed: boolean
  /** Backup settings for aircraft tasks */
  backup: {
    liveries: boolean
    configFiles: boolean
    navdata: boolean
  }
}

/** Delay to batch multiple CLI file selections (500ms) */
const CLI_ARGS_BATCH_DELAY_MS = 500

export const useAppStore = defineStore('app', () => {
  const xplanePath = ref<string>('')
  const currentTasks = ref<InstallTask[]>([])
  const isAnalyzing = ref(false)
  const isInstalling = ref(false)

  // Mutex flag for analyzeFiles to prevent concurrent calls (TOCTOU protection)
  const isAnalyzeInProgress = ref(false)

  // Platform detection (initialized at app startup)
  const isWindows = ref(false)
  const isContextMenuRegistered = ref(false)

  // Log level setting (basic, full, debug)
  const logLevel = ref<LogLevel>('full')

  // Pending CLI arguments to be processed by Home.vue
  const pendingCliArgs = ref<string[] | null>(null)

  // Batch processing for CLI args (to handle multiple file selections)
  // Using a Set for deduplication and atomic batch collection
  const cliArgsBatch = ref<Set<string>>(new Set())
  const cliArgsBatchTimerId = ref<ReturnType<typeof setTimeout> | null>(null)

  // Installation result state
  const installResult = ref<InstallResult | null>(null)
  const showCompletion = ref(false)
  const showCompletionAnimation = ref(false)

  // Tasks currently being installed (persists across view switches)
  const installingTasks = ref<InstallTask[]>([])

  // Default: all enabled
  const installPreferences = ref<Record<AddonType, boolean>>({
    [AddonType.Aircraft]: true,
    [AddonType.Scenery]: true,
    [AddonType.SceneryLibrary]: true,
    [AddonType.Plugin]: true,
    [AddonType.Navdata]: true,
    [AddonType.Livery]: true,
    [AddonType.LuaScript]: true,
  })

  // Verification preferences by source type (default: all enabled except RAR)
  const verificationPreferences = ref<Record<string, boolean>>({
    zip: true,
    '7z': true,
    rar: false,  // Disabled by default since RAR verification doesn't work
    directory: true,
  })

  // Atomic installation mode (default: disabled)
  const atomicInstallEnabled = ref(false)

  // Delete source files after successful installation (default: disabled)
  const deleteSourceAfterInstall = ref(false)

  // Scenery auto-sorting (default: disabled)
  const autoSortScenery = ref(false)
  const sceneryManagerHintVisible = ref(false)
  const sceneryManagerHintMessageKey = ref<string | null>(null)

  // X-Plane launch arguments (default: empty)
  const xplaneLaunchArgs = ref('')

  // Confirmation modal state (for exit confirmation)
  const isConfirmationOpen = ref(false)

  // Unified per-task state management (taskId -> TaskState)
  const taskStates = ref<Record<string, TaskState>>({})

  // Initialization flag
  const isInitialized = ref(false)

  /** Get default task state */
  function getDefaultTaskState(enabled = true): TaskState {
    return {
      enabled,
      overwrite: false,
      sizeConfirmed: false,
      backup: { liveries: true, configFiles: true, navdata: true }
    }
  }

  /** Get or create task state with defaults */
  function getTaskState(taskId: string): TaskState {
    if (!taskStates.value[taskId]) {
      taskStates.value[taskId] = getDefaultTaskState()
    }
    return taskStates.value[taskId]
  }

  // Config file patterns for backup (stored in storage)
  const configFilePatterns = ref<string[]>(['*_prefs.txt'])

  // Check if any task has conflicts
  const hasConflicts = computed(() => {
    return currentTasks.value.some(task => task.conflictExists === true)
  })

  // Check if any task has size warnings
  const hasSizeWarnings = computed(() => {
    return currentTasks.value.some(task => task.sizeWarning)
  })

  // Check if all size warnings have been confirmed
  const allSizeWarningsConfirmed = computed(() => {
    const tasksWithWarnings = currentTasks.value.filter(task => task.sizeWarning)
    if (tasksWithWarnings.length === 0) return true
    return tasksWithWarnings.every(task => getTaskState(task.id).sizeConfirmed)
  })

  // Get count of enabled tasks
  const enabledTasksCount = computed(() => {
    return currentTasks.value.filter(task => getTaskEnabled(task.id)).length
  })

  /** Initialize store by loading saved settings from Tauri Store */
  async function initStore(): Promise<void> {
    if (isInitialized.value) return

    // Load xplanePath
    const savedPath = await getItem<string>(STORAGE_KEYS.XPLANE_PATH)
    if (savedPath) xplanePath.value = savedPath

    // Load install preferences
    const savedPrefs = await getItem<Record<AddonType, boolean>>(STORAGE_KEYS.INSTALL_PREFERENCES)
    if (savedPrefs && typeof savedPrefs === 'object') {
      installPreferences.value = { ...installPreferences.value, ...savedPrefs }
    }

    // Load log level
    const savedLogLevel = await getItem<LogLevel>(STORAGE_KEYS.LOG_LEVEL)
    if (savedLogLevel && ['basic', 'full', 'debug'].includes(savedLogLevel)) {
      logLevel.value = savedLogLevel
    }
    // Sync log level to backend
    const backendLevel = logLevel.value === 'basic' ? 'error' : logLevel.value === 'full' ? 'info' : 'debug'
    invoke('set_log_level', { level: backendLevel })

    // Load config file patterns
    const savedPatterns = await getItem<string[]>(STORAGE_KEYS.CONFIG_FILE_PATTERNS)
    if (Array.isArray(savedPatterns) && savedPatterns.every(item => typeof item === 'string')) {
      configFilePatterns.value = savedPatterns
    }

    // Load verification preferences
    const savedVerificationPrefs = await getItem<Record<string, boolean>>(STORAGE_KEYS.VERIFICATION_PREFERENCES)
    if (savedVerificationPrefs && typeof savedVerificationPrefs === 'object') {
      verificationPreferences.value = { ...verificationPreferences.value, ...savedVerificationPrefs }
    }

    // Load atomic install setting
    const savedAtomicInstall = await getItem<boolean>(STORAGE_KEYS.ATOMIC_INSTALL_ENABLED)
    if (typeof savedAtomicInstall === 'boolean') {
      atomicInstallEnabled.value = savedAtomicInstall
    }

    // Load delete source after install setting
    const savedDeleteSource = await getItem<boolean>(STORAGE_KEYS.DELETE_SOURCE_AFTER_INSTALL)
    if (typeof savedDeleteSource === 'boolean') {
      deleteSourceAfterInstall.value = savedDeleteSource
    }

    // Load scenery auto-sort setting
    const savedAutoSortScenery = await getItem<boolean>(STORAGE_KEYS.AUTO_SORT_SCENERY)
    if (typeof savedAutoSortScenery === 'boolean') {
      autoSortScenery.value = savedAutoSortScenery
    }

    // Load X-Plane launch arguments
    const savedLaunchArgs = await getItem<string>(STORAGE_KEYS.XPLANE_LAUNCH_ARGS)
    if (typeof savedLaunchArgs === 'string') {
      xplaneLaunchArgs.value = savedLaunchArgs
    }

    isInitialized.value = true
  }

  async function setXplanePath(path: string) {
    xplanePath.value = path
    await setItem(STORAGE_KEYS.XPLANE_PATH, path)
  }

  async function loadXplanePath() {
    const saved = await getItem<string>(STORAGE_KEYS.XPLANE_PATH)
    if (saved) {
      xplanePath.value = saved
    }
  }

  async function togglePreference(type: AddonType) {
    installPreferences.value[type] = !installPreferences.value[type]
    await setItem(STORAGE_KEYS.INSTALL_PREFERENCES, installPreferences.value)
  }

  async function toggleVerificationPreference(sourceType: string) {
    verificationPreferences.value[sourceType] = !verificationPreferences.value[sourceType]
    await setItem(STORAGE_KEYS.VERIFICATION_PREFERENCES, verificationPreferences.value)
  }

  async function toggleAtomicInstall() {
    atomicInstallEnabled.value = !atomicInstallEnabled.value
    await setItem(STORAGE_KEYS.ATOMIC_INSTALL_ENABLED, atomicInstallEnabled.value)
  }

  async function toggleDeleteSourceAfterInstall() {
    deleteSourceAfterInstall.value = !deleteSourceAfterInstall.value
    await setItem(STORAGE_KEYS.DELETE_SOURCE_AFTER_INSTALL, deleteSourceAfterInstall.value)
  }

  async function toggleAutoSortScenery() {
    autoSortScenery.value = !autoSortScenery.value
    await setItem(STORAGE_KEYS.AUTO_SORT_SCENERY, autoSortScenery.value)
  }

  async function setXplaneLaunchArgs(args: string) {
    xplaneLaunchArgs.value = args
    await setItem(STORAGE_KEYS.XPLANE_LAUNCH_ARGS, args)
  }

  function showSceneryManagerHint(messageKey: string) {
    sceneryManagerHintMessageKey.value = messageKey
    sceneryManagerHintVisible.value = true
  }

  function dismissSceneryManagerHint() {
    sceneryManagerHintVisible.value = false
  }

  function setConfirmationOpen(open: boolean) {
    isConfirmationOpen.value = open
  }

  async function setLogLevel(level: LogLevel) {
    logLevel.value = level
    await setItem(STORAGE_KEYS.LOG_LEVEL, level)
    // Also set the backend log level
    const backendLevel = level === 'basic' ? 'error' : level === 'full' ? 'info' : 'debug'
    invoke('set_log_level', { level: backendLevel })
  }

  function setCurrentTasks(tasks: InstallTask[]) {
    currentTasks.value = tasks
    // Reset all task states
    taskStates.value = {}
    // Initialize task states for each task
    // Disable livery tasks where target aircraft is not found
    // Disable tasks where target is locked
    const lockStore = useLockStore()
    tasks.forEach(task => {
      const isLiveryWithoutAircraft = task.type === AddonType.Livery && task.liveryAircraftFound === false
      const isLockedConflict = task.conflictExists && lockStore.isPathLocked(task.targetPath, xplanePath.value)
      const enabled = !isLiveryWithoutAircraft && !isLockedConflict
      taskStates.value[task.id] = getDefaultTaskState(enabled)
    })
  }

  /** Append new tasks to current list (for adding files while confirmation modal is open) */
  function appendTasks(newTasks: InstallTask[]): number {
    // Deduplicate by sourcePath to avoid adding the same file twice
    const existingPaths = new Set(currentTasks.value.map(t => t.sourcePath))
    const uniqueNewTasks = newTasks.filter(t => !existingPaths.has(t.sourcePath))

    if (uniqueNewTasks.length === 0) return 0

    // Append to existing task list
    currentTasks.value = [...currentTasks.value, ...uniqueNewTasks]

    // Initialize state only for new tasks (preserve existing task settings)
    const lockStore = useLockStore()
    uniqueNewTasks.forEach(task => {
      const isLiveryWithoutAircraft = task.type === AddonType.Livery && task.liveryAircraftFound === false
      const isLockedConflict = task.conflictExists && lockStore.isPathLocked(task.targetPath, xplanePath.value)
      const enabled = !isLiveryWithoutAircraft && !isLockedConflict
      taskStates.value[task.id] = getDefaultTaskState(enabled)
    })

    return uniqueNewTasks.length
  }

  function clearTasks() {
    currentTasks.value = []
    taskStates.value = {}
  }

  // Set overwrite for a specific task
  function setTaskOverwrite(taskId: string, shouldOverwrite: boolean) {
    getTaskState(taskId).overwrite = shouldOverwrite
  }

  // Set overwrite for all conflicting tasks
  function setGlobalOverwrite(shouldOverwrite: boolean) {
    for (const task of currentTasks.value) {
      if (task.conflictExists) {
        getTaskState(task.id).overwrite = shouldOverwrite
      }
    }
  }

  // Get overwrite setting for a task
  function getTaskOverwrite(taskId: string): boolean {
    return getTaskState(taskId).overwrite
  }

  // Prepare tasks with overwrite, size confirmation, and backup settings for installation
  function getTasksWithOverwrite(): InstallTask[] {
    return currentTasks.value.map(task => {
      const state = getTaskState(task.id)
      return {
        ...task,
        shouldOverwrite: state.overwrite,
        sizeConfirmed: state.sizeConfirmed,
        backupLiveries: state.backup.liveries,
        // Only enable config file backup if patterns are configured
        backupConfigFiles: (configFilePatterns.value.length > 0) && state.backup.configFiles,
        configFilePatterns: configFilePatterns.value,
        backupNavdata: state.backup.navdata,
      }
    })
  }

  // Set size confirmation for a specific task
  function setTaskSizeConfirmed(taskId: string, confirmed: boolean) {
    getTaskState(taskId).sizeConfirmed = confirmed
  }

  // Get size confirmation for a task
  function getTaskSizeConfirmed(taskId: string): boolean {
    return getTaskState(taskId).sizeConfirmed
  }

  // Confirm all size warnings at once
  function confirmAllSizeWarnings(confirmed: boolean) {
    for (const task of currentTasks.value) {
      if (task.sizeWarning) {
        getTaskState(task.id).sizeConfirmed = confirmed
      }
    }
  }

  // Set task enabled state
  function setTaskEnabled(taskId: string, enabled: boolean) {
    getTaskState(taskId).enabled = enabled
  }

  // Get task enabled state (default true)
  function getTaskEnabled(taskId: string): boolean {
    return getTaskState(taskId).enabled
  }

  // Toggle all tasks enabled/disabled
  function setAllTasksEnabled(enabled: boolean) {
    for (const task of currentTasks.value) {
      getTaskState(task.id).enabled = enabled
    }
  }

  // Set backup settings for a specific task
  function setTaskBackupSettings(taskId: string, liveries: boolean, configFiles: boolean) {
    const state = getTaskState(taskId)
    state.backup = { ...state.backup, liveries, configFiles }
  }

  // Get backup settings for a task (default both true)
  function getTaskBackupSettings(taskId: string): { liveries: boolean, configFiles: boolean } {
    return getTaskState(taskId).backup
  }

  // Set navdata backup setting for a specific task
  function setBackupNavdata(taskId: string, value: boolean) {
    getTaskState(taskId).backup.navdata = value
  }

  // Get navdata backup setting for a task (default true)
  function getBackupNavdata(taskId: string): boolean {
    return getTaskState(taskId).backup.navdata
  }

  // Set config file patterns
  async function setConfigFilePatterns(patterns: string[]) {
    configFilePatterns.value = patterns
    await setItem(STORAGE_KEYS.CONFIG_FILE_PATTERNS, patterns)
  }

  // Get config file patterns
  function getConfigFilePatterns(): string[] {
    return configFilePatterns.value
  }

  // Set pending CLI args for Home.vue to process
  function setPendingCliArgs(args: string[]) {
    pendingCliArgs.value = args
  }

  // Add CLI args to batch (for handling multiple file selections)
  // Thread-safe: Uses Set for deduplication and reactive timer for proper cleanup
  function addCliArgsToBatch(args: string[]) {
    // Add new args to batch using Set for automatic deduplication
    args.forEach(arg => cliArgsBatch.value.add(arg))

    // Clear existing timer if any
    if (cliArgsBatchTimerId.value !== null) {
      clearTimeout(cliArgsBatchTimerId.value)
      cliArgsBatchTimerId.value = null
    }

    // Capture current batch reference for closure safety
    const currentBatch = cliArgsBatch.value

    // Set new timer to process batch after delay
    cliArgsBatchTimerId.value = setTimeout(() => {
      // Verify this timer hasn't been superseded
      if (cliArgsBatchTimerId.value === null) {
        return
      }

      if (currentBatch.size > 0) {
        // Convert Set to array
        const uniqueArgs = Array.from(currentBatch)
        setPendingCliArgs(uniqueArgs)
        currentBatch.clear()
      }
      cliArgsBatchTimerId.value = null
    }, CLI_ARGS_BATCH_DELAY_MS)
  }

  // Clear pending CLI args after processing
  function clearPendingCliArgs() {
    pendingCliArgs.value = null
    // Also clear any pending batch timer
    if (cliArgsBatchTimerId.value !== null) {
      clearTimeout(cliArgsBatchTimerId.value)
      cliArgsBatchTimerId.value = null
    }
    cliArgsBatch.value.clear()
  }

  // Set installation result
  function setInstallResult(result: InstallResult) {
    installResult.value = result
    // First show the completion view (without icon)
    showCompletion.value = true
    // Small delay to let completion view render, then start animation
    setTimeout(() => {
      showCompletionAnimation.value = true
    }, 50)
    // Animation stays visible (doesn't hide) - it becomes the completion icon
  }

  // Set installing tasks (called when installation starts)
  function setInstallingTasks(tasks: InstallTask[]) {
    installingTasks.value = tasks
  }

  // Clear installation result
  function clearInstallResult() {
    installResult.value = null
    showCompletion.value = false
    showCompletionAnimation.value = false
    installingTasks.value = []
  }

  return {
    xplanePath,
    currentTasks,
    isAnalyzing,
    isInstalling,
    isAnalyzeInProgress,
    isWindows,
    isContextMenuRegistered,
    installPreferences,
    verificationPreferences,
    atomicInstallEnabled,
    deleteSourceAfterInstall,
    autoSortScenery,
    xplaneLaunchArgs,
    sceneryManagerHintVisible,
    sceneryManagerHintMessageKey,
    isConfirmationOpen,
    logLevel,
    taskStates,
    getTaskState,
    hasConflicts,
    hasSizeWarnings,
    allSizeWarningsConfirmed,
    enabledTasksCount,
    pendingCliArgs,
    installResult,
    showCompletion,
    showCompletionAnimation,
    installingTasks,
    isInitialized,
    initStore,
    setXplanePath,
    loadXplanePath,
    togglePreference,
    toggleVerificationPreference,
    toggleAtomicInstall,
    toggleDeleteSourceAfterInstall,
    toggleAutoSortScenery,
    setXplaneLaunchArgs,
    showSceneryManagerHint,
    dismissSceneryManagerHint,
    setConfirmationOpen,
    setLogLevel,
    setCurrentTasks,
    appendTasks,
    clearTasks,
    setTaskOverwrite,
    setGlobalOverwrite,
    getTaskOverwrite,
    getTasksWithOverwrite,
    setTaskSizeConfirmed,
    getTaskSizeConfirmed,
    confirmAllSizeWarnings,
    setTaskEnabled,
    getTaskEnabled,
    setAllTasksEnabled,
    setTaskBackupSettings,
    getTaskBackupSettings,
    setBackupNavdata,
    getBackupNavdata,
    setConfigFilePatterns,
    getConfigFilePatterns,
    setPendingCliArgs,
    addCliArgsToBatch,
    clearPendingCliArgs,
    setInstallResult,
    setInstallingTasks,
    clearInstallResult,
  }
})
