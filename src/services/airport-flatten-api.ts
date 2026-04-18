import { invokeCommand } from '@/services/api'
import type {
  AirportFlattenApplyAllResult,
  AirportFlattenOverride,
  AirportFlattenSearchResult,
  AirportFlattenTarget,
  SetAirportFlattenRequest,
} from '@/types'

export async function airportFlattenSearchAirports(
  xplanePath: string,
  query: string,
  limit = 20,
): Promise<AirportFlattenSearchResult[]> {
  return invokeCommand<AirportFlattenSearchResult[]>('airport_flatten_search_airports', {
    xplanePath,
    query,
    limit,
  })
}

export async function airportFlattenGetTargets(
  xplanePath: string,
  icao: string,
): Promise<AirportFlattenTarget[]> {
  return invokeCommand<AirportFlattenTarget[]>('airport_flatten_get_targets', {
    xplanePath,
    icao,
  })
}

export async function airportFlattenSetState(
  request: SetAirportFlattenRequest,
): Promise<AirportFlattenTarget> {
  return invokeCommand<AirportFlattenTarget>('airport_flatten_set_state', {
    request,
  })
}

export async function airportFlattenListOverrides(
  xplanePath: string,
): Promise<AirportFlattenOverride[]> {
  return invokeCommand<AirportFlattenOverride[]>('airport_flatten_list_overrides', {
    xplanePath,
  })
}

export async function airportFlattenClearOverride(
  xplanePath: string,
  icao: string,
  sourcePath: string,
): Promise<void> {
  return invokeCommand<void>('airport_flatten_clear_override', {
    xplanePath,
    icao,
    sourcePath,
  })
}

export async function airportFlattenApplyAllDrifted(
  xplanePath: string,
): Promise<AirportFlattenApplyAllResult> {
  return invokeCommand<AirportFlattenApplyAllResult>('airport_flatten_apply_all_drifted', {
    xplanePath,
  })
}

export async function sceneryGetFlattenTarget(
  xplanePath: string,
  folderName: string,
): Promise<AirportFlattenTarget | null> {
  return invokeCommand<AirportFlattenTarget | null>('scenery_get_flatten_target', {
    xplanePath,
    folderName,
  })
}
