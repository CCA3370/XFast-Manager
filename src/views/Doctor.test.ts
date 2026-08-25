// @vitest-environment happy-dom

import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DoctorCheckResult, DoctorRun } from '@/types/doctor'

const dialog = vi.hoisted(() => ({
  save: vi.fn<() => Promise<string | null>>(async () => null),
}))

const history = vi.hoisted(() => ({
  loadDoctorRunsForPath: vi.fn<(path: string) => Promise<DoctorRun[]>>(async () => []),
  saveDoctorRun: vi.fn<(path: string, run: DoctorRun) => Promise<DoctorRun[]>>(
    async (_path, run) => [run],
  ),
}))

const logging = vi.hoisted(() => ({
  logError: vi.fn(),
  logger: {
    error: vi.fn(),
    info: vi.fn(),
  },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: dialog.save }))
vi.mock('@/services/doctorHistory', () => history)
vi.mock('@/services/logger', () => logging)
vi.mock('@/services/storage', () => ({
  getItem: vi.fn(async () => null),
  setItem: vi.fn(async () => {}),
  STORAGE_KEYS: {
    INCLUDE_PRE_RELEASE: 'includePreRelease',
    ADDON_UPDATE_ITEM_BETA_PREFERENCES: 'addonUpdateItemBetaPreferences',
    CSL_CUSTOM_PATHS: 'cslCustomPaths',
    CSL_INSTALL_LOCATION: 'cslInstallLocation',
    CSL_ACTIVE_SERVER_BASE_URL: 'cslActiveServerBaseUrl',
  },
}))

import Doctor from './Doctor.vue'
import { i18n } from '@/i18n'
import { useAppStore } from '@/stores/app'
import { useDoctorStore } from '@/stores/doctor'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise
  })
  return { promise, resolve }
}

function runFixture(state: DoctorRun['state'], checks: DoctorCheckResult[] = []): DoctorRun {
  const counts = {
    pass: checks.filter((check) => check.outcome === 'pass').length,
    info: checks.filter((check) => check.outcome === 'info').length,
    warning: checks.filter((check) => check.outcome === 'warning').length,
    critical: checks.filter((check) => check.outcome === 'critical').length,
    unavailable: checks.filter((check) => check.outcome === 'unavailable').length,
    notApplicable: checks.filter((check) => check.outcome === 'notApplicable').length,
    cancelled: checks.filter((check) => check.outcome === 'cancelled').length,
  }
  const eligible = checks.length - counts.notApplicable
  const covered = counts.pass + counts.info + counts.warning + counts.critical
  return {
    schemaVersion: 1,
    id: `run-${state}`,
    installationId: 'installation-1',
    mode: 'quick',
    state,
    startedAt: Date.now() - 2_000,
    completedAt: state === 'running' ? null : Date.now(),
    durationMs: state === 'running' ? 0 : 2_000,
    appVersion: '1.2.5',
    checks,
    summary: {
      severity: counts.critical
        ? 'critical'
        : counts.warning
          ? 'warning'
          : counts.info
            ? 'info'
            : 'ok',
      completeness:
        state === 'cancelled' || counts.cancelled
          ? 'cancelled'
          : counts.unavailable
            ? 'partial'
            : 'complete',
      total: checks.length,
      eligible,
      covered,
      coveragePercent: eligible === 0 ? 100 : Math.round((covered / eligible) * 100),
      ...counts,
    },
    system: null,
  }
}

interface MountedDoctor {
  wrapper: VueWrapper
  pinia: Pinia
  router: Router
  appStore: ReturnType<typeof useAppStore>
  doctorStore: ReturnType<typeof useDoctorStore>
}

async function mountDoctor(xplanePath: string, waitForInitialLoad = true): Promise<MountedDoctor> {
  const pinia = createPinia()
  setActivePinia(pinia)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/doctor', component: Doctor },
      { path: '/settings', component: { template: '<div>Settings</div>' } },
    ],
  })
  await router.push('/doctor')
  await router.isReady()

  const appStore = useAppStore()
  appStore.xplanePath = xplanePath
  const doctorStore = useDoctorStore()
  doctorStore.hasAutoRunThisSession = true
  const wrapper = mount(Doctor, {
    global: {
      plugins: [pinia, router, i18n],
    },
  })
  if (waitForInitialLoad) await flushPromises()
  else await nextTick()
  return { wrapper, pinia, router, appStore, doctorStore }
}

describe('Health page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    history.loadDoctorRunsForPath.mockResolvedValue([])
    dialog.save.mockResolvedValue(null)
    i18n.global.locale.value = 'en'
  })

  it('uses the same compact page shell as the other management views', async () => {
    const { wrapper } = await mountDoctor('')

    expect(wrapper.get('[data-testid="health-page"]').classes()).toEqual(
      expect.arrayContaining(['h-full', 'px-6', 'pt-3', 'pb-6', 'overflow-hidden']),
    )
    expect(wrapper.get('h1').classes()).toContain('text-xl')
    expect(wrapper.get('main').classes()).toContain('overflow-y-auto')
    expect(wrapper.text()).toContain('Connect an X-Plane installation')
    expect(wrapper.get('a[href="/settings"]').text()).toBe('Configure X-Plane path')
  })

  it('shows scan progress instead of an all-clear result while work is running', async () => {
    const { wrapper, doctorStore } = await mountDoctor('/xplane')
    doctorStore.currentRun = runFixture('running')
    doctorStore.currentRunIsLive = true
    doctorStore.phase = 'local'
    doctorStore.checkRuntime = [
      { id: 'environment', section: 'installation', state: 'completed' },
      { id: 'runtime', section: 'installation', state: 'running' },
    ]
    await nextTick()

    const summary = wrapper.get('[data-testid="health-summary"]')
    expect(summary.text()).toContain('Inspecting this computer')
    expect(summary.text()).toContain('1 of 2 checks completed')
    expect(summary.text()).toContain('50%')
    expect(summary.text()).not.toContain('All checked systems look healthy')
    expect(wrapper.find('[data-testid="cancel-health-scan"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="quick-health-scan"]').exists()).toBe(false)
    expect(wrapper.get('button[disabled]').attributes('disabled')).toBeDefined()
  })

  it('shows a stable loading state while installation history is pending', async () => {
    const pendingHistory = deferred<DoctorRun[]>()
    history.loadDoctorRunsForPath.mockReturnValueOnce(pendingHistory.promise)

    const { wrapper } = await mountDoctor('/xplane', false)
    expect(wrapper.get('[data-testid="health-history-loading"]').text()).toContain('Loading...')

    pendingHistory.resolve([])
    await flushPromises()
    expect(wrapper.find('[data-testid="health-history-loading"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Ready to diagnose')
  })

  it('keeps quick and full scans available from the compact header', async () => {
    const { wrapper, doctorStore } = await mountDoctor('/xplane')
    const runDiagnostics = vi.spyOn(doctorStore, 'runDiagnostics').mockResolvedValue()

    await wrapper.get('[data-testid="quick-health-scan"]').trigger('click')
    await wrapper.get('[data-testid="full-health-scan"]').trigger('click')

    expect(runDiagnostics).toHaveBeenNthCalledWith(1, 'quick')
    expect(runDiagnostics).toHaveBeenNthCalledWith(2, 'full')
  })

  it('disables competing scans while a repair is in progress', async () => {
    const { wrapper, doctorStore } = await mountDoctor('/xplane')
    doctorStore.fixingId = 'xfast.scenery_index'
    await nextTick()

    expect(wrapper.get('[data-testid="quick-health-scan"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-testid="full-health-scan"]').attributes('disabled')).toBeDefined()
  })

  it('renders internal failures as localized user-facing errors', async () => {
    const { wrapper, doctorStore } = await mountDoctor('/xplane')
    doctorStore.error = 'C:\\Users\\alex\\private failure detail'
    await nextTick()

    const alert = wrapper.get('[role="alert"]')
    expect(alert.text()).toContain('The repair could not be completed.')
    expect(alert.text()).not.toContain('alex')
  })

  it('handles native save-dialog failures without leaking details or rejecting the UI action', async () => {
    const { wrapper, doctorStore, pinia } = await mountDoctor('/xplane')
    doctorStore.currentRun = runFixture('completed', [
      {
        id: 'installation.structure',
        section: 'installation',
        outcome: 'pass',
        durationMs: 59_600,
      },
    ])
    doctorStore.currentRunIsLive = true
    dialog.save.mockRejectedValueOnce(new Error('native dialog secret'))
    await nextTick()

    const jsonButton = wrapper.findAll('button').find((button) => button.text() === 'JSON data')
    expect(jsonButton).toBeDefined()
    await jsonButton?.trigger('click')
    await flushPromises()

    const toastStore = useToastStore(pinia)
    expect(toastStore.toasts.at(-1)).toMatchObject({
      type: 'error',
      message: 'Could not export the health report.',
    })
    expect(logging.logError).toHaveBeenCalledWith(
      expect.stringContaining('native dialog secret'),
      'doctor',
    )
    expect(wrapper.text()).toContain('1m 0s')
  })

  it('requires a fresh privacy opt-in when the displayed report changes', async () => {
    const { wrapper, doctorStore, pinia } = await mountDoctor('/xplane')
    doctorStore.currentRun = { ...runFixture('completed'), id: 'run-1' }
    doctorStore.currentRunIsLive = true
    await nextTick()

    const includePathsButton = wrapper
      .findAll('button')
      .find((button) => button.text() === 'Include original paths')
    await includePathsButton?.trigger('click')
    useModalStore(pinia).confirmAction()
    await nextTick()
    expect(wrapper.text()).toContain('Original local paths will be included.')

    doctorStore.currentRun = { ...runFixture('completed'), id: 'run-2' }
    await nextTick()
    expect(wrapper.text()).toContain('Private paths are redacted by default.')
  })
})
