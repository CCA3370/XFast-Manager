import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type {
  GatewayAirportDetail,
  GatewayAirportSearchResult,
  GatewayInstallWarning,
  GatewayInstalledAirport,
  GatewayReleaseContext,
  GatewaySceneryDetail,
} from '@/types'

const gatewayApi = vi.hoisted(() => ({
  gatewayCheckUpdates: vi.fn<(xplanePath: string) => Promise<GatewayInstalledAirport[]>>(),
  gatewayCheckInstallWarning:
    vi.fn<(xplanePath: string, icao: string) => Promise<GatewayInstallWarning | null>>(),
  gatewayGetAirport: vi.fn<(icao: string) => Promise<GatewayAirportDetail>>(),
  gatewayGetScenery: vi.fn<(sceneryId: number) => Promise<GatewaySceneryDetail>>(),
  gatewayInstallScenery:
    vi.fn<(request: Record<string, unknown>) => Promise<GatewayInstalledAirport>>(),
  gatewayListInstalled: vi.fn<(xplanePath: string) => Promise<GatewayInstalledAirport[]>>(),
  gatewayResolveReleaseContext: vi.fn<(xplanePath: string) => Promise<GatewayReleaseContext>>(),
  gatewaySearchAirports: vi.fn<(query: string) => Promise<GatewayAirportSearchResult[]>>(),
  gatewayUninstallAirport: vi.fn<(xplanePath: string, airportIcao: string) => Promise<void>>(),
}))

vi.mock('@/services/gateway-api', () => gatewayApi)

import { useGatewayStore } from './gateway'

function createDeferred<T>() {
  let resolvePromise: (value: T) => void = () => {}
  let rejectPromise: (reason?: unknown) => void = () => {}
  const promise = new Promise<T>((resolve, reject) => {
    resolvePromise = resolve
    rejectPromise = reject
  })

  return { promise, resolve: resolvePromise, reject: rejectPromise }
}

function createAirportDetail(icao = 'EDDE', sceneryId = 110996): GatewayAirportDetail {
  return {
    icao,
    airportName: `${icao} Airport`,
    sceneryCount: 1,
    recommendedSceneryId: sceneryId,
    recommendedArtist: 'Gateway Artist',
    recommendedAcceptedAt: '2026-01-01',
    sceneries: [
      {
        sceneryId,
        artist: 'Gateway Artist',
        status: 'Approved',
        approvedDate: '2026-01-01',
        comment: null,
        recommended: true,
      },
    ],
  }
}

function createSceneryDetail(icao = 'EDDE', sceneryId = 110996): GatewaySceneryDetail {
  return {
    sceneryId,
    icao,
    airportName: `${icao} Airport`,
    status: 'Approved',
    artist: 'Gateway Artist',
    approvedDate: '2026-01-01',
    comment: null,
    features: [],
  }
}

function createInstalledAirport(airportIcao = 'EDDE', sceneryId = 110996): GatewayInstalledAirport {
  return {
    id: sceneryId,
    airportIcao,
    airportName: `${airportIcao} Airport`,
    sceneryId,
    folderName: `Gateway ${airportIcao}`,
    artist: 'Gateway Artist',
    approvedDate: '2026-01-01',
    installedAt: 1,
    updateAvailable: false,
    latestSceneryId: sceneryId,
    latestArtist: 'Gateway Artist',
    latestApprovedDate: '2026-01-01',
  }
}

describe('Gateway store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    gatewayApi.gatewayCheckInstallWarning.mockResolvedValue(null)
    gatewayApi.gatewayGetAirport.mockImplementation(async (icao) => createAirportDetail(icao))
    gatewayApi.gatewayGetScenery.mockImplementation(async (sceneryId) =>
      createSceneryDetail('EDDE', sceneryId),
    )
    gatewayApi.gatewayListInstalled.mockResolvedValue([])
    gatewayApi.gatewayResolveReleaseContext.mockResolvedValue({
      detectedVersionRaw: null,
      matchedReleaseVersion: null,
      matchedReleaseDate: null,
      comparisonAvailable: false,
    })
  })

  it('keeps the newest installed-list response when refreshes finish out of order', async () => {
    const first = createDeferred<GatewayInstalledAirport[]>()
    const second = createDeferred<GatewayInstalledAirport[]>()
    gatewayApi.gatewayListInstalled
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise)
    const store = useGatewayStore()

    const firstLoad = store.loadInstalled('/xplane')
    const secondLoad = store.loadInstalled('/xplane')
    second.resolve([createInstalledAirport('KJFK', 200)])
    await secondLoad
    first.resolve([createInstalledAirport('EDDE', 100)])
    await firstLoad

    expect(store.installed).toHaveLength(1)
    expect(store.installed[0]?.airportIcao).toBe('KJFK')
    expect(store.isLoadingInstalled).toBe(false)
  })

  it('matches installed airports without depending on ICAO letter case', async () => {
    gatewayApi.gatewayListInstalled.mockResolvedValue([createInstalledAirport('edde')])
    const store = useGatewayStore()

    await store.loadInstalled('/xplane')
    await store.openAirport('EDDE')

    expect(store.selectedInstalledRecord?.airportIcao).toBe('edde')
    expect(store.selectedInstallSelection).toEqual({
      airportIcao: 'EDDE',
      sceneryId: 110996,
    })
  })

  it('installs the captured selection even after the visible selection is reset', async () => {
    const installedRecord = createInstalledAirport('EDDE', 110996)
    gatewayApi.gatewayInstallScenery.mockResolvedValue(installedRecord)
    gatewayApi.gatewayListInstalled.mockResolvedValue([installedRecord])
    const store = useGatewayStore()
    await store.openAirport('EDDE')
    const capturedSelection = store.selectedInstallSelection
    expect(capturedSelection).not.toBeNull()
    store.resetAirportSelection()

    const result = await store.installSelection('/xplane', capturedSelection!)

    expect(result).toEqual(installedRecord)
    expect(gatewayApi.gatewayInstallScenery).toHaveBeenCalledWith(
      expect.objectContaining({
        xplanePath: '/xplane',
        icao: 'EDDE',
        sceneryId: 110996,
      }),
    )
    expect(store.installed).toEqual([installedRecord])
  })

  it('keeps a completed install successful when the follow-up list refresh fails', async () => {
    const installedRecord = createInstalledAirport('EDDE', 110996)
    gatewayApi.gatewayInstallScenery.mockResolvedValue(installedRecord)
    gatewayApi.gatewayListInstalled.mockRejectedValue(new Error('Local refresh failed'))
    const store = useGatewayStore()

    const result = await store.installSelection('/xplane', {
      airportIcao: 'EDDE',
      sceneryId: 110996,
    })

    expect(result).toEqual(installedRecord)
    expect(store.installed).toEqual([installedRecord])
    expect(store.installingIcao).toBeNull()
  })

  it('rejects a duplicate start while a Gateway install is already running', async () => {
    const install = createDeferred<GatewayInstalledAirport>()
    const installedRecord = createInstalledAirport()
    gatewayApi.gatewayInstallScenery.mockReturnValueOnce(install.promise)
    gatewayApi.gatewayListInstalled.mockResolvedValue([installedRecord])
    const store = useGatewayStore()
    const selection = { airportIcao: 'EDDE', sceneryId: 110996 }

    const firstInstall = store.installSelection('/xplane', selection)
    await expect(store.installSelection('/xplane', selection)).rejects.toMatchObject({
      code: 'conflict_exists',
    })
    install.resolve(installedRecord)
    await firstInstall

    expect(gatewayApi.gatewayInstallScenery).toHaveBeenCalledTimes(1)
    expect(store.installingIcao).toBeNull()
  })
})
