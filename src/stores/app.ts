import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { AddonType, type InstallTask } from '@/types'

export type LogLevel = 'basic' | 'full' | 'debug'

export const useAppStore = defineStore('app', () => {
  const xplanePath = ref<string>('')
  const currentTasks = ref<InstallTask[]>([])
  const isAnalyzing = ref(false)
  const isInstalling = ref(false)

  // Log level setting (basic, full, debug)
  const logLevel = ref<LogLevel>('full')

  // Pending CLI arguments to be processed by Home.vue
  const pendingCliArgs = ref<string[] | null>(null)

  // Default: all enabled
  const installPreferences = ref<Record<AddonType, boolean>>({
    [AddonType.Aircraft]: true,
    [AddonType.Scenery]: true,
    [AddonType.SceneryLibrary]: true,
    [AddonType.Plugin]: true,
    [AddonType.Navdata]: true,
  })

  // Overwrite settings per task (taskId -> shouldOverwrite)
  const overwriteSettings = ref<Record<string, boolean>>({})

  // Size confirmation per task (taskId -> confirmed)
  const sizeConfirmations = ref<Record<string, boolean>>({})

  // Task enabled state per task (taskId -> enabled), default all enabled
  const taskEnabledState = ref<Record<string, boolean>>({})

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

  // Load settings
  const savedPath = localStorage.getItem('xplanePath')
  if (savedPath) xplanePath.value = savedPath

  const savedPrefs = localStorage.getItem('installPreferences')
  if (savedPrefs) {
    try {
      installPreferences.value = { ...installPreferences.value, ...JSON.parse(savedPrefs) }
    } catch (e) {
      console.error('Failed to parse install preferences', e)
    }
  }

  // Load log level
  const savedLogLevel = localStorage.getItem('logLevel')
  if (savedLogLevel && ['basic', 'full', 'debug'].includes(savedLogLevel)) {
    logLevel.value = savedLogLevel as LogLevel
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

  function setLogLevel(level: LogLevel) {
    logLevel.value = level
    localStorage.setItem('logLevel', level)
  }

  function setCurrentTasks(tasks: InstallTask[]) {
    currentTasks.value = tasks
    // Reset overwrite settings, size confirmations, and enable all tasks by default
    overwriteSettings.value = {}
    sizeConfirmations.value = {}
    taskEnabledState.value = {}
    // Enable all tasks by default
    tasks.forEach(task => {
      taskEnabledState.value[task.id] = true
    })
  }

  function clearTasks() {
    currentTasks.value = []
    overwriteSettings.value = {}
    sizeConfirmations.value = {}
    taskEnabledState.value = {}
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

  // Prepare tasks with overwrite and size confirmation settings for installation
  function getTasksWithOverwrite(): InstallTask[] {
    return currentTasks.value.map(task => ({
      ...task,
      shouldOverwrite: overwriteSettings.value[task.id] ?? false,
      sizeConfirmed: sizeConfirmations.value[task.id] ?? false
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

  // Set pending CLI args for Home.vue to process
  function setPendingCliArgs(args: string[]) {
    pendingCliArgs.value = args
  }

  // Clear pending CLI args after processing
  function clearPendingCliArgs() {
    pendingCliArgs.value = null
  }

  return {
    xplanePath,
    currentTasks,
    isAnalyzing,
    isInstalling,
    installPreferences,
    logLevel,
    overwriteSettings,
    sizeConfirmations,
    taskEnabledState,
    hasConflicts,
    hasSizeWarnings,
    allSizeWarningsConfirmed,
    enabledTasksCount,
    pendingCliArgs,
    setXplanePath,
    loadXplanePath,
    togglePreference,
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
    setPendingCliArgs,
    clearPendingCliArgs,
  }
})
