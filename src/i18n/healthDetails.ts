export const healthDetailMessages = {
  en: {
    coverage: {
      addonUpdates:
        'Compared {checked} of {installed} supported add-ons. Updates found: {count}. External updater: {external}.',
      navFuture:
        'Some installed navigation packages are not yet effective. Review the listed dates.',
    },
    fields: {
      count: 'Affected items',
      installed: 'Installed airports',
      checked: 'Items checked',
      free: 'Available space',
      total: 'Total capacity',
      available: 'Available memory',
      line: 'Log line',
      days: 'Log age (days)',
      errors: 'Read errors',
      indexed: 'Indexed entries',
      found: 'Entries on disk',
      files: 'Files',
      size: 'Storage used',
      platform: 'Host platform',
      version: 'Version',
      expired: 'Expired data packages',
      expiring: 'Expiring data packages',
      unknown: 'Validity unknown',
      cycles: 'Detected AIRAC cycles',
      aircraft: 'Aircraft',
      plugins: 'Plugins',
      csl: 'X-CSL models',
      altitude: 'Altitude data',
      cause: 'Suspected cause',
      confidence: 'Evidence score',
      module: 'Related module',
      os: 'Operating system',
      arch: 'Architecture',
      cores: 'Logical processors',
      memory: 'Installed memory',
      capped: 'Scan limit reached',
      sessionEnd: 'Session ending',
      steam: 'Steam installation',
    },
    scope: {
      installation: 'Installation folders and file access were checked.',
      stability: 'The latest session log and available crash evidence were reviewed.',
      scenery: 'Scenery registrations, dependencies and loading consistency were checked.',
      addons: 'Installed addons and their compatibility were checked.',
      navdata: 'Installed navigation data was checked for completeness and consistency.',
      performance: 'Available system resources were checked.',
      storage: 'Disk capacity and files eligible for cleanup were checked.',
      system: 'System information and graphics environment were inspected.',
      xfast: 'Application data access, database and scenery index were checked.',
      updates: 'Installed versions were compared with available update information.',
    },
    nav: {
      expired:
        'Some installed navigation packages have expired. Update the listed data sources before relying on their procedures and waypoints.',
      expiring: 'Some navigation packages expire soon. Plan their next update.',
      unknown:
        'Validity could not be established for some packages. Check their cycle and source metadata.',
      current: 'The navigation packages with known validity are current.',
    },
    yes: 'Yes',
    no: 'No',
    findings: {
      scenery_files: {
        title: 'Enabled scenery files',
        description:
          'Some enabled packages contain unreadable, empty, or invalid scenery files. Review the listed files.',
      },
      installation_write_access: {
        title: 'Installation write access',
        description:
          'The listed folders cannot accept new files. Check their permissions and volume state.',
      },
      installation_permission_access: {
        title: 'Permission check incomplete',
        description: 'Some permissions could not be verified. Review the listed errors.',
      },
      system_vulkan_runtime: {
        title: 'Vulkan runtime',
        description:
          'The system Vulkan loader is missing or below 1.3. Install or update the graphics driver.',
      },
      installation_structure: {
        title: 'X-Plane installation structure',
        description:
          'Restore the listed startup files with the X-Plane installer or Steam file verification.',
      },
      installation_stock_aircraft: {
        title: 'Stock aircraft',
        description:
          'The listed aircraft files are missing from the installation. Restore the affected stock aircraft.',
      },
      installation_airfoils: {
        title: 'Airfoils',
        description:
          'The listed shared or aircraft airfoils are missing or damaged. Restore the affected files.',
      },
      installation_asset_access: {
        title: 'Installation file access',
        description: 'Some installation files could not be verified. Review the listed errors.',
      },
      navdata_bundled: {
        title: 'Bundled navigation data',
        description: 'Repair the X-Plane installation to restore the listed bundled files.',
      },
      navdata_supplemental: {
        title: 'Supplemental navigation files',
        description: 'Review the listed ARINC 424, FAA procedure, or user navigation files.',
      },
      navdata_scan_access: {
        title: 'Navigation data access',
        description:
          'Some navigation data or cycle metadata could not be inspected. Review the listed errors.',
      },
      scenery_registration: {
        title: 'Scenery registration file',
        description:
          'scenery_packs.ini is missing. X-Plane may not have generated it yet. Synchronize the scenery list to create it.',
      },
      scenery_invalid_entries: {
        title: 'Invalid scenery registrations',
        description:
          'Some enabled entries in scenery_packs.ini have no path. Review the listed lines.',
      },
      scenery_missing_paths: {
        title: 'Missing scenery paths',
        description:
          'Enabled scenery registrations point to missing folders. Check the listed paths and reconnect external drives before changing the scenery list.',
      },
      scenery_duplicate_paths: {
        title: 'Duplicate scenery registrations',
        description:
          'The same enabled scenery folder is registered more than once. Review the listed line numbers in scenery_packs.ini.',
      },
      scenery_broken_links: {
        title: 'Broken scenery links',
        description:
          'Scenery links or shortcuts have unavailable targets. Reconnect the target disk or correct the link before rescanning.',
      },
      navdata_empty_files: {
        title: 'Empty navigation data files',
        description:
          'Custom navigation files contain zero bytes. Restore a complete navigation data package; empty files cannot supply routes or waypoints.',
      },
      addons_binary_integrity: {
        title: 'Plugin binary headers and architecture',
        description:
          'An entry selected for this platform is empty or has an invalid or unsupported executable header. Review the file and the reason listed in the evidence.',
        scope:
          'The scan checks executable headers of global, aircraft and enabled registered scenery plugin entries. Dependencies and actual loading are outside this check.',
        outdated:
          'This saved result predates platform entry selection. Run a new full scan to verify the plugin files.',
      },
      installation_scan_access: {
        title: 'Inspection access and limits',
        description:
          'Some files could not be read, or the inspection reached its entry limit. Results cover only the files inspected; see the recorded access errors.',
      },
      stability_log_freshness: {
        title: 'Log age',
        description:
          'Log.txt is over seven days old. Its findings describe an older session; reproduce the problem and scan again using the resulting log.',
      },
      updates_gateway_unavailable: {
        title: 'Gateway airports not checked',
        description:
          'Gateway did not return an update result for every airport. The listed airports are unverified; retry the full diagnosis later.',
      },
    },
    purposes: {
      installation_structure:
        'Required X-Plane folders are checked for missing installation components.',
      installation_xplane_running:
        'The simulator process is checked before file repairs are offered.',
      installation_location: 'The installation location is checked for system-protected folders.',
      installation_readonly:
        'Installation files are checked for read-only attributes that can block updates.',
      storage_xplane_space:
        'Free space on the simulator drive is checked against installation requirements.',
      storage_app_data_space: 'Free space on the application data drive is checked.',
      system_inventory:
        'Operating system, processor architecture and installed memory are recorded for troubleshooting.',
      system_injectors:
        'The installation root is checked for graphics proxy libraries and injector configuration.',
      performance_available_memory:
        'Available physical memory is checked for resource pressure before running X-Plane.',
      xfast_app_data: 'The application data directory is checked for write access.',
      xfast_database: 'The application database is checked for readable and consistent records.',
      xfast_scenery_index: 'The scenery index is compared with packages present on disk.',
      stability_log_available:
        'The available Log.txt is checked for an identifiable X-Plane session.',
      stability_crash_dump:
        'Available crash dump evidence is reviewed for probable causes, not a confirmed diagnosis.',
      system_beta_build:
        'The recorded simulator version is checked for a beta or release candidate build.',
      scenery_global_airports:
        'The Global Airports registration is checked for disabled default airports.',
      scenery_load_order: 'The scenery loading list is checked against the managed package order.',
      scenery_dependencies: 'Enabled scenery is checked for missing library dependencies.',
      scenery_overlaps: 'Scenery packages are checked for overlapping geographic tiles.',
      scenery_duplicate_airports:
        'Airport registrations are checked for duplicate airport identifiers.',
      scenery_flatten:
        'Managed airport flattening overrides are checked for drift from their saved state.',
      navdata_custom_data:
        'The Custom Data directory is checked for a custom simulator navigation database.',
      navdata_cifp: 'Instrument procedure data is checked for an available CIFP directory.',
      navdata_integrity: 'Required navigation data files are checked for missing components.',
      navdata_consistency:
        'Installed navigation data components are checked for differing AIRAC cycles.',
      xfast_recent_failures:
        'Recent installation and update records are checked for failed operations.',
      storage_cleanable:
        'Recognized caches and generated output are measured for optional cleanup.',
      addons_inventory:
        'Installed aircraft and plugins are counted from the selected installation.',
      addons_platform: 'Installed plugin platforms are compared with the host operating system.',
      updates_addons: 'Supported addon update sources are checked for newer releases.',
      updates_app: 'The installed XFast Manager version is compared with release information.',
      updates_csl:
        'Installed model and altitude data versions are compared with available updates.',
    },
    sessions: {
      crash:
        'Log.txt contains an explicit X-Plane crash marker. The log alone does not establish the root cause.',
      normal: 'Log.txt records a normal X-Plane shutdown.',
      unknown:
        'The log has no explicit crash or normal shutdown marker. The session ending cannot be determined.',
      running: 'X-Plane is running and the current log is still being written.',
    },
  },
  zh: {
    coverage: {
      addonUpdates:
        '已比对 {checked}/{installed} 个受支持的扩展，发现 {count} 个更新，{external} 个需使用外部更新器。',
      navFuture: '部分导航数据包尚未生效，请核对列出的生效日期。',
    },
    fields: {
      count: '涉及条目',
      installed: '已安装机场',
      checked: '已检查条目',
      free: '可用空间',
      total: '总容量',
      available: '可用内存',
      line: '日志行号',
      days: '日志距今天数',
      errors: '读取错误',
      indexed: '索引内条目',
      found: '磁盘条目',
      files: '文件数量',
      size: '占用空间',
      platform: '当前平台',
      version: '版本',
      expired: '已过期数据包',
      expiring: '临近到期数据包',
      unknown: '有效期未知',
      cycles: '检测到的 AIRAC 周期',
      aircraft: '机模数量',
      plugins: '插件数量',
      csl: 'X-CSL 模型',
      altitude: '高度数据',
      cause: '疑似原因',
      confidence: '线索评分',
      module: '关联模块',
      os: '操作系统',
      arch: '处理器架构',
      cores: '逻辑处理器',
      memory: '已安装内存',
      capped: '达到扫描上限',
      sessionEnd: '会话结束方式',
      steam: 'Steam 安装',
    },
    scope: {
      installation: '已检查安装目录结构和文件访问情况。',
      stability: '已检查最近会话日志及可用的崩溃证据。',
      scenery: '已检查地景注册、依赖关系和加载一致性。',
      addons: '已检查扩展内容及其兼容性。',
      navdata: '已检查导航数据的完整性与周期一致性。',
      performance: '已检查当前可用的系统资源。',
      storage: '已检查磁盘容量及可清理文件。',
      system: '已检查系统信息及图形运行环境。',
      xfast: '已检查应用数据访问、数据库及地景索引。',
      updates: '已将已安装版本与可用更新信息进行比较。',
    },
    nav: {
      expired:
        '部分已安装导航数据包已过期。请更新证据中列出的数据源，并核对航路、航点和程序所用周期。',
      expiring: '部分导航数据包即将到期，请安排更新。',
      unknown: '部分数据包无法确定有效期，请核对其周期和来源信息。',
      current: '已确认有效期的导航数据包均处于有效期内。',
    },
    yes: '是',
    no: '否',
    findings: {
      scenery_files: {
        title: '已启用地景文件',
        description: '部分已启用地景包含不可读、为空或格式无效的文件，请核对列出的项目。',
      },
      installation_write_access: {
        title: '安装目录写入权限',
        description: '无法在列出的目录中新增文件，请检查权限和卷状态。',
      },
      installation_permission_access: {
        title: '权限检查未完成',
        description: '部分权限无法核实，请查看列出的错误。',
      },
      system_vulkan_runtime: {
        title: 'Vulkan 运行库',
        description: '系统 Vulkan 加载器缺失或低于 1.3，请安装或更新显卡驱动。',
      },
      installation_structure: {
        title: 'X-Plane 安装结构',
        description: '请使用 X-Plane 安装器或 Steam 文件验证恢复列出的启动文件。',
      },
      installation_stock_aircraft: {
        title: '原厂机模',
        description: '缺少列出的机模文件，请恢复对应的原厂机模。',
      },
      installation_airfoils: {
        title: '翼型文件',
        description: '列出的共享或机模专用翼型缺失或损坏，请恢复相应文件。',
      },
      installation_asset_access: {
        title: '安装文件访问',
        description: '部分安装文件无法核实，请查看列出的错误。',
      },
      navdata_bundled: {
        title: '原厂导航数据',
        description: '请修复 X-Plane 安装，以恢复列出的原厂数据文件。',
      },
      navdata_supplemental: {
        title: '补充导航数据',
        description: '请核对列出的 ARINC 424、FAA 程序或用户自定义导航文件。',
      },
      navdata_scan_access: {
        title: '导航数据读取范围',
        description: '部分导航数据或周期资料无法检查，请查看列出的错误。',
      },
      scenery_registration: {
        title: '地景注册文件',
        description: 'scenery_packs.ini 不存在，可能尚未由 X-Plane 生成。可同步地景列表来创建。',
      },
      scenery_invalid_entries: {
        title: '无效地景注册',
        description: 'scenery_packs.ini 中有已启用条目未填写路径，请核对列出的行号。',
      },
      scenery_missing_paths: {
        title: '失效地景路径',
        description:
          '已启用的地景条目指向不存在的目录。请核对证据中的路径，并先确认外接磁盘已连接。',
      },
      scenery_duplicate_paths: {
        title: '重复地景注册',
        description: '同一已启用地景目录被重复注册，请核对 scenery_packs.ini 中列出的行号。',
      },
      scenery_broken_links: {
        title: '失效地景链接',
        description: '地景符号链接或快捷方式的目标不可用。请连接目标磁盘或修正链接后重新检查。',
      },
      navdata_empty_files: {
        title: '空导航数据文件',
        description:
          '自定义导航数据中存在零字节文件。请重新安装完整数据包，空文件无法提供航路或航点。',
      },
      addons_binary_integrity: {
        title: '插件二进制头与架构',
        description:
          '当前平台选中的插件入口为空，或文件头无效、架构不受支持。请查看证据中列出的文件和原因。',
        scope:
          '已检查全局插件、飞机插件和已启用的注册地景插件的入口文件头。依赖库和实际加载情况需要另行验证。',
        outdated: '此记录来自旧版插件检查。请重新运行全面检查，以验证 X-Plane 会选择的插件入口。',
      },
      installation_scan_access: {
        title: '扫描可读性与范围',
        description:
          '部分文件无法读取，或扫描达到条目上限。结果仅覆盖已检查文件，请查看证据中的读取错误。',
      },
      stability_log_freshness: {
        title: '日志时效',
        description:
          'Log.txt 已超过七天，其中的现象属于较早的会话。请复现故障后，使用新日志重新检查。',
      },
      updates_gateway_unavailable: {
        title: 'Gateway 未完成查询的机场',
        description:
          'Gateway 未能返回所有机场的更新状态。列出的机场尚未完成检查，请稍后重试完整诊断。',
      },
    },
    purposes: {
      installation_structure: '核对 X-Plane 必需目录，判断安装组件是否缺失。',
      installation_xplane_running: '核对模拟器进程状态，决定是否可以执行文件修复。',
      installation_location: '检查安装位置是否属于系统保护目录，避免权限限制影响更新。',
      installation_readonly: '检查安装文件的只读属性，排查更新时无法写入的问题。',
      storage_xplane_space: '核对模拟器所在磁盘的可用容量，评估安装、更新和缓存写入空间。',
      storage_app_data_space: '核对应用数据所在磁盘的可用容量，评估下载、数据库和日志写入空间。',
      system_inventory: '记录操作系统、处理器架构和内存配置，供故障排查核对。',
      system_injectors: '检查安装根目录内的图形代理库和注入器配置，文件存在不代表已加载。',
      performance_available_memory:
        '核对当前可用物理内存，排查资源紧张导致的加载失败或运行不稳定。',
      xfast_app_data: '验证应用数据目录是否可写，确保设置、日志和下载状态可以保存。',
      xfast_database: '验证应用数据库的可读性与完整性，排查管理数据读取异常。',
      xfast_scenery_index: '对比地景索引和磁盘实际内容，排查缺失、残留或尚未收录的条目。',
      stability_log_available: '检查 Log.txt 是否可读取，以及是否能够识别为 X-Plane 会话日志。',
      stability_crash_dump:
        '核对可用崩溃转储中的线索。评分表示证据关联程度，不能单独作为确定根因的结论。',
      system_beta_build: '核对日志记录的模拟器版本，识别测试版或候选版本。',
      scenery_global_airports: '检查默认 Global Airports 地景是否被停用。',
      scenery_load_order: '对比地景加载列表与管理顺序，排查排序或同步异常。',
      scenery_dependencies: '检查已启用地景依赖的公共资源库是否缺失。',
      scenery_overlaps: '检查地景覆盖范围是否重叠，辅助排查加载冲突。',
      scenery_duplicate_airports: '检查机场标识是否存在重复注册，辅助排查同一机场多份地景的冲突。',
      scenery_flatten: '核对机场平整设置与保存状态是否一致。',
      navdata_custom_data: '检查 Custom Data 中是否存在供模拟器使用的自定义导航数据库。',
      navdata_cifp: '检查仪表程序数据的 CIFP 目录是否存在。',
      navdata_integrity: '核对导航数据必需文件是否齐全，排查不完整的数据覆盖。',
      navdata_consistency: '对比已安装导航数据组件的 AIRAC 周期，排查混用不同周期的数据。',
      xfast_recent_failures: '检查近期安装和更新记录中的失败操作，供定位仍未解决的问题。',
      storage_cleanable: '统计可识别的缓存和生成文件，供选择清理范围。',
      addons_inventory: '统计当前安装目录中的机模和插件。',
      addons_platform: '对比插件提供的平台文件与当前操作系统，检查是否有可用版本。',
      updates_addons: '向受支持的扩展更新源查询新版本。',
      updates_app: '对比当前 XFast Manager 版本与发布信息。',
      updates_csl: '对比已安装联飞机模与高度数据的版本，检查可用更新。',
    },
    sessions: {
      crash: 'Log.txt 包含明确的 X-Plane 崩溃标记，但日志本身不一定能确定根因。',
      normal: 'Log.txt 记录了 X-Plane 的正常退出。',
      unknown: '日志没有明确的崩溃或正常退出标记，无法据此判断会话如何结束。',
      running: 'X-Plane 正在运行，当前日志仍在写入。',
    },
  },
  de: {
    coverage: {
      addonUpdates:
        '{checked} von {installed} unterstützten Add-ons verglichen. Updates: {count}. Externe Aktualisierung: {external}.',
      navFuture:
        'Einige installierte Navigationspakete sind noch nicht gültig. Prüfe die angegebenen Daten.',
    },
    fields: {
      count: 'Betroffene Einträge',
      installed: 'Installierte Flughäfen',
      checked: 'Geprüfte Einträge',
      free: 'Freier Speicher',
      total: 'Gesamtkapazität',
      available: 'Verfügbarer Arbeitsspeicher',
      line: 'Protokollzeile',
      days: 'Protokollalter (Tage)',
      errors: 'Lesefehler',
      indexed: 'Indexeinträge',
      found: 'Einträge auf Datenträger',
      files: 'Dateien',
      size: 'Belegter Speicher',
      platform: 'Plattform',
      version: 'Version',
      expired: 'Abgelaufene Datenpakete',
      expiring: 'Bald ablaufende Pakete',
      unknown: 'Gültigkeit unbekannt',
      cycles: 'Erkannte AIRAC-Zyklen',
      aircraft: 'Flugzeuge',
      plugins: 'Plugins',
      csl: 'X-CSL-Modelle',
      altitude: 'Höhendaten',
      cause: 'Vermutete Ursache',
      confidence: 'Indizienbewertung',
      module: 'Betroffenes Modul',
      os: 'Betriebssystem',
      arch: 'Architektur',
      cores: 'Logische Prozessoren',
      memory: 'Installierter Arbeitsspeicher',
      capped: 'Scanlimit erreicht',
      sessionEnd: 'Sitzungsende',
      steam: 'Steam-Installation',
    },
    scope: {
      installation: 'Installationsordner und Dateizugriff wurden geprüft.',
      stability: 'Sitzungsprotokoll und verfügbare Absturzbelege wurden geprüft.',
      scenery: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      addons: 'Addons und Kompatibilität wurden geprüft.',
      navdata: 'Navigationsdaten wurden auf Vollständigkeit und Konsistenz geprüft.',
      performance: 'Verfügbare Systemressourcen wurden geprüft.',
      storage: 'Speicherplatz und bereinigbare Dateien wurden geprüft.',
      system: 'System und Grafikumgebung wurden geprüft.',
      xfast: 'App-Daten, Datenbank und Szenerieindex wurden geprüft.',
      updates: 'Installierte Versionen wurden mit Updateinformationen verglichen.',
    },
    nav: {
      expired:
        'Einige Navigationspakete sind abgelaufen. Aktualisieren Sie die aufgeführten Quellen.',
      expiring: 'Einige Pakete laufen bald ab. Planen Sie ein Update.',
      unknown: 'Die Gültigkeit einiger Pakete ist unklar. Prüfen Sie Zyklus und Quelle.',
      current: 'Pakete mit bekannter Gültigkeit sind aktuell.',
    },
    yes: 'Ja',
    no: 'Nein',
    findings: {
      scenery_files: {
        title: 'Aktive Szeneriedateien',
        description:
          'Einige aktive Pakete enthalten nicht lesbare, leere oder ungültige Dateien. Prüfe die aufgeführten Dateien.',
      },
      installation_write_access: {
        title: 'Schreibzugriff auf die Installation',
        description:
          'In den aufgeführten Ordnern können keine Dateien angelegt werden. Prüfe Berechtigungen und Laufwerk.',
      },
      installation_permission_access: {
        title: 'Berechtigungsprüfung unvollständig',
        description: 'Einige Berechtigungen konnten nicht geprüft werden. Siehe die Fehler.',
      },
      system_vulkan_runtime: {
        title: 'Vulkan-Laufzeit',
        description:
          'Der Vulkan-Loader fehlt oder ist älter als 1.3. Installiere oder aktualisiere den Grafiktreiber.',
      },
      installation_structure: {
        title: 'X-Plane-Installation',
        description:
          'Stelle die aufgeführten Startdateien mit dem X-Plane-Installer oder der Steam-Dateiprüfung wieder her.',
      },
      installation_stock_aircraft: {
        title: 'Standardflugzeuge',
        description:
          'Die aufgeführten Flugzeugdateien fehlen. Stelle die betroffenen Flugzeuge wieder her.',
      },
      installation_airfoils: {
        title: 'Tragflächenprofile',
        description:
          'Die aufgeführten Profile fehlen oder sind beschädigt. Stelle die betroffenen Dateien wieder her.',
      },
      installation_asset_access: {
        title: 'Zugriff auf Installationsdateien',
        description: 'Einige Dateien konnten nicht geprüft werden. Siehe die aufgeführten Fehler.',
      },
      navdata_bundled: {
        title: 'Mitgelieferte Navigationsdaten',
        description:
          'Repariere die X-Plane-Installation, um die aufgeführten Dateien wiederherzustellen.',
      },
      navdata_supplemental: {
        title: 'Ergänzende Navigationsdaten',
        description: 'Prüfe die aufgeführten ARINC-424-, FAA- oder Benutzernavigationsdateien.',
      },
      navdata_scan_access: {
        title: 'Zugriff auf Navigationsdaten',
        description:
          'Einige Navigationsdaten oder Zyklusinformationen konnten nicht geprüft werden. Siehe die aufgeführten Fehler.',
      },
      scenery_invalid_entries: {
        title: 'Ungültige Szenerieeinträge',
        description:
          'Bei aktivierten Einträgen in scenery_packs.ini fehlt ein Pfad. Prüfe die aufgeführten Zeilen.',
      },
      scenery_registration: {
        title: 'Szenerieregistrierung',
        description: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      },
      scenery_missing_paths: {
        title: 'Fehlende Szeneriepfade',
        description: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      },
      scenery_duplicate_paths: {
        title: 'Doppelte Szenerieeinträge',
        description: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      },
      scenery_broken_links: {
        title: 'Defekte Szenerieverknüpfungen',
        description: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      },
      navdata_empty_files: {
        title: 'Leere Navigationsdateien',
        description: 'Navigationsdaten wurden auf Vollständigkeit und Konsistenz geprüft.',
      },
      addons_binary_integrity: {
        title: 'Plugin-Dateien und Architektur',
        description:
          'Ein für diese Plattform ausgewählter Plugin-Einstieg ist leer oder hat einen ungültigen oder nicht unterstützten Dateikopf. Datei und Grund stehen in den Details.',
        scope:
          'Geprüft wurden die Dateiköpfe globaler Plugins sowie der Plugins von Flugzeugen und aktivierten registrierten Szenerien. Abhängigkeiten und tatsächliches Laden werden separat geprüft.',
        outdated:
          'Dieses Ergebnis stammt aus der früheren Plugin-Prüfung. Bitte eine neue vollständige Prüfung starten.',
      },
      installation_scan_access: {
        title: 'Scan-Zugriff und Grenzen',
        description: 'Installationsordner und Dateizugriff wurden geprüft.',
      },
      stability_log_freshness: {
        title: 'Aktualität des Protokolls',
        description: 'Sitzungsprotokoll und verfügbare Absturzbelege wurden geprüft.',
      },
      updates_gateway_unavailable: {
        title: 'Nicht geprüfte Gateway-Flughäfen',
        description: 'Installierte Versionen wurden mit Updateinformationen verglichen.',
      },
    },
    purposes: {
      installation_structure: 'Installationsordner und Dateizugriff wurden geprüft.',
      installation_xplane_running: 'Installationsordner und Dateizugriff wurden geprüft.',
      installation_location: 'Installationsordner und Dateizugriff wurden geprüft.',
      installation_readonly: 'Installationsordner und Dateizugriff wurden geprüft.',
      storage_xplane_space: 'Speicherplatz und bereinigbare Dateien wurden geprüft.',
      storage_app_data_space: 'Speicherplatz und bereinigbare Dateien wurden geprüft.',
      system_inventory: 'System und Grafikumgebung wurden geprüft.',
      system_injectors: 'System und Grafikumgebung wurden geprüft.',
      performance_available_memory: 'Verfügbare Systemressourcen wurden geprüft.',
      xfast_app_data: 'App-Daten, Datenbank und Szenerieindex wurden geprüft.',
      xfast_database: 'App-Daten, Datenbank und Szenerieindex wurden geprüft.',
      xfast_scenery_index: 'App-Daten, Datenbank und Szenerieindex wurden geprüft.',
      stability_log_available: 'Sitzungsprotokoll und verfügbare Absturzbelege wurden geprüft.',
      stability_crash_dump: 'Sitzungsprotokoll und verfügbare Absturzbelege wurden geprüft.',
      system_beta_build: 'System und Grafikumgebung wurden geprüft.',
      scenery_global_airports:
        'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      scenery_load_order: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      scenery_dependencies: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      scenery_overlaps: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      scenery_duplicate_airports:
        'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      scenery_flatten: 'Szenerieeinträge, Abhängigkeiten und Ladekonsistenz wurden geprüft.',
      navdata_custom_data: 'Navigationsdaten wurden auf Vollständigkeit und Konsistenz geprüft.',
      navdata_cifp: 'Navigationsdaten wurden auf Vollständigkeit und Konsistenz geprüft.',
      navdata_integrity: 'Navigationsdaten wurden auf Vollständigkeit und Konsistenz geprüft.',
      navdata_consistency: 'Navigationsdaten wurden auf Vollständigkeit und Konsistenz geprüft.',
      xfast_recent_failures: 'App-Daten, Datenbank und Szenerieindex wurden geprüft.',
      storage_cleanable: 'Speicherplatz und bereinigbare Dateien wurden geprüft.',
      addons_inventory: 'Addons und Kompatibilität wurden geprüft.',
      addons_platform: 'Addons und Kompatibilität wurden geprüft.',
      updates_addons: 'Installierte Versionen wurden mit Updateinformationen verglichen.',
      updates_app: 'Installierte Versionen wurden mit Updateinformationen verglichen.',
      updates_csl: 'Installierte Versionen wurden mit Updateinformationen verglichen.',
    },
    sessions: {
      crash:
        'Log.txt enthält einen ausdrücklichen Absturzmarker. Dies bestimmt nicht allein die Ursache.',
      normal: 'Log.txt enthält eine normale Beendigung von X-Plane.',
      unknown: 'Kein eindeutiger Absturz- oder Beendigungsmarker. Das Sitzungsende ist unbestimmt.',
      running: 'X-Plane läuft; das Protokoll wird noch geschrieben.',
    },
  },
  fr: {
    coverage: {
      addonUpdates:
        '{checked} extensions comparées sur {installed} prises en charge. Mises à jour : {count}. Outil externe : {external}.',
      navFuture:
        'Certaines données de navigation installées ne sont pas encore valides. Vérifiez les dates indiquées.',
    },
    fields: {
      count: 'Éléments concernés',
      installed: 'Aéroports installés',
      checked: 'Éléments vérifiés',
      free: 'Espace disponible',
      total: 'Capacité totale',
      available: 'Mémoire disponible',
      line: 'Ligne du journal',
      days: 'Âge du journal (jours)',
      errors: 'Erreurs de lecture',
      indexed: 'Entrées indexées',
      found: 'Entrées sur disque',
      files: 'Fichiers',
      size: 'Espace utilisé',
      platform: 'Plateforme',
      version: 'Version',
      expired: 'Paquets expirés',
      expiring: 'Paquets bientôt expirés',
      unknown: 'Validité inconnue',
      cycles: 'Cycles AIRAC détectés',
      aircraft: 'Avions',
      plugins: 'Extensions',
      csl: 'Modèles X-CSL',
      altitude: 'Données de hauteur',
      cause: 'Cause suspectée',
      confidence: 'Score des indices',
      module: 'Module concerné',
      os: 'Système',
      arch: 'Architecture',
      cores: 'Processeurs logiques',
      memory: 'Mémoire installée',
      capped: 'Limite atteinte',
      sessionEnd: 'Fin de session',
      steam: 'Installation Steam',
    },
    scope: {
      installation: 'Dossiers et accès aux fichiers vérifiés.',
      stability: 'Journal de session et indices de plantage examinés.',
      scenery: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      addons: 'Extensions et compatibilité vérifiées.',
      navdata: 'Intégrité et cohérence des données de navigation vérifiées.',
      performance: 'Ressources système disponibles vérifiées.',
      storage: 'Capacité du disque et fichiers nettoyables vérifiés.',
      system: 'Système et environnement graphique examinés.',
      xfast: 'Données locales, base et index des scènes vérifiés.',
      updates: 'Versions installées comparées aux mises à jour disponibles.',
    },
    nav: {
      expired: 'Des paquets de navigation ont expiré. Mettez à jour les sources indiquées.',
      expiring: 'Des paquets expirent bientôt. Prévoyez leur mise à jour.',
      unknown: 'Validité indéterminée pour certains paquets. Vérifiez cycle et source.',
      current: 'Les paquets à validité connue sont à jour.',
    },
    yes: 'Oui',
    no: 'Non',
    findings: {
      scenery_files: {
        title: 'Fichiers des scènes actives',
        description:
          'Certaines scènes actives contiennent des fichiers illisibles, vides ou non valides. Consultez la liste.',
      },
      installation_write_access: {
        title: 'Accès en écriture à l’installation',
        description:
          'Impossible de créer des fichiers dans les dossiers indiqués. Vérifiez les autorisations et le volume.',
      },
      installation_permission_access: {
        title: 'Vérification des autorisations incomplète',
        description: 'Certaines autorisations n’ont pas pu être vérifiées. Consultez les erreurs.',
      },
      system_vulkan_runtime: {
        title: 'Environnement Vulkan',
        description:
          'Le chargeur Vulkan est absent ou antérieur à 1.3. Installez ou mettez à jour le pilote graphique.',
      },
      installation_structure: {
        title: 'Installation de X-Plane',
        description:
          'Restaurez les fichiers de démarrage indiqués via le programme X-Plane ou la vérification Steam.',
      },
      installation_stock_aircraft: {
        title: 'Avions inclus',
        description: 'Les fichiers indiqués sont absents. Restaurez les avions concernés.',
      },
      installation_airfoils: {
        title: 'Profils aérodynamiques',
        description:
          'Des profils indiqués sont manquants ou endommagés. Restaurez les fichiers concernés.',
      },
      installation_asset_access: {
        title: 'Accès aux fichiers de X-Plane',
        description:
          'Certains fichiers ne peuvent pas être vérifiés. Consultez les erreurs indiquées.',
      },
      navdata_bundled: {
        title: 'Données de navigation intégrées',
        description: 'Réparez l’installation de X-Plane pour restaurer les fichiers indiqués.',
      },
      navdata_supplemental: {
        title: 'Données de navigation complémentaires',
        description: 'Vérifiez les fichiers ARINC 424, FAA ou utilisateur indiqués.',
      },
      navdata_scan_access: {
        title: 'Accès aux données de navigation',
        description:
          'Certaines données ou métadonnées de cycle n’ont pas pu être vérifiées. Consultez les erreurs indiquées.',
      },
      scenery_invalid_entries: {
        title: 'Entrées de scènes non valides',
        description:
          'Des entrées actives dans scenery_packs.ini n’ont pas de chemin. Vérifiez les lignes indiquées.',
      },
      scenery_registration: {
        title: 'Enregistrement des scènes',
        description: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      },
      scenery_missing_paths: {
        title: 'Chemins de scènes absents',
        description: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      },
      scenery_duplicate_paths: {
        title: 'Scènes en double',
        description: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      },
      scenery_broken_links: {
        title: 'Liens de scènes rompus',
        description: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      },
      navdata_empty_files: {
        title: 'Fichiers de navigation vides',
        description: 'Intégrité et cohérence des données de navigation vérifiées.',
      },
      addons_binary_integrity: {
        title: 'Fichiers et architecture des extensions',
        description:
          'Un fichier de plugin sélectionné pour cette plateforme est vide ou possède un en-tête invalide ou non pris en charge. Consultez le fichier et la raison dans les détails.',
        scope:
          'Le contrôle examine les en-têtes des plugins globaux, des avions et des scènes enregistrées et activées. Les dépendances et le chargement réel nécessitent une vérification distincte.',
        outdated:
          'Ce résultat provient de l’ancienne vérification des plugins. Lancez une nouvelle analyse complète.',
      },
      installation_scan_access: {
        title: 'Accès et limites du contrôle',
        description: 'Dossiers et accès aux fichiers vérifiés.',
      },
      stability_log_freshness: {
        title: 'Ancienneté du journal',
        description: 'Journal de session et indices de plantage examinés.',
      },
      updates_gateway_unavailable: {
        title: 'Aéroports Gateway non vérifiés',
        description: 'Versions installées comparées aux mises à jour disponibles.',
      },
    },
    purposes: {
      installation_structure: 'Dossiers et accès aux fichiers vérifiés.',
      installation_xplane_running: 'Dossiers et accès aux fichiers vérifiés.',
      installation_location: 'Dossiers et accès aux fichiers vérifiés.',
      installation_readonly: 'Dossiers et accès aux fichiers vérifiés.',
      storage_xplane_space: 'Capacité du disque et fichiers nettoyables vérifiés.',
      storage_app_data_space: 'Capacité du disque et fichiers nettoyables vérifiés.',
      system_inventory: 'Système et environnement graphique examinés.',
      system_injectors: 'Système et environnement graphique examinés.',
      performance_available_memory: 'Ressources système disponibles vérifiées.',
      xfast_app_data: 'Données locales, base et index des scènes vérifiés.',
      xfast_database: 'Données locales, base et index des scènes vérifiés.',
      xfast_scenery_index: 'Données locales, base et index des scènes vérifiés.',
      stability_log_available: 'Journal de session et indices de plantage examinés.',
      stability_crash_dump: 'Journal de session et indices de plantage examinés.',
      system_beta_build: 'Système et environnement graphique examinés.',
      scenery_global_airports: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      scenery_load_order: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      scenery_dependencies: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      scenery_overlaps: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      scenery_duplicate_airports: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      scenery_flatten: 'Scènes, dépendances et cohérence du chargement vérifiées.',
      navdata_custom_data: 'Intégrité et cohérence des données de navigation vérifiées.',
      navdata_cifp: 'Intégrité et cohérence des données de navigation vérifiées.',
      navdata_integrity: 'Intégrité et cohérence des données de navigation vérifiées.',
      navdata_consistency: 'Intégrité et cohérence des données de navigation vérifiées.',
      xfast_recent_failures: 'Données locales, base et index des scènes vérifiés.',
      storage_cleanable: 'Capacité du disque et fichiers nettoyables vérifiés.',
      addons_inventory: 'Extensions et compatibilité vérifiées.',
      addons_platform: 'Extensions et compatibilité vérifiées.',
      updates_addons: 'Versions installées comparées aux mises à jour disponibles.',
      updates_app: 'Versions installées comparées aux mises à jour disponibles.',
      updates_csl: 'Versions installées comparées aux mises à jour disponibles.',
    },
    sessions: {
      crash:
        'Log.txt contient un marqueur explicite de plantage, sans établir à lui seul la cause.',
      normal: 'Log.txt indique un arrêt normal de X-Plane.',
      unknown:
        'Aucun marqueur explicite de plantage ou de fermeture normale. La fin de session reste indéterminée.',
      running: 'X-Plane fonctionne et le journal est encore en cours de rédaction.',
    },
  },
  es: {
    coverage: {
      addonUpdates:
        '{checked} de {installed} complementos compatibles comparados. Actualizaciones: {count}. Actualizador externo: {external}.',
      navFuture:
        'Algunos paquetes de navegación instalados aún no están vigentes. Revisa las fechas indicadas.',
    },
    fields: {
      count: 'Elementos afectados',
      installed: 'Aeropuertos instalados',
      checked: 'Elementos revisados',
      free: 'Espacio disponible',
      total: 'Capacidad total',
      available: 'Memoria disponible',
      line: 'Línea del registro',
      days: 'Edad del registro (días)',
      errors: 'Errores de lectura',
      indexed: 'Entradas indexadas',
      found: 'Entradas en disco',
      files: 'Archivos',
      size: 'Espacio utilizado',
      platform: 'Plataforma',
      version: 'Versión',
      expired: 'Paquetes vencidos',
      expiring: 'Paquetes próximos a vencer',
      unknown: 'Validez desconocida',
      cycles: 'Ciclos AIRAC detectados',
      aircraft: 'Aviones',
      plugins: 'Complementos',
      csl: 'Modelos X-CSL',
      altitude: 'Datos de altura',
      cause: 'Causa sospechada',
      confidence: 'Puntuación de indicios',
      module: 'Módulo relacionado',
      os: 'Sistema operativo',
      arch: 'Arquitectura',
      cores: 'Procesadores lógicos',
      memory: 'Memoria instalada',
      capped: 'Límite alcanzado',
      sessionEnd: 'Fin de sesión',
      steam: 'Instalación Steam',
    },
    scope: {
      installation: 'Se revisaron las carpetas y el acceso a archivos.',
      stability: 'Se revisaron el registro de sesión y los indicios de fallos.',
      scenery: 'Se revisaron escenarios, dependencias y carga.',
      addons: 'Se revisaron los complementos y su compatibilidad.',
      navdata: 'Se revisaron la integridad y coherencia de los datos de navegación.',
      performance: 'Se revisaron los recursos disponibles.',
      storage: 'Se revisaron el espacio y los archivos limpiables.',
      system: 'Se revisaron el sistema y el entorno gráfico.',
      xfast: 'Se revisaron datos locales, base e índice de escenarios.',
      updates: 'Se compararon las versiones con las actualizaciones disponibles.',
    },
    nav: {
      expired: 'Algunos paquetes de navegación han vencido. Actualiza las fuentes indicadas.',
      expiring: 'Algunos paquetes vencen pronto. Programa una actualización.',
      unknown: 'No se pudo determinar la validez. Revisa el ciclo y la fuente.',
      current: 'Los paquetes con validez conocida están vigentes.',
    },
    yes: 'Sí',
    no: 'No',
    findings: {
      scenery_files: {
        title: 'Archivos de escenarios activos',
        description:
          'Algunos paquetes activos contienen archivos ilegibles, vacíos o no válidos. Revisa los archivos indicados.',
      },
      installation_write_access: {
        title: 'Acceso de escritura a la instalación',
        description:
          'No se pueden crear archivos en las carpetas indicadas. Revisa los permisos y el volumen.',
      },
      installation_permission_access: {
        title: 'Comprobación de permisos incompleta',
        description: 'No se pudieron verificar algunos permisos. Revisa los errores.',
      },
      system_vulkan_runtime: {
        title: 'Entorno de Vulkan',
        description:
          'Falta el cargador de Vulkan o es anterior a 1.3. Instala o actualiza el controlador gráfico.',
      },
      installation_structure: {
        title: 'Instalación de X-Plane',
        description:
          'Restaura los archivos de inicio indicados con el instalador de X-Plane o la verificación de Steam.',
      },
      installation_stock_aircraft: {
        title: 'Aviones incluidos',
        description: 'Faltan los archivos indicados. Restaura los aviones afectados.',
      },
      installation_airfoils: {
        title: 'Perfiles aerodinámicos',
        description: 'Los perfiles indicados faltan o están dañados. Restaura esos archivos.',
      },
      installation_asset_access: {
        title: 'Acceso a archivos de instalación',
        description: 'No se pudieron verificar algunos archivos. Revisa los errores indicados.',
      },
      navdata_bundled: {
        title: 'Datos de navegación incluidos',
        description: 'Repara la instalación de X-Plane para restaurar los archivos indicados.',
      },
      navdata_supplemental: {
        title: 'Datos de navegación adicionales',
        description: 'Revisa los archivos ARINC 424, FAA o de usuario indicados.',
      },
      navdata_scan_access: {
        title: 'Acceso a datos de navegación',
        description:
          'No se pudieron comprobar algunos datos o metadatos de ciclo. Revisa los errores indicados.',
      },
      scenery_invalid_entries: {
        title: 'Registros de escenarios no válidos',
        description:
          'Algunas entradas activas de scenery_packs.ini no tienen ruta. Revisa las líneas indicadas.',
      },
      scenery_registration: {
        title: 'Registro de escenarios',
        description: 'Se revisaron escenarios, dependencias y carga.',
      },
      scenery_missing_paths: {
        title: 'Rutas de escenarios ausentes',
        description: 'Se revisaron escenarios, dependencias y carga.',
      },
      scenery_duplicate_paths: {
        title: 'Registros duplicados',
        description: 'Se revisaron escenarios, dependencias y carga.',
      },
      scenery_broken_links: {
        title: 'Enlaces de escenarios rotos',
        description: 'Se revisaron escenarios, dependencias y carga.',
      },
      navdata_empty_files: {
        title: 'Archivos de navegación vacíos',
        description: 'Se revisaron la integridad y coherencia de los datos de navegación.',
      },
      addons_binary_integrity: {
        title: 'Archivos y arquitectura de complementos',
        description:
          'Un archivo de complemento seleccionado para esta plataforma está vacío o tiene una cabecera inválida o incompatible. Consulta el archivo y el motivo en los detalles.',
        scope:
          'Se revisan las cabeceras de los complementos globales, de aeronaves y de escenarios registrados y activados. Las dependencias y la carga real requieren otra comprobación.',
        outdated:
          'Este resultado procede de la revisión anterior. Ejecuta un nuevo análisis completo.',
      },
      installation_scan_access: {
        title: 'Acceso y límites de revisión',
        description: 'Se revisaron las carpetas y el acceso a archivos.',
      },
      stability_log_freshness: {
        title: 'Antigüedad del registro',
        description: 'Se revisaron el registro de sesión y los indicios de fallos.',
      },
      updates_gateway_unavailable: {
        title: 'Aeropuertos Gateway sin revisar',
        description: 'Se compararon las versiones con las actualizaciones disponibles.',
      },
    },
    purposes: {
      installation_structure: 'Se revisaron las carpetas y el acceso a archivos.',
      installation_xplane_running: 'Se revisaron las carpetas y el acceso a archivos.',
      installation_location: 'Se revisaron las carpetas y el acceso a archivos.',
      installation_readonly: 'Se revisaron las carpetas y el acceso a archivos.',
      storage_xplane_space: 'Se revisaron el espacio y los archivos limpiables.',
      storage_app_data_space: 'Se revisaron el espacio y los archivos limpiables.',
      system_inventory: 'Se revisaron el sistema y el entorno gráfico.',
      system_injectors: 'Se revisaron el sistema y el entorno gráfico.',
      performance_available_memory: 'Se revisaron los recursos disponibles.',
      xfast_app_data: 'Se revisaron datos locales, base e índice de escenarios.',
      xfast_database: 'Se revisaron datos locales, base e índice de escenarios.',
      xfast_scenery_index: 'Se revisaron datos locales, base e índice de escenarios.',
      stability_log_available: 'Se revisaron el registro de sesión y los indicios de fallos.',
      stability_crash_dump: 'Se revisaron el registro de sesión y los indicios de fallos.',
      system_beta_build: 'Se revisaron el sistema y el entorno gráfico.',
      scenery_global_airports: 'Se revisaron escenarios, dependencias y carga.',
      scenery_load_order: 'Se revisaron escenarios, dependencias y carga.',
      scenery_dependencies: 'Se revisaron escenarios, dependencias y carga.',
      scenery_overlaps: 'Se revisaron escenarios, dependencias y carga.',
      scenery_duplicate_airports: 'Se revisaron escenarios, dependencias y carga.',
      scenery_flatten: 'Se revisaron escenarios, dependencias y carga.',
      navdata_custom_data: 'Se revisaron la integridad y coherencia de los datos de navegación.',
      navdata_cifp: 'Se revisaron la integridad y coherencia de los datos de navegación.',
      navdata_integrity: 'Se revisaron la integridad y coherencia de los datos de navegación.',
      navdata_consistency: 'Se revisaron la integridad y coherencia de los datos de navegación.',
      xfast_recent_failures: 'Se revisaron datos locales, base e índice de escenarios.',
      storage_cleanable: 'Se revisaron el espacio y los archivos limpiables.',
      addons_inventory: 'Se revisaron los complementos y su compatibilidad.',
      addons_platform: 'Se revisaron los complementos y su compatibilidad.',
      updates_addons: 'Se compararon las versiones con las actualizaciones disponibles.',
      updates_app: 'Se compararon las versiones con las actualizaciones disponibles.',
      updates_csl: 'Se compararon las versiones con las actualizaciones disponibles.',
    },
    sessions: {
      crash: 'Log.txt contiene una marca explícita de fallo; no basta para establecer la causa.',
      normal: 'Log.txt indica un cierre normal de X-Plane.',
      unknown:
        'No hay una marca explícita de fallo o cierre normal. No se puede determinar cómo terminó la sesión.',
      running: 'X-Plane está ejecutándose y sigue escribiendo el registro.',
    },
  },
  pt: {
    coverage: {
      addonUpdates:
        '{checked} de {installed} extensões suportadas comparadas. Atualizações: {count}. Atualizador externo: {external}.',
      navFuture:
        'Alguns pacotes de navegação instalados ainda não estão em vigor. Verifique as datas indicadas.',
    },
    fields: {
      count: 'Itens afetados',
      installed: 'Aeroportos instalados',
      checked: 'Itens verificados',
      free: 'Espaço disponível',
      total: 'Capacidade total',
      available: 'Memória disponível',
      line: 'Linha do registro',
      days: 'Idade do registro (dias)',
      errors: 'Erros de leitura',
      indexed: 'Entradas indexadas',
      found: 'Entradas no disco',
      files: 'Arquivos',
      size: 'Espaço usado',
      platform: 'Plataforma',
      version: 'Versão',
      expired: 'Pacotes expirados',
      expiring: 'Pacotes prestes a expirar',
      unknown: 'Validade desconhecida',
      cycles: 'Ciclos AIRAC encontrados',
      aircraft: 'Aeronaves',
      plugins: 'Plugins',
      csl: 'Modelos X-CSL',
      altitude: 'Dados de altura',
      cause: 'Causa suspeita',
      confidence: 'Pontuação dos indícios',
      module: 'Módulo relacionado',
      os: 'Sistema operacional',
      arch: 'Arquitetura',
      cores: 'Processadores lógicos',
      memory: 'Memória instalada',
      capped: 'Limite atingido',
      sessionEnd: 'Fim da sessão',
      steam: 'Instalação Steam',
    },
    scope: {
      installation: 'Pastas e acesso aos arquivos verificados.',
      stability: 'Registro da sessão e indícios de falha examinados.',
      scenery: 'Cenários, dependências e carregamento verificados.',
      addons: 'Addons e compatibilidade verificados.',
      navdata: 'Integridade e consistência dos dados de navegação verificadas.',
      performance: 'Recursos disponíveis verificados.',
      storage: 'Espaço e arquivos removíveis verificados.',
      system: 'Sistema e ambiente gráfico examinados.',
      xfast: 'Dados locais, banco e índice de cenários verificados.',
      updates: 'Versões comparadas com atualizações disponíveis.',
    },
    nav: {
      expired: 'Alguns pacotes de navegação expiraram. Atualize as fontes indicadas.',
      expiring: 'Alguns pacotes expiram em breve. Planeje a atualização.',
      unknown: 'Não foi possível determinar a validade. Verifique ciclo e origem.',
      current: 'Os pacotes com validade conhecida estão atuais.',
    },
    yes: 'Sim',
    no: 'Não',
    findings: {
      scenery_files: {
        title: 'Ficheiros de cenários ativos',
        description:
          'Alguns pacotes ativos contêm ficheiros ilegíveis, vazios ou inválidos. Verifique os ficheiros indicados.',
      },
      installation_write_access: {
        title: 'Acesso de escrita à instalação',
        description:
          'Não é possível criar ficheiros nas pastas indicadas. Verifique as permissões e o volume.',
      },
      installation_permission_access: {
        title: 'Verificação de permissões incompleta',
        description: 'Não foi possível verificar algumas permissões. Consulte os erros.',
      },
      system_vulkan_runtime: {
        title: 'Ambiente Vulkan',
        description:
          'O carregador Vulkan está ausente ou é anterior a 1.3. Instale ou atualize o controlador gráfico.',
      },
      installation_structure: {
        title: 'Instalação do X-Plane',
        description:
          'Recupere os ficheiros de arranque indicados com o instalador do X-Plane ou a verificação do Steam.',
      },
      installation_stock_aircraft: {
        title: 'Aeronaves incluídas',
        description: 'Os ficheiros indicados estão em falta. Recupere as aeronaves afetadas.',
      },
      installation_airfoils: {
        title: 'Perfis aerodinâmicos',
        description:
          'Os perfis indicados estão ausentes ou danificados. Recupere os ficheiros afetados.',
      },
      installation_asset_access: {
        title: 'Acesso aos ficheiros do X-Plane',
        description: 'Não foi possível verificar alguns ficheiros. Consulte os erros indicados.',
      },
      navdata_bundled: {
        title: 'Dados de navegação incluídos',
        description: 'Repare a instalação do X-Plane para recuperar os ficheiros indicados.',
      },
      navdata_supplemental: {
        title: 'Dados de navegação complementares',
        description: 'Verifique os ficheiros ARINC 424, FAA ou do utilizador indicados.',
      },
      navdata_scan_access: {
        title: 'Acesso aos dados de navegação',
        description:
          'Não foi possível verificar alguns dados ou metadados de ciclo. Consulte os erros indicados.',
      },
      scenery_invalid_entries: {
        title: 'Registos de cenários inválidos',
        description:
          'Algumas entradas ativas em scenery_packs.ini não têm caminho. Verifique as linhas indicadas.',
      },
      scenery_registration: {
        title: 'Registro de cenários',
        description: 'Cenários, dependências e carregamento verificados.',
      },
      scenery_missing_paths: {
        title: 'Pastas de cenários ausentes',
        description: 'Cenários, dependências e carregamento verificados.',
      },
      scenery_duplicate_paths: {
        title: 'Registros duplicados',
        description: 'Cenários, dependências e carregamento verificados.',
      },
      scenery_broken_links: {
        title: 'Links de cenários quebrados',
        description: 'Cenários, dependências e carregamento verificados.',
      },
      navdata_empty_files: {
        title: 'Arquivos de navegação vazios',
        description: 'Integridade e consistência dos dados de navegação verificadas.',
      },
      addons_binary_integrity: {
        title: 'Arquivos e arquitetura dos plugins',
        description:
          'Um arquivo de plugin selecionado para esta plataforma está vazio ou tem um cabeçalho inválido ou incompatível. Consulte o arquivo e o motivo nos detalhes.',
        scope:
          'A verificação abrange os cabeçalhos dos plugins globais, de aeronaves e dos cenários registrados e ativados. As dependências e o carregamento real exigem uma verificação separada.',
        outdated: 'Este resultado é da verificação anterior. Execute uma nova análise completa.',
      },
      installation_scan_access: {
        title: 'Acesso e limites da verificação',
        description: 'Pastas e acesso aos arquivos verificados.',
      },
      stability_log_freshness: {
        title: 'Idade do registro',
        description: 'Registro da sessão e indícios de falha examinados.',
      },
      updates_gateway_unavailable: {
        title: 'Aeroportos Gateway não verificados',
        description: 'Versões comparadas com atualizações disponíveis.',
      },
    },
    purposes: {
      installation_structure: 'Pastas e acesso aos arquivos verificados.',
      installation_xplane_running: 'Pastas e acesso aos arquivos verificados.',
      installation_location: 'Pastas e acesso aos arquivos verificados.',
      installation_readonly: 'Pastas e acesso aos arquivos verificados.',
      storage_xplane_space: 'Espaço e arquivos removíveis verificados.',
      storage_app_data_space: 'Espaço e arquivos removíveis verificados.',
      system_inventory: 'Sistema e ambiente gráfico examinados.',
      system_injectors: 'Sistema e ambiente gráfico examinados.',
      performance_available_memory: 'Recursos disponíveis verificados.',
      xfast_app_data: 'Dados locais, banco e índice de cenários verificados.',
      xfast_database: 'Dados locais, banco e índice de cenários verificados.',
      xfast_scenery_index: 'Dados locais, banco e índice de cenários verificados.',
      stability_log_available: 'Registro da sessão e indícios de falha examinados.',
      stability_crash_dump: 'Registro da sessão e indícios de falha examinados.',
      system_beta_build: 'Sistema e ambiente gráfico examinados.',
      scenery_global_airports: 'Cenários, dependências e carregamento verificados.',
      scenery_load_order: 'Cenários, dependências e carregamento verificados.',
      scenery_dependencies: 'Cenários, dependências e carregamento verificados.',
      scenery_overlaps: 'Cenários, dependências e carregamento verificados.',
      scenery_duplicate_airports: 'Cenários, dependências e carregamento verificados.',
      scenery_flatten: 'Cenários, dependências e carregamento verificados.',
      navdata_custom_data: 'Integridade e consistência dos dados de navegação verificadas.',
      navdata_cifp: 'Integridade e consistência dos dados de navegação verificadas.',
      navdata_integrity: 'Integridade e consistência dos dados de navegação verificadas.',
      navdata_consistency: 'Integridade e consistência dos dados de navegação verificadas.',
      xfast_recent_failures: 'Dados locais, banco e índice de cenários verificados.',
      storage_cleanable: 'Espaço e arquivos removíveis verificados.',
      addons_inventory: 'Addons e compatibilidade verificados.',
      addons_platform: 'Addons e compatibilidade verificados.',
      updates_addons: 'Versões comparadas com atualizações disponíveis.',
      updates_app: 'Versões comparadas com atualizações disponíveis.',
      updates_csl: 'Versões comparadas com atualizações disponíveis.',
    },
    sessions: {
      crash: 'Log.txt contém uma marca explícita de falha, mas não determina sozinho a causa.',
      normal: 'Log.txt registra um encerramento normal do X-Plane.',
      unknown:
        'Não há marca explícita de falha ou encerramento normal. Não é possível determinar o fim da sessão.',
      running: 'O X-Plane está em execução e o registro continua sendo escrito.',
    },
  },
  ja: {
    coverage: {
      addonUpdates:
        '対応アドオン {installed} 件中 {checked} 件を比較しました。更新 {count} 件、外部アップデーター {external} 件。',
      navFuture:
        '一部のナビゲーションパッケージはまだ有効ではありません。表示された日付を確認してください。',
    },
    fields: {
      count: '対象項目',
      installed: '導入済み空港',
      checked: '確認した項目',
      free: '空き容量',
      total: '総容量',
      available: '利用可能メモリ',
      line: 'ログ行',
      days: 'ログ経過日数',
      errors: '読み取りエラー',
      indexed: '索引の項目',
      found: 'ディスクの項目',
      files: 'ファイル数',
      size: '使用容量',
      platform: '実行環境',
      version: 'バージョン',
      expired: '期限切れのデータ',
      expiring: '期限が近いデータ',
      unknown: '有効期限不明',
      cycles: '検出した AIRAC',
      aircraft: '機体数',
      plugins: 'プラグイン数',
      csl: 'X-CSL モデル',
      altitude: '高度データ',
      cause: '推定原因',
      confidence: '証拠スコア',
      module: '関連モジュール',
      os: 'OS',
      arch: 'アーキテクチャ',
      cores: '論理プロセッサ',
      memory: '搭載メモリ',
      capped: '走査上限に到達',
      sessionEnd: 'セッションの終了',
      steam: 'Steam インストール',
    },
    scope: {
      installation: '設置フォルダーとファイルへのアクセスを確認しました。',
      stability: 'セッションログとクラッシュの証拠を確認しました。',
      scenery: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      addons: 'アドオンと互換性を確認しました。',
      navdata: '航法データの完全性と整合性を確認しました。',
      performance: '利用可能なリソースを確認しました。',
      storage: 'ディスク容量と削除可能なファイルを確認しました。',
      system: 'システムと描画環境を確認しました。',
      xfast: 'アプリデータ、データベース、索引を確認しました。',
      updates: '導入済みバージョンと更新情報を比較しました。',
    },
    nav: {
      expired: '期限切れの航法データがあります。記載されたデータソースを更新してください。',
      expiring: 'まもなく期限切れのデータがあります。更新を準備してください。',
      unknown: '有効期限を確認できないデータがあります。サイクルと提供元を確認してください。',
      current: '有効期限が確認できたデータはすべて有効です。',
    },
    yes: 'はい',
    no: 'いいえ',
    findings: {
      scenery_files: {
        title: '有効なシーナリーファイル',
        description:
          '有効なパッケージに読み取れない、空、または形式が無効なファイルがあります。表示された項目を確認してください。',
      },
      installation_write_access: {
        title: 'インストール先への書き込み',
        description:
          '表示されたフォルダーにファイルを作成できません。権限とドライブの状態を確認してください。',
      },
      installation_permission_access: {
        title: '権限の確認が未完了',
        description: '一部の権限を確認できませんでした。表示されたエラーを確認してください。',
      },
      system_vulkan_runtime: {
        title: 'Vulkan ランタイム',
        description:
          'Vulkan ローダーが存在しないか、1.3 未満です。グラフィックスドライバーをインストールまたは更新してください。',
      },
      installation_structure: {
        title: 'X-Plane インストール',
        description:
          'X-Plane インストーラーまたは Steam の整合性確認で起動ファイルを復元してください。',
      },
      installation_stock_aircraft: {
        title: '標準機体',
        description: '表示された機体ファイルが見つかりません。該当する標準機体を復元してください。',
      },
      installation_airfoils: {
        title: '翼型ファイル',
        description:
          '表示された翼型が欠落または破損しています。該当するファイルを復元してください。',
      },
      installation_asset_access: {
        title: 'インストールファイルへのアクセス',
        description: '一部のファイルを確認できませんでした。表示されたエラーを確認してください。',
      },
      navdata_bundled: {
        title: '標準ナビゲーションデータ',
        description: 'X-Plane のインストールを修復し、表示された標準ファイルを復元してください。',
      },
      navdata_supplemental: {
        title: '補足ナビゲーションデータ',
        description:
          '表示された ARINC 424、FAA またはユーザーのナビゲーションファイルを確認してください。',
      },
      navdata_scan_access: {
        title: 'ナビゲーションデータへのアクセス',
        description:
          '一部のデータまたはサイクル情報を確認できませんでした。表示されたエラーを確認してください。',
      },
      scenery_invalid_entries: {
        title: '無効なシーナリー登録',
        description:
          'scenery_packs.ini の有効な項目にパスがありません。表示された行を確認してください。',
      },
      scenery_registration: {
        title: 'シーナリー登録',
        description: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      },
      scenery_missing_paths: {
        title: '存在しないシーナリーパス',
        description: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      },
      scenery_duplicate_paths: {
        title: '重複したシーナリー登録',
        description: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      },
      scenery_broken_links: {
        title: '切れたシーナリーリンク',
        description: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      },
      navdata_empty_files: {
        title: '空の航法データ',
        description: '航法データの完全性と整合性を確認しました。',
      },
      addons_binary_integrity: {
        title: 'プラグインの形式とアーキテクチャ',
        description:
          'このプラットフォーム向けのプラグイン入口が空か、実行形式のヘッダーが無効または非対応です。詳細で対象ファイルと理由を確認してください。',
        scope:
          'グローバル、機体、および登録済みで有効なシーナリーのプラグインのヘッダーを確認しました。依存関係と実際の読み込みは別途確認が必要です。',
        outdated:
          '以前のプラグイン検査で保存された結果です。再度、完全スキャンを実行してください。',
      },
      installation_scan_access: {
        title: '走査のアクセスと制限',
        description: '設置フォルダーとファイルへのアクセスを確認しました。',
      },
      stability_log_freshness: {
        title: 'ログの鮮度',
        description: 'セッションログとクラッシュの証拠を確認しました。',
      },
      updates_gateway_unavailable: {
        title: '未確認の Gateway 空港',
        description: '導入済みバージョンと更新情報を比較しました。',
      },
    },
    purposes: {
      installation_structure: '設置フォルダーとファイルへのアクセスを確認しました。',
      installation_xplane_running: '設置フォルダーとファイルへのアクセスを確認しました。',
      installation_location: '設置フォルダーとファイルへのアクセスを確認しました。',
      installation_readonly: '設置フォルダーとファイルへのアクセスを確認しました。',
      storage_xplane_space: 'ディスク容量と削除可能なファイルを確認しました。',
      storage_app_data_space: 'ディスク容量と削除可能なファイルを確認しました。',
      system_inventory: 'システムと描画環境を確認しました。',
      system_injectors: 'システムと描画環境を確認しました。',
      performance_available_memory: '利用可能なリソースを確認しました。',
      xfast_app_data: 'アプリデータ、データベース、索引を確認しました。',
      xfast_database: 'アプリデータ、データベース、索引を確認しました。',
      xfast_scenery_index: 'アプリデータ、データベース、索引を確認しました。',
      stability_log_available: 'セッションログとクラッシュの証拠を確認しました。',
      stability_crash_dump: 'セッションログとクラッシュの証拠を確認しました。',
      system_beta_build: 'システムと描画環境を確認しました。',
      scenery_global_airports: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      scenery_load_order: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      scenery_dependencies: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      scenery_overlaps: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      scenery_duplicate_airports: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      scenery_flatten: 'シーナリー登録、依存関係、読み込みの整合性を確認しました。',
      navdata_custom_data: '航法データの完全性と整合性を確認しました。',
      navdata_cifp: '航法データの完全性と整合性を確認しました。',
      navdata_integrity: '航法データの完全性と整合性を確認しました。',
      navdata_consistency: '航法データの完全性と整合性を確認しました。',
      xfast_recent_failures: 'アプリデータ、データベース、索引を確認しました。',
      storage_cleanable: 'ディスク容量と削除可能なファイルを確認しました。',
      addons_inventory: 'アドオンと互換性を確認しました。',
      addons_platform: 'アドオンと互換性を確認しました。',
      updates_addons: '導入済みバージョンと更新情報を比較しました。',
      updates_app: '導入済みバージョンと更新情報を比較しました。',
      updates_csl: '導入済みバージョンと更新情報を比較しました。',
    },
    sessions: {
      crash:
        'Log.txt に明確なクラッシュ記録があります。ただしログだけでは根本原因を断定できません。',
      normal: 'Log.txt に X-Plane の正常終了が記録されています。',
      unknown: 'クラッシュや正常終了の明確な記録がなく、終了状態を判断できません。',
      running: 'X-Plane は動作中で、ログはまだ書き込み中です。',
    },
  },
  ko: {
    coverage: {
      addonUpdates:
        '지원 애드온 {installed}개 중 {checked}개 비교. 업데이트 {count}개, 외부 업데이트 도구 {external}개.',
      navFuture: '설치된 일부 항법 패키지가 아직 유효하지 않습니다. 표시된 날짜를 확인하세요.',
    },
    fields: {
      count: '대상 항목',
      installed: '설치된 공항',
      checked: '확인한 항목',
      free: '여유 공간',
      total: '전체 용량',
      available: '사용 가능한 메모리',
      line: '로그 줄',
      days: '로그 경과 일수',
      errors: '읽기 오류',
      indexed: '색인 항목',
      found: '디스크 항목',
      files: '파일 수',
      size: '사용 공간',
      platform: '실행 환경',
      version: '버전',
      expired: '만료된 데이터',
      expiring: '곧 만료되는 데이터',
      unknown: '유효 기간 불명',
      cycles: '감지된 AIRAC 주기',
      aircraft: '기체 수',
      plugins: '플러그인 수',
      csl: 'X-CSL 모델',
      altitude: '고도 데이터',
      cause: '추정 원인',
      confidence: '증거 점수',
      module: '관련 모듈',
      os: '운영 체제',
      arch: '아키텍처',
      cores: '논리 프로세서',
      memory: '설치된 메모리',
      capped: '검사 한도 도달',
      sessionEnd: '세션 종료',
      steam: 'Steam 설치',
    },
    scope: {
      installation: '설치 폴더와 파일 접근을 확인했습니다.',
      stability: '세션 로그와 충돌 증거를 확인했습니다.',
      scenery: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      addons: '애드온과 호환성을 확인했습니다.',
      navdata: '항법 데이터의 무결성과 일관성을 확인했습니다.',
      performance: '사용 가능한 리소스를 확인했습니다.',
      storage: '디스크 공간과 정리 가능한 파일을 확인했습니다.',
      system: '시스템과 그래픽 환경을 확인했습니다.',
      xfast: '앱 데이터, 데이터베이스와 색인을 확인했습니다.',
      updates: '설치 버전과 업데이트 정보를 비교했습니다.',
    },
    nav: {
      expired: '만료된 항법 데이터가 있습니다. 표시된 데이터 출처를 업데이트하세요.',
      expiring: '일부 데이터가 곧 만료됩니다. 업데이트를 준비하세요.',
      unknown: '유효 기간을 확인할 수 없는 데이터가 있습니다. 주기와 출처를 확인하세요.',
      current: '유효 기간이 확인된 데이터는 모두 유효합니다.',
    },
    yes: '예',
    no: '아니요',
    findings: {
      scenery_files: {
        title: '활성 시너리 파일',
        description:
          '활성 패키지에 읽을 수 없거나 비어 있거나 잘못된 파일이 있습니다. 표시된 파일을 확인하세요.',
      },
      installation_write_access: {
        title: '설치 폴더 쓰기 권한',
        description: '표시된 폴더에 파일을 만들 수 없습니다. 권한과 볼륨 상태를 확인하세요.',
      },
      installation_permission_access: {
        title: '권한 확인 미완료',
        description: '일부 권한을 확인할 수 없습니다. 표시된 오류를 확인하세요.',
      },
      system_vulkan_runtime: {
        title: 'Vulkan 런타임',
        description:
          'Vulkan 로더가 없거나 1.3 미만입니다. 그래픽 드라이버를 설치하거나 업데이트하세요.',
      },
      installation_structure: {
        title: 'X-Plane 설치 구조',
        description:
          'X-Plane 설치 프로그램 또는 Steam 파일 확인으로 표시된 시작 파일을 복구하세요.',
      },
      installation_stock_aircraft: {
        title: '기본 항공기',
        description: '표시된 항공기 파일이 누락되었습니다. 해당 기본 항공기를 복구하세요.',
      },
      installation_airfoils: {
        title: '익형 파일',
        description: '표시된 익형 파일이 없거나 손상되었습니다. 해당 파일을 복구하세요.',
      },
      installation_asset_access: {
        title: '설치 파일 접근',
        description: '일부 파일을 확인할 수 없습니다. 표시된 오류를 확인하세요.',
      },
      navdata_bundled: {
        title: '기본 항법 데이터',
        description: 'X-Plane 설치를 복구하여 표시된 기본 파일을 복원하세요.',
      },
      navdata_supplemental: {
        title: '추가 항법 데이터',
        description: '표시된 ARINC 424, FAA 또는 사용자 항법 파일을 확인하세요.',
      },
      navdata_scan_access: {
        title: '항법 데이터 접근',
        description:
          '일부 항법 데이터 또는 주기 정보를 확인할 수 없습니다. 표시된 오류를 확인하세요.',
      },
      scenery_invalid_entries: {
        title: '잘못된 시너리 등록',
        description: 'scenery_packs.ini의 활성 항목에 경로가 없습니다. 표시된 줄을 확인하세요.',
      },
      scenery_registration: {
        title: '시너리 등록',
        description: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      },
      scenery_missing_paths: {
        title: '없는 시너리 경로',
        description: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      },
      scenery_duplicate_paths: {
        title: '중복 시너리 등록',
        description: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      },
      scenery_broken_links: {
        title: '끊어진 시너리 링크',
        description: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      },
      navdata_empty_files: {
        title: '빈 항법 데이터 파일',
        description: '항법 데이터의 무결성과 일관성을 확인했습니다.',
      },
      addons_binary_integrity: {
        title: '플러그인 파일과 아키텍처',
        description:
          '이 플랫폼용으로 선택된 플러그인 파일이 비어 있거나 실행 파일 헤더가 잘못되었거나 지원되지 않습니다. 세부 정보에서 파일과 원인을 확인하세요.',
        scope:
          '전역, 항공기 및 등록되어 활성화된 시너리 플러그인의 헤더를 검사했습니다. 종속성과 실제 로드는 별도 확인이 필요합니다.',
        outdated: '이 결과는 이전 플러그인 검사에서 저장되었습니다. 전체 검사를 다시 실행하세요.',
      },
      installation_scan_access: {
        title: '검사 접근 및 한도',
        description: '설치 폴더와 파일 접근을 확인했습니다.',
      },
      stability_log_freshness: {
        title: '로그 최신성',
        description: '세션 로그와 충돌 증거를 확인했습니다.',
      },
      updates_gateway_unavailable: {
        title: '미확인 Gateway 공항',
        description: '설치 버전과 업데이트 정보를 비교했습니다.',
      },
    },
    purposes: {
      installation_structure: '설치 폴더와 파일 접근을 확인했습니다.',
      installation_xplane_running: '설치 폴더와 파일 접근을 확인했습니다.',
      installation_location: '설치 폴더와 파일 접근을 확인했습니다.',
      installation_readonly: '설치 폴더와 파일 접근을 확인했습니다.',
      storage_xplane_space: '디스크 공간과 정리 가능한 파일을 확인했습니다.',
      storage_app_data_space: '디스크 공간과 정리 가능한 파일을 확인했습니다.',
      system_inventory: '시스템과 그래픽 환경을 확인했습니다.',
      system_injectors: '시스템과 그래픽 환경을 확인했습니다.',
      performance_available_memory: '사용 가능한 리소스를 확인했습니다.',
      xfast_app_data: '앱 데이터, 데이터베이스와 색인을 확인했습니다.',
      xfast_database: '앱 데이터, 데이터베이스와 색인을 확인했습니다.',
      xfast_scenery_index: '앱 데이터, 데이터베이스와 색인을 확인했습니다.',
      stability_log_available: '세션 로그와 충돌 증거를 확인했습니다.',
      stability_crash_dump: '세션 로그와 충돌 증거를 확인했습니다.',
      system_beta_build: '시스템과 그래픽 환경을 확인했습니다.',
      scenery_global_airports: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      scenery_load_order: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      scenery_dependencies: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      scenery_overlaps: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      scenery_duplicate_airports: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      scenery_flatten: '시너리 등록, 종속성 및 로딩 일관성을 확인했습니다.',
      navdata_custom_data: '항법 데이터의 무결성과 일관성을 확인했습니다.',
      navdata_cifp: '항법 데이터의 무결성과 일관성을 확인했습니다.',
      navdata_integrity: '항법 데이터의 무결성과 일관성을 확인했습니다.',
      navdata_consistency: '항법 데이터의 무결성과 일관성을 확인했습니다.',
      xfast_recent_failures: '앱 데이터, 데이터베이스와 색인을 확인했습니다.',
      storage_cleanable: '디스크 공간과 정리 가능한 파일을 확인했습니다.',
      addons_inventory: '애드온과 호환성을 확인했습니다.',
      addons_platform: '애드온과 호환성을 확인했습니다.',
      updates_addons: '설치 버전과 업데이트 정보를 비교했습니다.',
      updates_app: '설치 버전과 업데이트 정보를 비교했습니다.',
      updates_csl: '설치 버전과 업데이트 정보를 비교했습니다.',
    },
    sessions: {
      crash: 'Log.txt에 명확한 충돌 표식이 있습니다. 로그만으로 원인을 단정할 수는 없습니다.',
      normal: 'Log.txt에 X-Plane의 정상 종료가 기록되어 있습니다.',
      unknown: '명확한 충돌 또는 정상 종료 표식이 없어 종료 상태를 판단할 수 없습니다.',
      running: 'X-Plane이 실행 중이며 로그가 계속 기록되고 있습니다.',
    },
  },
  ru: {
    coverage: {
      addonUpdates:
        'Сопоставлено {checked} из {installed} поддерживаемых дополнений. Обновления: {count}. Внешняя программа: {external}.',
      navFuture:
        'Некоторые установленные навигационные пакеты ещё не вступили в силу. Проверьте указанные даты.',
    },
    fields: {
      count: 'Затронутые элементы',
      installed: 'Установленные аэропорты',
      checked: 'Проверено элементов',
      free: 'Свободное место',
      total: 'Общая ёмкость',
      available: 'Доступная память',
      line: 'Строка журнала',
      days: 'Возраст журнала (дни)',
      errors: 'Ошибки чтения',
      indexed: 'Записи индекса',
      found: 'Записи на диске',
      files: 'Файлы',
      size: 'Занятое место',
      platform: 'Платформа',
      version: 'Версия',
      expired: 'Просроченные пакеты',
      expiring: 'Скоро истекающие пакеты',
      unknown: 'Срок неизвестен',
      cycles: 'Обнаруженные циклы AIRAC',
      aircraft: 'Самолёты',
      plugins: 'Плагины',
      csl: 'Модели X-CSL',
      altitude: 'Данные высоты',
      cause: 'Предполагаемая причина',
      confidence: 'Оценка признаков',
      module: 'Связанный модуль',
      os: 'Операционная система',
      arch: 'Архитектура',
      cores: 'Логические процессоры',
      memory: 'Установленная память',
      capped: 'Достигнут предел',
      sessionEnd: 'Завершение сеанса',
      steam: 'Установка Steam',
    },
    scope: {
      installation: 'Проверены папки и доступ к файлам.',
      stability: 'Изучены журнал сеанса и признаки сбоев.',
      scenery: 'Проверены сценарии, зависимости и загрузка.',
      addons: 'Проверены дополнения и совместимость.',
      navdata: 'Проверены целостность и согласованность навигационных данных.',
      performance: 'Проверены доступные ресурсы.',
      storage: 'Проверены место на диске и очищаемые файлы.',
      system: 'Проверены система и графическая среда.',
      xfast: 'Проверены данные приложения, база и индекс.',
      updates: 'Версии сопоставлены с доступными обновлениями.',
    },
    nav: {
      expired: 'Некоторые навигационные пакеты устарели. Обновите указанные источники.',
      expiring: 'Срок некоторых пакетов скоро истечёт. Запланируйте обновление.',
      unknown: 'Срок действия части пакетов неизвестен. Проверьте цикл и источник.',
      current: 'Все пакеты с известным сроком действуют.',
    },
    yes: 'Да',
    no: 'Нет',
    findings: {
      scenery_files: {
        title: 'Файлы активных сценариев',
        description:
          'Некоторые активные пакеты содержат нечитаемые, пустые или некорректные файлы. Проверьте список.',
      },
      installation_write_access: {
        title: 'Доступ на запись в папки установки',
        description:
          'В указанных папках нельзя создать файлы. Проверьте разрешения и состояние диска.',
      },
      installation_permission_access: {
        title: 'Проверка разрешений не завершена',
        description: 'Не все разрешения удалось проверить. См. указанные ошибки.',
      },
      system_vulkan_runtime: {
        title: 'Среда выполнения Vulkan',
        description:
          'Загрузчик Vulkan отсутствует или старше версии 1.3. Установите или обновите видеодрайвер.',
      },
      installation_structure: {
        title: 'Установка X-Plane',
        description:
          'Восстановите указанные файлы запуска с помощью установщика X-Plane или проверки файлов Steam.',
      },
      installation_stock_aircraft: {
        title: 'Штатные самолёты',
        description: 'Указанные файлы отсутствуют. Восстановите соответствующие самолёты.',
      },
      installation_airfoils: {
        title: 'Файлы профилей крыла',
        description: 'Указанные файлы отсутствуют или повреждены. Восстановите их.',
      },
      installation_asset_access: {
        title: 'Доступ к файлам установки',
        description: 'Не удалось проверить часть файлов. См. указанные ошибки.',
      },
      navdata_bundled: {
        title: 'Встроенные навигационные данные',
        description: 'Восстановите установку X-Plane, чтобы вернуть указанные файлы.',
      },
      navdata_supplemental: {
        title: 'Дополнительные навигационные данные',
        description: 'Проверьте указанные файлы ARINC 424, FAA или пользователя.',
      },
      navdata_scan_access: {
        title: 'Доступ к навигационным данным',
        description:
          'Часть данных или сведений о циклах не удалось проверить. См. указанные ошибки.',
      },
      scenery_invalid_entries: {
        title: 'Недопустимые записи сценариев',
        description:
          'В активных записях scenery_packs.ini отсутствуют пути. Проверьте указанные строки.',
      },
      scenery_registration: {
        title: 'Регистрация сценариев',
        description: 'Проверены сценарии, зависимости и загрузка.',
      },
      scenery_missing_paths: {
        title: 'Отсутствующие пути сценариев',
        description: 'Проверены сценарии, зависимости и загрузка.',
      },
      scenery_duplicate_paths: {
        title: 'Повторные записи сценариев',
        description: 'Проверены сценарии, зависимости и загрузка.',
      },
      scenery_broken_links: {
        title: 'Повреждённые ссылки сценариев',
        description: 'Проверены сценарии, зависимости и загрузка.',
      },
      navdata_empty_files: {
        title: 'Пустые навигационные файлы',
        description: 'Проверены целостность и согласованность навигационных данных.',
      },
      addons_binary_integrity: {
        title: 'Файлы и архитектура плагинов',
        description:
          'Файл плагина, выбранный для этой платформы, пуст либо имеет неверный или неподдерживаемый заголовок. Файл и причина указаны в подробностях.',
        scope:
          'Проверены заголовки глобальных плагинов, плагинов самолётов и зарегистрированных включённых сценариев. Зависимости и фактическую загрузку нужно проверять отдельно.',
        outdated:
          'Результат получен при старой проверке плагинов. Запустите новое полное сканирование.',
      },
      installation_scan_access: {
        title: 'Доступ и пределы проверки',
        description: 'Проверены папки и доступ к файлам.',
      },
      stability_log_freshness: {
        title: 'Возраст журнала',
        description: 'Изучены журнал сеанса и признаки сбоев.',
      },
      updates_gateway_unavailable: {
        title: 'Непроверенные аэропорты Gateway',
        description: 'Версии сопоставлены с доступными обновлениями.',
      },
    },
    purposes: {
      installation_structure: 'Проверены папки и доступ к файлам.',
      installation_xplane_running: 'Проверены папки и доступ к файлам.',
      installation_location: 'Проверены папки и доступ к файлам.',
      installation_readonly: 'Проверены папки и доступ к файлам.',
      storage_xplane_space: 'Проверены место на диске и очищаемые файлы.',
      storage_app_data_space: 'Проверены место на диске и очищаемые файлы.',
      system_inventory: 'Проверены система и графическая среда.',
      system_injectors: 'Проверены система и графическая среда.',
      performance_available_memory: 'Проверены доступные ресурсы.',
      xfast_app_data: 'Проверены данные приложения, база и индекс.',
      xfast_database: 'Проверены данные приложения, база и индекс.',
      xfast_scenery_index: 'Проверены данные приложения, база и индекс.',
      stability_log_available: 'Изучены журнал сеанса и признаки сбоев.',
      stability_crash_dump: 'Изучены журнал сеанса и признаки сбоев.',
      system_beta_build: 'Проверены система и графическая среда.',
      scenery_global_airports: 'Проверены сценарии, зависимости и загрузка.',
      scenery_load_order: 'Проверены сценарии, зависимости и загрузка.',
      scenery_dependencies: 'Проверены сценарии, зависимости и загрузка.',
      scenery_overlaps: 'Проверены сценарии, зависимости и загрузка.',
      scenery_duplicate_airports: 'Проверены сценарии, зависимости и загрузка.',
      scenery_flatten: 'Проверены сценарии, зависимости и загрузка.',
      navdata_custom_data: 'Проверены целостность и согласованность навигационных данных.',
      navdata_cifp: 'Проверены целостность и согласованность навигационных данных.',
      navdata_integrity: 'Проверены целостность и согласованность навигационных данных.',
      navdata_consistency: 'Проверены целостность и согласованность навигационных данных.',
      xfast_recent_failures: 'Проверены данные приложения, база и индекс.',
      storage_cleanable: 'Проверены место на диске и очищаемые файлы.',
      addons_inventory: 'Проверены дополнения и совместимость.',
      addons_platform: 'Проверены дополнения и совместимость.',
      updates_addons: 'Версии сопоставлены с доступными обновлениями.',
      updates_app: 'Версии сопоставлены с доступными обновлениями.',
      updates_csl: 'Версии сопоставлены с доступными обновлениями.',
    },
    sessions: {
      crash: 'В Log.txt есть явный маркер сбоя, но он сам по себе не определяет причину.',
      normal: 'В Log.txt записано нормальное завершение X-Plane.',
      unknown: 'Нет явного маркера сбоя или нормального завершения. Исход сеанса неизвестен.',
      running: 'X-Plane работает; журнал ещё записывается.',
    },
  },
  ar: {
    coverage: {
      addonUpdates:
        'تمت مقارنة {checked} من {installed} إضافة مدعومة. التحديثات: {count}. أداة تحديث خارجية: {external}.',
      navFuture: 'بعض حزم الملاحة المثبتة لم تدخل حيز التنفيذ بعد. راجع التواريخ المذكورة.',
    },
    fields: {
      count: 'العناصر المتأثرة',
      installed: 'المطارات المثبتة',
      checked: 'العناصر المفحوصة',
      free: 'المساحة المتاحة',
      total: 'السعة الإجمالية',
      available: 'الذاكرة المتاحة',
      line: 'سطر السجل',
      days: 'عمر السجل بالأيام',
      errors: 'أخطاء القراءة',
      indexed: 'عناصر الفهرس',
      found: 'عناصر القرص',
      files: 'الملفات',
      size: 'المساحة المستخدمة',
      platform: 'المنصة',
      version: 'الإصدار',
      expired: 'حزم منتهية',
      expiring: 'حزم توشك على الانتهاء',
      unknown: 'صلاحية غير معروفة',
      cycles: 'دورات AIRAC المكتشفة',
      aircraft: 'الطائرات',
      plugins: 'الإضافات',
      csl: 'نماذج X-CSL',
      altitude: 'بيانات الارتفاع',
      cause: 'السبب المحتمل',
      confidence: 'تقييم الأدلة',
      module: 'الوحدة المرتبطة',
      os: 'نظام التشغيل',
      arch: 'البنية',
      cores: 'المعالجات المنطقية',
      memory: 'الذاكرة المثبتة',
      capped: 'بلوغ حد الفحص',
      sessionEnd: 'نهاية الجلسة',
      steam: 'تثبيت Steam',
    },
    scope: {
      installation: 'تم فحص المجلدات والوصول إلى الملفات.',
      stability: 'تمت مراجعة سجل الجلسة وأدلة التعطل.',
      scenery: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      addons: 'تم فحص الإضافات والتوافق.',
      navdata: 'تم فحص اكتمال بيانات الملاحة واتساقها.',
      performance: 'تم فحص موارد النظام المتاحة.',
      storage: 'تم فحص سعة القرص والملفات القابلة للتنظيف.',
      system: 'تم فحص النظام والبيئة الرسومية.',
      xfast: 'تم فحص بيانات التطبيق وقاعدة البيانات والفهرس.',
      updates: 'تمت مقارنة الإصدارات بالتحديثات المتاحة.',
    },
    nav: {
      expired: 'انتهت صلاحية بعض حزم الملاحة. حدّث المصادر المذكورة.',
      expiring: 'بعض الحزم ستنتهي قريباً. خطط للتحديث.',
      unknown: 'تعذر تحديد صلاحية بعض الحزم. تحقق من الدورة والمصدر.',
      current: 'الحزم ذات الصلاحية المعروفة سارية.',
    },
    yes: 'نعم',
    no: 'لا',
    findings: {
      scenery_files: {
        title: 'ملفات المشاهد المفعّلة',
        description:
          'تحتوي بعض الحزم المفعّلة على ملفات غير قابلة للقراءة أو فارغة أو غير صالحة. راجع القائمة.',
      },
      installation_write_access: {
        title: 'إذن الكتابة في مجلدات التثبيت',
        description: 'تعذّر إنشاء ملفات في المجلدات المذكورة. راجع الأذونات وحالة وحدة التخزين.',
      },
      installation_permission_access: {
        title: 'فحص الأذونات غير مكتمل',
        description: 'تعذّر التحقق من بعض الأذونات. راجع الأخطاء المذكورة.',
      },
      system_vulkan_runtime: {
        title: 'بيئة تشغيل Vulkan',
        description: 'محمّل Vulkan مفقود أو أقدم من 1.3. ثبّت برنامج تشغيل الرسوميات أو حدّثه.',
      },
      installation_structure: {
        title: 'تثبيت X-Plane',
        description: 'استعد ملفات التشغيل المذكورة عبر مثبت X-Plane أو التحقق من الملفات في Steam.',
      },
      installation_stock_aircraft: {
        title: 'الطائرات المضمّنة',
        description: 'ملفات الطائرات المذكورة مفقودة. استعد الطائرات المعنية.',
      },
      installation_airfoils: {
        title: 'ملفات المقاطع الهوائية',
        description: 'المقاطع المذكورة مفقودة أو تالفة. استعد الملفات المعنية.',
      },
      installation_asset_access: {
        title: 'الوصول إلى ملفات التثبيت',
        description: 'تعذّر التحقق من بعض الملفات. راجع الأخطاء المذكورة.',
      },
      navdata_bundled: {
        title: 'بيانات الملاحة المضمّنة',
        description: 'أصلح تثبيت X-Plane لاستعادة الملفات المذكورة.',
      },
      navdata_supplemental: {
        title: 'بيانات الملاحة التكميلية',
        description: 'راجع ملفات ARINC 424 أو FAA أو ملفات الملاحة المخصصة المذكورة.',
      },
      navdata_scan_access: {
        title: 'الوصول إلى بيانات الملاحة',
        description: 'تعذّر فحص بعض بيانات الملاحة أو بيانات الدورات. راجع الأخطاء المذكورة.',
      },
      scenery_invalid_entries: {
        title: 'تسجيلات مشاهد غير صالحة',
        description: 'بعض الإدخالات المفعّلة في scenery_packs.ini بلا مسار. راجع الأسطر المذكورة.',
      },
      scenery_registration: {
        title: 'تسجيل المشاهد',
        description: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      },
      scenery_missing_paths: {
        title: 'مسارات مشاهد مفقودة',
        description: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      },
      scenery_duplicate_paths: {
        title: 'تسجيلات مكررة',
        description: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      },
      scenery_broken_links: {
        title: 'روابط مشاهد معطلة',
        description: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      },
      navdata_empty_files: {
        title: 'ملفات ملاحة فارغة',
        description: 'تم فحص اكتمال بيانات الملاحة واتساقها.',
      },
      addons_binary_integrity: {
        title: 'ملفات الإضافات وبنيتها',
        description:
          'ملف الإضافة المحدد لهذه المنصة فارغ أو يحمل ترويسة تنفيذية غير صالحة أو غير مدعومة. راجع الملف والسبب في التفاصيل.',
        scope:
          'يفحص هذا الإجراء ترويسات الإضافات العامة وإضافات الطائرات والمشاهد المسجلة والمفعلة. يتطلب التحقق من التبعيات والتحميل الفعلي فحصًا منفصلًا.',
        outdated: 'حُفظت هذه النتيجة بواسطة فحص الإضافات القديم. أعد إجراء الفحص الشامل.',
      },
      installation_scan_access: {
        title: 'الوصول وحدود الفحص',
        description: 'تم فحص المجلدات والوصول إلى الملفات.',
      },
      stability_log_freshness: {
        title: 'عمر السجل',
        description: 'تمت مراجعة سجل الجلسة وأدلة التعطل.',
      },
      updates_gateway_unavailable: {
        title: 'مطارات Gateway غير المفحوصة',
        description: 'تمت مقارنة الإصدارات بالتحديثات المتاحة.',
      },
    },
    purposes: {
      installation_structure: 'تم فحص المجلدات والوصول إلى الملفات.',
      installation_xplane_running: 'تم فحص المجلدات والوصول إلى الملفات.',
      installation_location: 'تم فحص المجلدات والوصول إلى الملفات.',
      installation_readonly: 'تم فحص المجلدات والوصول إلى الملفات.',
      storage_xplane_space: 'تم فحص سعة القرص والملفات القابلة للتنظيف.',
      storage_app_data_space: 'تم فحص سعة القرص والملفات القابلة للتنظيف.',
      system_inventory: 'تم فحص النظام والبيئة الرسومية.',
      system_injectors: 'تم فحص النظام والبيئة الرسومية.',
      performance_available_memory: 'تم فحص موارد النظام المتاحة.',
      xfast_app_data: 'تم فحص بيانات التطبيق وقاعدة البيانات والفهرس.',
      xfast_database: 'تم فحص بيانات التطبيق وقاعدة البيانات والفهرس.',
      xfast_scenery_index: 'تم فحص بيانات التطبيق وقاعدة البيانات والفهرس.',
      stability_log_available: 'تمت مراجعة سجل الجلسة وأدلة التعطل.',
      stability_crash_dump: 'تمت مراجعة سجل الجلسة وأدلة التعطل.',
      system_beta_build: 'تم فحص النظام والبيئة الرسومية.',
      scenery_global_airports: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      scenery_load_order: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      scenery_dependencies: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      scenery_overlaps: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      scenery_duplicate_airports: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      scenery_flatten: 'تم فحص المشاهد والتبعيات واتساق التحميل.',
      navdata_custom_data: 'تم فحص اكتمال بيانات الملاحة واتساقها.',
      navdata_cifp: 'تم فحص اكتمال بيانات الملاحة واتساقها.',
      navdata_integrity: 'تم فحص اكتمال بيانات الملاحة واتساقها.',
      navdata_consistency: 'تم فحص اكتمال بيانات الملاحة واتساقها.',
      xfast_recent_failures: 'تم فحص بيانات التطبيق وقاعدة البيانات والفهرس.',
      storage_cleanable: 'تم فحص سعة القرص والملفات القابلة للتنظيف.',
      addons_inventory: 'تم فحص الإضافات والتوافق.',
      addons_platform: 'تم فحص الإضافات والتوافق.',
      updates_addons: 'تمت مقارنة الإصدارات بالتحديثات المتاحة.',
      updates_app: 'تمت مقارنة الإصدارات بالتحديثات المتاحة.',
      updates_csl: 'تمت مقارنة الإصدارات بالتحديثات المتاحة.',
    },
    sessions: {
      crash: 'يتضمن Log.txt علامة تعطل صريحة، لكنها لا تثبت السبب وحدها.',
      normal: 'يسجل Log.txt إغلاقاً طبيعياً لـ X-Plane.',
      unknown: 'لا توجد علامة صريحة لتعطل أو إغلاق طبيعي. لا يمكن تحديد نهاية الجلسة.',
      running: 'X-Plane قيد التشغيل ولا يزال السجل قيد الكتابة.',
    },
  },
  hi: {
    coverage: {
      addonUpdates:
        '{installed} समर्थित ऐड-ऑन में से {checked} की तुलना हुई। अपडेट: {count}। बाहरी अपडेटर: {external}।',
      navFuture: 'कुछ स्थापित नेविगेशन पैकेज अभी प्रभावी नहीं हैं। सूचीबद्ध तिथियाँ जाँचें।',
    },
    fields: {
      count: 'प्रभावित मदें',
      installed: 'स्थापित हवाई अड्डे',
      checked: 'जाँची गई मदें',
      free: 'उपलब्ध स्थान',
      total: 'कुल क्षमता',
      available: 'उपलब्ध मेमोरी',
      line: 'लॉग पंक्ति',
      days: 'लॉग की आयु (दिन)',
      errors: 'पठन त्रुटियाँ',
      indexed: 'अनुक्रमित मदें',
      found: 'डिस्क पर मदें',
      files: 'फ़ाइलें',
      size: 'प्रयुक्त स्थान',
      platform: 'प्लैटफ़ॉर्म',
      version: 'संस्करण',
      expired: 'समाप्त डेटा पैकेज',
      expiring: 'जल्द समाप्त पैकेज',
      unknown: 'वैधता अज्ञात',
      cycles: 'मिले AIRAC चक्र',
      aircraft: 'विमान',
      plugins: 'प्लगइन',
      csl: 'X-CSL मॉडल',
      altitude: 'ऊँचाई डेटा',
      cause: 'संदिग्ध कारण',
      confidence: 'साक्ष्य अंक',
      module: 'संबंधित मॉड्यूल',
      os: 'ऑपरेटिंग सिस्टम',
      arch: 'आर्किटेक्चर',
      cores: 'तार्किक प्रोसेसर',
      memory: 'स्थापित मेमोरी',
      capped: 'स्कैन सीमा पूरी',
      sessionEnd: 'सत्र समाप्ति',
      steam: 'Steam स्थापना',
    },
    scope: {
      installation: 'स्थापना फ़ोल्डर और फ़ाइल पहुँच जाँची गई।',
      stability: 'सत्र लॉग और क्रैश साक्ष्य जाँचे गए।',
      scenery: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      addons: 'ऐडऑन और संगतता जाँची गई।',
      navdata: 'नेविगेशन डेटा की पूर्णता और संगति जाँची गई।',
      performance: 'उपलब्ध संसाधन जाँचे गए।',
      storage: 'डिस्क क्षमता और साफ़ किए जा सकने वाले फ़ाइल जाँचे गए।',
      system: 'सिस्टम और ग्राफ़िक्स वातावरण जाँचा गया।',
      xfast: 'ऐप डेटा, डेटाबेस और सूचकांक जाँचे गए।',
      updates: 'स्थापित संस्करणों की अपडेट से तुलना की गई।',
    },
    nav: {
      expired: 'कुछ नेविगेशन पैकेज समाप्त हो चुके हैं। बताए गए स्रोत अपडेट करें।',
      expiring: 'कुछ पैकेज जल्द समाप्त होंगे। अपडेट की योजना बनाएँ।',
      unknown: 'कुछ पैकेजों की वैधता अज्ञात है। चक्र और स्रोत जाँचें।',
      current: 'ज्ञात वैधता वाले सभी पैकेज वर्तमान हैं।',
    },
    yes: 'हाँ',
    no: 'नहीं',
    findings: {
      scenery_files: {
        title: 'सक्रिय सीनरी फ़ाइलें',
        description:
          'कुछ सक्रिय पैकेज में अपठनीय, खाली या अमान्य फ़ाइलें हैं। सूचीबद्ध फ़ाइलें जाँचें।',
      },
      installation_write_access: {
        title: 'इंस्टॉलेशन में लिखने की अनुमति',
        description:
          'सूचीबद्ध फ़ोल्डर में फ़ाइलें नहीं बनाई जा सकतीं। अनुमति और वॉल्यूम की स्थिति जाँचें।',
      },
      installation_permission_access: {
        title: 'अनुमति जाँच अधूरी',
        description: 'कुछ अनुमतियाँ सत्यापित नहीं हो सकीं। सूचीबद्ध त्रुटियाँ देखें।',
      },
      system_vulkan_runtime: {
        title: 'Vulkan रनटाइम',
        description:
          'Vulkan लोडर गुम है या 1.3 से पुराना है। ग्राफ़िक्स ड्राइवर इंस्टॉल या अपडेट करें।',
      },
      installation_structure: {
        title: 'X-Plane इंस्टॉलेशन',
        description:
          'X-Plane इंस्टॉलर या Steam फ़ाइल सत्यापन से सूचीबद्ध स्टार्टअप फ़ाइलें वापस लाएँ।',
      },
      installation_stock_aircraft: {
        title: 'मूल विमान',
        description: 'सूचीबद्ध विमान फ़ाइलें गुम हैं। प्रभावित विमान वापस लाएँ।',
      },
      installation_airfoils: {
        title: 'एयरफ़ॉइल फ़ाइलें',
        description: 'सूचीबद्ध एयरफ़ॉइल गुम या क्षतिग्रस्त हैं। प्रभावित फ़ाइलें वापस लाएँ।',
      },
      installation_asset_access: {
        title: 'इंस्टॉलेशन फ़ाइलों तक पहुँच',
        description: 'कुछ फ़ाइलों का सत्यापन नहीं हुआ। सूचीबद्ध त्रुटियाँ देखें।',
      },
      navdata_bundled: {
        title: 'अंतर्निहित नेविगेशन डेटा',
        description: 'सूचीबद्ध फ़ाइलें वापस पाने के लिए X-Plane इंस्टॉलेशन की मरम्मत करें।',
      },
      navdata_supplemental: {
        title: 'पूरक नेविगेशन डेटा',
        description: 'सूचीबद्ध ARINC 424, FAA या उपयोगकर्ता नेविगेशन फ़ाइलें जाँचें।',
      },
      navdata_scan_access: {
        title: 'नेविगेशन डेटा तक पहुँच',
        description: 'कुछ डेटा या चक्र की जानकारी की जाँच नहीं हो सकी। सूचीबद्ध त्रुटियाँ देखें।',
      },
      scenery_invalid_entries: {
        title: 'अमान्य सीनरी प्रविष्टियाँ',
        description:
          'scenery_packs.ini में कुछ सक्रिय प्रविष्टियों का पथ नहीं है। सूचीबद्ध पंक्तियाँ देखें।',
      },
      scenery_registration: {
        title: 'दृश्य पंजीकरण',
        description: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      },
      scenery_missing_paths: {
        title: 'अनुपस्थित दृश्य पथ',
        description: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      },
      scenery_duplicate_paths: {
        title: 'दोहरा पंजीकरण',
        description: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      },
      scenery_broken_links: {
        title: 'टूटे दृश्य लिंक',
        description: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      },
      navdata_empty_files: {
        title: 'खाली नेविगेशन फ़ाइलें',
        description: 'नेविगेशन डेटा की पूर्णता और संगति जाँची गई।',
      },
      addons_binary_integrity: {
        title: 'प्लगइन फ़ाइल और आर्किटेक्चर',
        description:
          'इस प्लेटफ़ॉर्म के लिए चुनी गई प्लगइन फ़ाइल खाली है या उसका निष्पादन हेडर अमान्य या असमर्थित है। विवरण में फ़ाइल और कारण देखें।',
        scope:
          'वैश्विक, विमान और पंजीकृत सक्रिय दृश्यों के प्लगइन हेडर जाँचे गए। निर्भरताओं और वास्तविक लोडिंग की अलग जाँच आवश्यक है।',
        outdated: 'यह परिणाम पुराने प्लगइन स्कैन से है। फिर से पूर्ण स्कैन चलाएँ।',
      },
      installation_scan_access: {
        title: 'स्कैन पहुँच और सीमाएँ',
        description: 'स्थापना फ़ोल्डर और फ़ाइल पहुँच जाँची गई।',
      },
      stability_log_freshness: {
        title: 'लॉग की आयु',
        description: 'सत्र लॉग और क्रैश साक्ष्य जाँचे गए।',
      },
      updates_gateway_unavailable: {
        title: 'असत्यापित Gateway हवाई अड्डे',
        description: 'स्थापित संस्करणों की अपडेट से तुलना की गई।',
      },
    },
    purposes: {
      installation_structure: 'स्थापना फ़ोल्डर और फ़ाइल पहुँच जाँची गई।',
      installation_xplane_running: 'स्थापना फ़ोल्डर और फ़ाइल पहुँच जाँची गई।',
      installation_location: 'स्थापना फ़ोल्डर और फ़ाइल पहुँच जाँची गई।',
      installation_readonly: 'स्थापना फ़ोल्डर और फ़ाइल पहुँच जाँची गई।',
      storage_xplane_space: 'डिस्क क्षमता और साफ़ किए जा सकने वाले फ़ाइल जाँचे गए।',
      storage_app_data_space: 'डिस्क क्षमता और साफ़ किए जा सकने वाले फ़ाइल जाँचे गए।',
      system_inventory: 'सिस्टम और ग्राफ़िक्स वातावरण जाँचा गया।',
      system_injectors: 'सिस्टम और ग्राफ़िक्स वातावरण जाँचा गया।',
      performance_available_memory: 'उपलब्ध संसाधन जाँचे गए।',
      xfast_app_data: 'ऐप डेटा, डेटाबेस और सूचकांक जाँचे गए।',
      xfast_database: 'ऐप डेटा, डेटाबेस और सूचकांक जाँचे गए।',
      xfast_scenery_index: 'ऐप डेटा, डेटाबेस और सूचकांक जाँचे गए।',
      stability_log_available: 'सत्र लॉग और क्रैश साक्ष्य जाँचे गए।',
      stability_crash_dump: 'सत्र लॉग और क्रैश साक्ष्य जाँचे गए।',
      system_beta_build: 'सिस्टम और ग्राफ़िक्स वातावरण जाँचा गया।',
      scenery_global_airports: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      scenery_load_order: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      scenery_dependencies: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      scenery_overlaps: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      scenery_duplicate_airports: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      scenery_flatten: 'दृश्य पंजीकरण, निर्भरताएँ और लोडिंग जाँची गई।',
      navdata_custom_data: 'नेविगेशन डेटा की पूर्णता और संगति जाँची गई।',
      navdata_cifp: 'नेविगेशन डेटा की पूर्णता और संगति जाँची गई।',
      navdata_integrity: 'नेविगेशन डेटा की पूर्णता और संगति जाँची गई।',
      navdata_consistency: 'नेविगेशन डेटा की पूर्णता और संगति जाँची गई।',
      xfast_recent_failures: 'ऐप डेटा, डेटाबेस और सूचकांक जाँचे गए।',
      storage_cleanable: 'डिस्क क्षमता और साफ़ किए जा सकने वाले फ़ाइल जाँचे गए।',
      addons_inventory: 'ऐडऑन और संगतता जाँची गई।',
      addons_platform: 'ऐडऑन और संगतता जाँची गई।',
      updates_addons: 'स्थापित संस्करणों की अपडेट से तुलना की गई।',
      updates_app: 'स्थापित संस्करणों की अपडेट से तुलना की गई।',
      updates_csl: 'स्थापित संस्करणों की अपडेट से तुलना की गई।',
    },
    sessions: {
      crash: 'Log.txt में स्पष्ट क्रैश चिह्न है, लेकिन केवल लॉग से मूल कारण तय नहीं होता।',
      normal: 'Log.txt में X-Plane का सामान्य बंद होना दर्ज है।',
      unknown:
        'क्रैश या सामान्य बंद होने का स्पष्ट चिह्न नहीं है। सत्र का अंत तय नहीं किया जा सकता।',
      running: 'X-Plane चल रहा है और लॉग लिखा जा रहा है।',
    },
  },
}
