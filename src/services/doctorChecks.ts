import { invoke } from '@tauri-apps/api/core'
import type {
  AircraftInfo,
  AddonUpdatableItemType,
  CslScanResult,
  DoctorEnvironmentReport,
  DoctorNavdataReport,
  DoctorXfastHealthReport,
  GatewayInstalledAirport,
  PluginInfo,
  SceneryManagerData,
  SceneryManagerEntry,
} from '@/types'
import type {
  DoctorCheckResult,
  DoctorRunMode,
  DoctorSection,
  DoctorSystemSnapshot,
} from '@/types/doctor'
import { getItem, STORAGE_KEYS } from '@/services/storage'
import {
  hasAddonUpdateBetaPreference,
  normalizeAddonUpdateItemBetaPreferences,
  type AddonUpdateItemBetaPreferences,
} from '@/utils/addonUpdatePreferences'

const LOW_DISK_WARNING_BYTES = 10 * 1024 * 1024 * 1024
const LOW_DISK_CRITICAL_BYTES = 1024 * 1024 * 1024
const LOW_MEMORY_WARNING_BYTES = 2 * 1024 * 1024 * 1024
const LOW_MEMORY_CRITICAL_BYTES = 512 * 1024 * 1024
const CLEANABLE_WARNING_BYTES = 100 * 1024 * 1024
const READONLY_WARNING_COUNT = 10
const DEFAULT_CSL_SERVER = 'http://x-csl.ru'

interface LogIssue {
  category: string
  severity: string
  line_numbers: number[]
  sample_line: string
}

interface XPlaneLogAnalysis {
  log_path: string
  is_xplane_log: boolean
  crash_detected: boolean
  crash_info: string | null
  issues: LogIssue[]
  system_info: {
    xplane_version: string | null
    gpu_model: string | null
    gpu_driver: string | null
  }
}

interface CrashCause {
  cause_key: string
  score: number
  evidence: string[]
  blamed_module: string | null
}

interface DeepCrashAnalysis {
  report_info: { file_name: string; file_size: number; timestamp: number }
  crash_causes: CrashCause[]
  loaded_plugins: string[]
  parse_success: boolean
}

interface CleanupReport {
  totalBytes: number
  totalFiles: number
}

interface ActivityLogPage {
  entries: Array<{
    timestamp: number
    operation: string
    itemName: string
    success: boolean
  }>
}

export interface DoctorCheckContext {
  xplanePath: string
  mode: DoctorRunMode
  crashAnalysisDmpEnabled: boolean
  crashAnalysisIgnoreDateCheck: boolean
  installationId: string
  xplaneRunning: boolean
  environment: DoctorEnvironmentReport | null
  xfast: DoctorXfastHealthReport | null
  log: XPlaneLogAnalysis | null
  system: DoctorSystemSnapshot | null
}

export interface DoctorCheckDefinition {
  id: string
  section: DoctorSection
  phase: 'local' | 'network'
  modes: DoctorRunMode[]
  timeoutMs: number
  run: () => Promise<DoctorCheckResult[]>
}

function evidence(values: string[], kind: 'text' | 'path' | 'log' = 'text') {
  return values.slice(0, 20).map((value) => ({ kind, value, sensitive: kind !== 'text' }))
}

function result(
  id: string,
  section: DoctorSection,
  outcome: DoctorCheckResult['outcome'],
  options: Omit<DoctorCheckResult, 'id' | 'section' | 'outcome' | 'durationMs'> = {},
): DoctorCheckResult {
  return { id, section, outcome, durationMs: 0, ...options }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
}

function detectBeta(raw: string | null): boolean {
  return Boolean(raw && /(-b\d|beta|-d\d|\bdev\b|alpha|-rc)/i.test(raw))
}

function betaFoldersFor(
  preferences: AddonUpdateItemBetaPreferences,
  itemType: AddonUpdatableItemType,
  items: Array<{ folderName: string }>,
): string[] {
  return items
    .filter((item) => hasAddonUpdateBetaPreference(preferences, itemType, item.folderName))
    .map((item) => item.folderName)
}

function logSection(category: string): DoctorSection {
  if (
    [
      'out_of_memory',
      'heavy_memory_pressure',
      'memory_status_critical',
      'severe_texture_downscale',
      'runloop_backlog',
    ].includes(category)
  ) {
    return 'performance'
  }
  if (
    [
      'plugin_error',
      'plugin_assert',
      'plugin_manager_error',
      'duplicate_plugin',
      'missing_plugin_support',
      'deprecated_dataref',
    ].includes(category)
  ) {
    return 'addons'
  }
  if (['dsf_error', 'scenery_error', 'scenery_load_failed'].includes(category)) return 'scenery'
  if (['nvidia_permission', 'third_party_blocked', 'ssl_failed'].includes(category)) return 'system'
  return 'stability'
}

function logOutcome(issue: LogIssue): DoctorCheckResult['outcome'] {
  if (
    [
      'vulkan_device_error',
      'out_of_memory',
      'heavy_memory_pressure',
      'plugin_assert',
      'plugin_manager_error',
    ].includes(issue.category)
  ) {
    return 'critical'
  }
  if (issue.severity === 'high' || issue.severity === 'medium') return 'warning'
  return 'info'
}

function createEnvironmentCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'environment',
    section: 'installation',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: context.mode === 'full' ? 30_000 : 10_000,
    async run() {
      const env = await invoke<DoctorEnvironmentReport>('doctor_scan_environment', {
        xplanePath: context.xplanePath,
        depth: context.mode,
      })
      context.environment = env
      context.installationId = env.installationId
      context.system = {
        os: env.system.os,
        osVersion: env.system.osVersion,
        architecture: env.system.architecture,
        cpuModel: env.system.cpuModel,
        logicalCores: env.system.logicalCores,
        totalMemoryBytes: env.system.totalMemoryBytes,
        availableMemoryBytes: env.system.availableMemoryBytes,
        xplaneVersion: null,
        xplaneVersionRaw: null,
        isBeta: false,
        gpuModel: null,
        gpuDriver: null,
        xplaneFreeBytes: env.freeBytes,
        xplaneTotalBytes: env.totalBytes,
        appDataFreeBytes: null,
        appDataTotalBytes: null,
      }

      const checks: DoctorCheckResult[] = []
      const missing = [...env.missingCoreDirs]
      if (!env.executablePresent) missing.unshift('X-Plane executable')
      checks.push(
        result('installation.structure', 'installation', missing.length ? 'critical' : 'pass', {
          params: { count: missing.length },
          evidence: evidence(missing),
          remediation: missing.length
            ? { id: 'configure_xplane_path', kind: 'navigate', risk: 'safe', route: '/settings' }
            : undefined,
        }),
      )

      checks.push(
        result(
          'storage.xplane_space',
          'storage',
          env.freeBytes < LOW_DISK_CRITICAL_BYTES
            ? 'critical'
            : env.freeBytes < LOW_DISK_WARNING_BYTES
              ? 'warning'
              : 'pass',
          {
            params: { free: formatBytes(env.freeBytes), total: formatBytes(env.totalBytes) },
            remediation:
              env.freeBytes < LOW_DISK_WARNING_BYTES
                ? { id: 'open_cleanup', kind: 'navigate', risk: 'safe', route: '/disk-usage' }
                : undefined,
          },
        ),
      )

      checks.push(
        result('installation.location', 'installation', env.inProgramFiles ? 'warning' : 'pass', {
          params: { steam: env.isSteamInstall ? 1 : 0 },
        }),
      )

      checks.push(
        result(
          'installation.readonly',
          'installation',
          !env.readonlyScanPerformed
            ? 'notApplicable'
            : env.readonlyCount >= READONLY_WARNING_COUNT
              ? 'warning'
              : env.readonlyCount > 0
                ? 'info'
                : 'pass',
          {
            params: {
              count: env.readonlyScanCapped ? `${env.readonlyCount}+` : env.readonlyCount,
            },
          },
        ),
      )

      checks.push(
        result('system.injectors', 'system', env.injectors.length ? 'warning' : 'pass', {
          params: { count: env.injectors.length },
          evidence: evidence(
            env.injectors.map((item) => item.evidence),
            'path',
          ),
          remediation: env.injectors.length
            ? { id: 'guide_injectors', kind: 'guide', risk: 'destructive' }
            : undefined,
        }),
      )

      checks.push(
        result(
          'scenery.competing_organizer',
          'scenery',
          env.competingOrganizers.length ? 'info' : 'pass',
          {
            evidence: evidence(
              env.competingOrganizers.map((item) => item.evidence),
              'path',
            ),
          },
        ),
      )

      checks.push(
        result('system.inventory', 'system', 'pass', {
          params: {
            os: env.system.os,
            arch: env.system.architecture,
            cores: env.system.logicalCores,
            memory: formatBytes(env.system.totalMemoryBytes),
          },
        }),
      )

      const memoryOutcome =
        env.system.availableMemoryBytes < LOW_MEMORY_CRITICAL_BYTES
          ? 'critical'
          : env.system.availableMemoryBytes < LOW_MEMORY_WARNING_BYTES ||
              env.system.availableMemoryBytes < env.system.totalMemoryBytes * 0.05
            ? 'warning'
            : 'pass'
      checks.push(
        result('performance.available_memory', 'performance', memoryOutcome, {
          params: {
            available: formatBytes(env.system.availableMemoryBytes),
            total: formatBytes(env.system.totalMemoryBytes),
          },
        }),
      )
      return checks
    },
  }
}

function createRuntimeCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'runtime',
    section: 'installation',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 5_000,
    async run() {
      context.xplaneRunning = await invoke<boolean>('is_xplane_running')
      return [
        result(
          'installation.xplane_running',
          'installation',
          context.xplaneRunning ? 'info' : 'pass',
        ),
      ]
    },
  }
}

function createXfastCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'xfast',
    section: 'xfast',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 15_000,
    async run() {
      const report = await invoke<DoctorXfastHealthReport>('doctor_scan_xfast_health', {
        xplanePath: context.xplanePath,
      })
      context.xfast = report
      if (context.system) {
        context.system.appDataFreeBytes = report.appDataFreeBytes
        context.system.appDataTotalBytes = report.appDataTotalBytes
      }

      const staleIndex =
        report.sceneryIndex.missingFromIndex.length > 0 ||
        report.sceneryIndex.missingFromDisk.length > 0
      const indexOutcome = !report.schemaCompatible
        ? 'critical'
        : !report.sceneryIndex.indexExists && report.sceneryIndex.filesystemCount > 0
          ? 'warning'
          : staleIndex
            ? 'warning'
            : 'pass'
      const indexEvidence = [
        ...report.sceneryIndex.missingFromIndex.map((name) => `+ ${name}`),
        ...report.sceneryIndex.missingFromDisk.map((name) => `- ${name}`),
      ]

      return [
        result(
          'storage.app_data_space',
          'storage',
          report.appDataFreeBytes < LOW_DISK_CRITICAL_BYTES
            ? 'critical'
            : report.appDataFreeBytes < LOW_DISK_WARNING_BYTES
              ? 'warning'
              : 'pass',
          {
            params: {
              free: formatBytes(report.appDataFreeBytes),
              total: formatBytes(report.appDataTotalBytes),
            },
          },
        ),
        result('xfast.app_data', 'xfast', report.appDataWritable ? 'pass' : 'critical', {
          evidence: report.appDataWriteError ? evidence([report.appDataWriteError]) : undefined,
        }),
        result('xfast.database', 'xfast', report.databaseOk ? 'pass' : 'critical', {
          evidence: report.databaseDetail ? evidence([report.databaseDetail]) : undefined,
          remediation: report.databaseOk
            ? undefined
            : { id: 'reset_database', kind: 'guide', risk: 'destructive', route: '/settings' },
        }),
        result('xfast.scenery_index', 'xfast', indexOutcome, {
          params: {
            indexed: report.sceneryIndex.indexedCount,
            found: report.sceneryIndex.filesystemCount,
          },
          evidence: evidence(indexEvidence),
          remediation:
            indexOutcome === 'pass'
              ? undefined
              : report.schemaCompatible
                ? { id: 'refresh_scenery_index', kind: 'automatic', risk: 'safe' }
                : { id: 'rebuild_scenery_index', kind: 'automatic', risk: 'confirm' },
        }),
      ]
    },
  }
}

function createLogCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'log',
    section: 'stability',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: context.mode === 'full' ? 45_000 : 15_000,
    async run() {
      let log: XPlaneLogAnalysis
      try {
        log = await invoke<XPlaneLogAnalysis>('analyze_xplane_log', {
          xplanePath: context.xplanePath,
        })
      } catch (error) {
        const message = String(error)
        if (/not found|no such file/i.test(message)) {
          return [result('stability.log_available', 'stability', 'notApplicable')]
        }
        throw error
      }
      context.log = log
      if (!log.is_xplane_log) {
        return [
          result('stability.log_available', 'stability', 'unavailable', {
            evidence: evidence([log.log_path], 'path'),
          }),
        ]
      }

      const rawVersion = log.system_info.xplane_version
      if (context.system) {
        context.system.xplaneVersion = rawVersion ? rawVersion.split('-')[0] : null
        context.system.xplaneVersionRaw = rawVersion
        context.system.isBeta = detectBeta(rawVersion)
        context.system.gpuModel = log.system_info.gpu_model
        context.system.gpuDriver = log.system_info.gpu_driver
      }

      const checks: DoctorCheckResult[] = [
        result('stability.last_session', 'stability', log.crash_detected ? 'critical' : 'pass', {
          evidence: log.crash_info
            ? evidence(log.crash_info.split('\n').slice(0, 20), 'log')
            : undefined,
        }),
      ]

      if (detectBeta(rawVersion)) {
        checks.push(
          result('system.beta_build', 'system', 'info', { params: { version: rawVersion ?? '' } }),
        )
      }

      const seen = new Set<string>()
      for (const issue of log.issues) {
        if (issue.category === 'crash' || seen.has(issue.category)) continue
        seen.add(issue.category)
        checks.push(
          result(`log.${issue.category}`, logSection(issue.category), logOutcome(issue), {
            params: { line: issue.line_numbers[0] ?? 0 },
            evidence: issue.sample_line
              ? evidence(issue.sample_line.split('\n').slice(0, 8), 'log')
              : undefined,
          }),
        )
      }

      if (log.crash_detected && context.mode === 'full') {
        if (!context.crashAnalysisDmpEnabled) {
          checks.push(
            result('stability.crash_dump', 'stability', 'notApplicable', {
              remediation: {
                id: 'enable_dmp_analysis',
                kind: 'navigate',
                risk: 'safe',
                route: '/settings',
              },
            }),
          )
        } else {
          const crash = await invoke<DeepCrashAnalysis | null>('analyze_crash_report', {
            xplanePath: context.xplanePath,
            logIssues: log.issues,
            skipDateCheck: context.crashAnalysisIgnoreDateCheck,
          })
          if (!crash) {
            checks.push(result('stability.crash_dump', 'stability', 'notApplicable'))
          } else {
            const top = crash.crash_causes[0]
            checks.push(
              result('stability.crash_dump', 'stability', top ? 'critical' : 'warning', {
                params: {
                  cause: top?.cause_key ?? 'unknown',
                  confidence: top ? Math.round(top.score) : 0,
                  module: top?.blamed_module ?? '',
                },
                evidence: evidence(
                  [
                    ...crash.crash_causes.flatMap((cause) => [
                      `${cause.cause_key}: ${Math.round(cause.score)}%${cause.blamed_module ? ` (${cause.blamed_module})` : ''}`,
                      ...cause.evidence,
                    ]),
                    ...crash.loaded_plugins.map((plugin) => `plugin: ${plugin}`),
                  ].slice(0, 20),
                  'log',
                ),
              }),
            )
          }
        }
      }

      return checks
    },
  }
}

function createSceneryCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'scenery',
    section: 'scenery',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 30_000,
    async run() {
      const data = await invoke<SceneryManagerData>('get_scenery_manager_data', {
        xplanePath: context.xplanePath,
      })
      const globalAirports = data.entries.find((entry) => entry.folderName === '*GLOBAL_AIRPORTS*')
      const missingLibraries = new Set<string>()
      for (const entry of data.entries) {
        for (const library of entry.missingLibraries ?? []) missingLibraries.add(library)
      }
      const overrides = await invoke<{ status: string; sourcePath?: string }[]>(
        'airport_flatten_list_overrides',
        { xplanePath: context.xplanePath },
      )
      const drifted = overrides.filter((item) => item.status === 'drifted')

      return [
        result(
          'scenery.global_airports',
          'scenery',
          globalAirports && !globalAirports.enabled ? 'critical' : 'pass',
          {
            remediation:
              globalAirports && !globalAirports.enabled
                ? { id: 'enable_global_airports', kind: 'automatic', risk: 'safe' }
                : undefined,
          },
        ),
        result('scenery.load_order', 'scenery', data.needsSync ? 'warning' : 'pass', {
          remediation: data.needsSync
            ? { id: 'sort_scenery', kind: 'automatic', risk: 'safe' }
            : undefined,
        }),
        result('scenery.dependencies', 'scenery', data.missingDepsCount > 0 ? 'warning' : 'pass', {
          params: { count: data.missingDepsCount },
          evidence: evidence([...missingLibraries]),
          remediation:
            data.missingDepsCount > 0
              ? {
                  id: 'manage_scenery',
                  kind: 'navigate',
                  risk: 'safe',
                  route: '/management?tab=scenery',
                }
              : undefined,
        }),
        result('scenery.overlaps', 'scenery', data.duplicateTilesCount > 0 ? 'warning' : 'pass', {
          params: { count: data.duplicateTilesCount },
          remediation:
            data.duplicateTilesCount > 0
              ? {
                  id: 'manage_scenery',
                  kind: 'navigate',
                  risk: 'safe',
                  route: '/management?tab=scenery',
                }
              : undefined,
        }),
        result(
          'scenery.duplicate_airports',
          'scenery',
          data.duplicateAirportsCount > 0 ? 'info' : 'pass',
          { params: { count: data.duplicateAirportsCount } },
        ),
        result('scenery.flatten', 'scenery', drifted.length ? 'warning' : 'pass', {
          params: { count: drifted.length },
          evidence: evidence(
            drifted.flatMap((item) => (item.sourcePath ? [item.sourcePath] : [])),
            'path',
          ),
          remediation: drifted.length
            ? { id: 'apply_flatten', kind: 'automatic', risk: 'safe' }
            : undefined,
        }),
      ]
    },
  }
}

function createNavdataCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'navdata',
    section: 'navdata',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 15_000,
    async run() {
      const report = await invoke<DoctorNavdataReport>('doctor_navdata_status', {
        xplanePath: context.xplanePath,
      })
      if (!report.customDataExists || report.cycles.length === 0) {
        return [result('navdata.custom_data', 'navdata', 'notApplicable')]
      }

      const expired = report.cycles.filter((cycle) => cycle.status === 'expired')
      const expiring = report.cycles.filter((cycle) => cycle.status === 'expiring_soon')
      const unknown = report.cycles.filter((cycle) => cycle.status === 'unknown')
      const distinctCycles = new Set(report.cycles.map((cycle) => cycle.cycle).filter(Boolean))
      const cycleOutcome = expired.length
        ? 'warning'
        : expiring.length || unknown.length
          ? 'info'
          : 'pass'

      return [
        result('navdata.cycles', 'navdata', cycleOutcome, {
          params: { expired: expired.length, expiring: expiring.length, unknown: unknown.length },
          evidence: evidence(
            [...expired, ...expiring, ...unknown].map(
              (cycle) =>
                `${cycle.providerName} ${cycle.cycle ?? '?'} · ${cycle.status} · ${cycle.expiryDate ?? '?'}`,
            ),
          ),
        }),
        result('navdata.cifp', 'navdata', report.cifpPresent ? 'pass' : 'warning'),
        result('navdata.integrity', 'navdata', report.earthDatMissing.length ? 'warning' : 'pass', {
          evidence: evidence(report.earthDatMissing, 'path'),
        }),
        result('navdata.consistency', 'navdata', distinctCycles.size > 1 ? 'info' : 'pass', {
          params: { cycles: [...distinctCycles].join(', ') },
        }),
      ]
    },
  }
}

function createActivityCheck(): DoctorCheckDefinition {
  return {
    id: 'activity',
    section: 'xfast',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 10_000,
    async run() {
      const page = await invoke<ActivityLogPage>('get_activity_log', { limit: 100, offset: 0 })
      const sevenDaysAgo = Math.floor(Date.now() / 1000) - 7 * 24 * 60 * 60
      const failures = page.entries.filter(
        (entry) =>
          entry.timestamp >= sevenDaysAgo &&
          !entry.success &&
          (entry.operation === 'install' || entry.operation === 'update'),
      )
      return [
        result('xfast.recent_failures', 'xfast', failures.length ? 'info' : 'pass', {
          params: { count: failures.length },
          evidence: evidence(
            failures.slice(0, 10).map((entry) => `${entry.operation} · ${entry.itemName}`),
          ),
          remediation: failures.length
            ? { id: 'open_activity', kind: 'navigate', risk: 'safe', route: '/activity' }
            : undefined,
        }),
      ]
    },
  }
}

function createCleanupCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'cleanup',
    section: 'storage',
    phase: 'local',
    modes: ['full'],
    timeoutMs: 30_000,
    async run() {
      const cleanup = await invoke<CleanupReport>('scan_output_cleanup_items', {
        xplanePath: context.xplanePath,
      })
      return [
        result(
          'storage.cleanable',
          'storage',
          cleanup.totalBytes >= CLEANABLE_WARNING_BYTES ? 'info' : 'pass',
          {
            params: { size: formatBytes(cleanup.totalBytes), files: cleanup.totalFiles },
            remediation:
              cleanup.totalBytes > 0
                ? {
                    id: 'open_cleanup',
                    kind: 'navigate',
                    risk: 'safe',
                    route: '/disk-usage/output-cleanup',
                  }
                : undefined,
          },
        ),
      ]
    },
  }
}

function createAddonCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'addons',
    section: 'addons',
    phase: 'local',
    modes: ['full'],
    timeoutMs: 30_000,
    async run() {
      const [platform, aircraft, plugins] = await Promise.all([
        invoke<string>('get_platform'),
        invoke<{ entries: AircraftInfo[] }>('scan_aircraft', { xplanePath: context.xplanePath }),
        invoke<{ entries: PluginInfo[] }>('scan_plugins', { xplanePath: context.xplanePath }),
      ])
      const expectedPlatform = platform === 'windows' ? 'win' : platform === 'macos' ? 'mac' : 'lin'
      const incompatible = plugins.entries.filter(
        (plugin) =>
          plugin.enabled &&
          plugin.platform !== 'multi' &&
          plugin.platform !== 'unknown' &&
          plugin.platform !== expectedPlatform,
      )
      return [
        result('addons.inventory', 'addons', 'pass', {
          params: { aircraft: aircraft.entries.length, plugins: plugins.entries.length },
        }),
        result('addons.platform', 'addons', incompatible.length ? 'warning' : 'pass', {
          params: { count: incompatible.length, platform },
          evidence: evidence(incompatible.map((plugin) => plugin.displayName)),
          remediation: incompatible.length
            ? {
                id: 'manage_plugins',
                kind: 'navigate',
                risk: 'safe',
                route: '/management?tab=plugin',
              }
            : undefined,
        }),
      ]
    },
  }
}

function createAddonUpdateCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'addon_updates',
    section: 'updates',
    phase: 'network',
    modes: ['full'],
    timeoutMs: 60_000,
    async run() {
      let total = 0
      const details: string[] = []
      const betaPreferences = normalizeAddonUpdateItemBetaPreferences(
        await getItem<unknown>(STORAGE_KEYS.ADDON_UPDATE_ITEM_BETA_PREFERENCES),
      )

      const aircraft = await invoke<{ entries: AircraftInfo[] }>('scan_aircraft', {
        xplanePath: context.xplanePath,
      })
      const aircraftCandidates = aircraft.entries.filter((item) => item.updateUrl)
      if (aircraftCandidates.length) {
        const checked = await invoke<AircraftInfo[]>('check_aircraft_updates', {
          xplanePath: context.xplanePath,
          aircraft: aircraftCandidates,
          betaFolders: betaFoldersFor(betaPreferences, 'aircraft', aircraftCandidates),
        })
        const count = checked.filter((item) => item.hasUpdate).length
        total += count
        if (count) details.push(`aircraft: ${count}`)
      }

      const plugins = await invoke<{ entries: PluginInfo[] }>('scan_plugins', {
        xplanePath: context.xplanePath,
      })
      const pluginCandidates = plugins.entries.filter((item) => item.updateUrl)
      if (pluginCandidates.length) {
        const checked = await invoke<PluginInfo[]>('check_plugins_updates', {
          xplanePath: context.xplanePath,
          plugins: pluginCandidates,
          betaFolders: betaFoldersFor(betaPreferences, 'plugin', pluginCandidates),
        })
        const count = checked.filter((item) => item.hasUpdate).length
        total += count
        if (count) details.push(`plugins: ${count}`)
      }

      const scenery = await invoke<SceneryManagerData>('get_scenery_manager_data', {
        xplanePath: context.xplanePath,
      })
      const sceneryCandidates = scenery.entries.filter((item) => item.updateUrl)
      if (sceneryCandidates.length) {
        const checked = await invoke<SceneryManagerEntry[]>('check_scenery_updates', {
          xplanePath: context.xplanePath,
          scenery: sceneryCandidates,
          betaFolders: betaFoldersFor(betaPreferences, 'scenery', sceneryCandidates),
        })
        const count = checked.filter((item) => item.hasUpdate).length
        total += count
        if (count) details.push(`scenery: ${count}`)
      }

      return [
        result('updates.addons', 'updates', total ? 'info' : 'pass', {
          params: { count: total },
          evidence: evidence(details),
          remediation: total
            ? { id: 'manage_updates', kind: 'navigate', risk: 'safe', route: '/management' }
            : undefined,
        }),
      ]
    },
  }
}

function createGatewayUpdateCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'gateway_updates',
    section: 'updates',
    phase: 'network',
    modes: ['full'],
    timeoutMs: 30_000,
    async run() {
      const installed = await invoke<GatewayInstalledAirport[]>('gateway_list_installed', {
        xplanePath: context.xplanePath,
      })
      if (!installed.length) return [result('updates.gateway', 'updates', 'notApplicable')]
      const checked = await invoke<GatewayInstalledAirport[]>('gateway_check_updates', {
        xplanePath: context.xplanePath,
      })
      const count = checked.filter((item) => item.updateAvailable).length
      return [
        result('updates.gateway', 'updates', count ? 'info' : 'pass', {
          params: { count },
          remediation: count
            ? { id: 'manage_gateway', kind: 'navigate', risk: 'safe', route: '/gateway' }
            : undefined,
        }),
      ]
    },
  }
}

function createAppUpdateCheck(): DoctorCheckDefinition {
  return {
    id: 'app_update',
    section: 'updates',
    phase: 'network',
    modes: ['full'],
    timeoutMs: 20_000,
    async run() {
      const includePreRelease = (await getItem<boolean>(STORAGE_KEYS.INCLUDE_PRE_RELEASE)) === true
      const update = await invoke<{ isUpdateAvailable: boolean; latestVersion: string }>(
        'check_for_updates',
        { manual: false, includePreRelease },
      )
      return [
        result('updates.app', 'updates', update.isUpdateAvailable ? 'info' : 'pass', {
          params: { version: update.latestVersion },
          remediation: update.isUpdateAvailable
            ? {
                id: 'open_app_update',
                kind: 'navigate',
                risk: 'safe',
                route: '/settings',
              }
            : undefined,
        }),
      ]
    },
  }
}

function createCslUpdateCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'csl_updates',
    section: 'updates',
    phase: 'network',
    modes: ['full'],
    timeoutMs: 60_000,
    async run() {
      const [customPaths, installLocation, activeServer] = await Promise.all([
        getItem<string[]>(STORAGE_KEYS.CSL_CUSTOM_PATHS),
        getItem<string>(STORAGE_KEYS.CSL_INSTALL_LOCATION),
        getItem<string>(STORAGE_KEYS.CSL_ACTIVE_SERVER_BASE_URL),
      ])
      const serverBaseUrl = activeServer || DEFAULT_CSL_SERVER
      const requestId = `health-${Date.now().toString(36)}`
      const [csl, altitude] = await Promise.all([
        invoke<CslScanResult>('csl_scan_packages', {
          xplanePath: context.xplanePath,
          customPaths: customPaths ?? [],
          installLocation: installLocation || null,
          serverBaseUrl,
          requestId: `${requestId}-csl`,
        }),
        invoke<CslScanResult>('altitude_scan_packages', {
          xplanePath: context.xplanePath,
          serverBaseUrl,
          requestId: `${requestId}-altitude`,
        }),
      ])
      const cslUpdates = csl.packages.filter((item) => item.status === 'needs_update').length
      const altitudeUpdates = altitude.packages.filter(
        (item) => item.status === 'needs_update',
      ).length
      const count = cslUpdates + altitudeUpdates
      const warnings = [csl.index_warning, altitude.index_warning].filter(
        (warning): warning is string => Boolean(warning),
      )
      return [
        result('updates.csl', 'updates', count ? 'info' : warnings.length ? 'info' : 'pass', {
          params: { count, csl: cslUpdates, altitude: altitudeUpdates },
          evidence: evidence(warnings),
          remediation: count
            ? { id: 'manage_csl', kind: 'navigate', risk: 'safe', route: '/csl' }
            : undefined,
        }),
      ]
    },
  }
}

export function createDoctorCheckDefinitions(context: DoctorCheckContext): DoctorCheckDefinition[] {
  return [
    createEnvironmentCheck(context),
    createRuntimeCheck(context),
    createXfastCheck(context),
    createLogCheck(context),
    createSceneryCheck(context),
    createNavdataCheck(context),
    createActivityCheck(),
    createCleanupCheck(context),
    createAddonCheck(context),
    createAddonUpdateCheck(context),
    createGatewayUpdateCheck(context),
    createAppUpdateCheck(),
    createCslUpdateCheck(context),
  ]
}
