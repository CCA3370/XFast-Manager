import { beforeEach, describe, expect, it, vi } from 'vitest'
import { importFlightPlan } from './flightPlanImport'

const api = vi.hoisted(() => ({ invoke: vi.fn(), getItem: vi.fn(), setItem: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))
vi.mock('./storage', () => ({
  getItem: api.getItem,
  setItem: api.setItem,
  STORAGE_KEYS: { FLIGHT_PLAN_CONFLICT_ACTION: 'flightPlanConflictAction' },
}))
const conflict = { status: 'conflict', name: 'route.fms', existingSize: 30, incomingSize: 31 }
const renamed = { status: 'imported', name: 'route.1.fms', renamed: true, replaced: false }

describe('flight plan import decisions', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    api.getItem.mockResolvedValue(null)
    api.setItem.mockResolvedValue(undefined)
  })

  it('skips identical content without asking or writing a preference', async () => {
    api.invoke.mockResolvedValue({ status: 'duplicate', name: 'existing.fms' })
    const decide = vi.fn()
    expect(await importFlightPlan('/sim', '/new.fms', decide)).toEqual({
      status: 'duplicate',
      name: 'existing.fms',
    })
    expect(decide).not.toHaveBeenCalled()
    expect(api.invoke).toHaveBeenCalledTimes(1)
    expect(api.setItem).not.toHaveBeenCalled()
  })

  it('asks for different content under the same name and does not remember by default', async () => {
    api.invoke.mockResolvedValueOnce(conflict).mockResolvedValueOnce(renamed)
    const decide = vi.fn(async () => ({ action: 'rename' as const, remember: false }))
    expect(await importFlightPlan('/sim', '/route.fms', decide)).toEqual(renamed)
    expect(decide).toHaveBeenCalledWith(conflict)
    expect(api.invoke).toHaveBeenLastCalledWith('import_flight_plan', {
      xplanePath: '/sim',
      source: '/route.fms',
      conflictAction: 'rename',
    })
    expect(api.setItem).not.toHaveBeenCalled()
  })

  it('cancels only this file without writing it or remembering cancellation', async () => {
    api.invoke.mockResolvedValue(conflict)
    expect(
      await importFlightPlan('/sim', '/route.fms', async () => ({
        action: 'cancel',
        remember: true,
      })),
    ).toEqual({ status: 'cancelled', name: 'route.fms' })
    expect(api.invoke).toHaveBeenCalledTimes(1)
    expect(api.setItem).not.toHaveBeenCalled()
  })

  it('saves the selected policy only after successful import', async () => {
    api.invoke.mockResolvedValueOnce(conflict).mockResolvedValueOnce({
      status: 'imported',
      name: 'route.fms',
      replaced: true,
      renamed: false,
    })
    await importFlightPlan('/sim', '/route.fms', async () => ({
      action: 'replace',
      remember: true,
    }))
    expect(api.setItem).toHaveBeenCalledWith('flightPlanConflictAction', 'replace')
  })

  it('applies a saved policy without opening the dialog', async () => {
    api.getItem.mockResolvedValue('rename')
    api.invoke.mockResolvedValue(renamed)
    const decide = vi.fn()
    expect(await importFlightPlan('/sim', '/route.fms', decide)).toEqual(renamed)
    expect(decide).not.toHaveBeenCalled()
    expect(api.invoke).toHaveBeenCalledWith('import_flight_plan', {
      xplanePath: '/sim',
      source: '/route.fms',
      conflictAction: 'rename',
    })
  })

  it('does not misreport a completed import as failed if saving the preference fails', async () => {
    api.invoke.mockResolvedValueOnce(conflict).mockResolvedValueOnce(renamed)
    api.setItem.mockRejectedValue(new Error('store unavailable'))
    expect(
      await importFlightPlan('/sim', '/route.fms', async () => ({
        action: 'rename',
        remember: true,
      })),
    ).toEqual({ ...renamed, preferenceSaveFailed: true })
  })
})
