// @vitest-environment happy-dom

import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import en from '@/i18n/en'
import XplaneDirectoryForm from '@/components/XplaneDirectoryForm.vue'
import { STORAGE_KEYS } from '@/services/storage'
import { useAppStore } from '@/stores/app'
import Onboarding from './Onboarding.vue'

const { values, get, set, remove, save, invoke, open } = vi.hoisted(() => ({
  values: new Map<string, unknown>(),
  get: vi.fn(),
  set: vi.fn(),
  remove: vi.fn(),
  save: vi.fn(),
  invoke: vi.fn(),
  open: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-store', () => ({
  Store: { load: async () => ({ get, set, delete: remove, save }) },
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }))
vi.mock('@/services/logger', () => ({ logError: vi.fn() }))

const originalPath = '/sim/original'
const newPath = '/sim/replacement'

function pendingResult<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((finish) => {
    resolve = finish
  })
  return { promise, resolve }
}

beforeEach(() => {
  vi.resetAllMocks()
  values.clear()
  values.set(STORAGE_KEYS.XPLANE_PATH, originalPath)
  get.mockImplementation(async (key: string) => values.get(key))
  set.mockImplementation(async (key: string, value: unknown) => void values.set(key, value))
  remove.mockImplementation(async (key: string) => values.delete(key))
  invoke.mockImplementation(async (command: string) => command === 'validate_xplane_path')
})

function setup() {
  const pinia = createPinia()
  const store = useAppStore(pinia)
  store.xplanePath = originalPath
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
  return { store, plugins: [pinia, i18n] }
}

describe('X-Plane directory', () => {
  it('uses safe first-run defaults and preserves explicit choices on upgrade', async () => {
    invoke.mockImplementation(async (command: string) =>
      command === 'get_platform' ? 'windows' : undefined,
    )
    const { store } = setup()
    await store.initStore()
    expect(store.atomicInstallEnabled).toBe(true)
    expect(store.autoSortScenery).toBe(false)
    expect(store.deleteSourceAfterInstall).toBe(false)
    expect(values.has(STORAGE_KEYS.ONBOARDING_COMPLETED)).toBe(false)

    values.set(STORAGE_KEYS.ONBOARDING_COMPLETED, 'true')
    values.set(STORAGE_KEYS.ATOMIC_INSTALL_ENABLED, false)
    values.set(STORAGE_KEYS.AUTO_SORT_SCENERY, true)
    const existing = setup().store
    await existing.initStore()
    expect(existing.atomicInstallEnabled).toBe(false)
    expect(existing.autoSortScenery).toBe(true)
  })

  it('ignores earlier validation results even when they arrive after a newer submission', async () => {
    const { store, plugins } = setup()
    const oldValidation = pendingResult<boolean>()
    invoke.mockReturnValueOnce(oldValidation.promise).mockResolvedValueOnce(true)
    const wrapper = mount(XplaneDirectoryForm, { global: { plugins } })
    const input = wrapper.get('input')
    await input.setValue('/sim/previous-draft')
    await wrapper.get('form').trigger('submit')
    await input.setValue(newPath)
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    oldValidation.resolve(true)
    await flushPromises()

    expect(store.xplanePath).toBe(newPath)
    expect(set).toHaveBeenCalledExactlyOnceWith(STORAGE_KEYS.XPLANE_PATH, newPath)
    expect(wrapper.get('[role="status"]').text()).toBe(en.settings.saved)
    wrapper.unmount()
  })

  it('keeps the original directory when validation fails to run', async () => {
    const { store, plugins } = setup()
    invoke.mockRejectedValueOnce(new Error('validation IPC failed'))
    const wrapper = mount(XplaneDirectoryForm, { global: { plugins } })
    await wrapper.get('input').setValue(newPath)
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(store.xplanePath).toBe(originalPath)
    expect(set).not.toHaveBeenCalled()
    expect(wrapper.get('[role="status"]').text()).toBe(en.settingsSetup.validationFailed)
    wrapper.unmount()
  })

  it('can fill with the folder picker and clear a saved directory explicitly', async () => {
    const { store, plugins } = setup()
    open.mockResolvedValueOnce(newPath)
    const wrapper = mount(XplaneDirectoryForm, { global: { plugins } })
    const button = (label: string) => wrapper.findAll('button').find((b) => b.text() === label)!
    await button(en.common.browse).trigger('click')
    await flushPromises()
    expect(wrapper.get('input').element.value).toBe(newPath)
    expect(store.xplanePath).toBe(originalPath)

    await button(en.settingsSetup.clearDirectory).trigger('click')
    await flushPromises()
    expect(store.xplanePath).toBe('')
    expect(values.get(STORAGE_KEYS.XPLANE_PATH)).toBe('')
    expect(invoke).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('commits the path and completion together before navigating and lets the user retry a failed save', async () => {
    const { store, plugins } = setup()
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<div>Home</div>' } },
        { path: '/onboarding', component: Onboarding },
      ],
    })
    router.beforeEach((to) =>
      to.path === '/' && values.get(STORAGE_KEYS.ONBOARDING_COMPLETED) !== 'true'
        ? '/onboarding'
        : undefined,
    )
    await router.push('/onboarding')
    const wrapper = mount(Onboarding, { global: { plugins: [...plugins, router] } })
    save.mockRejectedValueOnce(new Error('disk is read-only'))
    await wrapper.get('input').setValue(newPath)
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(store.xplanePath).toBe(originalPath)
    expect(values.get(STORAGE_KEYS.XPLANE_PATH)).toBe(originalPath)
    expect(values.has(STORAGE_KEYS.ONBOARDING_COMPLETED)).toBe(false)
    expect(router.currentRoute.value.path).toBe('/onboarding')
    expect(wrapper.get('[role="status"]').text()).toBe(en.settingsSetup.saveFailed)

    const pendingSave = pendingResult<void>()
    save.mockReturnValueOnce(pendingSave.promise)
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/onboarding')
    expect(store.xplanePath).toBe(originalPath)
    pendingSave.resolve()
    await flushPromises()

    expect([...values]).toEqual([
      [STORAGE_KEYS.XPLANE_PATH, newPath],
      [STORAGE_KEYS.ONBOARDING_COMPLETED, 'true'],
    ])
    expect(store.xplanePath).toBe(newPath)
    expect(router.currentRoute.value.path).toBe('/')
    wrapper.unmount()
  })
})
