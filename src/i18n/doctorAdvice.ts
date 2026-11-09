export const doctorAdvice = {
  saveCrashLog: [
    'Save a copy of Log.txt before restarting. Repeat the same flight or startup steps and run a full diagnosis if the crash recurs.',
    '重启前先保留一份 Log.txt。按原来的飞行或启动步骤复测，若再次崩溃，请运行完整诊断。',
  ],
  oldCrashLog: [
    'This crash is from an older session. Save the existing log, then reproduce the problem and scan again before changing settings.',
    '崩溃记录来自较早的会话。请先保留现有日志，再复现问题并重新检查，然后决定是否调整设置。',
  ],
  unknownSession: [
    'If X-Plane closed unexpectedly, save the current Log.txt before reopening it. If the problem recurs, run a full diagnosis and keep the new log for comparison.',
    '如果 X-Plane 确实意外关闭，请在重启前保留当前的 Log.txt。若问题重现，请运行完整诊断，并保留新日志用于比较。',
  ],
  activeSession: [
    'Exit X-Plane normally, then run the check again to review the completed session.',
    '请正常退出 X-Plane，然后重新检查已结束的会话。',
  ],
  sessionStateUnavailable: [
    'Confirm X-Plane has exited, then scan again so the completed log can be evaluated.',
    '确认 X-Plane 已退出后重新检查，以便判断完整日志中的会话状态。',
  ],
  missingLog: [
    'Start X-Plane from the selected installation, exit normally, then scan again using the generated Log.txt.',
    '从当前选中的安装启动 X-Plane，正常退出后，使用生成的 Log.txt 重新检查。',
  ],
  unrecognizedLog: [
    'Confirm the selected folder is the simulator installation. Start X-Plane there and exit normally to generate a simulator Log.txt, then scan again.',
    '确认所选目录是模拟器的安装目录，从该目录启动 X-Plane 并正常退出，生成模拟器的 Log.txt 后重新检查。',
  ],
  oldLog: [
    'Save the current Log.txt if you need it for comparison. Reproduce the symptom and rerun the check against the new log.',
    '如需对比，请先保留当前的 Log.txt。复现问题后使用新日志重新检查。',
  ],
  dumpDisabled: [
    'Enable crash dump analysis in Settings and run a full diagnosis while the crash log and dump are still available.',
    '在设置中启用崩溃转储分析，并趁崩溃日志和转储仍可用时运行完整诊断。',
  ],
  dumpUnavailable: [
    'Keep the crash log and dump together. Use the log findings to continue troubleshooting and capture a fresh dump if the crash recurs.',
    '请一并保留崩溃日志和转储文件，先根据日志中的发现排查。如果崩溃重现，请保留新的转储。',
  ],
  dumpPlugin: [
    'If the related module belongs to a third-party plugin, temporarily disable that plugin and retry the same flight. Restore it before testing another plugin.',
    '若相关模块属于第三方插件，请先暂时停用该插件，并重试同一飞行场景。测试其他插件前先恢复这个插件。',
  ],
  dumpGpu: [
    'Use the graphics errors in Log.txt to guide the next test. If the crash began after a graphics driver change, compare with the previous working driver.',
    '根据 Log.txt 中的图形错误安排下一步测试。若崩溃从更换显卡驱动后开始，可用此前运行正常的驱动复测。',
  ],
  dumpMemory: [
    'Check Log.txt for memory pressure. If it appears, close memory-heavy apps and retry the same flight with a lower texture setting.',
    '查看 Log.txt 是否出现内存压力。若有相关记录，请关闭高内存占用的应用，并将纹理质量降低一档后复测同一场景。',
  ],
  dumpInconclusive: [
    'Keep Log.txt and the dump for comparison. Try the same scenario with a stock aircraft and change one variable at a time if it still crashes.',
    '保留 Log.txt 和转储以便对比。先用原厂机模测试同一场景，若仍然崩溃，每次只改变一个条件继续排查。',
  ],
} as const
