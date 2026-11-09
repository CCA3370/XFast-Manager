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
        { kind: 'path', value: 'C:\\Users\\alex\\X-Plane 12\\Log.txt' },
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
  it.each(['json', 'markdown'] as const)('omits old scope fields from %s', (format) => {
    const run = reportRun()
    Object.assign(run.checks[0], { sources: ['Old source'], scope: 'Old scope' })
    const output = buildDoctorReport(run, format, { xplanePath: '/xplane' })
    expect(output).not.toContain('Old source')
    expect(output).not.toContain('Old scope')
  })

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

  it.each(['json', 'markdown'] as const)('retains useful evidence in %s exports', (format) => {
    const run = reportRun()
    run.checks[0].evidence = [
      {
        kind: 'log',
        value: '12:01 E/PLG: Failed C:/Users/Alex/plugins/faulty.xpl: error 126',
        sensitive: true,
      },
      { kind: 'path', value: 'E:/Scenery/Demo Airport/Earth nav data/apt.dat', sensitive: true },
      { kind: 'path', value: '/Volumes/Private Disk/Scenery/apt.dat' },
      { kind: 'log', value: 'Cannot load \\\\server\\private\\apt.dat' },
    ]
    const exported = buildDoctorReport(run, format, { xplanePath: '/xplane' })
    for (const text of ['faulty.xpl', 'error 126', '12:01 E/PLG', 'Demo Airport', 'apt.dat']) {
      expect(exported).toContain(text)
    }
    for (const text of ['Alex', 'Private Disk', 'server', '<PRIVATE_EVIDENCE>']) {
      expect(exported).not.toContain(text)
    }
  })

  it('exports English titles and findings with diagnostic details in both formats', () => {
    const run = reportRun()
    run.checks[0] = {
      id: 'stability.last_session',
      section: 'stability',
      outcome: 'info',
      params: { sessionEnd: 'unknown' },
      durationMs: 1,
    }
    for (const format of ['json', 'markdown'] as const) {
      const output = buildDoctorReport(run, format, { xplanePath: '/xplane' })
      expect(output).toContain('Last X-Plane session')
      expect(output).toContain('cannot be determined')
      expect(output).toContain('If X-Plane closed unexpectedly, save the current Log.txt')
    }
  })

  it('uses the same run’s installation channel for repair guidance in an export', () => {
    const run = reportRun()
    run.checks = [
      { id: 'installation.structure', section: 'installation', outcome: 'critical', durationMs: 1 },
      {
        id: 'installation.location',
        section: 'installation',
        outcome: 'pass',
        params: { steam: 1 },
        durationMs: 1,
      },
    ]
    const output = buildDoctorReport(run, 'markdown', { xplanePath: '/xplane' })
    expect(output).toContain('Suggested action: Close X-Plane and use Steam’s Verify')
  })

  it('does not replace similarly prefixed paths or remove log explanations', () => {
    const output = redactDoctorText('C:/X-Plane Backup/plugins/foo.xpl: file not found', {
      xplanePath: 'C:/X-Plane',
    })
    expect(output).toContain('X-Plane Backup/plugins/foo.xpl: file not found')
  })

  it('preserves URLs when sanitizing log text', () => {
    expect(redactDoctorText('See https://example.com/support', { xplanePath: '/xplane' })).toBe(
      'See https://example.com/support',
    )
  })

  it('redacts a configured root consistently with or without a trailing separator', () => {
    expect(
      redactDoctorText('C:\\X-Plane 12', {
        xplanePath: 'C:\\X-Plane 12\\',
      }),
    ).toBe('<XPLANE_ROOT>')
  })
})
