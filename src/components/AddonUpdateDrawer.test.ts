// @vitest-environment happy-dom

import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { AddonUpdatePlan } from '@/types'
import AddonUpdateDrawer from './AddonUpdateDrawer.vue'

const mocks = vi.hoisted(() => ({
  buildPlan: vi.fn(),
  setBeta: vi.fn(),
  execute: vi.fn(),
}))

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))
vi.mock('@/stores/app', () => ({
  useAppStore: () => ({
    xplaneGeneration: 1,
    captureXplaneContext: () => ({ path: '/sim', isCurrent: () => true }),
  }),
}))
vi.mock('@/stores/modal', () => ({ useModalStore: () => ({}) }))
vi.mock('@/stores/toast', () => ({ useToastStore: () => ({ error: vi.fn() }) }))
vi.mock('@/stores/management', () => ({
  useManagementStore: () => ({
    buildAddonUpdatePlan: mocks.buildPlan,
    setAddonUpdateItemBetaPreference: mocks.setBeta,
    executeAddonUpdate: mocks.execute,
    addonUpdateOptions: { includeLiveries: true },
  }),
}))

const task = { itemType: 'plugin', folderName: 'Demo', displayName: 'Demo plugin' } as const
const stablePlan: AddonUpdatePlan = {
  provider: 'manifest',
  itemType: task.itemType,
  folderName: task.folderName,
  remoteModule: 'https://example.test/stable',
  remoteLocked: false,
  hasUpdate: true,
  estimatedDownloadBytes: 10,
  addFiles: ['file'],
  replaceFiles: [],
  deleteFiles: [],
  skipFiles: [],
  warnings: [],
  hasBetaConfig: true,
  legacyBetaEnabled: false,
}

beforeEach(() => vi.resetAllMocks())

describe('SkunkCrafts legacy beta control', () => {
  it('allows switching back to stable after loading the beta plan fails', async () => {
    mocks.buildPlan
      .mockResolvedValueOnce(stablePlan)
      .mockRejectedValueOnce(new Error('Beta server unavailable'))
      .mockResolvedValueOnce(stablePlan)
    const drawer = mount(AddonUpdateDrawer, {
      props: { show: true, tasks: [task] },
      global: { stubs: { Teleport: true } },
    })

    try {
      await flushPromises()
      await drawer.get('input[type="checkbox"]').setValue(true)
      await flushPromises()

      expect(drawer.text()).toContain('Beta server unavailable')
      expect(drawer.get('input').element.checked).toBe(true)
      expect(drawer.text()).not.toContain('management.freshInstallToggle')
      expect(mocks.execute).not.toHaveBeenCalled()

      await drawer.get('input[type="checkbox"]').setValue(false)
      await flushPromises()

      expect(mocks.setBeta.mock.calls).toEqual([
        ['plugin', 'Demo', true],
        ['plugin', 'Demo', false],
      ])
      expect(drawer.text()).not.toContain('Beta server unavailable')
    } finally {
      drawer.unmount()
    }
  })
})
