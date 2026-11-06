import type { DoctorCheckResult } from '@/types/doctor'
import en from '@/i18n/en'
import zh from '@/i18n/zh'
import { CHECK_NAME_KEYS, LEGACY_TEXT_ROOTS } from './doctorCatalog'

function message(
  locale: string,
  key: string,
  params: Record<string, string | number> = {},
): string {
  let value: unknown = locale === 'zh' ? zh : en
  for (const part of key.split('.')) {
    if (!value || typeof value !== 'object') return ''
    value = (value as Record<string, unknown>)[part]
  }
  return typeof value === 'string'
    ? value.replace(/\{(\w+)\}/g, (token, name: string) => String(params[name] ?? token))
    : ''
}

export function describeDoctorCheck(check: DoctorCheckResult, locale = 'en') {
  const params = check.params ?? {}
  const chinese = locale === 'zh'
  const name = CHECK_NAME_KEYS[check.id]
  const root = check.id.startsWith('log.')
    ? `doctor.checks.log.${check.id.slice(4)}`
    : LEGACY_TEXT_ROOTS[check.id]
  const title =
    (name && message(locale, `doctor.center.checks.${name}`, params)) ||
    (root && message(locale, `${root}.title`, params)) ||
    check.id
  const generic = message(locale, `doctor.center.checkDescriptions.${check.outcome}`)
  if (['unavailable', 'notApplicable', 'cancelled'].includes(check.outcome)) {
    return { title, description: generic, suggestion: '' }
  }
  if (check.id === 'stability.last_session') {
    const descriptions: Record<string, string> = chinese
      ? {
          crash: 'Log.txt 包含 X-Plane 的明确崩溃标记。日志本身不一定能确定根因。',
          normal: 'Log.txt 包含 X-Plane 的正常退出标记。',
          unknown: '日志中没有明确的崩溃或正常退出标记，无法确认本次会话如何结束。',
          running: 'X-Plane 正在运行，本次日志尚未结束。',
        }
      : {
          crash:
            'Log.txt contains an explicit X-Plane crash marker. This alone does not establish the root cause.',
          normal: 'Log.txt contains the normal X-Plane shutdown marker.',
          unknown:
            'No explicit crash or normal shutdown marker was recorded. The session outcome cannot be determined from this log.',
          running: 'X-Plane is running; the current log is still being written.',
        }
    return {
      title,
      description: descriptions[String(params.sessionEnd)] ?? descriptions.unknown!,
      suggestion: '',
    }
  }
  if (check.id === 'updates.gateway') {
    return {
      title,
      description: chinese
        ? `已检查 ${params.installed ?? 0} 个已安装机场，${params.count ?? 0} 个有更新。`
        : `Checked ${params.installed ?? 0} installed airports; ${params.count ?? 0} have updates.`,
      suggestion: '',
    }
  }
  if (check.id === 'system.injectors') {
    return {
      title,
      description:
        check.outcome === 'pass'
          ? generic
          : chinese
            ? 'X-Plane 根目录存在代理 DLL 和 ReShade 配置。文件存在不代表它已被加载。'
            : 'Proxy DLLs and ReShade configuration are present in the X-Plane root. Presence does not prove they were loaded.',
      suggestion: '',
    }
  }
  const hasFinding = check.outcome !== 'pass'
  return {
    title,
    description: (hasFinding && root && message(locale, `${root}.description`, params)) || generic,
    suggestion: (hasFinding && root && message(locale, `${root}.suggestion`, params)) || '',
  }
}

export function formatDoctorDuration(milliseconds: number): string {
  const safeMilliseconds = Math.max(0, Math.round(milliseconds))
  if (safeMilliseconds < 1_000) return `${safeMilliseconds} ms`

  const totalSeconds = Math.round(safeMilliseconds / 1_000)
  if (totalSeconds < 60) return `${(safeMilliseconds / 1_000).toFixed(1)} s`

  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}m ${seconds}s`
}
