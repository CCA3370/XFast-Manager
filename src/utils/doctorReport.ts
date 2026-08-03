import type { DoctorCheckResult, DoctorRun } from '@/types/doctor'

export type DoctorReportFormat = 'markdown' | 'json'

export interface DoctorReportOptions {
  xplanePath: string
  appDataPath?: string | null
  includeSensitive?: boolean
  checkLabel?: (check: DoctorCheckResult) => string
  sectionLabel?: (check: DoctorCheckResult) => string
}

function redactCommonUserPaths(value: string): string {
  return value
    .replace(/([A-Za-z]:[\\/]Users[\\/])[^\\/\s]+/gi, '$1<USER>')
    .replace(/\/(home|Users)\/[^/\s]+/g, '/$1/<USER>')
    .replace(/([A-Za-z]:[\\/]Windows[\\/]Temp|\/tmp)(?=[\\/\s]|$)/gi, '<TEMP>')
}

function replacePath(value: string, path: string, token: string): string {
  const normalized = path.trim().replace(/\\/g, '/')
  if (!normalized) return value
  const variants = [normalized, normalized.replace(/\//g, '\\')]
  let result = value
  for (const variant of variants) {
    const pattern = variant.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    result = result.replace(new RegExp(pattern, 'gi'), token)
  }
  return result
}

export function redactDoctorText(value: string, options: DoctorReportOptions): string {
  if (options.includeSensitive) return value
  let redacted = replacePath(value, options.xplanePath, '<XPLANE_ROOT>')
  if (options.appDataPath) redacted = replacePath(redacted, options.appDataPath, '<XFAST_DATA>')
  return redactCommonUserPaths(redacted)
}

function reportCheck(check: DoctorCheckResult, options: DoctorReportOptions) {
  return {
    ...check,
    evidence: check.evidence?.map((evidence) => ({
      ...evidence,
      value: redactDoctorText(evidence.value, options),
    })),
  }
}

export function buildDoctorReport(
  run: DoctorRun,
  format: DoctorReportFormat,
  options: DoctorReportOptions,
): string {
  const report = { ...run, checks: run.checks.map((check) => reportCheck(check, options)) }
  if (format === 'json') return JSON.stringify(report, null, 2)

  const lines = [
    '# XFast Manager Health Report',
    '',
    `- Run: ${run.id}`,
    `- Mode: ${run.mode}`,
    `- Started: ${new Date(run.startedAt).toISOString()}`,
    `- State: ${run.state}`,
    `- Health: ${run.summary.severity}`,
    `- Completeness: ${run.summary.completeness} (${run.summary.coveragePercent}%)`,
    `- Results: ${run.summary.pass} passed, ${run.summary.warning} warnings, ${run.summary.critical} critical, ${run.summary.unavailable} unavailable`,
    '',
  ]

  for (const check of report.checks) {
    lines.push(
      `## ${options.checkLabel?.(check) ?? check.id}`,
      '',
      `- Section: ${options.sectionLabel?.(check) ?? check.section}`,
      `- Outcome: ${check.outcome}`,
      `- Duration: ${check.durationMs} ms`,
    )
    if (check.params && Object.keys(check.params).length > 0) {
      lines.push(`- Parameters: ${JSON.stringify(check.params)}`)
    }
    if (check.evidence?.length) {
      lines.push('', 'Evidence:')
      for (const evidence of check.evidence.slice(0, 20)) {
        lines.push(`- ${evidence.label ? `${evidence.label}: ` : ''}${evidence.value}`)
      }
    }
    lines.push('')
  }

  return lines.join('\n')
}
