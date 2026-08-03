import { describe, expect, it } from 'vitest'
import type { DoctorRun } from '@/types/doctor'
import { summarizeDoctorRun } from './doctor'
import { buildDoctorReport } from './doctorReport'

function reportRun(): DoctorRun {
  const checks: DoctorRun['checks'] = [
    {
      id: 'log.crash',
      section: 'stability',
      outcome: 'critical',
      durationMs: 10,
      evidence: [
        {
          kind: 'path',
          value: 'C:\\Users\\alex\\X-Plane 12\\Log.txt',
          sensitive: true,
        },
        { kind: 'log', value: '/home/alex/.local/share/com.xfastmanager.tool/logs/app.log' },
      ],
    },
  ]
  return {
    schemaVersion: 1,
    id: 'run',
    installationId: 'installation',
    mode: 'full',
    state: 'completed',
    startedAt: 1,
    completedAt: 2,
    durationMs: 1,
    appVersion: '1.0.0',
    checks,
    summary: summarizeDoctorRun(checks, 'completed'),
    system: null,
  }
}

describe('doctor report export', () => {
  it('redacts installation, app data, and user paths by default', () => {
    const result = buildDoctorReport(reportRun(), 'json', {
      xplanePath: 'C:\\Users\\alex\\X-Plane 12',
      appDataPath: '/home/alex/.local/share/com.xfastmanager.tool',
    })

    expect(result).toContain('<XPLANE_ROOT>')
    expect(result).toContain('<XFAST_DATA>')
    expect(result).not.toContain('alex')
  })

  it('keeps raw evidence only after explicit opt-in', () => {
    const result = buildDoctorReport(reportRun(), 'markdown', {
      xplanePath: 'C:\\Users\\alex\\X-Plane 12',
      includeSensitive: true,
    })
    expect(result).toContain('C:\\Users\\alex\\X-Plane 12\\Log.txt')
  })
})
