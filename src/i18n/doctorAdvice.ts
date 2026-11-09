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
  dumpMissing: [
    'Save Log.txt for the confirmed crash. If the crash recurs and produces a dump, keep it with the new log and rerun the full diagnosis.',
    '保留这次已确认崩溃的 Log.txt。如果崩溃重现且生成了转储，请同时保留新日志和转储，再运行完整诊断。',
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
  selectInstallation: [
    'In Settings, select the X-Plane installation folder that contains the simulator executable. Reconnect its drive first if it is external, then scan again.',
    '在设置中选择包含模拟器主程序的 X-Plane 安装目录。如果安装在外接盘上，请先重新连接磁盘，再重新检查。',
  ],
  repairSteam: [
    'Close X-Plane and use Steam’s Verify integrity of game files for this installation. Rerun the check after Steam finishes.',
    '关闭 X-Plane，在 Steam 中对这套安装执行“验证游戏文件的完整性”。校验完成后重新检查。',
  ],
  repairInstaller: [
    'Close X-Plane and use the official X-Plane installer to update or repair this installation. Rerun the check afterward.',
    '关闭 X-Plane，使用官方安装器更新或修复当前安装，完成后重新检查。',
  ],
  repairStockFiles: [
    'Restore the listed stock files using Steam file verification or the official X-Plane installer, depending on how X-Plane was installed.',
    '根据安装来源，用 Steam 文件校验或 X-Plane 官方安装器恢复证据中列出的原厂文件。',
  ],
  restoreAirfoils: [
    'Use the evidence to identify the missing or invalid airfoil. Restore stock files through your simulator installer and aircraft-specific files from the matching aircraft package.',
    '根据证据定位缺失或无效的翼型。原厂文件用对应的模拟器安装工具恢复，机模专用文件从匹配版本的机模安装包恢复。',
  ],
  readAccess: [
    'Check that the listed paths are reachable and readable from your account, then scan again. Reconnect any external volumes before changing files.',
    '确认当前账户能访问并读取证据中列出的路径，再重新检查。若路径在外接盘上，请先连接磁盘。',
  ],
  scanLimit: [
    'The scan covered only part of the library. Check the listed area or package individually if you need to verify it now.',
    '本次只扫描了部分文件。如需立即核实，请对证据中列出的范围或包单独检查。',
  ],
  writableFolders: [
    'Check the listed folders and make sure the volume and your account allow writes. If a regular file occupies a required folder path, back it up before correcting that path.',
    '核对列出的目录，确认磁盘和当前账户允许写入。如果所需目录的位置被普通文件占用，请先备份该文件，再修正路径。',
  ],
  readOnlyFiles: [
    'If an install or update fails, inspect the specific files it could not write to. Change permissions only for files that should be writable.',
    '如果安装或更新失败，请核对操作中无法写入的具体文件，只调整本应可写的文件权限。',
  ],
  steamLocation: [
    'If Steam updates or add-on installs fail because of permissions, use Steam’s storage settings to move the installation to a writable library. Then select that location in XFast Manager.',
    '如果 Steam 更新或扩展安装因权限失败，请用 Steam 的存储设置把游戏迁移到可写的库，再在 XFast Manager 中选择新位置。',
  ],
  protectedLocation: [
    'If X-Plane or its add-ons cannot be updated here, use a writable installation folder outside Program Files and select that folder in XFast Manager.',
    '如果当前目录妨碍 X-Plane 或扩展更新，请使用 Program Files 以外的可写安装目录，并在 XFast Manager 中选择该目录。',
  ],
  simulatorSpace: [
    'Free space on the X-Plane volume before the next install or update. Use Disk Usage to review large items and keep any files you still need.',
    '下次安装或更新前，先释放 X-Plane 所在磁盘的空间。可在“磁盘占用”查看大文件，并保留仍需使用的内容。',
  ],
  applicationSpace: [
    'Free space on the volume that contains XFast Manager’s app data. Review other files on that volume first and keep the app database and settings.',
    '请释放 XFast Manager 应用数据所在磁盘的空间。优先检查该磁盘上的其他文件，保留应用数据库和设置。',
  ],
  diskUnavailable: [
    'Check that the relevant volume is mounted and view its available space in your operating system before retrying the check.',
    '确认相关磁盘已挂载，在系统中查看剩余空间，然后重新检查。',
  ],
  cleanable: [
    'Review the candidates in Output Cleanup. Clean only items you can regenerate or no longer need.',
    '在“输出清理”中逐项确认，只清理能够重新生成或已不再需要的内容。',
  ],
  availableMemory: [
    'Close memory-heavy applications and check again before starting X-Plane. If available memory is still low during flight, compare the same flight with a lower texture setting.',
    '启动 X-Plane 前，先关闭高内存占用的应用并重新检查。如果飞行时可用内存仍低，可将纹理质量降低一档后复测同一场景。',
  ],
  hardware: [
    'Compare the CPU or memory listed in the evidence with the requirements for your X-Plane version before planning any hardware changes.',
    '在考虑更换硬件前，请把证据中列出的 CPU 或内存与当前 X-Plane 版本的系统要求对照。',
  ],
  hardwareUnavailable: [
    'Check the processor and installed memory in your operating system’s system information, then retry the check.',
    '在操作系统的系统信息中确认处理器和已安装内存，然后重新检查。',
  ],
  vulkan: [
    'Check the graphics driver for your GPU and operating system. Install a supported driver, restart if required, then check the Vulkan runtime again.',
    '检查与显卡和操作系统匹配的驱动。安装受支持的驱动，按需重启系统，再检查 Vulkan 运行时。',
  ],
  injector: [
    'If you are investigating a graphics or startup problem, close X-Plane and use the backed-up injector repair on a fresh check. Retest the same scenario and restore the DLLs if the symptom persists.',
    '如果正在排查画面或启动故障，请关闭 X-Plane，在新检查结果中使用带备份的停用注入 DLL 操作。复测同一场景，若故障仍在，可从备份恢复。',
  ],
  beta: [
    'If the issue started after switching to a beta, compare the same scenario on a stable build supported by the affected add-on.',
    '如果问题从切换到测试版后开始，请用受影响扩展所支持的稳定版复测同一场景。',
  ],
  registrationMissing: [
    'Reconnect any external scenery drives first. With X-Plane closed, use the sync action on a fresh check to create scenery_packs.ini, then review the resulting order.',
    '先连接地景所用的外接盘。关闭 X-Plane，在新检查结果中同步并创建 scenery_packs.ini，然后核对生成的顺序。',
  ],
  registrationUnreadable: [
    'Keep a copy of scenery_packs.ini. Check whether your account can read it and whether it opens as a text file. Restore a known-good copy if it is damaged, then scan again.',
    '先保留一份 scenery_packs.ini。检查当前账户是否有读取权限，以及文件能否作为文本打开。如果文件损坏，请恢复正常的备份，再重新检查。',
  ],
  invalidEntries: [
    'Back up scenery_packs.ini, then use the listed line numbers to fix empty or invalid entries. Keep valid entries in their existing order.',
    '先备份 scenery_packs.ini，再根据证据中的行号修正空条目或无效条目，保留有效条目的原有顺序。',
  ],
  missingPaths: [
    'Reconnect the drive or correct the path to the package first. If you intentionally removed it, review the stale entry in Scenery Management after confirming the package is no longer needed.',
    '先连接磁盘或修正地景包路径。如果你已主动移除该地景包，请确认以后不再需要它，再到地景管理检查遗留条目。',
  ],
  duplicatePaths: [
    'Back up scenery_packs.ini and compare the listed entries. Keep one registration at the priority you want for that package.',
    '备份 scenery_packs.ini 后对照列出的条目，在你希望使用的优先级位置保留一条注册。',
  ],
  brokenLinks: [
    'Reconnect the target drive or repair the shortcut or link. Verify that it opens the intended scenery folder before running the check again.',
    '连接目标磁盘或修复快捷方式、符号链接。确认它指向正确的地景目录后再重新检查。',
  ],
  globalAirports: [
    'If you want the default airports to load, close X-Plane and re-enable Global Airports. Keep custom airports that should take priority above it.',
    '如果需要加载默认机场，请关闭 X-Plane 后重新启用 Global Airports。需要优先加载的自定义机场应排在它前面。',
  ],
  sortScenery: [
    'Reconnect external scenery drives and review the intended priorities before sorting. Close X-Plane before applying the new order.',
    '排序前先连接地景所用的外接盘，并核对期望的优先级。应用新顺序前请关闭 X-Plane。',
  ],
  manualScenery: [
    'Review the registered packages and adjust the order in Scenery Management if needed. Keep any intentional priority overrides.',
    '在地景管理中核对已注册的地景包，按需手动调整加载顺序，并保留有意设置的优先级。',
  ],
  sceneryOrganizers: [
    'If two tools are changing scenery_packs.ini, choose one to manage the order and turn off automatic sorting in the other.',
    '如果两个工具都在改写 scenery_packs.ini，请选一个负责地景排序，并关闭另一个工具的自动排序。',
  ],
  dependencies: [
    'For each listed package, check the developer’s requirements and install or enable the named libraries. If a library is already present, check that its package folder is registered.',
    '按证据中的地景包逐一核对作者的依赖要求，安装或启用列出的库。若库已安装，请确认其地景包目录已注册。',
  ],
  overlaps: [
    'Compare the listed packages for each tile and check their intended layering. If two packages replace the same layer, choose the one you want and test with the other temporarily disabled.',
    '按图块核对列出的地景包及其预期层级。如果两个包替换同一层，请选定要使用的包，暂时停用另一个后复测。',
  ],
  duplicateAirports: [
    'If the wrong airport appears in the simulator, compare the listed packages for that airport and put the preferred custom airport above the other definitions.',
    '如果模拟器加载了错误的机场版本，请核对该机场列出的地景包，将要使用的自定义机场排在其他定义上方。',
  ],
  flattenDrifted: [
    'Review the listed airports and reapply any saved flattening overrides you still want after closing X-Plane.',
    '核对列出的机场。若仍需使用已保存的地形平整覆盖，请关闭 X-Plane 后重新应用。',
  ],
  flattenMissing: [
    'For missing source files, locate or restore the airport package before reapplying its override.',
    '对于源文件缺失的机场，请先找到或恢复对应地景包，再重新应用覆盖。',
  ],
  sceneryFiles: [
    'Use the paths in the evidence to identify the affected packages. Restore damaged files from a trusted copy of the same package, or temporarily disable just that package to test.',
    '根据证据中的路径定位受影响的地景包。从同一地景包的可靠副本恢复损坏文件，或只暂时停用该包进行测试。',
  ],
  sceneryLog: [
    'Check the lines around the error for a package name and the failing file or missing library. Repair that dependency or temporarily disable just the affected package and retest.',
    '查看错误附近的日志，定位地景包以及加载失败的文件或缺失库。修复对应的依赖，或只暂时停用受影响的地景包后复测。',
  ],
  oldPluginScan: [
    'Run a new full diagnosis to check the plugin entry that this platform actually selects.',
    '运行一次新的完整诊断，检查当前平台实际选用的插件入口。',
  ],
  pluginPlatform: [
    'Check the plugin’s supported operating systems and CPU architectures. Install the matching build from its developer or disable the incompatible copy if you do not need it.',
    '核对插件支持的操作系统和 CPU 架构。从作者处安装匹配的版本，或停用不需要的不兼容版本。',
  ],
  pluginBinary: [
    'Use the listed executable and reason to identify the plugin. Restore it from the developer’s complete package for your platform. Retest with that plugin temporarily disabled if X-Plane still fails to load it.',
    '根据证据中的可执行文件和原因定位插件，从作者提供的对应平台完整包恢复。如果 X-Plane 仍无法加载它，请暂时停用该插件后复测。',
  ],
  isolatePlugin: [
    'Check the surrounding log lines for a plugin name. Temporarily disable that third-party plugin and repeat the same scenario. If no plugin is named, test recently changed plugins one at a time.',
    '查看该行前后的日志以定位插件。暂时停用对应的第三方插件并复测同一场景。如果日志未点名，可逐一测试近期改动过的插件。',
  ],
  duplicatePlugin: [
    'Compare the listed plugin copies and their installation locations. Keep the copy required by your setup and temporarily disable the extra copy to test.',
    '核对插件副本及各自的安装位置，保留当前配置所需的副本，并暂时停用额外的副本后复测。',
  ],
  graphicsLog: [
    'Compare the same scenario after restarting X-Plane. If the error recurs, check the driver version and test any recently changed graphics settings or add-ons one at a time.',
    '重启 X-Plane 后复测同一场景。如果错误重现，请检查驱动版本，并逐一测试近期改动过的图形设置或扩展。',
  ],
  blockedLayer: [
    'If the blocked graphics layer was intentional, check with its developer for a supported version. If rendering works as expected, keep the current setup.',
    '如果该图形层是你主动安装的，请向作者确认是否有受支持的版本。如果画面运行正常，可以保留当前配置。',
  ],
  logContext: [
    'Read the surrounding lines of Log.txt for the operation and any named component. If the symptom recurs, save the new log and compare the errors before making changes.',
    '查看 Log.txt 对应行的上下文，确认当时的操作和点名的组件。如果问题重现，请保存新日志并对照错误，再决定如何调整。',
  ],
  navOverride: [
    'Back up Custom Data and restore a complete X-Plane navigation package for one AIRAC cycle from your data provider. Check the listed files after reinstalling.',
    '先备份 Custom Data，再从数据提供方恢复同一个 AIRAC 周期的完整 X-Plane 导航数据包。重装后核对证据中的文件。',
  ],
  navSupplemental: [
    'Check the source of each listed file. Restore provider files from the matching package and recover user-edited fixes or navaids from your own backup.',
    '核对每个文件的来源。供应方文件从匹配的数据包恢复，用户自定义的定位点或导航台则从自己的备份恢复。',
  ],
  navFuture: [
    'Check the effective dates in the evidence and your system date. Use an already effective cycle for flights scheduled before the new cycle begins.',
    '核对证据中的生效日期和系统日期。对于新周期生效前的飞行，请使用已生效的周期。',
  ],
  navConflict: [
    'Compare the cycle numbers for each affected data source. Restore matching files and metadata from the same package before checking again.',
    '核对各受影响数据源的周期编号，从同一数据包恢复相匹配的文件和元数据，再重新检查。',
  ],
  navExpired: [
    'Update the expired sources listed in the evidence if your flight plans or charts require a current cycle. Keep the simulator and relevant aircraft databases aligned.',
    '如果飞行计划或航图需要当前周期，请更新证据中列出的过期数据源，并让模拟器与相关机模的数据库使用一致的周期。',
  ],
  navExpiring: [
    'Check the listed expiry dates and plan the next data update before a flight that requires the new cycle.',
    '核对证据中的到期日期，在需要新周期的飞行前安排下一次数据更新。',
  ],
  navUnknown: [
    'Compare the cycle and effective dates with the package’s source information. Restore its metadata from the same package if it is missing or unreadable.',
    '对照数据包来源核实周期和生效日期。如果元数据缺失或不可读，请从同一数据包恢复。',
  ],
  appData: [
    'Check space and write access on the XFast Manager data volume. Resolve the error shown in the evidence before retrying installs or updates.',
    '检查 XFast Manager 数据盘的剩余空间和写入权限。先处理证据中列出的错误，再重试安装或更新。',
  ],
  appDatabase: [
    'Keep a copy of the application data before making changes. If the problem persists after restarting XFast Manager, export the health report for support.',
    '调整前先保留应用数据的副本。如果重启 XFast Manager 后问题仍在，请导出健康报告以便进一步排查。',
  ],
  sceneryIndex: [
    'Confirm your scenery drives are connected, then refresh or rebuild the scenery index and verify that the expected packages appear.',
    '确认地景所用的磁盘已连接，再刷新或重建地景索引，并检查应有的地景包是否已显示。',
  ],
  recentFailures: [
    'Open Activity and read the error for the affected installation or update. Resolve that error before retrying the operation.',
    '打开活动记录，查看对应安装或更新的具体错误，处理后再重试该操作。',
  ],
  addonUpdate: [
    'Review the confirmed updates in Management. Check compatibility with your X-Plane version before installing.',
    '在管理页面查看已确认的更新。安装前先核对与当前 X-Plane 版本的兼容性。',
  ],
  addonManual: [
    'For entries marked as manual upgrades, follow the provider’s migration instructions and back up your current configuration first.',
    '对于标为手动升级的项目，请先备份现有配置，再按供应方的迁移说明操作。',
  ],
  addonExternal: [
    'For entries managed by an external updater, check their versions using the updater named in the evidence.',
    '对于由外部更新器管理的项目，请使用证据中列出的更新工具核对版本。',
  ],
  gatewayUpdate: [
    'In Gateway, review the airport and submission numbers in the evidence before updating. Keep a copy of any local edits you want to preserve.',
    '在 Gateway 页面根据证据核对机场和提交编号后再更新。需要保留的本地修改请先备份。',
  ],
  cslUpdate: [
    'Open CSL and review the listed model packages and changed files before applying their updates.',
    '打开 CSL 页面，核对证据中列出的模型包和变更文件后再更新。',
  ],
  appUpdate: [
    'Open Settings to review the available XFast Manager version and its release notes before updating.',
    '在设置中查看可用的 XFast Manager 版本和发行说明，再决定是否更新。',
  ],
  retryUpdate: [
    'Check the connection and any provider errors in the evidence, then retry the full diagnosis for the entries that could not be verified.',
    '核对网络连接和证据中的服务错误，再运行完整诊断，重新查询未能验证的项目。',
  ],
  retryCheck: [
    'Review any error in the evidence and run the check again. If it keeps failing, export the health report with that evidence.',
    '查看证据中列出的错误后重新检查。如果仍然失败，请导出包含该证据的健康报告。',
  ],
} as const
