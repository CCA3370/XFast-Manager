// @vitest-environment happy-dom

import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { PatchPlan, PatchInstallSummary } from '@/types'

const api = vi.hoisted(() => ({
  detectPatchTargetAircraft: vi.fn(),
  inferPatchMappings: vi.fn(),
  summarizePatchInstall: vi.fn(),
  buildPatchInstallTasks: vi.fn(),
}))
vi.mock('@/services/patch-api', () => api)
vi.mock('@/stores/app', () => ({ useAppStore: () => ({ xplanePath: '/sim' }) }))
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }))

import PatchInstallModal from './PatchInstallModal.vue'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

function plan(name: string): PatchPlan {
  return {
    archiveTree: [name],
    aircraftSubdirs: [`${name}-dest`],
    unmapped: [],
    suggestedMappings: [
      {
        archiveSubpath: name,
        destSubpath: `${name}-dest`,
        fileCount: 1,
        confidence: 'high',
        reason: 'matchedExisting:1',
      },
    ],
  }
}

let wrapper: VueWrapper
function installButton() {
  return wrapper.findAll('button').find((button) => button.text() === 'patch.install')!
}
async function open() {
  wrapper = mount(PatchInstallModal, {
    props: { archivePath: '/patch.zip' },
    global: { stubs: { teleport: true }, mocks: { $t: (key: string) => key } },
  })
  await flushPromises()
}
async function preview() {
  await vi.advanceTimersByTimeAsync(350)
  await flushPromises()
}

beforeEach(() => {
  vi.useFakeTimers()
  vi.resetAllMocks()
  api.detectPatchTargetAircraft.mockResolvedValue({
    candidates: ['A', 'B'].map((folderName) => ({ folderName, displayName: folderName })),
    recommendedFolder: 'A',
  })
  api.inferPatchMappings.mockResolvedValue(plan('A'))
  api.summarizePatchInstall.mockResolvedValue({ totalFiles: 1, overwriteCount: 1 })
  api.buildPatchInstallTasks.mockResolvedValue([])
})
afterEach(() => {
  wrapper?.unmount()
  vi.useRealTimers()
})

describe('patch installation confirmation', () => {
  it('keeps backup enabled when installed before the preview completes', async () => {
    api.summarizePatchInstall.mockReturnValue(deferred<PatchInstallSummary>().promise)
    await open()
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        aircraftFolder: 'A',
        backupOverwritten: true,
      }),
    )
  })

  it('keeps backup enabled when preview fails', async () => {
    api.summarizePatchInstall.mockRejectedValue(new Error('preview unavailable'))
    await open()
    await preview()
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        backupOverwritten: true,
      }),
    )
  })

  it('honors an explicit choice to disable backups', async () => {
    await open()
    await preview()
    await wrapper.get('input[type="checkbox"]').setValue(false)
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({ backupOverwritten: false }),
    )
  })

  it('clears old mappings and waits for the latest inference, ignoring stale success', async () => {
    await open()
    const older = deferred<PatchPlan>()
    const latest = deferred<PatchPlan>()
    api.inferPatchMappings.mockReturnValueOnce(older.promise).mockReturnValueOnce(latest.promise)
    await wrapper.get('select').setValue('B')
    await wrapper.get('select').setValue('A')
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(wrapper.findAll('input[list="patch-archive-dirs"]')).toHaveLength(0)
    older.resolve(plan('stale-B'))
    await flushPromises()
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(wrapper.findAll('input[list="patch-archive-dirs"]')).toHaveLength(0)
    latest.resolve(plan('latest-A'))
    await flushPromises()
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        aircraftFolder: 'A',
        mappings: [{ archiveSubpath: 'latest-A', destSubpath: 'latest-A-dest' }],
      }),
    )
  })

  it('ignores stale inference errors after another aircraft is ready', async () => {
    await open()
    const older = deferred<PatchPlan>()
    api.inferPatchMappings
      .mockReturnValueOnce(older.promise)
      .mockResolvedValueOnce(plan('latest-A'))
    await wrapper.get('select').setValue('B')
    await wrapper.get('select').setValue('A')
    await flushPromises()
    older.reject(new Error('old B error'))
    await flushPromises()
    expect(wrapper.text()).not.toContain('old B error')
    expect(installButton().attributes('disabled')).toBeUndefined()
  })

  it('cannot install a previous aircraft mapping after the latest inference fails', async () => {
    await open()
    api.inferPatchMappings.mockRejectedValueOnce(new Error('B unavailable'))
    await wrapper.get('select').setValue('B')
    await flushPromises()
    expect(wrapper.text()).toContain('B unavailable')
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(wrapper.findAll('input[list="patch-archive-dirs"]')).toHaveLength(0)
    expect(api.buildPatchInstallTasks).not.toHaveBeenCalled()
  })

  it('does not reuse stale preview counts or clear the latest loading indicator', async () => {
    const oldSummary = deferred<PatchInstallSummary>()
    const newSummary = deferred<PatchInstallSummary>()
    api.summarizePatchInstall
      .mockReturnValueOnce(oldSummary.promise)
      .mockReturnValueOnce(newSummary.promise)
    await open()
    await preview()
    api.inferPatchMappings.mockResolvedValueOnce(plan('B'))
    await wrapper.get('select').setValue('B')
    await flushPromises()
    await preview()
    oldSummary.resolve({ totalFiles: 900, overwriteCount: 900 })
    await flushPromises()
    expect(wrapper.find('input[type="checkbox"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('patch.inferring')
    newSummary.resolve({ totalFiles: 2, overwriteCount: 0 })
    await flushPromises()
    expect(wrapper.text()).not.toContain('patch.filesOverwritten')
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        aircraftFolder: 'B',
        backupOverwritten: true,
      }),
    )
  })

  it('invalidates in-flight preview counts as soon as mappings change', async () => {
    const oldSummary = deferred<PatchInstallSummary>()
    api.summarizePatchInstall.mockReturnValueOnce(oldSummary.promise)
    await open()
    await preview()
    await wrapper.get('input[list="patch-dest-dirs"]').setValue('new-destination')
    oldSummary.resolve({ totalFiles: 900, overwriteCount: 900 })
    await flushPromises()
    expect(wrapper.text()).toContain('patch.inferring')
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({
        backupOverwritten: true,
        mappings: [{ archiveSubpath: 'A', destSubpath: 'new-destination' }],
      }),
    )
  })

  it('does not schedule preview work when an inference finishes after unmount', async () => {
    const inference = deferred<PatchPlan>()
    api.inferPatchMappings.mockReturnValueOnce(inference.promise)
    await open()
    wrapper.unmount()
    inference.resolve(plan('late'))
    await flushPromises()
    await preview()
    expect(api.summarizePatchInstall).not.toHaveBeenCalled()
  })

  it('cancels deferred preview work when the modal unmounts', async () => {
    await open()
    wrapper.unmount()
    await preview()
    expect(api.summarizePatchInstall).not.toHaveBeenCalled()
  })
})
