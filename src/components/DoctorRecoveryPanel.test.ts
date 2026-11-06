// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import { useModalStore } from '@/stores/modal'
import DoctorRecoveryPanel from './DoctorRecoveryPanel.vue'

const doctor = vi.hoisted(() => ({
  canRepairCurrentRun: true,
  isBusy: false,
  backupError: null,
  error: null,
  backups: [
    {
      id: 'backup-1',
      action: 'reset_preferences',
      createdAt: 1,
      paths: ['Output/preferences'],
      restored: false,
    },
  ],
  displayedRun: { checks: [{ id: 'system.injectors', outcome: 'warning' }] },
  inspectionReport: { shaderCachePresent: true, preferencesPresent: true },
  runMaintenance: vi.fn(async () => true),
}))
vi.mock('@/stores/doctor', () => ({ useDoctorStore: () => doctor }))

describe('Health recovery controls', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    doctor.canRepairCurrentRun = true
    i18n.global.locale.value = 'zh'
  })

  it('explains the effect and waits for explicit confirmation before resetting preferences', async () => {
    const wrapper = mount(DoctorRecoveryPanel, { global: { plugins: [i18n] } })
    await wrapper.get('[data-testid="health-repair-reset_preferences"]').trigger('click')
    const modal = useModalStore()
    expect(modal.confirmModal.options?.message).toContain('摇杆分配')
    expect(doctor.runMaintenance).not.toHaveBeenCalled()
    modal.confirmAction()
    expect(doctor.runMaintenance).toHaveBeenCalledWith('reset_preferences', undefined)
  })

  it('restores the backup chosen by the user only after confirmation', async () => {
    const wrapper = mount(DoctorRecoveryPanel, { global: { plugins: [i18n] } })
    await wrapper.get('[data-testid="health-restore-backup-1"]').trigger('click')
    expect(useModalStore().confirmModal.options?.message).toContain('另行备份当前文件')
    expect(doctor.runMaintenance).not.toHaveBeenCalled()
    useModalStore().confirmAction()
    expect(doctor.runMaintenance).toHaveBeenCalledWith('restore_health_backup', 'backup-1')
  })

  it('disables repairs and restores for running, historical or unavailable results', () => {
    doctor.canRepairCurrentRun = false
    const wrapper = mount(DoctorRecoveryPanel, { global: { plugins: [i18n] } })
    expect(
      wrapper.findAll('button').every((button) => button.attributes('disabled') !== undefined),
    ).toBe(true)
  })
})
