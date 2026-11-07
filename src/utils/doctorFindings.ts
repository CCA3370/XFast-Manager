import { healthDetailMessages } from '@/i18n/healthDetails'

const enPlugin = healthDetailMessages.en.findings.addons_binary_integrity
const zhPlugin = healthDetailMessages.zh.findings.addons_binary_integrity

// Check-specific wording shared by the UI and English exports.
export const DOCTOR_FINDINGS: Record<string, { en: [string, string]; zh: [string, string] }> = {
  ...Object.fromEntries(
    (
      [
        'installation_structure',
        'installation_stock_aircraft',
        'installation_airfoils',
        'installation_asset_access',
        'installation_write_access',
        'installation_permission_access',
        'system_vulkan_runtime',
        'scenery_files',
      ] as const
    ).map((key) => {
      const en = healthDetailMessages.en.findings[key]
      const zh = healthDetailMessages.zh.findings[key]
      return [
        key.replace('_', '.'),
        { en: [en.title, en.description], zh: [zh.title, zh.description] },
      ]
    }),
  ),
  'navdata.bundled': {
    en: [
      healthDetailMessages.en.findings.navdata_bundled.title,
      healthDetailMessages.en.findings.navdata_bundled.description,
    ],
    zh: [
      healthDetailMessages.zh.findings.navdata_bundled.title,
      healthDetailMessages.zh.findings.navdata_bundled.description,
    ],
  },
  'navdata.supplemental': {
    en: [
      healthDetailMessages.en.findings.navdata_supplemental.title,
      healthDetailMessages.en.findings.navdata_supplemental.description,
    ],
    zh: [
      healthDetailMessages.zh.findings.navdata_supplemental.title,
      healthDetailMessages.zh.findings.navdata_supplemental.description,
    ],
  },
  'navdata.scan_access': {
    en: [
      healthDetailMessages.en.findings.navdata_scan_access.title,
      healthDetailMessages.en.findings.navdata_scan_access.description,
    ],
    zh: [
      healthDetailMessages.zh.findings.navdata_scan_access.title,
      healthDetailMessages.zh.findings.navdata_scan_access.description,
    ],
  },
  'scenery.registration': {
    en: [
      'Scenery registration file',
      'scenery_packs.ini is missing. X-Plane may not have generated it yet. Synchronize the scenery list to create it.',
    ],
    zh: [
      '地景注册文件',
      'scenery_packs.ini 不存在，可能尚未由 X-Plane 生成。可同步地景列表来创建。',
    ],
  },
  'scenery.invalid_entries': {
    en: [
      healthDetailMessages.en.findings.scenery_invalid_entries.title,
      healthDetailMessages.en.findings.scenery_invalid_entries.description,
    ],
    zh: [
      healthDetailMessages.zh.findings.scenery_invalid_entries.title,
      healthDetailMessages.zh.findings.scenery_invalid_entries.description,
    ],
  },
  'scenery.missing_paths': {
    en: [
      'Missing scenery paths',
      'Enabled scenery registrations point to missing folders. Check the listed paths and reconnect external drives before changing the scenery list.',
    ],
    zh: [
      '失效地景路径',
      '已启用的地景条目指向不存在的目录。请核对证据中的路径，并先确认外接磁盘已连接。',
    ],
  },
  'scenery.duplicate_paths': {
    en: [
      'Duplicate scenery registrations',
      'The same enabled scenery folder is registered more than once. Review the listed line numbers in scenery_packs.ini.',
    ],
    zh: ['重复地景注册', '同一已启用地景目录被重复注册，请核对 scenery_packs.ini 中列出的行号。'],
  },
  'scenery.broken_links': {
    en: [
      'Broken scenery links',
      'Scenery links or shortcuts have unavailable targets. Reconnect the target disk or correct the link before rescanning.',
    ],
    zh: [
      '失效地景链接',
      '地景符号链接或快捷方式的目标不可用。请连接目标磁盘或修正链接后重新检查。',
    ],
  },
  'navdata.empty_files': {
    en: [
      'Empty navigation data files',
      'Custom navigation files contain zero bytes. Restore a complete navigation data package; empty files cannot supply routes or waypoints.',
    ],
    zh: [
      '空导航数据文件',
      '自定义导航数据中存在零字节文件。请重新安装完整数据包，空文件无法提供航路或航点。',
    ],
  },
  'addons.binary_integrity': {
    en: [enPlugin.title, enPlugin.description],
    zh: [zhPlugin.title, zhPlugin.description],
  },
  'installation.scan_access': {
    en: [
      'Inspection access and limits',
      'Some files could not be read, or the inspection reached its entry limit. Results cover only the files inspected; see the recorded access errors.',
    ],
    zh: [
      '扫描可读性与范围',
      '部分文件无法读取，或扫描达到条目上限。结果仅覆盖已检查文件，请查看证据中的读取错误。',
    ],
  },
  'stability.log_freshness': {
    en: [
      'Log age',
      'Log.txt is over seven days old. Its findings describe an older session; reproduce the problem and scan again using the resulting log.',
    ],
    zh: [
      '日志时效',
      'Log.txt 已超过七天，其中的现象属于较早的会话。请复现故障后，使用新日志重新检查。',
    ],
  },
  'updates.gateway_unavailable': {
    en: [
      'Gateway airports not checked',
      'Gateway did not return an update result for every airport. The listed airports are unverified; retry the full diagnosis later.',
    ],
    zh: [
      'Gateway 未完成查询的机场',
      'Gateway 未能返回所有机场的更新状态。列出的机场尚未完成检查，请稍后重试完整诊断。',
    ],
  },
}

export const HEALTH_RECOVERY_COPY = {
  en: {
    title: 'Repairs & backups',
    noMatches: 'No matching checks',
    trySearch: 'Change the search or switch to All.',
    backupError: 'Could not read repair backups. Retry the check.',
    empty: 'No repair backups.',
    restore: 'Restore',
    confirm: 'Continue',
    restoreBody:
      'Restore this backup? Current files will be saved in another backup before replacement. Close X-Plane first.',
    backupNote: 'Each repair saves the original files locally. Restore them here.',
    englishReport: 'Reports are exported in English. Original log evidence is retained.',
    disable_injectors: 'Disable injector DLLs',
    rebuild_shader_cache: 'Rebuild shader cache',
    reset_preferences: 'Reset preferences',
    disable_injectorsBody:
      'Move detected ReShade proxy DLLs into a backup. The injector will no longer load. Other DLLs, shaders and configuration files are preserved.',
    rebuild_shader_cacheBody:
      'Back up the renderer shader cache. X-Plane will regenerate it; the next launch or flight may take longer.',
    reset_preferencesBody:
      'Back up and reset the entire Output/preferences folder, including graphics, controls and joystick assignments. X-Plane will generate defaults. Restore the backup here to undo this.',
  },
  zh: {
    title: '修复与备份',
    noMatches: '没有匹配的检查项',
    trySearch: '请调整搜索内容，或切换到全部。',
    backupError: '无法读取修复备份，请重新检查。',
    empty: '暂无修复备份。',
    restore: '恢复',
    confirm: '继续',
    restoreBody: '恢复这份备份？替换前会另行备份当前文件。请先关闭 X-Plane。',
    backupNote: '每次修复都会在本地保留原文件，可在此恢复。',
    englishReport: '报告统一使用英文，日志证据保留原文。',
    disable_injectors: '停用注入 DLL',
    rebuild_shader_cache: '重建着色器缓存',
    reset_preferences: '重置首选项',
    disable_injectorsBody:
      '将检测到的 ReShade 代理 DLL 移入备份，阻止注入器加载。其他 DLL、着色器和配置文件会保留。',
    rebuild_shader_cacheBody:
      '备份渲染器着色器缓存，由 X-Plane 重新生成。下次启动或加载飞行可能需要更长时间。',
    reset_preferencesBody:
      '备份并重置整个 Output/preferences 目录，包括画面、控制和摇杆分配。X-Plane 将生成默认设置，可在此恢复备份来撤销。',
  },
}
