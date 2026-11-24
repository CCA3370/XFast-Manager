// @vitest-environment happy-dom

import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { createI18n } from 'vue-i18n'
import ErrorModal from '@/components/ErrorModal.vue'
import ar from '@/i18n/ar'
import de from '@/i18n/de'
import en from '@/i18n/en'
import es from '@/i18n/es'
import fr from '@/i18n/fr'
import hi from '@/i18n/hi'
import ja from '@/i18n/ja'
import ko from '@/i18n/ko'
import pt from '@/i18n/pt'
import ru from '@/i18n/ru'
import zh from '@/i18n/zh'
import { useModalStore } from '@/stores/modal'
import { getErrorMessage, getErrorReportPolicy } from '@/types'
import { getXPlaneLaunchErrorPresentation } from './xplaneLaunchError'

vi.mock('@/services/logger', () => ({ logger: { error: vi.fn() }, logError: vi.fn() }))
vi.mock('@/services/bugReport', () => ({ submitBugReport: vi.fn() }))

const launchTarget = String.raw`D:\Simulateurs\X-Plane 12\X-Plane.exe`
const systemMessage = 'Une stratégie de contrôle d’application a bloqué ce fichier. (os error 4551)'
const policyError = {
  code: 'application_control_blocked',
  origin: 'environment',
  operation: 'launch_xplane',
  message: systemMessage,
  details: launchTarget,
  reportable: false,
} as const

const messages = { en, zh, fr, de, es, ja, pt, hi, ar, ru, ko }
const i18n = createI18n({ legacy: false, locale: 'en', messages })
const t = (key: string) => i18n.global.t(key)

describe('X-Plane launch error presentation', () => {
  it.each([
    ['a JSON string', JSON.stringify(policyError)],
    ['an object', policyError],
    ['an Error wrapping JSON', new Error(JSON.stringify(policyError))],
    ['a nested Tauri error', { error: JSON.stringify(policyError) }],
  ])('shows policy diagnostics for %s independently of the OS language', (_, error) => {
    const presentation = getXPlaneLaunchErrorPresentation(error, t)
    const message = getErrorMessage(presentation.error)

    expect(presentation.title).toContain('4551')
    expect(message).toContain('CodeIntegrity')
    expect(message).toContain('Operational')
    expect(message).toContain('3077')
    expect(message).toContain('3089')
    expect(message).toContain('Correlation ActivityID')
    expect(message).toContain('File Name')
    expect(message).toContain('Process Name')
    expect(message).toContain('PolicyName')
    expect(message).toContain('PolicyId')
    expect(message).toContain('Status')
    expect(message).toContain(launchTarget)
    expect(message).toContain(systemMessage)
    expect(message).not.toContain(en.home.elevationRequired.solutions)
    expect(getErrorReportPolicy(presentation.error)).toMatchObject({
      code: 'application_control_blocked',
      origin: 'environment',
      operation: 'launch_xplane',
      reportable: false,
    })
  })

  it.each(Object.entries(messages))(
    'provides diagnostics in %s without fallback',
    (locale, text) => {
      const localizedI18n = createI18n({
        legacy: false,
        locale,
        fallbackLocale: false,
        messages: { [locale]: text },
      })
      const presentation = getXPlaneLaunchErrorPresentation(policyError, (key) =>
        localizedI18n.global.t(key),
      )
      const message = getErrorMessage(presentation.error)

      expect(presentation.title).toContain('4551')
      expect(message).toContain(text.home.applicationControlBlocked.description)
      expect(message).toContain(text.home.applicationControlBlocked.resolution)
      expect(message).toContain(`${text.home.applicationControlBlocked.path} ${launchTarget}`)
      expect(message).toContain(
        `${text.home.applicationControlBlocked.systemError} ${systemMessage}`,
      )
      expect(message).toContain('CodeIntegrity')
      expect(message).toContain('Operational')
      expect(message).toContain('3077')
      expect(message).toContain('3089')
      expect(message).toContain('https://learn.microsoft.com/en-us/')
      expect(message).not.toContain('home.applicationControlBlocked.')
      if (locale !== 'en') {
        expect(presentation.title).not.toBe(en.home.applicationControlBlocked.title)
      }
    },
  )

  it('does not invent a blocked file when the launch target is unavailable', () => {
    const presentation = getXPlaneLaunchErrorPresentation({ ...policyError, details: undefined }, t)
    const message = getErrorMessage(presentation.error)
    expect(message).toContain('3077')
    expect(message).not.toContain(en.home.applicationControlBlocked.path)
    expect(message).not.toContain('undefined')
  })

  it('does not treat an unrelated structured error as a policy block', () => {
    const presentation = getXPlaneLaunchErrorPresentation({ ...policyError, code: 'internal' }, t)
    expect(presentation.error).toBe(`${en.home.launchFailed}: ${systemMessage}`)
    expect(presentation.title).toBe('')
  })

  it('preserves the existing 740 elevation guidance and full path', () => {
    const presentation = getXPlaneLaunchErrorPresentation(`ELEVATION_REQUIRED:${launchTarget}`, t)
    expect(getErrorMessage(presentation.error)).toContain(en.home.elevationRequired.title)
    expect(getErrorMessage(presentation.error)).toContain(en.home.elevationRequired.solution1)
    expect(getErrorMessage(presentation.error)).toContain(launchTarget)
    expect(presentation.hideReport).toBe(true)
    expect(getErrorMessage(presentation.error)).not.toContain('3077')
  })

  it.each([
    'Selected X-Plane executable is not a valid Windows executable.',
    'Failed to launch X-Plane: unknown process error',
    systemMessage,
    'ELEVATION_REQUIRED appears here, but is not an error prefix',
  ])('preserves generic handling without guessing from message text: %s', (error) => {
    const presentation = getXPlaneLaunchErrorPresentation(error, t)
    expect(presentation.error).toBe(`${en.home.launchFailed}: ${error}`)
    expect(presentation.title).toBe('')
    expect(getErrorMessage(presentation.error)).not.toContain('3077')
  })

  it('preserves the localized timeout message', () => {
    expect(getXPlaneLaunchErrorPresentation(new Error('timeout'), t).error).toBe(
      en.home.launchTimeout,
    )
  })

  it('renders the dedicated modal without a bug-report or retry button', () => {
    const pinia = createPinia()
    const modal = useModalStore(pinia)
    const presentation = getXPlaneLaunchErrorPresentation(JSON.stringify(policyError), t)
    modal.showError(presentation.error, presentation.title, { hideReport: presentation.hideReport })
    const wrapper = mount(ErrorModal, {
      global: { plugins: [pinia, i18n], stubs: { teleport: true } },
    })

    expect(wrapper.get('[role="dialog"]').text()).toContain('3077')
    expect(modal.errorModal.reportPolicy).toMatchObject({
      code: 'application_control_blocked',
      reportable: false,
    })
    expect(wrapper.find(`button[title="${en.modal.submitBugReport}"]`).exists()).toBe(false)
    expect(
      wrapper
        .findAll('button')
        .map((button) => button.text())
        .filter(Boolean),
    ).toEqual([en.copy.copy, en.common.close])
    wrapper.unmount()
  })
})
