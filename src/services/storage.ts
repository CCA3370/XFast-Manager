import { Store } from '@tauri-apps/plugin-store'

let store: Store | null = null

/**
 * Initialize the Tauri store. Must be called before using other storage functions.
 */
export async function initStorage(): Promise<void> {
  if (store) return
  store = await Store.load('settings.json')
}

/**
 * Get a value from storage.
 *
 * @param key - The storage key to retrieve
 * @returns The stored value or null if not found
 *
 * @example
 * ```typescript
 * const xplanePath = await getItem<string>(STORAGE_KEYS.XPLANE_PATH)
 * ```
 */
export async function getItem<T>(key: string): Promise<T | null> {
  if (!store) {
    await initStorage()
  }
  const value = await store!.get<T>(key)
  return value ?? null
}

/**
 * Set a value in storage.
 *
 * @param key - The storage key
 * @param value - The value to store
 *
 * @example
 * ```typescript
 * await setItem(STORAGE_KEYS.XPLANE_PATH, '/path/to/xplane')
 * ```
 */
export async function setItem<T>(key: string, value: T): Promise<void> {
  if (!store) {
    await initStorage()
  }
  await store!.set(key, value)
}

/**
 * Remove a value from storage.
 *
 * @param key - The storage key to remove
 */
export async function removeItem(key: string): Promise<void> {
  if (!store) {
    await initStorage()
  }
  await store!.delete(key)
}

/**
 * Clear all storage.
 */
export async function clearStorage(): Promise<void> {
  if (!store) {
    await initStorage()
  }
  await store!.clear()
}

/**
 * Get all keys in storage.
 *
 * @returns Array of all storage keys
 */
export async function getAllKeys(): Promise<string[]> {
  if (!store) {
    await initStorage()
  }
  return await store!.keys()
}

/**
 * Check if a key exists in storage.
 *
 * @param key - The storage key to check
 * @returns true if the key exists, false otherwise
 */
export async function hasKey(key: string): Promise<boolean> {
  if (!store) {
    await initStorage()
  }
  return await store!.has(key)
}

// Storage keys constants
/**
 * Centralized storage keys to prevent typos and ensure consistency.
 * Use these constants instead of hardcoded strings.
 */
export const STORAGE_KEYS = {
  XPLANE_PATH: 'xplanePath',
  INSTALL_PREFERENCES: 'installPreferences',
  VERIFICATION_PREFERENCES: 'verificationPreferences',
  ATOMIC_INSTALL_ENABLED: 'atomicInstallEnabled',
  DELETE_SOURCE_AFTER_INSTALL: 'deleteSourceAfterInstall',
  AUTO_SORT_SCENERY: 'autoSortScenery',
  LOG_LEVEL: 'logLevel',
  CONFIG_FILE_PATTERNS: 'configFilePatterns',
  THEME: 'theme',
  LOCKED_ITEMS: 'lockedItems',
  SCENERY_GROUPS_COLLAPSED: 'sceneryGroupsCollapsed',
  ONBOARDING_COMPLETED: 'onboardingCompleted',
  SCENERY_AUTO_SORT_HINT_SHOWN: 'sceneryAutoSortHintShown',
  LOG_ANALYSIS_HINT_SHOWN: 'logAnalysisHintShown',
  AUTO_CHECK_ENABLED: 'autoCheckEnabled',
  INCLUDE_PRE_RELEASE: 'includePreRelease',
  LAST_CHECK_TIME: 'lastCheckTime',
  LAST_SHOWN_CHANGELOG_VERSION: 'lastShownChangelogVersion',
  XPLANE_LAUNCH_ARGS: 'xplaneLaunchArgs',
  PARALLEL_INSTALL_ENABLED: 'parallelInstallEnabled',
  MAX_PARALLEL_TASKS: 'maxParallelTasks',
  REPORTED_ISSUES: 'reportedIssues',
  UNCONFIRMED_ISSUE_UPDATES: 'unconfirmedIssueUpdates',
  CRASH_ANALYSIS_DMP_ENABLED: 'crashAnalysisDmpEnabled',
  CRASH_ANALYSIS_IGNORE_DATE_CHECK: 'crashAnalysisIgnoreDateCheck',
} as const

export interface TrackedIssue {
  issueNumber: number
  issueTitle: string
  issueUrl: string
  state: 'open' | 'closed'
  commentCount: number
  reportedAt: string
  lastCheckedAt: string
}
