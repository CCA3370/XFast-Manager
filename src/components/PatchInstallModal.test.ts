// @vitest-environment happy-dom

import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { PatchPlan, PatchInstallSummary, PatchMappingInput } from '@/types'

const api = vi.hoisted(() => ({
  detectPatchTargetAircraft: vi.fn(),
  inferPatchMappings: vi.fn(),
  summarizePatchInstall: vi.fn(),
  buildPatchInstallTasks: vi.fn(),
}))
vi.mock('@/services/patch-api', () => api)
const savedMappings = vi.hoisted(() => new Map<string, PatchMappingInput[]>())
vi.mock('@/services/storage', () => ({
  STORAGE_KEYS: { PATCH_MAPPING_PREFIX: 'patch.' },
  getItem: async (key: string) => savedMappings.get(key) ?? null,
  setItem: async (key: string, value: PatchMappingInput[]) => {
    savedMappings.set(key, value)
  },
  removeItem: async (key: string) => {
    savedMappings.delete(key)
  },
}))
vi.mock('@/stores/app', () => ({ useAppStore: () => ({ xplanePath: '/sim' }) }))
vi.mock('@/stores/toast', () => ({ useToastStore: () => ({ warning: vi.fn() }) }))
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
    archiveTree: [name, 'extras', 'extras/deep', 'extras/deep/a.obj'],
    aircraftDirectories: [
      `${name}-dest`,
      'objects/deep/parts/left',
      'objects/deep/parts/right',
      'objects/deep/parts/center',
    ],
    mappingKey: name,
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

function summary(overrides: Partial<PatchInstallSummary> = {}): PatchInstallSummary {
  return {
    totalFiles: 1,
    overwriteCount: 1,
    planId: 'preview',
    mappingPreviews: [{ fileCount: 1, overwriteCount: 1, destinationExists: true }],
    unmapped: [],
    ...overrides,
  }
}

let wrapper: VueWrapper
function installButton() {
  return wrapper.findAll('button').find((button) => button.text() === 'patch.install')!
}
function destinationInputs() {
  return wrapper.findAll<HTMLInputElement>('input[aria-label="patch.destinationPath"]')
}
function button(label: string) {
  return wrapper.findAll('button').find((candidate) => candidate.text() === label)!
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
  savedMappings.clear()
  api.detectPatchTargetAircraft.mockResolvedValue({
    candidates: ['A', 'B'].map((folderName) => ({ folderName, displayName: folderName })),
    recommendedFolder: 'A',
  })
  api.inferPatchMappings.mockResolvedValue(plan('A'))
  api.summarizePatchInstall.mockResolvedValue(summary())
  api.buildPatchInstallTasks.mockResolvedValue([])
})
afterEach(() => {
  wrapper?.unmount()
  vi.restoreAllMocks()
  vi.useRealTimers()
})

describe('patch installation confirmation', () => {
  it('blocks installation until the preview completes', async () => {
    const pending = deferred<PatchInstallSummary>()
    api.summarizePatchInstall.mockReturnValue(pending.promise)
    await open()
    await preview()
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(api.buildPatchInstallTasks).not.toHaveBeenCalled()
    pending.resolve(summary())
    await flushPromises()
    await installButton().trigger('click')
    expect(api.buildPatchInstallTasks).toHaveBeenCalledWith(
      expect.objectContaining({ aircraftFolder: 'A', backupOverwritten: true }),
    )
  })

  it('blocks installation and displays the preview error', async () => {
    api.summarizePatchInstall.mockRejectedValue(new Error('preview unavailable'))
    await open()
    await preview()
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('preview unavailable')
    expect(api.buildPatchInstallTasks).not.toHaveBeenCalled()
  })

  it('requires an explicit aircraft choice when there is no clear match', async () => {
    api.detectPatchTargetAircraft.mockResolvedValue({
      candidates: ['A', 'B'].map((folderName) => ({ folderName, displayName: folderName })),
      recommendedFolder: null,
    })
    await open()
    expect(wrapper.get('select').element.value).toBe('')
    expect(wrapper.text()).toContain('patch.noReliableMatch')
    expect(api.inferPatchMappings).not.toHaveBeenCalled()
    expect(installButton().attributes('disabled')).toBeDefined()
    await wrapper.get('select').setValue('A')
    await flushPromises()
    await preview()
    expect(installButton().attributes('disabled')).toBeUndefined()
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
    expect(destinationInputs()).toHaveLength(0)
    older.resolve(plan('stale-B'))
    await flushPromises()
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(destinationInputs()).toHaveLength(0)
    latest.resolve(plan('latest-A'))
    await flushPromises()
    await preview()
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
    await preview()
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
    expect(destinationInputs()).toHaveLength(0)
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
    oldSummary.resolve(summary({ totalFiles: 900, overwriteCount: 900 }))
    await flushPromises()
    expect(wrapper.find('input[type="checkbox"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('patch.inferring')
    newSummary.resolve(summary({ totalFiles: 2, overwriteCount: 0 }))
    await flushPromises()
    expect(wrapper.text()).not.toContain('patch.filesOverwritten')
    expect(wrapper.get<HTMLInputElement>('input[type=checkbox]').element.checked).toBe(true)
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
    await open()
    api.summarizePatchInstall.mockReturnValueOnce(oldSummary.promise)
    await destinationInputs()[0]!.setValue('earlier-destination')
    await preview()
    await destinationInputs()[0]!.setValue('new-destination')
    oldSummary.resolve(summary({ totalFiles: 900, overwriteCount: 900 }))
    await flushPromises()
    expect(wrapper.text()).toContain('patch.inferring')
    expect(installButton().attributes('disabled')).toBeDefined()
    await preview()
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

  it('suggests two deep destinations and replaces the row preview as the path changes', async () => {
    await open()
    await preview()
    expect(wrapper.get('select').text()).not.toContain('patch.matchingFiles')
    expect(wrapper.text()).toContain('patch.mappingExistingFolder')
    const destination = destinationInputs()[0]!
    await destination.setValue('/objects/deep/parts/')
    const suggestions = destination.element.parentElement!.querySelectorAll('ul button')
    expect([...suggestions].map((option) => option.textContent)).toEqual([
      '/objects/deep/parts/left',
      '/objects/deep/parts/right',
    ])
    await button('/objects/deep/parts/left').trigger('click')
    expect(destination.element.value).toBe('/objects/deep/parts/left')
    await preview()
    expect(api.summarizePatchInstall).toHaveBeenLastCalledWith(
      expect.objectContaining({
        mappings: [{ archiveSubpath: 'A', destSubpath: '/objects/deep/parts/left' }],
      }),
    )
    await destination.setValue('custom/deep/new')
    expect(wrapper.text()).toContain('custom/deep/new · patch.mappingChecking')
    expect(wrapper.text()).not.toContain('patch.mappingExistingFolder')
    api.summarizePatchInstall.mockResolvedValueOnce(
      summary({
        overwriteCount: 0,
        mappingPreviews: [{ fileCount: 1, overwriteCount: 0, destinationExists: false }],
      }),
    )
    await preview()
    expect(wrapper.text()).toContain('custom/deep/new · patch.mappingNewFolder')
  })

  it('remembers the confirmed mappings and clears them when automatic mappings are installed', async () => {
    await open()
    await preview()
    await button('+ patch.addMapping').trigger('click')
    await wrapper.findAll('button[aria-expanded]')[1]!.trigger('click')
    const search = wrapper.get('input[type="search"]')
    await search.setValue('/extras/deep/a.obj')
    await search.trigger('keydown.enter')
    await destinationInputs()[1]!.setValue('custom/deep/folder')
    await preview()
    expect(savedMappings.has('patch.A')).toBe(false)
    await wrapper.findAll('button[title="common.delete"]')[0]!.trigger('click')
    await preview()
    expect(savedMappings.has('patch.A')).toBe(false)
    await installButton().trigger('click')
    await flushPromises()
    expect(savedMappings.get('patch.A')).toEqual([
      { archiveSubpath: 'extras/deep/a.obj', destSubpath: 'custom/deep/folder' },
    ])
    wrapper.unmount()

    await open()
    await preview()
    expect(destinationInputs().map(({ element }) => element.value)).toEqual(['custom/deep/folder'])
    expect(wrapper.text()).toContain('extras/deep/a.obj')
    expect(wrapper.text()).toContain('patch.rememberedMappings')
    await button('patch.resetMappings').trigger('click')
    await preview()
    expect(savedMappings.has('patch.A')).toBe(true)
    expect(destinationInputs().map(({ element }) => element.value)).toEqual(['A-dest'])
    await installButton().trigger('click')
    await flushPromises()
    expect(savedMappings.has('patch.A')).toBe(false)
  })

  it('keeps the saved mapping when edits are cancelled or installation validation fails', async () => {
    const previous = [{ archiveSubpath: 'A', destSubpath: 'custom/good' }]
    savedMappings.set('patch.A', previous)
    await open()
    await preview()
    await destinationInputs()[0]!.setValue('custom/draft')
    await preview()
    await button('common.cancel').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
    expect(savedMappings.get('patch.A')).toEqual(previous)
    wrapper.unmount()

    await open()
    await preview()
    api.summarizePatchInstall.mockRejectedValueOnce(new Error('invalid destination'))
    await destinationInputs()[0]!.setValue('../outside')
    await preview()
    expect(wrapper.text()).toContain('patch.mappingUnavailable')
    expect(installButton().attributes('disabled')).toBeDefined()
    expect(savedMappings.get('patch.A')).toEqual(previous)

    await destinationInputs()[0]!.setValue('custom/another')
    await preview()
    api.buildPatchInstallTasks.mockRejectedValueOnce(new Error('patch changed'))
    await installButton().trigger('click')
    await flushPromises()
    expect(wrapper.emitted('install')).toBeUndefined()
    expect(savedMappings.get('patch.A')).toEqual(previous)
  })

  it('measures the initial preview once and keeps the summary card mounted while refreshing', async () => {
    const bounds = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect')
    bounds.mockReturnValue(new DOMRect(0, 0, 500, 340))
    const firstPreview = deferred<PatchInstallSummary>()
    api.summarizePatchInstall.mockReturnValueOnce(firstPreview.promise)
    await open()
    const dialog = wrapper.get<HTMLElement>('.modal-content')
    expect(dialog.classes()).toContain('invisible')
    firstPreview.resolve(summary())
    await flushPromises()
    expect(dialog.classes()).not.toContain('invisible')
    expect(dialog.element.style.height).toBe('340px')
    const card = wrapper.get('[aria-busy="false"]').element
    const checkbox = wrapper.get('input[type="checkbox"]').element

    bounds.mockReturnValue(new DOMRect(0, 0, 500, 640))
    await destinationInputs()[0]!.setValue('custom/folder')
    expect(wrapper.get('[aria-busy="true"]').element).toBe(card)
    expect(wrapper.get('input[type="checkbox"]').element).toBe(checkbox)
    await preview()
    expect(wrapper.get('[aria-busy="false"]').element).toBe(card)
    expect(dialog.element.style.height).toBe('340px')
  })

  it('cancels deferred preview work when the modal unmounts', async () => {
    await open()
    api.summarizePatchInstall.mockClear()
    await destinationInputs()[0]!.setValue('custom/folder')
    wrapper.unmount()
    await preview()
    expect(api.summarizePatchInstall).not.toHaveBeenCalled()
  })
})
