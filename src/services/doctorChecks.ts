import { invoke } from '@tauri-apps/api/core'
import type {
  AircraftInfo,
  AirportFlattenOverride,
  AddonUpdatableItemType,
  CslScanResult,
  DoctorEnvironmentReport,
  DoctorNavdataLayer,
  DoctorNavdataReport,
  DoctorXfastHealthReport,
  DoctorVolumeSpace,
  GatewayInstalledAirport,
  PluginInfo,
  SceneryManagerData,
  SceneryManagerEntry,
} from '@/types'
import type {
  DoctorCheckResult,
  DoctorInspectionReport,
  DoctorRunMode,
  DoctorSection,
  DoctorSystemSnapshot,
} from '@/types/doctor'
import { getItem, STORAGE_KEYS } from '@/services/storage'
import {
  hasAddonUpdateBetaPreference,
  getUpdateProvider,
  usesRemoteUpdateCheck,
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
  session_end: 'crash' | 'normal' | 'unknown'
  log_tail: string
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
  items: Array<{
    relativePath: string
    level: string
    recognized: boolean
    sizeBytes: number
    fileCount: number
  }>
}

interface ActivityLogPage {
  entries: Array<{ operation: string; itemName: string }>
  totalCount: number
}

export interface DoctorCheckContext {
  xplanePath: string
  mode: DoctorRunMode
  crashAnalysisDmpEnabled: boolean
  crashAnalysisIgnoreDateCheck: boolean
  installationId: string
  xplaneRunning: boolean | null
  environment: DoctorEnvironmentReport | null
  xfast: DoctorXfastHealthReport | null
  log: XPlaneLogAnalysis | null
  system: DoctorSystemSnapshot | null
  inspection?: DoctorInspectionReport | null
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

function diskSpaceCheck(id: string, space: DoctorVolumeSpace, openCleanup = false) {
  const { availableBytes, totalBytes, error } = space
  if (availableBytes === null || totalBytes === null || totalBytes === 0) {
    return result(id, 'storage', 'unavailable', {
      evidence: error ? evidence([error], 'path') : undefined,
    })
  }
  const outcome =
    availableBytes < LOW_DISK_CRITICAL_BYTES
      ? 'critical'
      : availableBytes < LOW_DISK_WARNING_BYTES
        ? 'warning'
        : 'pass'
  return result(id, 'storage', outcome, {
    params: { free: formatBytes(availableBytes), total: formatBytes(totalBytes) },
    remediation:
      openCleanup && outcome !== 'pass'
        ? { id: 'open_cleanup', kind: 'navigate', risk: 'safe', route: '/disk-usage' }
        : undefined,
  })
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
        xplaneVersion: context.system?.xplaneVersion ?? null,
        xplaneVersionRaw: context.system?.xplaneVersionRaw ?? null,
        isBeta: context.system?.isBeta ?? false,
        gpuModel: context.system?.gpuModel ?? null,
        gpuDriver: context.system?.gpuDriver ?? null,
        xplaneFreeBytes: env.diskSpace.availableBytes,
        xplaneTotalBytes: env.diskSpace.totalBytes,
        appDataFreeBytes: context.system?.appDataFreeBytes ?? null,
        appDataTotalBytes: context.system?.appDataTotalBytes ?? null,
      }

      const { rootUsable, core, aircraft, airfoils } = env.installation
      const checks: DoctorCheckResult[] = [
        result(
          'installation.structure',
          'installation',
          core.problems.length ? 'critical' : core.errors.length ? 'unavailable' : 'pass',
          {
            params: { count: core.problems.length },
            evidence: evidence([...core.problems, ...core.errors], 'path'),
            remediation:
              !rootUsable && core.problems.length
                ? {
                    id: 'configure_xplane_path',
                    kind: 'navigate',
                    risk: 'safe',
                    route: '/settings',
                  }
                : undefined,
          },
        ),
      ]
      if (!rootUsable) return checks
      for (const [id, finding] of [
        ['installation.stock_aircraft', aircraft],
        ['installation.airfoils', airfoils],
      ] as const) {
        checks.push(
          result(
            id,
            'installation',
            finding.problems.length ? 'warning' : finding.errors.length ? 'unavailable' : 'pass',
            {
              params: { count: finding.problems.length },
              evidence: evidence([...finding.problems, ...finding.errors], 'path'),
            },
          ),
        )
      }
      const assetErrors = [core, aircraft, airfoils]
        .filter((finding) => finding.problems.length > 0)
        .flatMap((finding) => finding.errors)
      if (assetErrors.length) {
        checks.push(
          result('installation.asset_access', 'installation', 'unavailable', {
            evidence: evidence(assetErrors, 'path'),
          }),
        )
      }

      checks.push(diskSpaceCheck('storage.xplane_space', env.diskSpace, true))
      checks.push(
        result('installation.location', 'installation', env.inProgramFiles ? 'info' : 'pass', {
          params: { steam: env.isSteamInstall ? 1 : 0 },
        }),
      )
      const access = env.directoryAccess
      checks.push(
        result(
          'installation.write_access',
          'installation',
          access.blocked.length ? 'warning' : access.errors.length ? 'unavailable' : 'pass',
          {
            params: { count: access.blocked.length },
            evidence: evidence([...access.blocked, ...access.errors], 'path'),
          },
        ),
      )

      const readonlyIncomplete = env.readonlyScanCapped || env.readonlyScanErrors.length > 0
      checks.push(
        result(
          'installation.readonly',
          'installation',
          !env.readonlyScanPerformed
            ? 'notApplicable'
            : env.readonlyCount >= READONLY_WARNING_COUNT
              ? 'warning'
              : readonlyIncomplete
                ? 'unavailable'
                : env.readonlyCount > 0
                  ? 'info'
                  : 'pass',
          {
            params: {
              count: readonlyIncomplete ? `${env.readonlyCount}+` : env.readonlyCount,
              ...(readonlyIncomplete
                ? { capped: Number(env.readonlyScanCapped), errors: env.readonlyScanErrors.length }
                : {}),
            },
            evidence: evidence(
              [
                ...env.readonlyScanErrors,
                ...(env.readonlyScanCapped
                  ? ['Reached the add-on filesystem inspection limit']
                  : []),
              ],
              'path',
            ),
          },
        ),
      )
      if (
        (access.blocked.length && access.errors.length) ||
        (env.readonlyCount >= READONLY_WARNING_COUNT && readonlyIncomplete)
      ) {
        checks.push(
          result('installation.permission_access', 'installation', 'unavailable', {
            evidence: evidence(
              [
                ...access.errors,
                ...env.readonlyScanErrors,
                ...(env.readonlyScanCapped
                  ? ['Reached the add-on filesystem inspection limit']
                  : []),
              ],
              'path',
            ),
          }),
        )
      }

      checks.push(
        result('system.injectors', 'system', env.injectors.length ? 'warning' : 'pass', {
          params: { count: env.injectors.length },
          evidence: evidence(
            env.injectors.map((item) => item.evidence),
            'path',
          ),
          remediation: env.injectors.length
            ? { id: 'disable_injectors', kind: 'automatic', risk: 'confirm' }
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

      const physicalCores = env.system.physicalCores
      const totalMemory = env.system.totalMemoryBytes
      const hardwareIssues: string[] = []
      if (
        (physicalCores ?? env.system.logicalCores) > 0 &&
        (physicalCores ?? env.system.logicalCores) < 4
      ) {
        hardwareIssues.push(
          `CPU: ${physicalCores ?? env.system.logicalCores} cores reported, 4 required`,
        )
      }
      // Allow a small hardware-reserved portion of an 8 GiB system.
      if (totalMemory > 0 && totalMemory < 7.5 * 1024 ** 3) {
        hardwareIssues.push(`RAM: ${formatBytes(totalMemory)} reported, 8 GB required`)
      }
      checks.push(
        result(
          'system.inventory',
          'system',
          hardwareIssues.length
            ? 'warning'
            : physicalCores === null || !totalMemory
              ? 'unavailable'
              : 'pass',
          {
            params: {
              os: env.system.os,
              arch: env.system.architecture,
              cores: physicalCores ?? env.system.logicalCores,
              memory: formatBytes(totalMemory),
            },
            evidence: evidence(hardwareIssues),
          },
        ),
      )

      const vulkan = env.vulkanRuntime
      if (vulkan) {
        const available = vulkan.state === 'available'
        checks.push(
          result(
            'system.vulkan_runtime',
            'system',
            available
              ? vulkan.meetsMinimum
                ? 'pass'
                : 'warning'
              : vulkan.state === 'missing'
                ? 'warning'
                : 'unavailable',
            {
              params: available ? { version: vulkan.version } : undefined,
              evidence: available ? undefined : evidence([vulkan.detail], 'path'),
            },
          ),
        )
      }

      const memoryOutcome = !totalMemory
        ? 'unavailable'
        : env.system.availableMemoryBytes < LOW_MEMORY_CRITICAL_BYTES
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

function createInspectionCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'inspection',
    section: 'installation',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 45_000,
    async run() {
      const report = await invoke<DoctorInspectionReport>('doctor_inspect_installation', {
        xplanePath: context.xplanePath,
        depth: context.mode,
      })
      context.inspection = report
      const plugins = report.plugins
      const files = report.sceneryFiles
      const registrationErrors = report.sceneryRegistrationError
        ? [report.sceneryRegistrationError]
        : []
      const limits = [
        ...(report.sceneryLinksCapped ? ['Custom Scenery links: inspection limit reached'] : []),
        ...(files?.capped ? ['Enabled scenery files: inspection limit reached'] : []),
        ...(plugins?.capped ? ['Plugin files: inspection limit reached'] : []),
      ]
      const scanErrors = [
        ...new Set([
          ...registrationErrors,
          ...report.sceneryPathErrors,
          ...report.sceneryLinkErrors,
          ...(plugins?.errors ?? []),
          ...(files?.errors ?? []),
        ]),
      ]
      const registrationOutcome = {
        missing: 'info',
        readable: 'pass',
        unavailable: 'unavailable',
      } as const
      const registeredCheck =
        report.sceneryRegistration === 'missing'
          ? 'notApplicable'
          : report.sceneryRegistration === 'unavailable'
            ? 'unavailable'
            : 'pass'
      const checks: DoctorCheckResult[] = [
        result('scenery.registration', 'scenery', registrationOutcome[report.sceneryRegistration], {
          evidence: evidence(registrationErrors, 'path'),
          remediation:
            report.sceneryRegistration === 'missing'
              ? { id: 'sync_scenery_file', kind: 'automatic', risk: 'confirm' }
              : undefined,
        }),
        result(
          'scenery.global_airports',
          'scenery',
          report.globalAirportsEnabled === false
            ? 'critical'
            : report.globalAirportsEnabled === true
              ? 'pass'
              : registeredCheck === 'unavailable'
                ? 'unavailable'
                : 'notApplicable',
          {
            evidence:
              report.globalAirportsEnabled === false
                ? evidence(['SCENERY_PACK_DISABLED *GLOBAL_AIRPORTS*'], 'text')
                : evidence(registrationErrors, 'path'),
            remediation:
              report.globalAirportsEnabled === false
                ? { id: 'enable_global_airports', kind: 'automatic', risk: 'safe' }
                : undefined,
          },
        ),
        ...(
          [
            [
              'scenery.invalid_entries',
              report.invalidSceneryEntries,
              registrationErrors,
              registeredCheck,
            ],
            [
              'scenery.missing_paths',
              report.missingSceneryPaths,
              [...registrationErrors, ...report.sceneryPathErrors],
              registeredCheck,
            ],
            [
              'scenery.duplicate_paths',
              report.duplicateSceneryPaths,
              [...registrationErrors, ...report.sceneryPathErrors],
              registeredCheck,
            ],
            [
              'scenery.broken_links',
              report.brokenSceneryLinks,
              report.sceneryLinkErrors,
              report.sceneryLinksCapped ? 'unavailable' : 'pass',
            ],
          ] as const
        ).map(([id, paths, errors, fallback]) =>
          result(
            id,
            'scenery',
            paths.length ? 'warning' : errors.length ? 'unavailable' : fallback,
            {
              params: { count: paths.length },
              evidence: evidence([...paths, ...errors], 'path'),
              remediation: paths.length
                ? {
                    id: 'manage_scenery',
                    kind: 'navigate',
                    risk: 'safe',
                    route: '/management?tab=scenery',
                  }
                : undefined,
            },
          ),
        ),
        result(
          'stability.log_freshness',
          'stability',
          report.logAgeDays === null ? 'notApplicable' : report.logAgeDays > 7 ? 'info' : 'pass',
          {
            params: { days: report.logAgeDays ?? 0 },
          },
        ),
      ]
      if (scanErrors.length || limits.length) {
        checks.push(
          result('installation.scan_access', 'installation', 'unavailable', {
            params: { errors: scanErrors.length, capped: Number(limits.length > 0) },
            evidence: evidence([...limits, ...scanErrors], 'path'),
          }),
        )
      }
      if (context.mode === 'full') {
        const incompatible = plugins?.incompatible ?? []
        checks.push(
          result(
            'scenery.files',
            'scenery',
            files?.problems.length
              ? 'warning'
              : files?.errors.length ||
                  files?.capped ||
                  report.sceneryRegistration === 'unavailable'
                ? 'unavailable'
                : files?.checked
                  ? 'pass'
                  : 'notApplicable',
            {
              params: {
                checked: files?.checked ?? 0,
                count: files?.problems.length ?? 0,
                ...(files?.capped ? { capped: 1 } : {}),
              },
              evidence: evidence(
                [
                  ...(files?.capped ? ['Enabled scenery files: inspection limit reached'] : []),
                  ...(files?.problems ?? []),
                  ...(files?.errors ?? []),
                  ...registrationErrors,
                ],
                'path',
              ),
              remediation: files?.problems.length
                ? {
                    id: 'manage_scenery',
                    kind: 'navigate',
                    risk: 'safe',
                    route: '/management?tab=scenery',
                  }
                : undefined,
            },
          ),
          result(
            'addons.platform',
            'addons',
            incompatible.length
              ? 'warning'
              : !plugins || plugins.capped || plugins.errors.length
                ? 'unavailable'
                : plugins.checked
                  ? 'pass'
                  : 'notApplicable',
            {
              params: { count: incompatible.length },
              evidence: evidence(incompatible, 'path'),
              remediation: incompatible.length
                ? {
                    id: 'manage_plugins',
                    kind: 'navigate',
                    risk: 'safe',
                    route: '/management?tab=plugin',
                  }
                : undefined,
            },
          ),
        )
        checks.push(
          result(
            'addons.binary_integrity',
            'addons',
            plugins?.problems.length
              ? 'warning'
              : !plugins || plugins.capped || plugins.errors.length
                ? 'unavailable'
                : plugins.checked
                  ? 'pass'
                  : 'notApplicable',
            {
              params: {
                checked: plugins?.checked ?? 0,
                count: plugins?.problems.length ?? 0,
                loaderScoped: 1,
              },
              evidence: evidence(
                plugins?.problems.length ? plugins.problems : (plugins?.errors ?? []),
                'path',
              ),
              remediation: plugins?.problems.length
                ? {
                    id: 'manage_plugins',
                    kind: 'navigate',
                    risk: 'safe',
                    route: '/management?tab=plugin',
                  }
                : undefined,
            },
          ),
        )
      }
      return checks
    },
  }
}

function createXfastCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'xfast',
    section: 'xfast',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: context.mode === 'full' ? 45_000 : 15_000,
    async run() {
      const report = await invoke<DoctorXfastHealthReport>('doctor_scan_xfast_health', {
        xplanePath: context.xplanePath,
        depth: context.mode,
      })
      context.xfast = report
      if (context.system) {
        context.system.appDataFreeBytes = report.diskSpace.availableBytes
        context.system.appDataTotalBytes = report.diskSpace.totalBytes
      }

      const index = report.sceneryIndex
      const staleIndex =
        index && (index.missingFromIndex.length > 0 || index.missingFromDisk.length > 0)
      const indexEvidence = index
        ? [
            ...index.missingFromIndex.map((name) => `+ ${name}`),
            ...index.missingFromDisk.map((name) => `- ${name}`),
          ]
        : report.sceneryIndexError
          ? [report.sceneryIndexError]
          : []
      const databaseOutcome =
        report.databaseOk === false || report.schemaCompatible === false
          ? 'critical'
          : report.databaseOk === null || report.schemaCompatible === null
            ? 'unavailable'
            : 'pass'

      return [
        diskSpaceCheck('storage.app_data_space', report.diskSpace),
        result('xfast.app_data', 'xfast', report.appDataWritable ? 'pass' : 'critical', {
          evidence: report.appDataWriteError
            ? evidence([report.appDataWriteError], 'path')
            : undefined,
        }),
        result('xfast.database', 'xfast', databaseOutcome, {
          evidence: evidence(report.databaseDetails),
          remediation:
            databaseOutcome === 'critical'
              ? { id: 'reset_database', kind: 'guide', risk: 'destructive', route: '/settings' }
              : undefined,
        }),
        result(
          'xfast.scenery_index',
          'xfast',
          index ? (staleIndex ? 'warning' : 'pass') : 'unavailable',
          {
            params: index
              ? { indexed: index.indexedCount, found: index.filesystemCount }
              : undefined,
            evidence: evidence(indexEvidence, 'path'),
            remediation: staleIndex
              ? index.indexExists
                ? { id: 'refresh_scenery_index', kind: 'automatic', risk: 'safe' }
                : { id: 'rebuild_scenery_index', kind: 'automatic', risk: 'confirm' }
              : undefined,
          },
        ),
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
    timeoutMs: 15_000,
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

      const canEvaluateSessionEnd = context.xplaneRunning === false
      const sessionEnd = log.session_end ?? 'unknown'
      const checks: DoctorCheckResult[] = [
        result('stability.log_available', 'stability', 'pass'),
        result(
          'stability.last_session',
          'stability',
          canEvaluateSessionEnd
            ? sessionEnd === 'crash'
              ? 'critical'
              : sessionEnd === 'normal'
                ? 'pass'
                : 'info'
            : context.xplaneRunning === true
              ? 'notApplicable'
              : 'unavailable',
          {
            params: {
              sessionEnd:
                context.xplaneRunning === true
                  ? 'running'
                  : canEvaluateSessionEnd
                    ? sessionEnd
                    : 'unknown',
            },
            evidence:
              canEvaluateSessionEnd && (log.crash_info || log.log_tail)
                ? evidence((log.crash_info || log.log_tail).split('\n').slice(-20), 'log')
                : undefined,
          },
        ),
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

      return checks
    },
  }
}

function createCrashDumpCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'crash_dump',
    section: 'stability',
    phase: 'local',
    modes: ['full'],
    timeoutMs: 30_000,
    async run() {
      const checkId = 'stability.crash_dump'
      if (
        context.xplaneRunning !== false ||
        !context.log?.is_xplane_log ||
        context.log.session_end !== 'crash'
      ) {
        return [result(checkId, 'stability', 'notApplicable')]
      }
      if (!context.crashAnalysisDmpEnabled) {
        return [
          result(checkId, 'stability', 'notApplicable', {
            remediation: {
              id: 'enable_dmp_analysis',
              kind: 'navigate',
              risk: 'safe',
              route: '/settings',
            },
          }),
        ]
      }
      const crash = await invoke<DeepCrashAnalysis | null>('analyze_crash_report', {
        xplanePath: context.xplanePath,
        logIssues: context.log.issues,
        skipDateCheck: context.crashAnalysisIgnoreDateCheck,
      })
      if (!crash) return [result(checkId, 'stability', 'notApplicable')]
      if (!crash.parse_success) {
        return [
          result(checkId, 'stability', 'unavailable', {
            evidence: evidence([crash.report_info.file_name], 'path'),
          }),
        ]
      }
      const top = crash.crash_causes[0]
      return [
        result(checkId, 'stability', top ? 'warning' : 'info', {
          params: {
            cause: top?.cause_key ?? 'unknown',
            confidence: top ? Math.round(top.score) : 0,
            module: top?.blamed_module ?? '',
          },
          evidence: evidence(
            [
              crash.report_info.file_name,
              ...crash.crash_causes.flatMap((cause) => [
                `${cause.cause_key}: ${Math.round(cause.score)}%${cause.blamed_module ? ` (${cause.blamed_module})` : ''}`,
                ...cause.evidence,
              ]),
              ...crash.loaded_plugins.map((plugin) => `plugin: ${plugin}`),
            ],
            'log',
          ),
        }),
      ]
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
      const enabled = data.entries.filter((entry) => entry.enabled)
      const dependencies = enabled.filter((entry) => entry.missingLibraries?.length > 0)
      const conflicts = (field: 'duplicateTiles' | 'duplicateAirports') => {
        const byId = new Map<string, Set<string>>()
        for (const entry of enabled) {
          for (const id of entry[field] ?? []) {
            const packages = byId.get(id) ?? new Set()
            packages.add(entry.folderName)
            byId.set(id, packages)
          }
        }
        return [...byId]
          .filter(([, packages]) => packages.size > 1)
          .map(([id, packages]) => `${id}: ${[...packages].join(', ')}`)
      }
      const overlaps = conflicts('duplicateTiles')
      const airports = conflicts('duplicateAirports')
      const manageScenery = {
        id: 'manage_scenery',
        kind: 'navigate',
        risk: 'safe',
        route: '/management?tab=scenery',
      } as const
      return [
        result('scenery.load_order', 'scenery', data.needsSync ? 'warning' : 'pass', {
          remediation: data.needsSync
            ? { id: 'sort_scenery', kind: 'automatic', risk: 'safe' }
            : undefined,
        }),
        result('scenery.dependencies', 'scenery', dependencies.length ? 'warning' : 'pass', {
          params: { count: dependencies.length },
          evidence: evidence(
            dependencies.map(
              (entry) => `${entry.folderName}: ${entry.missingLibraries.join(', ')}`,
            ),
          ),
          remediation: dependencies.length ? manageScenery : undefined,
        }),
        result('scenery.overlaps', 'scenery', overlaps.length ? 'warning' : 'pass', {
          params: { count: overlaps.length },
          evidence: evidence(overlaps),
          remediation: overlaps.length ? manageScenery : undefined,
        }),
        result('scenery.duplicate_airports', 'scenery', airports.length ? 'info' : 'pass', {
          params: { count: airports.length },
          evidence: evidence(airports),
        }),
      ]
    },
  }
}

function createFlattenCheck(context: DoctorCheckContext): DoctorCheckDefinition {
  return {
    id: 'scenery_flatten',
    section: 'scenery',
    phase: 'local',
    modes: ['quick', 'full'],
    timeoutMs: 15_000,
    async run() {
      const overrides = await invoke<AirportFlattenOverride[]>('airport_flatten_list_overrides', {
        xplanePath: context.xplanePath,
      })
      const drifted = overrides.filter((item) => item.status === 'drifted')
      const missing = overrides.filter((item) => item.status === 'sourceMissing')
      return [
        result(
          'scenery.flatten',
          'scenery',
          drifted.length || missing.length
            ? 'warning'
            : overrides.length
              ? 'pass'
              : 'notApplicable',
          {
            params: { count: drifted.length + missing.length },
            evidence: evidence(
              [...drifted, ...missing].map(
                (item) => `${item.icao}: ${item.sourcePath} (${item.status})`,
              ),
              'path',
            ),
            remediation: drifted.length
              ? { id: 'apply_flatten', kind: 'automatic', risk: 'safe' }
              : undefined,
          },
        ),
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
        depth: context.mode,
      })
      const layers = [
        ['navdata.bundled', report.bundled],
        ['navdata.integrity', report.overrideData],
        ['navdata.supplemental', report.supplemental],
      ] satisfies [string, DoctorNavdataLayer][]
      const checks = layers.map(([id, layer]) =>
        result(
          id,
          'navdata',
          layer.problems.length
            ? 'warning'
            : layer.errors.length
              ? 'unavailable'
              : layer.present
                ? 'pass'
                : 'notApplicable',
          {
            params: { count: layer.problems.length },
            evidence: evidence([...layer.problems, ...layer.errors, ...layer.headers], 'path'),
          },
        ),
      )
      const expired = report.cycles.filter((cycle) => cycle.status === 'expired').length
      const expiring = report.cycles.filter((cycle) => cycle.status === 'expiring_soon').length
      const future = report.cycles.filter((cycle) => cycle.status === 'not_yet_effective').length
      const unknown = report.cycles.filter((cycle) => cycle.status === 'unknown').length
      checks.push(
        result(
          'navdata.cycles',
          'navdata',
          expired || future
            ? 'warning'
            : expiring || unknown
              ? 'info'
              : report.metadataErrors.length
                ? 'unavailable'
                : report.cycles.length
                  ? 'pass'
                  : 'notApplicable',
          {
            params: { expired, expiring, future, unknown },
            evidence: evidence(
              report.cycles.map(
                (cycle) =>
                  `${cycle.source} · ${cycle.providerName} · AIRAC ${cycle.cycle ?? cycle.airac ?? '?'} · ${cycle.effectiveDate ?? '?'} to ${cycle.expiryDate ?? '?'}`,
              ),
              'path',
            ),
          },
        ),
      )
      const errors = [...layers.flatMap(([, layer]) => layer.errors), ...report.metadataErrors]
      if (errors.length) {
        checks.push(
          result('navdata.scan_access', 'navdata', 'unavailable', {
            evidence: evidence(errors, 'path'),
          }),
        )
      }
      return checks
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
      const since = Math.floor(Date.now() / 1000) - 7 * 24 * 60 * 60
      const page = await invoke<ActivityLogPage>('get_activity_log', {
        limit: 10,
        offset: 0,
        since,
        failedInstallsAndUpdates: true,
      })
      const count = page.totalCount
      return [
        result('xfast.recent_failures', 'xfast', count > 0 ? 'info' : 'pass', {
          params: { count },
          evidence: evidence(page.entries.map((entry) => `${entry.operation} · ${entry.itemName}`)),
          remediation:
            count > 0
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
      const candidates = cleanup.items
        .filter((item) => item.recognized && item.level === 'recommended' && item.sizeBytes > 0)
        .sort((a, b) => b.sizeBytes - a.sizeBytes)
      const bytes = candidates.reduce((total, item) => total + item.sizeBytes, 0)
      const files = candidates.reduce((total, item) => total + item.fileCount, 0)
      return [
        result('storage.cleanable', 'storage', bytes >= CLEANABLE_WARNING_BYTES ? 'info' : 'pass', {
          params: { size: formatBytes(bytes), files },
          evidence: evidence(
            candidates.map((item) => `${item.relativePath}: ${formatBytes(item.sizeBytes)}`),
            'path',
          ),
          remediation:
            bytes > 0
              ? {
                  id: 'open_cleanup',
                  kind: 'navigate',
                  risk: 'safe',
                  route: '/disk-usage/output-cleanup',
                }
              : undefined,
        }),
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
      const [aircraft, plugins] = await Promise.allSettled([
        invoke<{ entries: AircraftInfo[] }>('scan_aircraft', { xplanePath: context.xplanePath }),
        invoke<{ entries: PluginInfo[] }>('scan_plugins', { xplanePath: context.xplanePath }),
      ])
      const params: Record<string, number> = {}
      const errors: string[] = []
      for (const [name, inventory] of [
        ['aircraft', aircraft],
        ['plugins', plugins],
      ] as const) {
        if (inventory.status === 'fulfilled') {
          params[name] = inventory.value.entries.length
          params[name === 'aircraft' ? 'enabledAircraft' : 'enabledPlugins'] =
            inventory.value.entries.filter((item) => item.enabled).length
        } else {
          errors.push(`${name}: ${String(inventory.reason)}`)
        }
      }
      return [
        result('addons.inventory', 'addons', errors.length ? 'unavailable' : 'pass', {
          params,
          evidence: evidence(errors, 'path'),
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
      const betaPreferences = normalizeAddonUpdateItemBetaPreferences(
        await getItem<unknown>(STORAGE_KEYS.ADDON_UPDATE_ITEM_BETA_PREFERENCES),
      )
      const xplanePath = context.xplanePath
      const sources = [
        {
          type: 'aircraft',
          scan: 'scan_aircraft',
          command: 'check_aircraft_updates',
          argument: 'aircraft',
        },
        {
          type: 'plugin',
          scan: 'scan_plugins',
          command: 'check_plugins_updates',
          argument: 'plugins',
        },
        {
          type: 'scenery',
          scan: 'get_scenery_manager_data',
          command: 'check_scenery_updates',
          argument: 'scenery',
        },
      ] as const

      const groups = await Promise.allSettled(
        sources.map(async ({ type, scan, command, argument }) => {
          type Addon = AircraftInfo | PluginInfo | SceneryManagerEntry
          const { entries } = await invoke<{ entries: Addon[] }>(scan, { xplanePath })
          const candidates = entries.filter(usesRemoteUpdateCheck)
          const external = entries.filter(
            (item) => getUpdateProvider(item) && !usesRemoteUpdateCheck(item),
          )
          const details = external.map((item) => `${item.folderName} · ${getUpdateProvider(item)}`)
          if (!candidates.length) {
            return { candidates: 0, checked: 0, updates: 0, external: external.length, details }
          }

          let responses: Addon[]
          try {
            responses = await invoke<Addon[]>(command, {
              xplanePath,
              [argument]: candidates,
              betaFolders: betaFoldersFor(betaPreferences, type, candidates),
            })
          } catch (error) {
            return {
              candidates: candidates.length,
              checked: 0,
              updates: 0,
              external: external.length,
              details: [...details, ...candidates.map((item) => item.folderName), String(error)],
            }
          }

          const byFolder = new Map(responses.map((item) => [item.folderName, item]))
          let checked = 0
          let updates = 0
          for (const item of candidates) {
            const response = byFolder.get(item.folderName)
            const provider = getUpdateProvider(item)
            if (!response?.latestVersion?.trim()) {
              details.push(`${item.folderName} · ${provider} · no remote version`)
              continue
            }
            if (!item.version?.trim()) {
              details.push(`${item.folderName} · ${provider} · installed version unknown`)
              continue
            }
            checked++
            if (response.hasUpdate) {
              updates++
              details.push(
                `${item.folderName} · ${provider} · ${item.version} → ${response.latestVersion}`,
              )
            }
          }
          return {
            candidates: candidates.length,
            checked,
            updates,
            external: external.length,
            details,
          }
        }),
      )

      let candidates = 0
      let checked = 0
      let updates = 0
      let external = 0
      let failedSources = 0
      const details: string[] = []
      groups.forEach((group, index) => {
        const source = sources[index]!.type
        if (group.status === 'rejected') {
          failedSources++
          details.push(`${source}: ${String(group.reason)}`)
          return
        }
        candidates += group.value.candidates
        checked += group.value.checked
        updates += group.value.updates
        external += group.value.external
        details.push(...group.value.details.map((detail) => `${source}: ${detail}`))
      })
      const incomplete = failedSources > 0 || checked < candidates
      return [
        result(
          'updates.addons',
          'updates',
          incomplete ? 'unavailable' : updates ? 'info' : checked ? 'pass' : 'notApplicable',
          {
            params: { count: updates, installed: candidates, checked, external },
            evidence: evidence(details),
            remediation: updates
              ? { id: 'manage_updates', kind: 'navigate', risk: 'safe', route: '/management' }
              : undefined,
          },
        ),
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
      const byInstallation = new Map(checked.map((item) => [item.id, item]))
      const verified: GatewayInstalledAirport[] = []
      const unavailable: GatewayInstalledAirport[] = []
      for (const item of installed) {
        const response = byInstallation.get(item.id)
        if (
          response?.updateAvailable == null ||
          response.airportIcao !== item.airportIcao ||
          response.sceneryId !== item.sceneryId
        ) {
          unavailable.push(item)
        } else {
          verified.push(response)
        }
      }
      const updates = verified.filter((item) => item.updateAvailable)
      const count = updates.length
      return [
        ...(unavailable.length
          ? [
              result('updates.gateway_unavailable', 'updates', 'unavailable', {
                params: { count: unavailable.length },
                evidence: evidence(unavailable.map((item) => item.airportIcao)),
              }),
            ]
          : []),
        result(
          'updates.gateway',
          'updates',
          verified.length === 0 ? 'unavailable' : count ? 'info' : 'pass',
          {
            params: {
              count,
              installed: installed.length,
              checked: verified.length,
            },
            evidence: evidence(
              updates.map(
                (item) => `${item.airportIcao}: ${item.sceneryId} -> ${item.latestSceneryId}`,
              ),
            ),
            remediation: count
              ? { id: 'manage_gateway', kind: 'navigate', risk: 'safe', route: '/gateway' }
              : undefined,
          },
        ),
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
      const scans = await Promise.allSettled([
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
      let installed = 0
      let count = 0
      const errors: string[] = []
      const updates: string[] = []
      for (const [index, scan] of scans.entries()) {
        const label = index === 0 ? 'X-CSL' : 'ALTITUDE'
        if (scan.status === 'rejected') {
          errors.push(`${label}: ${String(scan.reason)}`)
          continue
        }
        if (scan.value.index_warning) {
          errors.push(`${label}: ${scan.value.index_warning}`)
          continue
        }
        for (const item of scan.value.packages) {
          if (item.status === 'needs_update') {
            count++
            updates.push(`${label} · ${item.name}: ${item.files_to_update} files`)
          }
          if (item.status === 'needs_update' || item.status === 'up_to_date') installed++
          if (item.status === 'checking') errors.push(`${label} · ${item.name}: still unverified`)
        }
      }
      return [
        result(
          'updates.csl',
          'updates',
          errors.length ? 'unavailable' : count ? 'info' : installed ? 'pass' : 'notApplicable',
          {
            params: { count, checked: installed },
            evidence: evidence([...errors, ...updates]),
            remediation: count
              ? { id: 'manage_csl', kind: 'navigate', risk: 'safe', route: '/csl' }
              : undefined,
          },
        ),
      ]
    },
  }
}

export function createDoctorCheckDefinitions(context: DoctorCheckContext): DoctorCheckDefinition[] {
  return [
    createEnvironmentCheck(context),
    createRuntimeCheck(context),
    createInspectionCheck(context),
    createXfastCheck(context),
    createLogCheck(context),
    createCrashDumpCheck(context),
    createSceneryCheck(context),
    createFlattenCheck(context),
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
