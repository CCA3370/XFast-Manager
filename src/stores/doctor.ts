import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from './app'
import { logError } from '@/services/logger'
import type {
  DoctorEnvironmentReport,
  DoctorNavdataReport,
  NavdataCycleReport,
  SceneryManagerData,
  AircraftInfo,
  PluginInfo,
  SceneryManagerEntry,
  ActivityLogPage,
  GatewayInstalledAirport,
} from '@/types'

// ---------------------------------------------------------------------------
// Finding model
// ---------------------------------------------------------------------------

export type DoctorSeverity = 'critical' | 'warning' | 'info' | 'ok'

export type DoctorSection =
  | 'integrity'
  | 'crashes'
  | 'navdata'
  | 'scenery'
  | 'plugins'
  | 'performance'
  | 'environment'
  | 'disk'
  | 'updates'

/** Auto-fix safety tier. 'none' = no auto-fix, guidance only. */
export type DoctorFixTier = 'safe' | 'confirm' | 'destructive' | 'none'

export interface DoctorFinding {
  /** Stable check id, e.g. "scenery.needs_sort". Maps to i18n doctor.checks.<id>.* */
  id: string
  section: DoctorSection
  severity: DoctorSeverity
  /** Interpolation params for the i18n title/description strings. */
  params?: Record<string, string | number>
  /** Extra free-form detail lines (already-localized or raw, shown verbatim, monospace). */
  detail?: string[]
  /** Auto-fix descriptor; absent when this finding is guidance-only. */
  fix?: {
    id: string
    tier: DoctorFixTier
    /** params for the fix button label / confirm dialog text. */
    params?: Record<string, string | number>
  }
  /** A route the user can jump to in order to resolve this manually. */
  route?: string
}

export type DoctorPhase = 'idle' | 'local' | 'network' | 'done'

const SECTION_ORDER: DoctorSection[] = [
  'integrity',
  'crashes',
  'navdata',
  'scenery',
  'plugins',
  'performance',
  'environment',
  'disk',
  'updates',
]

const SEVERITY_RANK: Record<DoctorSeverity, number> = {
  critical: 0,
  warning: 1,
  info: 2,
  ok: 3,
}

// Thresholds
const LOW_DISK_WARN_BYTES = 10 * 1024 * 1024 * 1024 // 10 GB
const LOW_DISK_CRIT_BYTES = 1 * 1024 * 1024 * 1024 // 1 GB
const CLEANABLE_CACHE_WARN_BYTES = 100 * 1024 * 1024 // 100 MB
const READONLY_WARN_COUNT = 10

// Log categories that the Doctor surfaces individually (beyond the crash banner).
// Maps a backend log category -> { section, severity }. Anything not listed is
// folded into a generic "other high-severity log errors" finding.
const LOG_CATEGORY_MAP: Record<string, { section: DoctorSection; severity: DoctorSeverity }> = {
  // Crashes / GPU
  vulkan_device_error: { section: 'crashes', severity: 'critical' },
  vulkan_gfx_error: { section: 'crashes', severity: 'critical' },
  gfx_error: { section: 'crashes', severity: 'warning' },
  nvidia_permission: { section: 'environment', severity: 'warning' },
  // Memory / performance
  out_of_memory: { section: 'performance', severity: 'critical' },
  heavy_memory_pressure: { section: 'performance', severity: 'critical' },
  memory_status_critical: { section: 'performance', severity: 'warning' },
  severe_texture_downscale: { section: 'performance', severity: 'warning' },
  runloop_backlog: { section: 'performance', severity: 'warning' },
  // Plugins
  plugin_error: { section: 'plugins', severity: 'warning' },
  plugin_assert: { section: 'plugins', severity: 'critical' },
  plugin_manager_error: { section: 'plugins', severity: 'critical' },
  duplicate_plugin: { section: 'plugins', severity: 'warning' },
  missing_plugin_support: { section: 'plugins', severity: 'warning' },
  deprecated_dataref: { section: 'plugins', severity: 'info' },
  // Scenery
  dsf_error: { section: 'scenery', severity: 'warning' },
  scenery_error: { section: 'scenery', severity: 'warning' },
  scenery_load_failed: { section: 'scenery', severity: 'warning' },
  // Environment
  third_party_blocked: { section: 'environment', severity: 'warning' },
  ssl_failed: { section: 'environment', severity: 'info' },
}

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
  total_high: number
  total_medium: number
  total_low: number
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

export interface DoctorSystemInfo {
  xplaneVersion: string | null
  xplaneVersionRaw: string | null
  isBeta: boolean
  gpuModel: string | null
  gpuDriver: string | null
}

/** Detect a beta/dev X-Plane build from the raw version token. */
function detectBeta(raw: string | null): boolean {
  if (!raw) return false
  const v = raw.toLowerCase()
  // Stable releases use "-r" (release) or a bare x.y.z; betas use -b / beta / -d (dev).
  return /(-b\d|beta|-d\d|\bdev\b|alpha|-rc)/i.test(v)
}

export const useDoctorStore = defineStore('doctor', () => {
  const appStore = useAppStore()

  const phase = ref<DoctorPhase>('idle')
  const findings = ref<DoctorFinding[]>([])
  const systemInfo = ref<DoctorSystemInfo | null>(null)
  const error = ref<string | null>(null)
  const lastRun = ref<number | null>(null)
  const xplaneRunning = ref(false)
  const fixingId = ref<string | null>(null)

  // Per-check progress so the UI can show what's still loading.
  const runningChecks = ref<Set<string>>(new Set())

  const isRunning = computed(() => phase.value === 'local' || phase.value === 'network')

  const sortedFindings = computed(() => {
    return [...findings.value].sort((a, b) => {
      const sa = SECTION_ORDER.indexOf(a.section)
      const sb = SECTION_ORDER.indexOf(b.section)
      if (sa !== sb) return sa - sb
      return SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity]
    })
  })

  const findingsBySection = computed(() => {
    const map = new Map<DoctorSection, DoctorFinding[]>()
    for (const section of SECTION_ORDER) map.set(section, [])
    for (const f of sortedFindings.value) {
      map.get(f.section)?.push(f)
    }
    // Drop empty sections
    const out: { section: DoctorSection; findings: DoctorFinding[] }[] = []
    for (const section of SECTION_ORDER) {
      const list = map.get(section) ?? []
      if (list.length > 0) out.push({ section, findings: list })
    }
    return out
  })

  const counts = computed(() => {
    let critical = 0
    let warning = 0
    let info = 0
    for (const f of findings.value) {
      if (f.severity === 'critical') critical++
      else if (f.severity === 'warning') warning++
      else if (f.severity === 'info') info++
    }
    return { critical, warning, info, total: findings.value.length }
  })

  const overallSeverity = computed<DoctorSeverity>(() => {
    if (counts.value.critical > 0) return 'critical'
    if (counts.value.warning > 0) return 'warning'
    if (counts.value.info > 0) return 'info'
    return 'ok'
  })

  function add(finding: DoctorFinding) {
    findings.value.push(finding)
  }

  function removeById(id: string) {
    findings.value = findings.value.filter((f) => f.id !== id)
  }

  function reset() {
    findings.value = []
    systemInfo.value = null
    error.value = null
    phase.value = 'idle'
    runningChecks.value = new Set()
  }

  // -------------------------------------------------------------------------
  // Main entry point
  // -------------------------------------------------------------------------

  async function runDiagnostics() {
    if (isRunning.value) return
    const xplanePath = appStore.xplanePath
    if (!xplanePath) {
      reset()
      return
    }

    findings.value = []
    error.value = null
    phase.value = 'local'
    runningChecks.value = new Set()

    // Gating: validate path + detect running sim first.
    try {
      const valid = await invoke<boolean>('validate_xplane_path', { path: xplanePath })
      if (!valid) {
        add({
          id: 'integrity.path_invalid',
          section: 'integrity',
          severity: 'critical',
          route: '/settings',
        })
        phase.value = 'done'
        lastRun.value = Date.now()
        return
      }
    } catch (e) {
      logError(`Doctor: path validation failed: ${e}`, 'doctor')
    }

    try {
      xplaneRunning.value = await invoke<boolean>('is_xplane_running')
    } catch {
      xplaneRunning.value = false
    }
    if (xplaneRunning.value) {
      add({ id: 'integrity.xplane_running', section: 'integrity', severity: 'info' })
    }

    // Run all local checks concurrently; each contributes findings independently.
    const localChecks: Promise<void>[] = [
      checkLog(xplanePath),
      checkScenery(xplanePath),
      checkNavdata(xplanePath),
      checkEnvironment(xplanePath),
      checkFlatten(xplanePath),
      checkCleanup(xplanePath),
      checkActivity(),
    ]
    await Promise.allSettled(localChecks)

    phase.value = 'network'
    const networkChecks: Promise<void>[] = [
      checkAddonUpdates(xplanePath),
      checkGatewayUpdates(xplanePath),
      checkAppUpdate(),
    ]
    await Promise.allSettled(networkChecks)

    phase.value = 'done'
    lastRun.value = Date.now()
  }

  function markRunning(id: string, on: boolean) {
    const next = new Set(runningChecks.value)
    if (on) next.add(id)
    else next.delete(id)
    runningChecks.value = next
  }

  // -------------------------------------------------------------------------
  // LOCAL: Log + crash analysis
  // -------------------------------------------------------------------------

  async function checkLog(xplanePath: string) {
    markRunning('log', true)
    try {
      const result = await invoke<XPlaneLogAnalysis>('analyze_xplane_log', { xplanePath })

      const raw = result.system_info.xplane_version
      systemInfo.value = {
        xplaneVersion: raw ? raw.split('-')[0] : null,
        xplaneVersionRaw: raw,
        isBeta: detectBeta(raw),
        gpuModel: result.system_info.gpu_model,
        gpuDriver: result.system_info.gpu_driver,
      }

      if (!result.is_xplane_log) {
        // Nothing actionable; skip log-derived findings.
        markRunning('log', false)
        return
      }

      // Beta build (frontend-derived from raw version)
      if (systemInfo.value.isBeta) {
        add({
          id: 'environment.beta_build',
          section: 'environment',
          severity: 'info',
          params: { version: systemInfo.value.xplaneVersionRaw ?? '' },
        })
      }

      // Intel iGPU (unsupported) — from gpu_model string
      const gpu = (result.system_info.gpu_model ?? '').toLowerCase()
      if (gpu.includes('intel') && !gpu.includes('arc')) {
        add({
          id: 'environment.intel_gpu',
          section: 'environment',
          severity: 'critical',
          params: { gpu: result.system_info.gpu_model ?? '' },
        })
      }

      // Crash banner
      if (result.crash_detected) {
        add({
          id: 'crashes.last_session_crashed',
          section: 'crashes',
          severity: 'critical',
          detail: result.crash_info ? result.crash_info.split('\n').slice(0, 12) : undefined,
        })
        // Deep crash analysis (newest .dmp), only when dmp analysis is enabled.
        if (appStore.crashAnalysisDmpEnabled) {
          await checkCrashReport(xplanePath, result.issues)
        }
      }

      // Map individual log categories to findings (dedup per category).
      const seen = new Set<string>()
      const otherHigh: string[] = []
      for (const issue of result.issues) {
        if (issue.category === 'crash') continue // covered by banner
        if (seen.has(issue.category)) continue
        seen.add(issue.category)

        const mapping = LOG_CATEGORY_MAP[issue.category]
        if (mapping) {
          const sample = issue.sample_line ? issue.sample_line.split('\n').slice(0, 4) : undefined
          add({
            id: `log.${issue.category}`,
            section: mapping.section,
            severity: mapping.severity,
            params: { line: issue.line_numbers[0] ?? 0 },
            detail: sample,
          })
        } else if (issue.severity === 'high') {
          otherHigh.push(issue.category)
        }
      }
      if (otherHigh.length > 0) {
        add({
          id: 'log.other_high',
          section: 'crashes',
          severity: 'warning',
          params: { count: otherHigh.length, categories: otherHigh.join(', ') },
        })
      }
    } catch (e) {
      // Log.txt missing is not fatal — many fresh installs lack it.
      logError(`Doctor: log analysis failed: ${e}`, 'doctor')
    } finally {
      markRunning('log', false)
    }
  }

  async function checkCrashReport(xplanePath: string, logIssues: LogIssue[]) {
    try {
      const crash = await invoke<DeepCrashAnalysis | null>('analyze_crash_report', {
        xplanePath,
        logIssues,
        skipDateCheck: appStore.crashAnalysisIgnoreDateCheck,
      })
      if (!crash) return
      const top = crash.crash_causes[0]
      if (top) {
        // Map cause -> section/severity
        const causeSection: DoctorSection =
          top.cause_key === 'plugin_crash'
            ? 'plugins'
            : top.cause_key === 'gpu_driver_crash'
              ? 'environment'
              : 'crashes'
        add({
          id: `crash_cause.${top.cause_key}`,
          section: causeSection,
          severity: 'critical',
          params: {
            score: top.score.toFixed(0),
            module: top.blamed_module ?? '',
          },
          detail: crash.loaded_plugins.slice(0, 8),
        })
      }
    } catch (e) {
      logError(`Doctor: crash report analysis failed: ${e}`, 'doctor')
    }
  }

  // -------------------------------------------------------------------------
  // LOCAL: Scenery
  // -------------------------------------------------------------------------

  async function checkScenery(xplanePath: string) {
    markRunning('scenery', true)
    try {
      const data = await invoke<SceneryManagerData>('get_scenery_manager_data', { xplanePath })

      // Global Airports disabled
      const ga = data.entries.find((e) => e.folderName === '*GLOBAL_AIRPORTS*')
      if (ga && !ga.enabled) {
        add({
          id: 'scenery.global_airports_disabled',
          section: 'scenery',
          severity: 'critical',
          fix: { id: 'enable_global_airports', tier: 'safe' },
        })
      }

      // Out-of-sync / wrong load order
      if (data.needsSync) {
        add({
          id: 'scenery.needs_sort',
          section: 'scenery',
          severity: 'warning',
          fix: { id: 'sort_scenery', tier: 'safe' },
          route: '/management?tab=scenery',
        })
      }

      // Missing libraries
      if (data.missingDepsCount > 0) {
        const affected = data.entries
          .filter((e) => e.missingLibraries && e.missingLibraries.length > 0)
          .slice(0, 8)
        const libs = new Set<string>()
        for (const e of affected) for (const l of e.missingLibraries) libs.add(l)
        add({
          id: 'scenery.missing_libraries',
          section: 'scenery',
          severity: 'warning',
          params: { count: data.missingDepsCount },
          detail: [...libs].slice(0, 10),
          route: '/management?tab=scenery',
        })
      }

      // Overlapping tiles
      if (data.duplicateTilesCount > 0) {
        add({
          id: 'scenery.duplicate_tiles',
          section: 'scenery',
          severity: 'warning',
          params: { count: data.duplicateTilesCount },
          route: '/management?tab=scenery',
        })
      }

      // Duplicate airports
      if (data.duplicateAirportsCount > 0) {
        add({
          id: 'scenery.duplicate_airports',
          section: 'scenery',
          severity: 'info',
          params: { count: data.duplicateAirportsCount },
          route: '/management?tab=scenery',
        })
      }
    } catch (e) {
      logError(`Doctor: scenery check failed: ${e}`, 'doctor')
    } finally {
      markRunning('scenery', false)
    }
  }

  // -------------------------------------------------------------------------
  // LOCAL: Navdata
  // -------------------------------------------------------------------------

  async function checkNavdata(xplanePath: string) {
    markRunning('navdata', true)
    try {
      const report = await invoke<DoctorNavdataReport>('doctor_navdata_status', { xplanePath })

      if (!report.customDataExists) {
        add({ id: 'navdata.no_custom_data', section: 'navdata', severity: 'info' })
        markRunning('navdata', false)
        return
      }

      // CIFP missing (procedures unavailable)
      if (!report.cifpPresent && report.cycles.length > 0) {
        add({ id: 'navdata.cifp_missing', section: 'navdata', severity: 'warning' })
      }

      // earth_*.dat partial set
      if (report.earthDatMissing.length > 0) {
        add({
          id: 'navdata.earth_dat_missing',
          section: 'navdata',
          severity: 'warning',
          detail: report.earthDatMissing,
        })
      }

      // Per-cycle expiry
      let flaggedExpiry = false
      for (const cycle of report.cycles) {
        if (cycle.status === 'expired') {
          flaggedExpiry = true
          add({
            id: 'navdata.expired',
            section: 'navdata',
            severity: 'warning',
            params: cycleParams(cycle),
          })
        } else if (cycle.status === 'expiringSoon') {
          flaggedExpiry = true
          add({
            id: 'navdata.expiring_soon',
            section: 'navdata',
            severity: 'info',
            params: cycleParams(cycle),
          })
        }
      }

      // Mismatched cycles across folders (distinct cycle values > 1)
      const distinctCycles = new Set(report.cycles.map((c) => c.cycle).filter(Boolean))
      if (distinctCycles.size > 1) {
        add({
          id: 'navdata.cycle_mismatch',
          section: 'navdata',
          severity: 'info',
          params: { cycles: [...distinctCycles].join(', ') },
        })
      }

      void flaggedExpiry
    } catch (e) {
      logError(`Doctor: navdata check failed: ${e}`, 'doctor')
    } finally {
      markRunning('navdata', false)
    }
  }

  function cycleParams(cycle: NavdataCycleReport): Record<string, string | number> {
    return {
      provider: cycle.providerName,
      cycle: cycle.cycle ?? '?',
      expiry: cycle.expiryDate ?? '?',
      days: cycle.daysRemaining ?? 0,
    }
  }

  // -------------------------------------------------------------------------
  // LOCAL: Environment (disk, injectors, organizers, install integrity)
  // -------------------------------------------------------------------------

  async function checkEnvironment(xplanePath: string) {
    markRunning('environment', true)
    try {
      const env = await invoke<DoctorEnvironmentReport>('doctor_scan_environment', { xplanePath })

      // Missing core dirs => broken install (critical)
      if (env.missingCoreDirs.length > 0) {
        add({
          id: 'integrity.missing_core_dirs',
          section: 'integrity',
          severity: 'critical',
          detail: env.missingCoreDirs,
        })
      }

      // Disk space
      if (env.freeBytes < LOW_DISK_CRIT_BYTES) {
        add({
          id: 'disk.low_space',
          section: 'disk',
          severity: 'critical',
          params: { free: formatBytes(env.freeBytes) },
        })
      } else if (env.freeBytes < LOW_DISK_WARN_BYTES) {
        add({
          id: 'disk.low_space',
          section: 'disk',
          severity: 'warning',
          params: { free: formatBytes(env.freeBytes) },
        })
      }

      // Program Files (UAC trap)
      if (env.inProgramFiles) {
        add({ id: 'integrity.program_files', section: 'integrity', severity: 'warning' })
      }

      // Read-only files blocking updates
      if (env.readonlyCount >= READONLY_WARN_COUNT) {
        add({
          id: 'integrity.readonly_files',
          section: 'integrity',
          severity: 'warning',
          params: {
            count: env.readonlyScanCapped ? `${env.readonlyCount}+` : env.readonlyCount,
          },
        })
      }

      // Injectors (ReShade etc.) — destructive guidance only
      if (env.injectors.length > 0) {
        add({
          id: 'environment.injectors',
          section: 'environment',
          severity: 'warning',
          fix: { id: 'guide_injectors', tier: 'destructive' },
          detail: env.injectors.map((i) => i.evidence),
        })
      }

      // Competing scenery organizers
      if (env.competingOrganizers.length > 0) {
        add({
          id: 'scenery.competing_organizer',
          section: 'scenery',
          severity: 'info',
          detail: env.competingOrganizers.map((o) => o.evidence),
        })
      }
    } catch (e) {
      logError(`Doctor: environment scan failed: ${e}`, 'doctor')
    } finally {
      markRunning('environment', false)
    }
  }

  // -------------------------------------------------------------------------
  // LOCAL: Airport flatten drift
  // -------------------------------------------------------------------------

  async function checkFlatten(xplanePath: string) {
    markRunning('flatten', true)
    try {
      const overrides = await invoke<{ status: string }[]>('airport_flatten_list_overrides', {
        xplanePath,
      })
      const drifted = overrides.filter((o) => o.status === 'drifted')
      if (drifted.length > 0) {
        add({
          id: 'scenery.flatten_drift',
          section: 'scenery',
          severity: 'warning',
          params: { count: drifted.length },
          fix: { id: 'apply_flatten', tier: 'safe' },
          route: '/airport-flatten',
        })
      }
    } catch (e) {
      logError(`Doctor: flatten check failed: ${e}`, 'doctor')
    } finally {
      markRunning('flatten', false)
    }
  }

  // -------------------------------------------------------------------------
  // LOCAL: Output cleanup (cleanable caches)
  // -------------------------------------------------------------------------

  async function checkCleanup(xplanePath: string) {
    markRunning('cleanup', true)
    try {
      const report = await invoke<{ totalBytes: number; totalFiles: number }>(
        'scan_output_cleanup_items',
        { xplanePath },
      )
      if (report.totalBytes >= CLEANABLE_CACHE_WARN_BYTES) {
        add({
          id: 'disk.cleanable_caches',
          section: 'disk',
          severity: 'info',
          params: { size: formatBytes(report.totalBytes) },
          fix: { id: 'open_cleanup', tier: 'none' },
          route: '/disk-usage/output-cleanup',
        })
      }
    } catch (e) {
      logError(`Doctor: cleanup scan failed: ${e}`, 'doctor')
    } finally {
      markRunning('cleanup', false)
    }
  }

  // -------------------------------------------------------------------------
  // LOCAL: Activity log (recent install/update failures)
  // -------------------------------------------------------------------------

  async function checkActivity() {
    markRunning('activity', true)
    try {
      const page = await invoke<ActivityLogPage>('get_activity_log', { limit: 25, offset: 0 })
      const recentFailures = page.entries.filter(
        (e) => !e.success && (e.operation === 'install' || e.operation === 'update'),
      )
      if (recentFailures.length > 0) {
        add({
          id: 'integrity.recent_failures',
          section: 'integrity',
          severity: 'info',
          params: { count: recentFailures.length },
          detail: recentFailures.slice(0, 5).map((f) => `${f.operation} · ${f.itemName}`),
          route: '/activity',
        })
      }
    } catch (e) {
      logError(`Doctor: activity check failed: ${e}`, 'doctor')
    } finally {
      markRunning('activity', false)
    }
  }

  // -------------------------------------------------------------------------
  // NETWORK: Addon updates (aircraft + plugins + scenery)
  // -------------------------------------------------------------------------

  async function checkAddonUpdates(xplanePath: string) {
    markRunning('updates', true)
    try {
      let total = 0
      const breakdown: string[] = []

      // Aircraft
      try {
        const ac = await invoke<{ entries: AircraftInfo[] }>('scan_aircraft', { xplanePath })
        const candidates = ac.entries.filter((a) => a.updateUrl)
        if (candidates.length > 0) {
          const checked = await invoke<AircraftInfo[]>('check_aircraft_updates', {
            xplanePath,
            aircraft: candidates,
            betaFolders: [],
          })
          const n = checked.filter((a) => a.hasUpdate).length
          if (n > 0) {
            total += n
            breakdown.push(`aircraft:${n}`)
          }
        }
      } catch (e) {
        logError(`Doctor: aircraft update check failed: ${e}`, 'doctor')
      }

      // Plugins
      try {
        const pl = await invoke<{ entries: PluginInfo[] }>('scan_plugins', { xplanePath })
        const candidates = pl.entries.filter((p) => p.updateUrl)
        if (candidates.length > 0) {
          const checked = await invoke<PluginInfo[]>('check_plugins_updates', {
            xplanePath,
            plugins: candidates,
            betaFolders: [],
          })
          const n = checked.filter((p) => p.hasUpdate).length
          if (n > 0) {
            total += n
            breakdown.push(`plugins:${n}`)
          }
        }
      } catch (e) {
        logError(`Doctor: plugin update check failed: ${e}`, 'doctor')
      }

      // Scenery
      try {
        const sc = await invoke<SceneryManagerData>('get_scenery_manager_data', { xplanePath })
        const candidates = sc.entries.filter((s) => s.updateUrl)
        if (candidates.length > 0) {
          const checked = await invoke<SceneryManagerEntry[]>('check_scenery_updates', {
            xplanePath,
            scenery: candidates,
            betaFolders: [],
          })
          const n = checked.filter((s) => s.hasUpdate).length
          if (n > 0) {
            total += n
            breakdown.push(`scenery:${n}`)
          }
        }
      } catch (e) {
        logError(`Doctor: scenery update check failed: ${e}`, 'doctor')
      }

      if (total > 0) {
        add({
          id: 'updates.addons',
          section: 'updates',
          severity: 'info',
          params: { count: total },
          route: '/management',
        })
      }
    } finally {
      markRunning('updates', false)
    }
  }

  async function checkGatewayUpdates(xplanePath: string) {
    markRunning('gateway', true)
    try {
      const installed = await invoke<GatewayInstalledAirport[]>('gateway_list_installed', {
        xplanePath,
      })
      if (installed.length === 0) return
      const checked = await invoke<GatewayInstalledAirport[]>('gateway_check_updates', {
        xplanePath,
      })
      const n = checked.filter((a) => a.updateAvailable === true).length
      if (n > 0) {
        add({
          id: 'updates.gateway',
          section: 'updates',
          severity: 'info',
          params: { count: n },
          route: '/gateway',
        })
      }
    } catch (e) {
      logError(`Doctor: gateway update check failed: ${e}`, 'doctor')
    } finally {
      markRunning('gateway', false)
    }
  }

  async function checkAppUpdate() {
    markRunning('app', true)
    try {
      const result = await invoke<{ isUpdateAvailable: boolean; latestVersion: string }>(
        'check_for_updates',
        { manual: false, includePreRelease: false },
      )
      if (result.isUpdateAvailable) {
        add({
          id: 'updates.app',
          section: 'updates',
          severity: 'info',
          params: { version: result.latestVersion },
          fix: { id: 'open_app_update', tier: 'safe' },
        })
      }
    } catch {
      // Update check is best-effort (rate limits, offline).
    } finally {
      markRunning('app', false)
    }
  }

  // -------------------------------------------------------------------------
  // FIX handlers
  // -------------------------------------------------------------------------

  /** Run a fix. Returns true on success. The page handles confirm dialogs for
   *  'confirm'/'destructive' tiers BEFORE calling this. */
  async function applyFix(finding: DoctorFinding): Promise<boolean> {
    if (!finding.fix) return false
    const xplanePath = appStore.xplanePath
    if (!xplanePath) return false

    // Re-check running state for filesystem-mutating fixes.
    if (finding.fix.id !== 'open_cleanup' && finding.fix.id !== 'open_app_update') {
      try {
        xplaneRunning.value = await invoke<boolean>('is_xplane_running')
      } catch {
        /* ignore */
      }
      if (xplaneRunning.value && isFsMutatingFix(finding.fix.id)) {
        error.value = 'xplane_running'
        return false
      }
    }

    fixingId.value = finding.id
    try {
      switch (finding.fix.id) {
        case 'sort_scenery':
          await invoke('sort_scenery_packs', { xplanePath, lockedFolderNames: null })
          removeById(finding.id)
          return true
        case 'enable_global_airports':
          await invoke('update_scenery_entry', {
            xplanePath,
            folderName: '*GLOBAL_AIRPORTS*',
            enabled: true,
            sortOrder: null,
            category: null,
          })
          removeById(finding.id)
          return true
        case 'apply_flatten':
          await invoke('airport_flatten_apply_all_drifted', { xplanePath })
          removeById(finding.id)
          return true
        default:
          // Guidance-only / route-based fixes are handled by the page (navigation).
          return false
      }
    } catch (e) {
      logError(`Doctor: fix ${finding.fix.id} failed: ${e}`, 'doctor')
      error.value = String(e)
      return false
    } finally {
      fixingId.value = null
    }
  }

  function isFsMutatingFix(fixId: string): boolean {
    return ['sort_scenery', 'enable_global_airports', 'apply_flatten'].includes(fixId)
  }

  return {
    // state
    phase,
    findings,
    systemInfo,
    error,
    lastRun,
    xplaneRunning,
    fixingId,
    runningChecks,
    // computed
    isRunning,
    sortedFindings,
    findingsBySection,
    counts,
    overallSeverity,
    // actions
    runDiagnostics,
    applyFix,
    reset,
  }
})

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
}
