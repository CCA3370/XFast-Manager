import { CommandError, invokeCommand, invokeVoidCommand } from '@/services/api'
import type {
  GatewayAirportDetail,
  GatewayAirportSearchResult,
  GatewayInstallWarning,
  GatewayInstalledAirport,
  GatewaySceneryDetail,
} from '@/types'

type GatewayInstallRequest = {
  xplanePath: string
  icao: string
  sceneryId: number
  autoSortScenery?: boolean
  ignoreExternalConflict?: boolean
}

function isGatewayInstallArgMismatch(error: unknown): boolean {
  if (!(error instanceof CommandError)) return false

  return (
    error.message.includes("invalid args `request`") ||
    error.message.includes("invalid args `ignoreExternalConflict`")
  )
}

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

export async function gatewayCheckInstallWarning(
  xplanePath: string,
  icao: string,
): Promise<GatewayInstallWarning | null> {
  return invokeCommand<GatewayInstallWarning | null>('gateway_check_install_warning', {
    xplanePath,
    icao,
  })
}

export async function gatewayInstallScenery(
  request: GatewayInstallRequest,
): Promise<GatewayInstalledAirport> {
  if (request.ignoreExternalConflict) {
    return invokeCommand<GatewayInstalledAirport>('gateway_force_install_scenery', {
      xplanePath: request.xplanePath,
      icao: request.icao,
      sceneryId: request.sceneryId,
      autoSortScenery: request.autoSortScenery,
    })
  }

  try {
    return await invokeCommand<GatewayInstalledAirport>('gateway_install_scenery', request)
  } catch (error) {
    if (
      !(
        error instanceof CommandError &&
        error.message.includes("invalid args `ignoreExternalConflict`")
      )
    ) {
      throw error
    }
  }

  try {
    return await invokeCommand<GatewayInstalledAirport>('gateway_install_scenery', { request })
  } catch (error) {
    if (!isGatewayInstallArgMismatch(error)) {
      throw error
    }
  }

  const { ignoreExternalConflict: _ignoreExternalConflict, ...legacyRequest } = request
  return invokeCommand<GatewayInstalledAirport>('gateway_install_scenery', legacyRequest)
}

export async function gatewayUninstallAirport(
  xplanePath: string,
  airportIcao: string,
): Promise<void> {
  await invokeVoidCommand('gateway_uninstall_airport', { xplanePath, airportIcao })
}
