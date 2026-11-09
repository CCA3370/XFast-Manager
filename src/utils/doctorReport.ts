import type { DoctorCheckResult, DoctorRun } from '@/types/doctor'
import { describeDoctorCheck, suggestDoctorCheck } from './doctorPresentation'

export type DoctorReportFormat = 'markdown' | 'json'

export interface DoctorReportOptions {
  xplanePath: string
  appDataPath?: string | null
  includeSensitive?: boolean
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
    result = result.replace(new RegExp(`${pattern}(?=$|[\\\\/"'(),;:])`, 'gi'), () => token)
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
  // Redact identifiable personal components, retaining diagnostic filenames,
  // relative paths, plugin names, timestamps and the rest of each log line.
  return redacted
    .replace(/\\\\[^\\/\s]+[\\/][^\\/\r\n"'<>|]+/g, '<NETWORK_SHARE>')
    .replace(/\b([A-Za-z]:)[\\/]/g, '<VOLUME>/')
    .replace(/\/Volumes\/[^/\r\n]+/g, '<VOLUME>')
    .replace(/\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/gi, '<EMAIL>')
}

function reportCheck(
  check: DoctorCheckResult,
  checks: readonly DoctorCheckResult[],
  options: DoctorReportOptions,
) {
  const text = describeDoctorCheck(check, 'en')
  return {
    id: check.id,
    section: check.section,
    outcome: check.outcome,
    durationMs: check.durationMs,
    title: redactDoctorText(text.title, options),
    description: redactDoctorText(text.description, options),
    suggestion: redactDoctorText(suggestDoctorCheck(check, 'en', checks), options),
    params: redactParams(check.params, options),
    evidence: check.evidence?.map((evidence) => ({
      ...evidence,
      label: evidence.label ? redactDoctorText(evidence.label, options) : undefined,
      value: redactDoctorText(evidence.value, options),
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
  const report = {
    ...run,
    checks: run.checks.map((check) => reportCheck(check, run.checks, options)),
  }
  if (format === 'json') return JSON.stringify(report, null, 2)

  const lines = [
    '# XFast Manager Health Report',
    '',
    `- Application version: ${run.appVersion}`,
    '- Report language: English',
    `- Privacy: ${options.includeSensitive ? 'Original paths included' : 'User names, installation roots and network shares redacted; evidence retained'}`,
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

  if (report.system) {
    lines.push('## System', '', '```json', JSON.stringify(report.system, null, 2), '```', '')
  }

  for (const check of report.checks) {
    lines.push(
      `## ${check.title}`,
      '',
      `- Section: ${check.section}`,
      `- Outcome: ${check.outcome}`,
      `- Duration: ${check.durationMs} ms`,
      '',
      check.description,
      ...(check.suggestion ? ['', `Suggested action: ${check.suggestion}`] : []),
    )
    if (check.params && Object.keys(check.params).length > 0) {
      lines.push(`- Parameters: ${JSON.stringify(check.params)}`)
    }
    if (check.evidence?.length) {
      lines.push('', 'Evidence:')
      for (const evidence of check.evidence.slice(0, 20)) {
        lines.push(
          '```text',
          `${evidence.label ? `${evidence.label}: ` : ''}${evidence.value}`,
          '```',
        )
      }
    }
    lines.push('')
  }

  return lines.join('\n')
}
