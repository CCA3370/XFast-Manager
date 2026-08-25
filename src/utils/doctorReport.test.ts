import { describe, expect, it } from 'vitest'
import type { DoctorRun } from '@/types/doctor'
import { summarizeDoctorRun } from './doctor'
import { buildDoctorReport, redactDoctorText } from './doctorReport'

function reportRun(): DoctorRun {
  const checks: DoctorRun['checks'] = [
    {
      id: 'log.crash',
      section: 'stability',
      outcome: 'critical',
      durationMs: 10,
      params: { module: 'C:\\Users\\Alex Smith\\plugins\\faulty.xpl' },
      evidence: [
        {
          kind: 'path',
          value: 'C:\\Users\\alex\\X-Plane 12\\Log.txt',
          label: 'C:\\Users\\Alex Smith\\logs',
          sensitive: true,
        },
        { kind: 'log', value: '/home/alex/.local/share/com.xfastmanager.tool/logs/app.log' },
      ],
      remediation: {
        id: 'test',
        kind: 'automatic',
        risk: 'safe',
        params: { source: 'C:\\Users\\Alex Smith\\plugins' },
      },
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
    expect(result).not.toContain('Alex Smith')
  })

  it('keeps raw evidence only after explicit opt-in', () => {
    const result = buildDoctorReport(reportRun(), 'markdown', {
      xplanePath: 'C:\\Users\\alex\\X-Plane 12',
      includeSensitive: true,
    })
    expect(result).toContain('C:\\Users\\alex\\X-Plane 12\\Log.txt')
    expect(result).toContain('Alex Smith')
  })

  it('redacts a configured root consistently with or without a trailing separator', () => {
    expect(
      redactDoctorText('C:\\X-Plane 12', {
        xplanePath: 'C:\\X-Plane 12\\',
      }),
    ).toBe('<XPLANE_ROOT>')
  })
})
