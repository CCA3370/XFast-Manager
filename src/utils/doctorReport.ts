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
    .replace(/([A-Za-z]:[\\/]Users[\\/])[^\\/\r\n]+/gi, '$1<USER>')
    .replace(/\/(home|Users)\/[^/\r\n]+/g, '/$1/<USER>')
    .replace(/([A-Za-z]:[\\/]Windows[\\/]Temp|\/tmp)(?=[\\/\s]|$)/gi, '<TEMP>')
}

function replacePath(value: string, path: string, token: string): string {
  const normalized = path.trim().replace(/\\/g, '/').replace(/\/+$/, '')
  if (!normalized) return value
  const variants = [normalized, normalized.replace(/\//g, '\\')]
  let result = value
  for (const variant of variants) {
    const pattern = variant.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    result = result.replace(new RegExp(pattern, 'gi'), token)
  }
  return result
}

function redactParams(
  params: Record<string, string | number> | undefined,
  options: DoctorReportOptions,
): Record<string, string | number> | undefined {
  if (!params) return undefined
  return Object.fromEntries(
    Object.entries(params).map(([key, value]) => [
      key,
      typeof value === 'string' ? redactDoctorText(value, options) : value,
    ]),
  )
}

export function redactDoctorText(value: string, options: DoctorReportOptions): string {
  if (options.includeSensitive) return value
  let redacted = replacePath(value, options.xplanePath, '<XPLANE_ROOT>')
  if (options.appDataPath) redacted = replacePath(redacted, options.appDataPath, '<XFAST_DATA>')
  redacted = redactCommonUserPaths(redacted)
  // External disks, linked scenery and network shares need the same protection as
  // configured roots. Preserve URLs and relative suffixes of our root tokens.
  return redacted.replace(
    /(^|[^\w:/>])(?:[A-Za-z]:[\\/]|\\\\|\/)([^\r\n"'<>|,;]*)/g,
    '$1<PRIVATE_PATH>',
  )
}

function reportCheck(check: DoctorCheckResult, options: DoctorReportOptions) {
  return {
    ...check,
    params: redactParams(check.params, options),
    evidence: check.evidence?.map((evidence) => ({
      ...evidence,
      label: evidence.label
        ? evidence.sensitive && !options.includeSensitive
          ? '<PRIVATE_EVIDENCE>'
          : redactDoctorText(evidence.label, options)
        : undefined,
      value:
        evidence.sensitive && !options.includeSensitive
          ? '<PRIVATE_EVIDENCE>'
          : redactDoctorText(evidence.value, options),
    })),
    remediation: check.remediation
      ? {
          ...check.remediation,
          params: redactParams(check.remediation.params, options),
        }
      : undefined,
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
      `## ${redactDoctorText(options.checkLabel?.(check) ?? check.id, options)}`,
      '',
      `- Section: ${redactDoctorText(options.sectionLabel?.(check) ?? check.section, options)}`,
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
