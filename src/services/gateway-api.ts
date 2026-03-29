import { invokeCommand, invokeVoidCommand } from '@/services/api'
import type {
  GatewayAirportDetail,
  GatewayAirportSearchResult,
  GatewayInstalledAirport,
  GatewaySceneryDetail,
} from '@/types'

export async function gatewaySearchAirports(
  query: string,
  limit = 20,
): Promise<GatewayAirportSearchResult[]> {
  return invokeCommand<GatewayAirportSearchResult[]>('gateway_search_airports', {
    query,
    limit,
  })
}

export async function gatewayGetAirport(icao: string): Promise<GatewayAirportDetail> {
  return invokeCommand<GatewayAirportDetail>('gateway_get_airport', { icao })
}

export async function gatewayGetScenery(sceneryId: number): Promise<GatewaySceneryDetail> {
  return invokeCommand<GatewaySceneryDetail>('gateway_get_scenery', { sceneryId })
}

export async function gatewayListInstalled(xplanePath: string): Promise<GatewayInstalledAirport[]> {
  return invokeCommand<GatewayInstalledAirport[]>('gateway_list_installed', { xplanePath })
}

export async function gatewayCheckUpdates(xplanePath: string): Promise<GatewayInstalledAirport[]> {
  return invokeCommand<GatewayInstalledAirport[]>('gateway_check_updates', { xplanePath })
}

export async function gatewayInstallScenery(request: {
  xplanePath: string
  icao: string
  sceneryId: number
  autoSortScenery?: boolean
}): Promise<GatewayInstalledAirport> {
  return invokeCommand<GatewayInstalledAirport>('gateway_install_scenery', request)
}

export async function gatewayUninstallAirport(
  xplanePath: string,
  airportIcao: string,
): Promise<void> {
  await invokeVoidCommand('gateway_uninstall_airport', { xplanePath, airportIcao })
}
