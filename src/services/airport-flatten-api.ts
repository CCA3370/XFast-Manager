import { invokeCommand } from '@/services/api'
import type {
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

export async function sceneryGetFlattenTarget(
  xplanePath: string,
  folderName: string,
): Promise<AirportFlattenTarget | null> {
  return invokeCommand<AirportFlattenTarget | null>('scenery_get_flatten_target', {
    xplanePath,
    folderName,
  })
}
