// @vitest-environment happy-dom
import { defineComponent } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { useFlightPlanImport } from './useFlightPlanImport'
import type { FlightPlanConflict, FlightPlanConflictDecision } from '@/types/flightPlans'

const service = vi.hoisted(() => ({ importFlightPlan: vi.fn() }))
vi.mock('@/services/flightPlanImport', () => service)

describe('queued flight plan conflict decisions', () => {
  it('queues files and cancels the pending decision when Home is unmounted', async () => {
    service.importFlightPlan.mockImplementation(
      async (
        _root: string,
        source: string,
        decide: (conflict: FlightPlanConflict) => Promise<FlightPlanConflictDecision>,
      ) => {
        const name = source.split('/').pop()!
        const decision = await decide({
          status: 'conflict',
          name,
          existingSize: 1,
          incomingSize: 2,
        })
        return { status: decision.action === 'cancel' ? 'cancelled' : 'imported', name }
      },
    )
    let importer!: ReturnType<typeof useFlightPlanImport>
    const wrapper = mount(
      defineComponent({
        setup() {
          importer = useFlightPlanImport()
          return {}
        },
        template: '<div />',
      }),
    )
    const first = importer.importPlan('/sim', '/first.fms')
    const second = importer.importPlan('/sim', '/second.fms')
    await flushPromises()
    expect(importer.conflict.value?.name).toBe('first.fms')
    expect(service.importFlightPlan).toHaveBeenCalledTimes(1)
    importer.resolveConflict({ action: 'cancel', remember: false })
    expect((await first).status).toBe('cancelled')
    await flushPromises()
    expect(importer.conflict.value?.name).toBe('second.fms')
    wrapper.unmount()
    expect((await second).status).toBe('cancelled')
    expect(importer.conflict.value).toBeNull()
  })
})
