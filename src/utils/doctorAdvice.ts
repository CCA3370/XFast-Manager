import type { DoctorCheckResult } from '@/types/doctor'
import { doctorAdvice } from '@/i18n/doctorAdvice'

type AdviceKey = keyof typeof doctorAdvice

const findingAdvice: Partial<Record<string, AdviceKey>> = {
  'addons.platform': 'pluginPlatform',
  'installation.airfoils': 'restoreAirfoils',
  'installation.write_access': 'writableFolders',
  'performance.available_memory': 'availableMemory',
  'scenery.broken_links': 'brokenLinks',
  'scenery.competing_organizer': 'sceneryOrganizers',
  'scenery.dependencies': 'dependencies',
  'scenery.duplicate_airports': 'duplicateAirports',
  'scenery.duplicate_paths': 'duplicatePaths',
  'scenery.global_airports': 'globalAirports',
  'scenery.invalid_entries': 'invalidEntries',
  'scenery.missing_paths': 'missingPaths',
  'scenery.overlaps': 'overlaps',
  'storage.app_data_space': 'applicationSpace',
  'storage.cleanable': 'cleanable',
  'storage.xplane_space': 'simulatorSpace',
  'system.beta_build': 'beta',
  'system.injectors': 'injector',
  'system.inventory': 'hardware',
  'system.vulkan_runtime': 'vulkan',
  'navdata.consistency': 'navConflict',
  'navdata.integrity': 'navOverride',
  'navdata.empty_files': 'navOverride',
  'navdata.cifp': 'navOverride',
  'navdata.supplemental': 'navSupplemental',
  'updates.app': 'appUpdate',
  'xfast.app_data': 'appData',
  'xfast.database': 'appDatabase',
  'xfast.recent_failures': 'recentFailures',
  'xfast.scenery_index': 'sceneryIndex',
  'log.plugin_manager_error': 'isolatePlugin',
  'log.plugin_assert': 'isolatePlugin',
  'log.plugin_error': 'isolatePlugin',
  'log.duplicate_plugin': 'duplicatePlugin',
  'log.vulkan_device_error': 'graphicsLog',
  'log.vulkan_gfx_error': 'graphicsLog',
  'log.gfx_error': 'graphicsLog',
  'log.nvidia_permission': 'graphicsLog',
  'log.third_party_blocked': 'blockedLayer',
  'log.memory_access_error': 'logContext',
  'log.scenery_load_failed': 'sceneryLog',
  'log.scenery_error': 'sceneryLog',
  'log.dsf_error': 'sceneryLog',
  'log.out_of_memory': 'availableMemory',
  'log.heavy_memory_pressure': 'availableMemory',
  'log.memory_status_critical': 'availableMemory',
}

const unavailableAdvice: Partial<Record<string, AdviceKey>> = {
  'scenery.registration': 'registrationUnreadable',
  'storage.app_data_space': 'diskUnavailable',
  'storage.xplane_space': 'diskUnavailable',
  'system.inventory': 'hardwareUnavailable',
  'navdata.cycles': 'navUnknown',
  'navdata.scan_access': 'readAccess',
  'updates.gateway': 'retryUpdate',
  'updates.gateway_unavailable': 'retryUpdate',
  'updates.app': 'retryUpdate',
  'diagnostic.addon_updates': 'retryUpdate',
  'diagnostic.app_update': 'retryUpdate',
  'diagnostic.csl_updates': 'retryUpdate',
  'diagnostic.gateway_updates': 'retryUpdate',
  'xfast.database': 'appDatabase',
  'xfast.scenery_index': 'sceneryIndex',
}

const stockFileChecks = new Set([
  'installation.structure',
  'installation.stock_aircraft',
  'navdata.bundled',
  'log.missing_plugin_support',
])

const registrationChecks = new Set([
  'scenery.global_airports',
  'scenery.invalid_entries',
  'scenery.missing_paths',
  'scenery.duplicate_paths',
  'scenery.files',
])

function incompleteFileAdvice(check: DoctorCheckResult): AdviceKey[] {
  if (!check.params?.capped) return ['readAccess']
  return Number(check.params.errors) > 0 ? ['readAccess', 'scanLimit'] : ['scanLimit']
}

export function getDoctorAdvice(
  check: DoctorCheckResult,
  checks: readonly DoctorCheckResult[],
): AdviceKey[] | undefined {
  const { id, outcome, params, remediation } = check
  if (outcome === 'pass' || outcome === 'cancelled') return []

  // These checks carry their own context, including meaningful skipped results.
  switch (id) {
    case 'stability.last_session':
      if (params?.sessionEnd === 'running') return ['activeSession']
      if (outcome === 'unavailable') return ['sessionStateUnavailable']
      if (params?.sessionEnd === 'crash') {
        const oldLog = checks.some(
          (item) => item.id === 'stability.log_freshness' && item.outcome === 'info',
        )
        return [oldLog ? 'oldCrashLog' : 'saveCrashLog']
      }
      return outcome === 'info' ? ['unknownSession'] : []
    case 'stability.log_available':
      if (outcome === 'notApplicable') return ['missingLog']
      return outcome === 'unavailable' ? ['unrecognizedLog'] : []
    case 'stability.log_freshness':
      return outcome === 'info' ? ['oldLog'] : []
    case 'stability.crash_dump':
      if (remediation?.id === 'enable_dmp_analysis') return ['dumpDisabled']
      if (outcome === 'notApplicable') {
        return checks.some(
          (item) => item.id === 'stability.last_session' && item.params?.sessionEnd === 'crash',
        )
          ? ['dumpMissing']
          : []
      }
      if (outcome === 'unavailable') return ['dumpUnavailable']
      if (params?.cause === 'plugin_crash' && params.module) return ['dumpPlugin']
      if (params?.cause === 'gpu_driver_crash') return ['dumpGpu']
      if (params?.cause === 'memory_exhaustion') return ['dumpMemory']
      return ['dumpInconclusive']
    case 'updates.addons': {
      const advice: AdviceKey[] = []
      const manual = Number(params?.manual ?? 0)
      if (Number(params?.count) > manual) advice.push('addonUpdate')
      if (manual > 0) advice.push('addonManual')
      if (Number(params?.external) > 0) advice.push('addonExternal')
      if (outcome === 'unavailable') advice.push('retryUpdate')
      return advice
    }
    case 'updates.csl': {
      const advice: AdviceKey[] = []
      if (Number(params?.count) > 0) advice.push('cslUpdate')
      if (outcome === 'unavailable') advice.push('retryUpdate')
      return advice
    }
  }

  if (outcome === 'notApplicable') return []
  if (id === 'addons.binary_integrity' && params?.requiresRescan) return ['oldPluginScan']

  if (outcome === 'unavailable') {
    const specific = Object.hasOwn(unavailableAdvice, id) ? unavailableAdvice[id] : undefined
    if (specific) return [specific]
    if (
      registrationChecks.has(id) &&
      checks.some((item) => item.id === 'scenery.registration' && item.outcome === 'unavailable')
    ) {
      return params?.capped ? ['registrationUnreadable', 'scanLimit'] : ['registrationUnreadable']
    }
    if (
      ['installation', 'addons', 'navdata', 'scenery'].includes(check.section) &&
      !id.startsWith('diagnostic.')
    ) {
      return incompleteFileAdvice(check)
    }
    return ['retryCheck']
  }

  if (stockFileChecks.has(id)) {
    if (remediation?.id === 'configure_xplane_path') return ['selectInstallation']
    const steam = checks.find((item) => item.id === 'installation.location')?.params?.steam
    return [steam === 1 ? 'repairSteam' : steam === 0 ? 'repairInstaller' : 'repairStockFiles']
  }

  switch (id) {
    case 'installation.location':
      return [params?.steam === 1 ? 'steamLocation' : 'protectedLocation']
    case 'installation.xplane_running':
      return ['activeSession']
    case 'installation.readonly':
      return params?.capped ? ['readOnlyFiles', ...incompleteFileAdvice(check)] : ['readOnlyFiles']
    case 'scenery.registration':
      return ['registrationMissing']
    case 'scenery.load_order':
      return [remediation?.id === 'sort_scenery' ? 'sortScenery' : 'manualScenery']
    case 'scenery.files':
      return params?.capped ? ['sceneryFiles', ...incompleteFileAdvice(check)] : ['sceneryFiles']
    case 'scenery.flatten': {
      const advice: AdviceKey[] = []
      if (remediation?.id === 'apply_flatten') advice.push('flattenDrifted')
      if (
        !advice.length ||
        Number(params?.sourceMissing) > 0 ||
        (params?.sourceMissing === undefined &&
          check.evidence?.some((item) => item.value.endsWith(' (sourceMissing)')))
      ) {
        advice.push('flattenMissing')
      }
      return advice
    }
    case 'addons.binary_integrity':
      return params?.capped ? ['pluginBinary', ...incompleteFileAdvice(check)] : ['pluginBinary']
    case 'navdata.cycles': {
      const advice: AdviceKey[] = []
      if (Number(params?.future) > 0) advice.push('navFuture')
      if (Number(params?.conflicts) > 0) advice.push('navConflict')
      if (Number(params?.expired) > 0) advice.push('navExpired')
      else if (Number(params?.expiring) > 0) advice.push('navExpiring')
      return advice.length ? advice : ['navUnknown']
    }
    case 'updates.gateway':
      return Number(params?.count) > 0 ? ['gatewayUpdate'] : []
  }

  const specific = Object.hasOwn(findingAdvice, id) ? findingAdvice[id] : undefined
  if (specific) return params?.capped ? [specific, ...incompleteFileAdvice(check)] : [specific]
  if (outcome === 'warning' || outcome === 'critical') {
    return id.startsWith('log.') ? ['logContext'] : ['retryCheck']
  }
}
