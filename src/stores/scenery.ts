import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  SceneryIndexStatus,
  SceneryManagerData,
  SceneryManagerEntry,
  SceneryCategory,
} from '@/types'
import { parseApiError, getErrorMessage } from '@/types'
import { useAppStore } from './app'
import { useToastStore } from './toast'
import { useManagementStore } from './management'
import { logError } from '@/services/logger'
import { getItem, setItem, STORAGE_KEYS } from '@/services/storage'
import { validateXPlanePath } from '@/utils/validation'
import { i18n } from '@/i18n'
import type { SceneryCustomGroup, SceneryCustomGroupConfig } from '@/utils/scenerySmartGroups'

const SCENERY_CUSTOM_GROUP_CONFIG_VERSION = 1

type SceneryCustomGroupConfigsByPath = Record<string, SceneryCustomGroupConfig>

function createEmptyCustomGroupConfig(): SceneryCustomGroupConfig {
  return {
    version: SCENERY_CUSTOM_GROUP_CONFIG_VERSION,
    groups: [],
  }
}

function normalizeCustomGroupConfig(value: unknown): SceneryCustomGroupConfig {
  if (!value || typeof value !== 'object') {
    return createEmptyCustomGroupConfig()
  }

  const candidate = value as Partial<SceneryCustomGroupConfig>
  if (
    candidate.version !== SCENERY_CUSTOM_GROUP_CONFIG_VERSION ||
    !Array.isArray(candidate.groups)
  ) {
    return createEmptyCustomGroupConfig()
  }

  const groups: SceneryCustomGroup[] = []
  const seenGroupIds = new Set<string>()
  const seenManualNames = new Set<string>()
  for (const group of candidate.groups) {
    if (!group || typeof group !== 'object') continue

    const rawGroup = group as Partial<SceneryCustomGroup>
    const id = typeof rawGroup.id === 'string' ? rawGroup.id.trim() : ''
    const name = typeof rawGroup.name === 'string' ? rawGroup.name.trim() : ''
    if (!id || !name || seenGroupIds.has(id)) continue

    seenGroupIds.add(id)
    const manualFolderNames = (
      Array.isArray(rawGroup.manualFolderNames) ? rawGroup.manualFolderNames : []
    )
      .filter((folderName): folderName is string => typeof folderName === 'string')
      .map((folderName) => folderName.trim())
      .filter((folderName) => {
        if (!folderName || seenManualNames.has(folderName)) return false
        seenManualNames.add(folderName)
        return true
      })

    const seenRuleKeys = new Set<string>()
    const rules = (Array.isArray(rawGroup.rules) ? rawGroup.rules : [])
      .filter((rule): rule is NonNullable<SceneryCustomGroup['rules'][number]> => {
        if (!rule || typeof rule !== 'object') return false
        const rawRule = rule as Partial<SceneryCustomGroup['rules'][number]>
        return (
          typeof rawRule.id === 'string' &&
          (rawRule.mode === 'prefix' || rawRule.mode === 'contains') &&
          typeof rawRule.pattern === 'string'
        )
      })
      .map((rule) => ({
        id: rule.id.trim(),
        mode: rule.mode,
        pattern: rule.pattern.trim(),
      }))
      .filter((rule) => {
        if (!rule.id || !rule.pattern) return false
        const key = `${rule.mode}:${rule.pattern.toLowerCase()}`
        if (seenRuleKeys.has(key)) return false
        seenRuleKeys.add(key)
        return true
      })

    groups.push({ id, name, manualFolderNames, rules })
  }

  return {
    version: SCENERY_CUSTOM_GROUP_CONFIG_VERSION,
    groups,
  }
}

function getCustomGroupConfigPathKey(xplanePath: string): string {
  return xplanePath.trim().replaceAll('\\', '/')
}

export const useSceneryStore = defineStore('scenery', () => {
  const appStore = useAppStore()
  const t = i18n.global.t

  // State
  const data = ref<SceneryManagerData | null>(null)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const isCheckingUpdates = ref(false)
  const error = ref<string | null>(null)
  const indexExists = ref(false)
  const needsDatabaseReset = ref(false)
  const customGroupConfigsByPath = ref<SceneryCustomGroupConfigsByPath>({})

  // Track original state for change detection
  const originalEntries = ref<SceneryManagerEntry[]>([])

  // Collapsed groups state (persisted to Tauri Store)
  // Default: all groups are expanded (false = expanded, true = collapsed)
  const collapsedGroups = ref<Record<SceneryCategory, boolean>>(
    {} as Record<SceneryCategory, boolean>,
  )

  // Load collapsed groups from storage
  async function initStore(): Promise<void> {
    const saved = await getItem<Record<SceneryCategory, boolean>>(
      STORAGE_KEYS.SCENERY_GROUPS_COLLAPSED,
    )
    if (saved && typeof saved === 'object') {
      collapsedGroups.value = saved
    }

    const savedCustomGroups = await getItem<SceneryCustomGroupConfigsByPath>(
      STORAGE_KEYS.SCENERY_CUSTOM_GROUPS_BY_PATH,
    )
    if (savedCustomGroups && typeof savedCustomGroups === 'object') {
      const normalized: SceneryCustomGroupConfigsByPath = {}
      for (const [pathKey, config] of Object.entries(savedCustomGroups)) {
        normalized[pathKey] = normalizeCustomGroupConfig(config)
      }
      customGroupConfigsByPath.value = normalized
    }
  }

  // Watch for changes and persist to Tauri Store
  watch(
    collapsedGroups,
    (newVal) => {
      setItem(STORAGE_KEYS.SCENERY_GROUPS_COLLAPSED, newVal)
    },
    { deep: true },
  )

  watch(
    customGroupConfigsByPath,
    (newVal) => {
      setItem(STORAGE_KEYS.SCENERY_CUSTOM_GROUPS_BY_PATH, newVal)
    },
    { deep: true },
  )

  // Computed properties
  const entries = computed(() => data.value?.entries ?? [])
  const totalCount = computed(() => data.value?.totalCount ?? 0)
  const enabledCount = computed(() => data.value?.enabledCount ?? 0)
  const updateCount = computed(() => entries.value.filter((e) => e.hasUpdate).length)
  const missingDepsCount = computed(() => data.value?.missingDepsCount ?? 0)
  const duplicateTilesCount = computed(() => data.value?.duplicateTilesCount ?? 0)
  const duplicateAirportsCount = computed(() => data.value?.duplicateAirportsCount ?? 0)
  const duplicatesCount = computed(() => {
    if (!data.value) return 0
    return data.value.entries.filter(
      (e) => e.duplicateTiles?.length > 0 || e.duplicateAirports?.length > 0,
    ).length
  })

  // Sort entries by sortOrder
  const sortedEntries = computed(() => {
    return [...entries.value].sort((a, b) => a.sortOrder - b.sortOrder)
  })

  const currentCustomGroupConfig = computed<SceneryCustomGroupConfig>(() => {
    if (!appStore.xplanePath) return createEmptyCustomGroupConfig()

    const pathKey = getCustomGroupConfigPathKey(appStore.xplanePath)
    return customGroupConfigsByPath.value[pathKey] ?? createEmptyCustomGroupConfig()
  })

  const customGroups = computed(() => currentCustomGroupConfig.value.groups)

  // Group entries by category
  const groupedEntries = computed(() => {
    const groups: Record<SceneryCategory, SceneryManagerEntry[]> = {
      FixedHighPriority: [],
      Airport: [],
      DefaultAirport: [],
      Library: [],
      Other: [],
      Overlay: [],
      AirportMesh: [],
      Mesh: [],
      Unrecognized: [],
    }

    for (const entry of sortedEntries.value) {
      groups[entry.category].push(entry)
    }

    return groups
  })

  // Check if there are unsaved changes (either local changes or index differs from ini)
  // Optimized with Map for O(1) lookups instead of O(n) .find() in loop
  const hasChanges = computed(() => {
    // If index differs from ini, we have changes to apply
    if (data.value?.needsSync) return true

    return hasLocalChanges.value
  })

  // Check if user has made local modifications (separate from needsSync)
  const hasLocalChanges = computed(() => {
    if (!data.value || originalEntries.value.length === 0) return false

    const current = entries.value
    if (current.length !== originalEntries.value.length) return true

    // Build a Map from original entries for O(1) lookup
    const originalMap = new Map(originalEntries.value.map((e) => [e.folderName, e]))

    for (const curr of current) {
      const orig = originalMap.get(curr.folderName)
      if (!orig) return true
      if (curr.enabled !== orig.enabled || curr.sortOrder !== orig.sortOrder) {
        return true
      }
    }

    return false
  })

  // Load scenery data from backend
  async function loadData() {
    if (!validateXPlanePath(error)) {
      indexExists.value = false
      return
    }

    // Set isLoading before any async operations
    isLoading.value = true
    error.value = null

    try {
      // Load index status first
      await loadIndexStatus()

      const result = await invoke<SceneryManagerData>('get_scenery_manager_data', {
        xplanePath: appStore.xplanePath,
      })
      data.value = result
      // Store original state for change detection
      originalEntries.value = JSON.parse(JSON.stringify(result.entries))
      // Clear any previous database reset flag on successful load
      needsDatabaseReset.value = false
      // Kick off a background SkunkCrafts update check — non-blocking so the UI
      // renders immediately while remote versions trickle in.
      void checkSceneryUpdates()
    } catch (e) {
      const errorStr = String(e)
      error.value = errorStr
      logError(`Failed to load scenery data: ${e}`, 'scenery')

      // Check if this is a schema incompatibility error (old database missing columns,
      // or a newer database version than what the current code supports).
      if (
        (errorStr.includes('migration_failed') && errorStr.includes('newer than supported')) ||
        errorStr.includes('no column found for name') ||
        errorStr.includes('no column for name')
      ) {
        needsDatabaseReset.value = true
      }
    } finally {
      isLoading.value = false
    }
  }

  async function loadIndexStatus() {
    if (!validateXPlanePath()) {
      indexExists.value = false
      return
    }

    try {
      const status = await invoke<SceneryIndexStatus>('get_scenery_index_status', {
        xplanePath: appStore.xplanePath,
      })
      indexExists.value = status.indexExists
    } catch (e) {
      indexExists.value = false
      logError(`Failed to load scenery index status: ${e}`, 'scenery')
    }
  }

  // Check remote SkunkCrafts versions for the currently-loaded scenery entries.
  // Mirrors checkAircraftUpdates / checkPluginsUpdates in the management store —
  // shares the same in-memory update cache so refresh-cycles are deduped across
  // tabs.
  async function checkSceneryUpdates(
    forceRefresh: boolean = false,
    showUpToDateToast: boolean = false,
  ) {
    if (!data.value || data.value.entries.length === 0) return
    if (!validateXPlanePath()) return

    const managementStore = useManagementStore()
    const toastStore = useToastStore()
    await managementStore.loadAddonUpdateOptions()

    // forceRefresh: rescan local data first so any cfg version edits on disk are
    // picked up before re-checking remote.
    if (forceRefresh) {
      try {
        const result = await invoke<SceneryManagerData>('get_scenery_manager_data', {
          xplanePath: appStore.xplanePath,
        })
        data.value = result
        originalEntries.value = JSON.parse(JSON.stringify(result.entries))
      } catch (e) {
        logError(`Failed to rescan scenery: ${e}`, 'scenery')
      }
    }

    // Use a Ref view of data.value.entries so the generic helper can mutate in place.
    const entriesRef = computed({
      get: () => data.value?.entries ?? [],
      set: (next: SceneryManagerEntry[]) => {
        if (data.value) data.value.entries = next
      },
    })

    isCheckingUpdates.value = true
    try {
      const result = await managementStore.checkItemUpdates<SceneryManagerEntry>({
        itemsRef: entriesRef,
        checkCommand: 'check_scenery_updates',
        checkParamName: 'scenery',
        logName: 'scenery',
        itemType: 'scenery',
        extraArgs: { xplanePath: appStore.xplanePath },
      })
      if (showUpToDateToast && result.checked && result.updateCount === 0) {
        toastStore.info(t('management.allUpToDate'))
      }
    } finally {
      isCheckingUpdates.value = false
    }
  }

  // Reset the database schema in-place and clear local state.
  async function resetDatabase() {
    try {
      await invoke('reset_and_reinitialize')
      needsDatabaseReset.value = false
      error.value = null
      indexExists.value = false
      data.value = null
      originalEntries.value = []
      return true
    } catch (e) {
      logError(`Failed to reset database: ${e}`, 'scenery')
      error.value = String(e)
      return false
    }
  }

  // Toggle enabled state for an entry (local only, no backend write)
  function toggleEnabled(folderName: string) {
    if (!data.value) return

    const entry = data.value.entries.find((e) => e.folderName === folderName)
    if (!entry) return

    // Update locally only - will be persisted when user clicks Apply
    entry.enabled = !entry.enabled

    // Update enabled count
    data.value.enabledCount = data.value.entries.filter((e) => e.enabled).length
  }

  // Update category for an entry
  async function updateCategory(folderName: string, newCategory: SceneryCategory) {
    if (!data.value) return

    const entry = data.value.entries.find((e) => e.folderName === folderName)
    if (!entry) return

    const oldCategory = entry.category

    try {
      // Update locally first for immediate UI feedback
      entry.category = newCategory

      // Update in backend
      await invoke('update_scenery_entry', {
        xplanePath: appStore.xplanePath,
        folderName,
        enabled: null,
        sortOrder: null,
        category: newCategory,
      })
    } catch (e) {
      // Revert on error
      entry.category = oldCategory
      error.value = String(e)
      logError(`Failed to update category: ${e}`, 'scenery')
      throw e
    }
  }

  // Recalculate duplicate tiles based on raw tile overlaps and current sort order.
  // This enables real-time conflict display when entries are reordered.
  // Creates new entry objects for changed entries to force Vue reactivity propagation
  // through computed properties (groupedEntries) into the template.
  let recalcTimer: ReturnType<typeof setTimeout> | null = null

  function recalcDuplicateTiles() {
    if (!data.value) return
    const overlaps = data.value.tileOverlaps
    if (!overlaps || Object.keys(overlaps).length === 0) return

    const entryMap = new Map(data.value.entries.map((e) => [e.folderName, e]))
    let anyChanged = false

    const newEntries = data.value.entries.map((entry) => {
      let newTiles: string[]

      if (entry.folderName.startsWith('XPME_')) {
        newTiles = []
      } else {
        const rawOverlaps = overlaps[entry.folderName]
        if (!rawOverlaps || rawOverlaps.length === 0) {
          newTiles = []
        } else {
          newTiles = rawOverlaps.filter((other) => {
            const otherEntry = entryMap.get(other)
            if (!otherEntry) return false
            if (other.startsWith('XPME_')) {
              return entry.sortOrder > otherEntry.sortOrder
            }
            return true
          })
        }
      }

      // Only create a new object when duplicateTiles actually differs
      // Note: duplicateTiles may be undefined — backend omits empty arrays via skip_serializing_if
      const prev = entry.duplicateTiles ?? []
      if (prev.length !== newTiles.length || prev.some((v, i) => v !== newTiles[i])) {
        anyChanged = true
        return { ...entry, duplicateTiles: newTiles }
      }
      return entry
    })

    if (anyChanged) {
      data.value.entries = newEntries
      data.value.duplicateTilesCount = newEntries.filter(
        (e) => (e.duplicateTiles ?? []).length > 0,
      ).length
    }
  }

  // Debounced, deferred recalculation — yields to the UI thread first
  function scheduleRecalcDuplicateTiles() {
    if (recalcTimer !== null) {
      clearTimeout(recalcTimer)
    }
    recalcTimer = setTimeout(() => {
      recalcTimer = null
      recalcDuplicateTiles()
    }, 0)
  }

  // Apply a local sort order without persisting immediately.
  // Mutates sortOrder in-place to avoid creating new objects and triggering
  // a full re-render of the entire list.
  // Runs recalcDuplicateTiles synchronously so that callers (e.g. handleDragEnd)
  // get entries with up-to-date duplicateTiles before syncLocalEntries copies them.
  function applyLocalOrder(newOrder: SceneryManagerEntry[]) {
    if (!data.value) return
    for (let i = 0; i < newOrder.length; i++) {
      newOrder[i].sortOrder = i
    }
    data.value.entries = newOrder
    recalcDuplicateTiles()
  }

  // Move an entry locally to a new position (no persistence until apply)
  async function moveEntry(folderName: string, newSortOrder: number) {
    if (!data.value) return

    const ordered = [...sortedEntries.value]
    const currentIndex = ordered.findIndex((e) => e.folderName === folderName)
    if (currentIndex === -1) return

    const targetIndex = Math.min(Math.max(newSortOrder, 0), ordered.length - 1)
    const [moved] = ordered.splice(currentIndex, 1)
    ordered.splice(targetIndex, 0, moved)
    applyLocalOrder(ordered)
  }

  // Reorder entries after drag-and-drop (staged locally)
  async function reorderEntries(newOrder: SceneryManagerEntry[]) {
    applyLocalOrder(newOrder)
  }

  // Apply changes to scenery_packs.ini
  async function applyChanges(orderedEntries?: SceneryManagerEntry[]) {
    // Prevent concurrent calls (race condition protection)
    if (isSaving.value) return

    if (!appStore.xplanePath || !data.value) {
      error.value = 'X-Plane path not set'
      return
    }

    isSaving.value = true
    error.value = null

    try {
      const entriesToApply = orderedEntries
        ? orderedEntries.slice()
        : data.value.entries.slice().sort((a, b) => a.sortOrder - b.sortOrder)

      // Ensure sortOrder fields are aligned with the order being applied
      const normalizedEntries = entriesToApply.map((entry, index) => ({
        ...entry,
        sortOrder: index,
      }))

      // Update local data with normalized sortOrder
      data.value.entries = normalizedEntries

      // Send only necessary fields to backend for batch update
      const updates = normalizedEntries.map((entry) => ({
        folderName: entry.folderName,
        enabled: entry.enabled,
        sortOrder: entry.sortOrder,
      }))

      await invoke('apply_scenery_changes', {
        xplanePath: appStore.xplanePath,
        entries: updates,
      })

      // Update original state after successful save
      originalEntries.value = JSON.parse(JSON.stringify(normalizedEntries))
      // Mark as synced since we just wrote to ini
      data.value.needsSync = false
    } catch (e) {
      error.value = String(e)
      logError(`Failed to apply changes: ${e}`, 'scenery')
      throw e
    } finally {
      isSaving.value = false
    }
  }

  // Reset to original state
  function resetChanges() {
    if (originalEntries.value.length > 0 && data.value) {
      data.value.entries = JSON.parse(JSON.stringify(originalEntries.value))
      data.value.enabledCount = data.value.entries.filter((e) => e.enabled).length
    }
  }

  // Delete a scenery entry (folder)
  async function deleteEntry(folderName: string) {
    if (!validateXPlanePath(error)) {
      throw new Error(error.value!)
    }

    try {
      await invoke('delete_scenery_folder', {
        xplanePath: appStore.xplanePath,
        folderName,
      })

      // Remove from local data
      if (data.value) {
        data.value.entries = data.value.entries.filter((e) => e.folderName !== folderName)

        // Recalculate sortOrder to eliminate gaps
        // Sort by current sortOrder first, then reassign consecutive values
        data.value.entries = data.value.entries
          .sort((a, b) => a.sortOrder - b.sortOrder)
          .map((entry, index) => ({
            ...entry,
            sortOrder: index,
          }))

        data.value.totalCount = data.value.entries.length
        data.value.enabledCount = data.value.entries.filter((e) => e.enabled).length
        data.value.missingDepsCount = data.value.entries.filter(
          (e) => e.missingLibraries.length > 0,
        ).length
        scheduleRecalcDuplicateTiles()
      }

      // Also remove from original entries and recalculate their sortOrder
      originalEntries.value = originalEntries.value
        .filter((e) => e.folderName !== folderName)
        .sort((a, b) => a.sortOrder - b.sortOrder)
        .map((entry, index) => ({
          ...entry,
          sortOrder: index,
        }))
    } catch (e) {
      // Parse structured error if available
      const apiError = parseApiError(e)
      if (apiError) {
        error.value = apiError.message
        logError(
          `Failed to delete scenery entry [${apiError.code}]: ${apiError.message}`,
          'scenery',
        )

        // Rethrow with structured error info for UI handling
        throw { ...apiError, isApiError: true }
      } else {
        error.value = getErrorMessage(e)
        logError(`Failed to delete scenery entry: ${error.value}`, 'scenery')
        throw e
      }
    }
  }

  function updateFlattenState(folderName: string, flattened: boolean, flattenAvailable = true) {
    if (!data.value) return

    const entry = data.value.entries.find((item) => item.folderName === folderName)
    if (!entry) return

    entry.flattened = flattened
    entry.flattenAvailable = flattenAvailable
  }

  function setCurrentCustomGroupConfig(config: SceneryCustomGroupConfig) {
    if (!appStore.xplanePath) return

    const pathKey = getCustomGroupConfigPathKey(appStore.xplanePath)
    customGroupConfigsByPath.value = {
      ...customGroupConfigsByPath.value,
      [pathKey]: normalizeCustomGroupConfig(config),
    }
  }

  function upsertCustomGroups(groups: SceneryCustomGroup[]) {
    setCurrentCustomGroupConfig({
      version: SCENERY_CUSTOM_GROUP_CONFIG_VERSION,
      groups,
    })
  }

  // Clear store state
  function clear() {
    data.value = null
    originalEntries.value = []
    error.value = null
    needsDatabaseReset.value = false
  }

  return {
    // State
    data,
    isLoading,
    isSaving,
    isCheckingUpdates,
    error,
    collapsedGroups,
    customGroups,
    currentCustomGroupConfig,
    needsDatabaseReset,

    // Computed
    entries,
    sortedEntries,
    groupedEntries,
    totalCount,
    enabledCount,
    updateCount,
    missingDepsCount,
    duplicateTilesCount,
    duplicateAirportsCount,
    duplicatesCount,
    hasChanges,
    hasLocalChanges,
    indexExists,

    // Actions
    initStore,
    loadData,
    loadIndexStatus,
    checkSceneryUpdates,
    resetDatabase,
    toggleEnabled,
    updateCategory,
    moveEntry,
    reorderEntries,
    applyChanges,
    resetChanges,
    deleteEntry,
    updateFlattenState,
    upsertCustomGroups,
    clear,
  }
})
