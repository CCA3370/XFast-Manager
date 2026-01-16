import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { AddonType, type InstallTask, type InstallResult } from '@/types'

export type LogLevel = 'basic' | 'full' | 'debug'

/** Delay to batch multiple CLI file selections (500ms) */
const CLI_ARGS_BATCH_DELAY_MS = 500

export const useAppStore = defineStore('app', () => {
  const xplanePath = ref<string>('')
  const currentTasks = ref<InstallTask[]>([])
  const isAnalyzing = ref(false)
  const isInstalling = ref(false)

  // Platform detection (initialized at app startup)
  const isWindows = ref(false)
  const isContextMenuRegistered = ref(false)

  // Log level setting (basic, full, debug)
  const logLevel = ref<LogLevel>('full')

  // Pending CLI arguments to be processed by Home.vue
  const pendingCliArgs = ref<string[] | null>(null)

  // Batch processing for CLI args (to handle multiple file selections)
  const cliArgsBatch = ref<string[]>([])
  let cliArgsBatchTimer: ReturnType<typeof setTimeout> | null = null

  // Installation result state
  const installResult = ref<InstallResult | null>(null)
  const showCompletion = ref(false)
  const showCompletionAnimation = ref(false)

  // Default: all enabled
  const installPreferences = ref<Record<AddonType, boolean>>({
    [AddonType.Aircraft]: true,
    [AddonType.Scenery]: true,
    [AddonType.SceneryLibrary]: true,
    [AddonType.Plugin]: true,
    [AddonType.Navdata]: true,
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

  // Overwrite settings per task (taskId -> shouldOverwrite)
  const overwriteSettings = ref<Record<string, boolean>>({})

  // Size confirmation per task (taskId -> confirmed)
  const sizeConfirmations = ref<Record<string, boolean>>({})

  // Task enabled state per task (taskId -> enabled), default all enabled
  const taskEnabledState = ref<Record<string, boolean>>({})

  // Backup settings per task (taskId -> { liveries: boolean, configFiles: boolean })
  const backupSettings = ref<Record<string, { liveries: boolean, configFiles: boolean }>>({})

  // Config file patterns for backup (stored in localStorage)
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
    return tasksWithWarnings.every(task => sizeConfirmations.value[task.id] === true)
  })

  // Get count of enabled tasks
  const enabledTasksCount = computed(() => {
    return currentTasks.value.filter(task => getTaskEnabled(task.id)).length
  })

  // Load settings with validation and error recovery
  const savedPath = localStorage.getItem('xplanePath')
  if (savedPath) xplanePath.value = savedPath

  const savedPrefs = localStorage.getItem('installPreferences')
  if (savedPrefs) {
    try {
      const parsed = JSON.parse(savedPrefs)
      // Validate that parsed data is an object
      if (typeof parsed === 'object' && parsed !== null) {
        installPreferences.value = { ...installPreferences.value, ...parsed }
      } else {
        console.warn('Invalid install preferences format, using defaults')
        localStorage.removeItem('installPreferences')
      }
    } catch (e) {
      console.error('Failed to parse install preferences, clearing corrupted data', e)
      localStorage.removeItem('installPreferences')
    }
  }

  // Load log level with validation
  const savedLogLevel = localStorage.getItem('logLevel')
  if (savedLogLevel && ['basic', 'full', 'debug'].includes(savedLogLevel)) {
    logLevel.value = savedLogLevel as LogLevel
  } else if (savedLogLevel) {
    console.warn('Invalid log level, using default')
    localStorage.removeItem('logLevel')
  }

  // Load config file patterns with validation
  const savedPatterns = localStorage.getItem('configFilePatterns')
  if (savedPatterns) {
    try {
      const parsed = JSON.parse(savedPatterns)
      // Validate that parsed data is an array of strings
      if (Array.isArray(parsed) && parsed.every(item => typeof item === 'string')) {
        configFilePatterns.value = parsed
      } else {
        console.warn('Invalid config file patterns format, using defaults')
        localStorage.removeItem('configFilePatterns')
      }
    } catch (e) {
      console.error('Failed to parse config file patterns, clearing corrupted data', e)
      localStorage.removeItem('configFilePatterns')
    }
  }

  // Load verification preferences with validation
  const savedVerificationPrefs = localStorage.getItem('verificationPreferences')
  if (savedVerificationPrefs) {
    try {
      const parsed = JSON.parse(savedVerificationPrefs)
      // Validate that parsed data is an object
      if (typeof parsed === 'object' && parsed !== null) {
        verificationPreferences.value = { ...verificationPreferences.value, ...parsed }
      } else {
        console.warn('Invalid verification preferences format, using defaults')
        localStorage.removeItem('verificationPreferences')
      }
    } catch (e) {
      console.error('Failed to parse verification preferences, clearing corrupted data', e)
      localStorage.removeItem('verificationPreferences')
    }
  }

  // Load atomic install setting
  const savedAtomicInstall = localStorage.getItem('atomicInstallEnabled')
  if (savedAtomicInstall !== null) {
    try {
      atomicInstallEnabled.value = JSON.parse(savedAtomicInstall)
    } catch (e) {
      console.error('Failed to parse atomic install setting, using default', e)
      localStorage.removeItem('atomicInstallEnabled')
    }
  }

  // Load delete source after install setting
  const savedDeleteSource = localStorage.getItem('deleteSourceAfterInstall')
  if (savedDeleteSource !== null) {
    try {
      deleteSourceAfterInstall.value = JSON.parse(savedDeleteSource)
    } catch (e) {
      console.error('Failed to parse delete source setting, using default', e)
      localStorage.removeItem('deleteSourceAfterInstall')
    }
  }

  function setXplanePath(path: string) {
    xplanePath.value = path
    localStorage.setItem('xplanePath', path)
  }

  function loadXplanePath() {
    // Already loaded in init, but kept for interface compatibility if needed
    const saved = localStorage.getItem('xplanePath')
    if (saved) {
      xplanePath.value = saved
    }
  }

  function togglePreference(type: AddonType) {
    installPreferences.value[type] = !installPreferences.value[type]
    localStorage.setItem('installPreferences', JSON.stringify(installPreferences.value))
  }

  function toggleVerificationPreference(sourceType: string) {
    verificationPreferences.value[sourceType] = !verificationPreferences.value[sourceType]
    localStorage.setItem('verificationPreferences', JSON.stringify(verificationPreferences.value))
  }

  function toggleAtomicInstall() {
    atomicInstallEnabled.value = !atomicInstallEnabled.value
    localStorage.setItem('atomicInstallEnabled', JSON.stringify(atomicInstallEnabled.value))
  }

  function toggleDeleteSourceAfterInstall() {
    deleteSourceAfterInstall.value = !deleteSourceAfterInstall.value
    localStorage.setItem('deleteSourceAfterInstall', JSON.stringify(deleteSourceAfterInstall.value))
  }

  function setLogLevel(level: LogLevel) {
    logLevel.value = level
    localStorage.setItem('logLevel', level)
  }

  function setCurrentTasks(tasks: InstallTask[]) {
    currentTasks.value = tasks
    // Reset overwrite settings, size confirmations, backup settings, and enable all tasks by default
    overwriteSettings.value = {}
    sizeConfirmations.value = {}
    taskEnabledState.value = {}
    backupSettings.value = {}
    // Enable all tasks by default and initialize backup settings for Aircraft
    tasks.forEach(task => {
      taskEnabledState.value[task.id] = true
      if (task.type === AddonType.Aircraft) {
        backupSettings.value[task.id] = { liveries: true, configFiles: true }
      }
    })
  }

  function clearTasks() {
    currentTasks.value = []
    overwriteSettings.value = {}
    sizeConfirmations.value = {}
    taskEnabledState.value = {}
    backupSettings.value = {}
  }

  // Set overwrite for a specific task
  function setTaskOverwrite(taskId: string, shouldOverwrite: boolean) {
    overwriteSettings.value[taskId] = shouldOverwrite
  }

  // Set overwrite for all conflicting tasks
  function setGlobalOverwrite(shouldOverwrite: boolean) {
    for (const task of currentTasks.value) {
      if (task.conflictExists) {
        overwriteSettings.value[task.id] = shouldOverwrite
      }
    }
  }

  // Get overwrite setting for a task
  function getTaskOverwrite(taskId: string): boolean {
    return overwriteSettings.value[taskId] ?? false
  }

  // Prepare tasks with overwrite, size confirmation, and backup settings for installation
  function getTasksWithOverwrite(): InstallTask[] {
    return currentTasks.value.map(task => ({
      ...task,
      shouldOverwrite: overwriteSettings.value[task.id] ?? false,
      sizeConfirmed: sizeConfirmations.value[task.id] ?? false,
      backupLiveries: backupSettings.value[task.id]?.liveries ?? true,
      // Only enable config file backup if patterns are configured
      backupConfigFiles: (configFilePatterns.value.length > 0) && (backupSettings.value[task.id]?.configFiles ?? true),
      configFilePatterns: configFilePatterns.value,
    }))
  }

  // Set size confirmation for a specific task
  function setTaskSizeConfirmed(taskId: string, confirmed: boolean) {
    sizeConfirmations.value[taskId] = confirmed
  }

  // Get size confirmation for a task
  function getTaskSizeConfirmed(taskId: string): boolean {
    return sizeConfirmations.value[taskId] ?? false
  }

  // Confirm all size warnings at once
  function confirmAllSizeWarnings(confirmed: boolean) {
    for (const task of currentTasks.value) {
      if (task.sizeWarning) {
        sizeConfirmations.value[task.id] = confirmed
      }
    }
  }

  // Set task enabled state
  function setTaskEnabled(taskId: string, enabled: boolean) {
    taskEnabledState.value[taskId] = enabled
  }

  // Get task enabled state (default true)
  function getTaskEnabled(taskId: string): boolean {
    return taskEnabledState.value[taskId] ?? true
  }

  // Toggle all tasks enabled/disabled
  function setAllTasksEnabled(enabled: boolean) {
    for (const task of currentTasks.value) {
      taskEnabledState.value[task.id] = enabled
    }
  }

  // Set backup settings for a specific task
  function setTaskBackupSettings(taskId: string, liveries: boolean, configFiles: boolean) {
    backupSettings.value[taskId] = { liveries, configFiles }
  }

  // Get backup settings for a task (default both true)
  function getTaskBackupSettings(taskId: string): { liveries: boolean, configFiles: boolean } {
    return backupSettings.value[taskId] ?? { liveries: true, configFiles: true }
  }

  // Set config file patterns
  function setConfigFilePatterns(patterns: string[]) {
    configFilePatterns.value = patterns
    localStorage.setItem('configFilePatterns', JSON.stringify(patterns))
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
  function addCliArgsToBatch(args: string[]) {
    // Add new args to batch
    cliArgsBatch.value.push(...args)

    // Clear existing timer
    if (cliArgsBatchTimer) {
      clearTimeout(cliArgsBatchTimer)
    }

    // Set new timer to process batch after delay
    cliArgsBatchTimer = setTimeout(() => {
      if (cliArgsBatch.value.length > 0) {
        // Remove duplicates
        const uniqueArgs = Array.from(new Set(cliArgsBatch.value)) as string[]
        setPendingCliArgs(uniqueArgs)
        cliArgsBatch.value = []
      }
      cliArgsBatchTimer = null
    }, CLI_ARGS_BATCH_DELAY_MS)
  }

  // Clear pending CLI args after processing
  function clearPendingCliArgs() {
    pendingCliArgs.value = null
    // Also clear any pending batch timer
    if (cliArgsBatchTimer) {
      clearTimeout(cliArgsBatchTimer)
      cliArgsBatchTimer = null
    }
    cliArgsBatch.value = []
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

  // Clear installation result
  function clearInstallResult() {
    installResult.value = null
    showCompletion.value = false
    showCompletionAnimation.value = false
  }

  return {
    xplanePath,
    currentTasks,
    isAnalyzing,
    isInstalling,
    isWindows,
    isContextMenuRegistered,
    installPreferences,
    verificationPreferences,
    atomicInstallEnabled,
    deleteSourceAfterInstall,
    logLevel,
    overwriteSettings,
    sizeConfirmations,
    taskEnabledState,
    hasConflicts,
    hasSizeWarnings,
    allSizeWarningsConfirmed,
    enabledTasksCount,
    pendingCliArgs,
    installResult,
    showCompletion,
    showCompletionAnimation,
    setXplanePath,
    loadXplanePath,
    togglePreference,
    toggleVerificationPreference,
    toggleAtomicInstall,
    toggleDeleteSourceAfterInstall,
    setLogLevel,
    setCurrentTasks,
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
    setConfigFilePatterns,
    getConfigFilePatterns,
    setPendingCliArgs,
    addCliArgsToBatch,
    clearPendingCliArgs,
    setInstallResult,
    clearInstallResult,
  }
})
