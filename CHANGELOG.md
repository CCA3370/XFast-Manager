# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.1] - Unreleased

### Added

- **Scenery Installation Options** - Choose a scenery variant before installing, or keep the package's default. Variants stored inside the scenery folder and packages with Chinese option names are offered too; backup copies that would change nothing are left out of the list.

### Changed

- **Installation Progress** - See when the app is preparing an installation, before files start copying.
- **Faster Installation** - Large ZIP archives and add-on folders take less time to prepare and install.

### Fixed

- **ZIP Installation Verification** - The app now checks files in their final locations, avoiding false verification failures when the installed folder layout differs from the archive.
- **FlyWithLua Script Bundles** - Bundles with nested folders are now recognized and verified correctly.

## [1.3.0] - 2026-11-09

<div class="changelog-important">
  <p class="changelog-important-label">IMPORTANT FOR USERS</p>
  <p class="changelog-important-title">About Scenery Auto Sorting</p>
  <p class="changelog-important-body">Automatic scenery sorting still has limitations. When you turn it off in Settings, you can still manage and reorder scenery by hand. Installations and other automatic operations preserve your existing load order.</p>
</div>

### Added

- **Aircraft Patches** - Install aircraft patches with a preview of affected files, optional backups and restoration.
- **Flight Plan Management** - Import FMS and SimBrief plans from Install, skip duplicates, resolve filename conflicts, and filter by airport or navigation-data cycle. Open a plan to see its route map, altitude profile, leg distances, waypoints and original file. Navigation data is detected automatically, and previously viewed maps are available offline.
- **X-Plane Health** - Run automatic quick checks or full diagnostics for X-Plane, scenery, plugins and navigation data. Search results, expand the evidence, and review up to 20 checks per installation. Export English diagnostic reports that hide private paths by default. Repair common scenery, graphics and settings problems and restore backups from Health.
- **Screenshot Preview Navigation** - Move between screenshots without closing the preview.

### Changed

- **Output Cleanup** - Recognizes more aircraft and plugin data folders and lets you temporarily disable and restore Output items for troubleshooting.
- **Desktop Text Selection** - Repeated clicks no longer select page labels. Text fields, errors, logs and diagnostic evidence remain selectable.
- **Custom Airport Priority** - Auto Sort places custom airports above Aerosoft airports and Global Airports.
- **Map and SimBrief Removed** - Removed the retired Map page and its SimBrief setting.
- **Clearer Error Guidance** - Error dialogs distinguish app failures from setup, file and network problems. Incomplete split archives list the missing parts without prompting for a bug report. Installation and scenery deletion errors explain how to handle files in use, denied access or security software blocks.
- **Duplicate Bug Reports** - Repeated reports for the same failure reuse the existing issue.

### Fixed

- **Moved Scenery Notices** - Moved or deleted scenery refreshes the list and shows clear guidance instead of a bug-report prompt.
- **Complete Disk Usage** - Storage totals now include X-Plane resources, settings and other files alongside add-ons.
- **Airport Flattening** - Flattening now targets the correct airport when its identifiers differ between data sources.
- **Library Detection** - Ordinary scenery folders are no longer mistaken for libraries because of unrelated files.
- **Resilient Add-on Updates** - Updates automatically retry temporary network and server failures.
- **Installation Verification** - Installation checks ignore harmless system files and explain genuine failures.
- **Batch FlyWithLua Installs** - Scripts from the same package install together without deleting one another's files.
- **Management State Changes** - Aircraft and plugin controls recover when files change outside the app.
- **Gateway Installation** - Install 2D and 3D airports. The app keeps the selected airport and submission, prevents duplicate starts, and refreshes the list after installation.
- **Windows Folder Names** - Add-ons with unsupported folder-name characters now install successfully.
- **Scenery Load Order** - SimHeaven and X-WORLD layers sort correctly, and unrecognized scenery can be categorized manually.
- **Missing Scenery Libraries** - Copy a single library name or the full list to the system clipboard. If the clipboard is unavailable, the names are shown for manual copying.

## [1.2.5] - 2026-05-31

### Added

- **Rotate MD-11 Freighter Liveries** - Rotate MD-11 Freighter livery packs are now recognized correctly during scanning.
- **Remember Window Size and Position** - The application now remembers the window size, position, and maximized state from the previous session and restores them on next launch.
- **X-CSL and ALTITUDE Offline Fallback** - When the X-CSL or ALTITUDE model index server is temporarily unreachable, scans now keep working from the most recently saved index instead of failing, with a notice that the listed package status may be out of date.

### Changed

- **Bundled Library Links** - Added a download link for the Tico Sim Library so the app can point you to it when a scenery package needs that library.
- **Clearer Model Index Errors** - When an X-CSL or ALTITUDE scan cannot reach the server and no saved copy is available, the error now explains the specific network or server problem instead of showing a generic failure message.
- **Bug Report Prompts** - File-access, archive and installation-path problems now show guidance instead of prompting a bug report.

## [1.2.4] - 2026-05-17

### Added

- **Batch Delete in Management** - Aircraft and plugin lists can now delete selected items at once, with failed items left in place and reported clearly.
- **Scenery Smart Groups** - The Scenery manager can group related SimHeaven, Ortho tile, airport, and product packages with one-click enable and disable controls.
- **Scenery Custom Groups** - Scenery packages can now be grouped manually in batches or from the right-click menu, removed from a group, and matched automatically by folder-name prefix or text per X-Plane path.

### Changed

- **Log Analysis Navigation** - Removed the first-time Log Analysis tip bubble from the top navigation.
- **Scenery Toolbar** - Scenery batch and custom group controls now use compact icon buttons, with a cleaner custom group icon.
- **Scenery Custom Groups** - The custom group editor dialog is now narrower and better balanced.
- **Addon Update Queue** - Update tasks in the addon drawer can now be started independently, so one running update no longer blocks another update or fresh install.

### Fixed

- **Scenery Update Status** - Scenery update checks, update badges, and `Update All` availability now stay reliable after switching away and back, with clearer feedback when a check fails.
- **Log Analysis Crash Detection** - Logs that stop before X-Plane writes a normal shutdown line are now reported as crashes.
- **Lua Script ZIP Installs** - Lua script packages now keep nearby resource folders/files with the script during install.
- **Archive Install Guidance** - Multi-part ZIP, 7z, and RAR downloads are now recognized as one addon source, and archive failures now explain missing volumes, incomplete downloads, and 7-Zip requirements more clearly.
- **Atomic Install Staging** - Atomic installs can now fall back to another staging location when the X-Plane root cannot create the temporary install folder.
- **Locked Folder Deletion** - Delete failures now explain likely causes such as X-Plane, file managers, antivirus, or folder permissions, and retry after clearing read-only permissions.
- **Moved or Deleted Items** - Opening an item whose folder no longer exists now refreshes the list and explains that the item may have moved or been deleted.
- **Aircraft Variant Toggles** - Per-variant aircraft enable/disable controls now stay in sync after each click and no longer fail when the file state changes while the ACF manager is open.
- **Airport Flatten Selection** - Airport search results now keep the selected airport open, refresh stale flatten indexes when needed, and show a clear retry action when sources cannot be loaded.
- **Gateway and Addon Update Errors** - Temporary upstream failures now retry briefly and show a clear service-unavailable message instead of raw server errors.
- **Scenery Group Display** - Smart, custom, and ungrouped Scenery groups now start collapsed while search still expands matches, and unmatched scenery stays inside Ungrouped.

## [1.2.3] - 2026-04-26

<div class="changelog-important">
  <p class="changelog-important-label">IMPORTANT FOR USERS UPGRADING FROM 1.2.2</p>
  <p class="changelog-important-title">If scenery sorting caused problems in the previous version, rebuild the scenery index before using Auto Sort again.</p>
  <p class="changelog-important-body">Go to Settings, rebuild the scenery index, and then run Auto Sort again.</p>
</div>

### Added

- **Output Cleanup** - Added a new page to clean selected X-Plane Output caches and generated files without removing the top-level Output folders.
- **CSL Path Sync** - CSL packages can now be synced across CSL paths with a dedicated progress window during sync.

### Changed

- **Scenery Index Recovery** - Rebuild and reset flow is now clearer when the local scenery index becomes outdated or incompatible.
- **Airport Flatten Workflow** - Airport Flatten Manager now makes it easier to compare default and custom apt.dat sources and re-apply saved flatten settings.
- **Renderer Cache Cleanup** - Log Analysis now includes a one-click renderer cache cleanup action.
- **Bundled Library Links** - Added more community-contributed download links for missing libraries.

### Fixed

- **Scenery Sorting Reliability** - Fixed scenery index and sorting issues that could affect load order stability.
- **Addon Install Path Handling** - Improved install and extraction reliability when target folders are missing or paths resolve unexpectedly.
- **Management Path Validation** - Reduced false path-related failures in addon analysis and management actions.

## [1.2.2] - 2026-04-18

### Fixed

- **Aircraft Enable/Disable Duplicate Entries** - Toggling an aircraft on or off no longer leaves a duplicate row in the management list.

## [1.2.1] - 2026-04-18

### Added

- **CSL and ALTITUDE Model Management** - Dedicated page to scan, search, install, update, and uninstall multiplayer model packages, with custom path support and queued/cancellable installs.
- **Gateway Airports** - Search airports, compare submissions, install or remove scenery, and check for updates. Results show whether an airport is included in your X-Plane version and warn about conflicting custom scenery.
- **Built-in Zibo Updater** - Version-aware Zibo update support with torrent downloads, manual download fallback for major-version packages, and preserve options for liveries and config files during major updates.
- **Airport Flatten Manager** - New page to search airports by ICAO or name and toggle their flatten setting on each scenery source. Every change is remembered, and a one-click action can re-apply all saved settings whenever the underlying scenery files no longer match.
- **Configuration Presets** - Save and apply preset workflows for aircraft, plugins, scenery, Lua scripts, and lock states, with full import/export support.
- **Activity Log** - New history page covering installs, updates, deletions, enable/disable actions, scenery sorting, and preset applies.
- **Disk Space Analysis** - New disk usage page that scans X-Plane content by category and drills down to per-folder size details.
- **Per-Variant ACF Management** - Aircraft entries with multiple top-level ACF/XFMA files now expose a dedicated management dialog so each variant can be enabled or disabled independently from the right-click menu.
- **Screenshot Background Management** - Screenshots can now be set or unset as X-Plane background images directly from Screenshot Manager.
- **Per-Item Beta Channel in Addon Update** - Aircraft and plugin update items can now opt into beta checks individually instead of relying on a single global beta toggle.
- **Nine New Interface Languages** - Added Arabic, German, Spanish, French, Hindi, Japanese, Korean, Portuguese, and Russian UI translations.
- **Arch Linux Native Release Package** - Releases now include a native Arch Linux `tar.gz` package, with updated Linux download and install guidance in the README.

### Changed

- **Navigation and Quick Access** - Refreshed the top navigation and home/dashboard layout, with a new command palette and keyboard shortcuts for faster page switching.
- **Add-on Updates** - Preview file changes and download details before updating. Installation options stay locked while an update runs.
- **Screenshot Previews** - Slider previews match the saved result without flashing the original image. Color temperature now produces the correct warm or cool effect.
- **Screenshot Editor Slider Feel** - Exposure, saturation, and denoise respond more gently across their range so subtle tweaks no longer overshoot.
- **Scenery Manager Airport Entries** - Scenery Manager now shows `*GLOBAL_AIRPORTS*` as a real indexed entry and keeps `DarkBlue-` airport mesh/overlay packages directly below their corresponding airport packages.
- **Nested Archives** - Faster scanning and verification for wrapped packages, with more reliable version labels.
- **Update Checks** - Manual update checks now start immediately.
- **Install / Extract Reliability** - Clearer error messages during addon extraction and installation, and missing target folders are now created automatically instead of causing silent failures.
- **Stable Feature Defaults** - Parallel Installation, Multi-Threaded Download, and DMP Crash Analysis are now enabled by default and no longer labeled as experimental in Settings.
- **Bundled Library Links** - Added new community-contributed entries to the bundled library link list.
- **Localized Strings** - Refreshed wording across the bundled UI languages for consistency.

### Fixed

- **Pre-release Update Downloads** - In-app update downloads now correctly follow the pre-release setting when checking and installing new versions.
- **Addon Update Cleanup Rules** - Blacklist-based deletions are handled more completely during addon updates, reducing leftover obsolete files.
- **Nested Archive Installation** - Some nested archive installs no longer produce an empty target folder instead of the actual addon content.
- **Windows Archive Filename Compatibility** - Archive entries with Windows-invalid filenames are now sanitized during install instead of failing.
- **Incomplete Aircraft Package Scanning** - Archives missing an expected `Aircraft` folder no longer abort addon analysis as a fatal error.
- **Scenery Misclassification for SAM Libraries** - Libraries with nested demo airport content are less likely to be misclassified as airports.
- **Navigation Data Safety** - Deletion preserves backups, custom files and other providers. Removed the right-click delete shortcut to reduce accidental removal.
- **Log Text Consistency** - Activity and app log entries were standardized to avoid locale-related garbled text in runtime logs.

## [1.1.2] - 2026-03-06

### Added

- **SkunkCrafts Update Integration** - Added integrated SkunkCrafts update workflow in Addon Update, including per-item update entry points and task-based update execution.
- **Parallel Download Support for Addon Update** - Added parallel/multi-thread download support in the update workflow to improve large addon update speed.
- **Crash Report Analysis (Experimental)** - Inspect crash reports for probable causes and involved plugins.
- **Feedback Center** - Added in-app feedback submission with issue tracking, issue detail view, and comment replies.
- **Screenshot Manager** - Added a dedicated page for browsing `Output/screenshots` images and clips with search/filter/sort, preview, copy/save/delete, and Reddit sharing.
- **Screenshot Editor** - Crop, rotate and adjust images, with live previews and sharpening or noise reduction.

### Changed

- **Scenery Lock Also Locks Sort Position** - Locked scenery entries now preserve their current sort slots during index refresh and auto-sort, so lock state also protects load-order position.

### Fixed

- **Delete Source After Install Safety** - Source files are now deleted only after all related install tasks succeed, avoiding premature cleanup when part of a grouped install fails.
- **Management Path Resolution** - Improved folder/path validation in management actions (including root-level navdata handling) to reduce false invalid-path failures.
- **Scenery Path Validation** - Applying scenery changes now validates the configured X-Plane root path more strictly and ensures the parent directory of `scenery_packs.ini` exists before writing.

## [1.1.1] - 2026-02-28

### Added

- **Lua Companion Files** - Install scripts with their supporting files and folders, shown together before confirmation.
- **Lua Lock Controls in Scripts Page** - Lua entries now support lock/unlock directly in the Lua management page.
- **Batch Mode for Aircraft/Plugin Management** - Added multi-select with full-row "Select All" behavior and batch enable/disable actions.
- **Drop-Zone File Picker** - Clicking the drag-and-drop area now opens a file picker for manual file selection.
- **UFMC Navdata Support** - Added installation support for UFMC-type navdata (providers `Universal FMC (UFMC)` and `Modern UFMC`); installs to `Custom Data/UFMC`.

### Changed

- **7z Analysis and Install Performance** - Improved 7z scanning speed (including nested archive handling) and password/encryption detection responsiveness.
- **7z Size Estimates** - Show more accurate installation sizes for 7z packages.
- **Scenery Priority Ordering** - `Lines3D` is now kept at the top of fixed high-priority scenery libraries.

### Fixed

- **X-Plane Elevation Error Guidance** - When launch fails with Windows error 740, the app now shows actionable cause/solution guidance instead of only a generic failure message.
- **Unsupported Navdata UX** - Unknown navdata formats no longer interrupt scan flow with a blocking failure modal.
- **Lua Install Compatibility** - Fixed Lua install flow so Lua tasks no longer fail with `Unsupported archive format: lua`.
- **Default Aircraft Protection** - Required default X-Plane aircraft can no longer be disabled by mistake.
- **Navdata Backup Consistency** - Backup location is now consistent with atomic mode (`Custom Data/Backup_Data`), cleanup removes only same-provider backups, and full navdata folder content is included in backup scope.
- **Failed Task Detail Navigation** - In the install-failed flow, clicking a failed task now opens only that task's error details instead of an aggregated all-task error panel.
- **Bug Report Availability in Failure Details** - Error details opened from failed tasks now use the unified error modal, so auto-report and privacy notice are available there as well.

## [1.1.0] - 2026-02-27

### Added

- **X-Plane Log Analysis** - Added a dedicated log analysis page to quickly inspect errors and troubleshooting details.
- **Parallel Installation (Experimental)** - Added optional parallel install mode with conflict awareness for overlapping target paths.
- **Update Changelog Display** - Added automatic changelog display on first startup after updates, with manual viewing option in Settings.
- **Bug Report Follow-Up UX** - Issue update flow and messaging were refined for clearer status communication after reporting problems.

### Changed

- **Installation Progress Feedback** - Progress display now reports phase/task updates more clearly, including improved tracking when parallel mode is enabled.

### Fixed

- **Scenery Error Messages** - Failure notifications now include clearer error details in scenery-related operations.
- **Path Conflict Handling** - Improved overlap detection to better prevent problematic installs into conflicting target directories.
- **Linux AppImage Startup** - Improved startup behavior and guidance in Wayland environments, including a more reliable X11 fallback path for problematic systems.

## [1.0.1] - 2026-02-24

### Fixed

- **Linux File Manager** - Open folders in a file manager instead of a web browser.
- **Linux X-Plane Detection** - Recognize X-Plane installations with versioned executable names.

### Changed

- **Update Download URL** - "Download" button now selects URL based on current language
- **Error Messages** - Updated invalid path error messages to remove platform-specific executable suffix

## [1.0.0] - 2026-02-24

This is the first official public release of XFast Manager.

### Fixed

- **Toggle-Enabled Event Propagation** - Fixed the enable/disable toggle button in management cards triggering unintended parent click events
- **Updater Lock Sync** - Disabling aircraft or plugins also locks them against updates; re-enabling unlocks them while preserving manual locks.

### Changed

- **Felis B742 Liveries** - More accurate livery recognition for the Classic and Freighter variants.
- **Update Download URL** - "Download" button now links to GitHub Releases

## [0.9.2] - 2026-02-23

### Added

- **Archive Version Detection** - Show aircraft and plugin versions directly from ZIP, 7z and RAR packages before installation.
- **Per-Task Progress Details** - Show the size and progress of each installation task.

### Changed

- **Faster Installation** - Faster installation of ZIP, 7z and RAR packages.
- **Per-Task Progress Bar** - Each task's progress bar now shows its own completion percentage.

## [0.9.1] - 2026-02-23

### Added

- **Privacy Notice for Bug Reports** - Error modal now displays privacy information when submitting bug reports
  - Clear explanation of what data is collected
  - Localization support for privacy notices

### Changed

- **Theme Transitions** - Improved theme switching performance
  - Reduced transition duration for faster theme changes
  - Smoother visual transitions between light and dark modes
- **Geographic Region Detection** - Improved continent calculation accuracy for scenery packages

## [0.9.0] - 2026-02-22

### Added

- **Bug Reports** - Submit reports from Settings with system information included.
- **Sponsor Dialog** - Support development through WeChat or Alipay.
- **Livery Detection** - Added X-Crafts and Felis B742 support, with recognition rules refreshed automatically.
- **Duplicate Airports** - Identify scenery packages containing the same airport to help find conflicts.
- **Installation Confirmation** - Improved the task list and confirmation layout.

### Changed

- **Responsiveness** - Smoother navigation and faster page loading.
- **Scenery Management** - Clearer feedback for index changes and load-order edits.
- **Library Links** - Refresh missing-library download links and show clearer submission errors.

### Fixed

- **Context Menus** - Menus stay within the window.
- **Logs** - Opening the app no longer rotates logs unnecessarily.
- **macOS Disk Space** - Corrected available-space calculations.
- **Error Guidance** - Clearer messages for locked files and invalid X-Plane paths.

## [0.8.11] - 2026-02-13

### Fixed

- Fixed minor bugs

## [0.8.10] - 2026-02-13

### Added

- **Missing Library Download Links** - Missing library dialog now supports direct download links for many common libraries.
- **Library Link Contributions** - Submit download links from the Missing Libraries dialog, with search and copy shortcuts.

### Changed

- **Smarter Link Source Updates** - Missing library links now load immediately from bundled data, then refresh in the background from the latest online list.

### Fixed

- **Missing Library False Positives** - Improved library matching logic to handle case differences.
- **Library Recognition** - Fewer false missing-library warnings for files saved in different text encodings.
- **Scenery Refresh** - Opening Scenery Manager refreshes incomplete library information more reliably.

## [0.8.9] - 2026-02-09

### Added

- **Aircraft Liveries** - Browse, preview, open and delete individual liveries.
- **Lua Scripts** - View, enable, disable and delete FlyWithLua scripts, with right-click shortcuts.
- **Scenery Conflicts** - Find overlapping terrain tiles and filter affected packages, including XPME load-order handling.
- **Scenery Filters** - Filter packages by enabled or disabled state.

### Changed

- **Downloads** - All platform installers are distributed as ZIP files.
- **Project Description** - Updated the tagline to "The Modern X-Plane Addon Installer & Manager".
- **Plugin Display** - Show the number of FlyWithLua scripts.

## [0.8.8] - 2026-02-05

### Fixed

- Fixed minor bugs

## [0.8.7] - 2026-02-04

### Fixed

- **Scenery External Links** - Opening and deleting scenery entries that are shortcuts/symlinks outside Custom Scenery now works, and deletion removes the link itself.
- **Windows Symlink Installs** - Atomic install now falls back to copying when symlink creation isn't permitted, preventing install failures on Windows.
- **Delete Source Safety** - "Delete source after install" now skips deletion when the dragged path and detected addon root are nested, avoiding accidental partial or parent deletions.
- **Management Path Validation** - Safer path checks for aircraft/plugin/navdata actions to prevent invalid folder operations.
- **Startup Window Flicker** - Fixed a brief window flicker when running.

### Changed

- **Linux Packages** - Added required GTK/WebKit dependencies for DEB packaging to improve first-run reliability.

## [0.8.6] - 2026-02-03

### Added

- **Region Grouping View** - New view mode in Scenery Manager to group scenery by geographic region
  - Click "By Region" button to switch between category view and region view
  - First level groups by continent (Asia, Europe, North America, etc.)
  - Second level groups by scenery type within each continent
  - Non-regional packages (libraries, global resources) grouped under "Other / Global"
  - Batch enable/disable toggle for entire regions or type groups within a region
  - All groups collapsed by default for cleaner overview
  - Drag sorting disabled in region view to preserve load order
- **Always on Top** - New pin button in the title bar to keep the window above other windows
  - Click to toggle window always-on-top state
  - Visual indicator shows when pinned
- **Lua Script Support** - Detection and installation of FlyWithLua scripts
  - Automatically detects `.lua` files intended for FlyWithLua
  - Installs to the correct FlyWithLua Scripts folder
  - Shows warning if FlyWithLua plugin is not installed

### Changed

- **Scenery Groups Default Collapsed** - All scenery category groups now start collapsed for a cleaner initial view
- **Search Auto-Expand** - When searching for scenery, groups containing matches automatically expand, then collapse again when navigating away

## [0.8.5] - 2026-02-03

### Added

- **Launch X-Plane Button** - New button on the home page to launch X-Plane directly from the app
  - Automatically detects pending scenery changes before launching
  - If changes exist, offers three options: Apply & Launch, Launch Directly, or Cancel
  - Supports custom command-line arguments (configurable in Settings)
- **Window Close Protection** - Improved safety when closing the app during active operations
  - Warning when closing during an installation in progress
  - Warning when closing with a pending installation confirmation dialog open

### Changed

- **Onboarding Defaults** - Worry-Free Installation and Scenery Auto-Sorting are now enabled by default for new users
- **Scenery Info Display** - Scenery cards now show continent instead of country for a cleaner layout

## [0.8.4] - 2026-02-02

### Added

- **Geographic Region Filter** - Filter scenery packages by continent in the Scenery Manager
  - Dropdown selector to show only scenery from a specific continent
  - Supports Asia, Europe, Africa, North America, South America, Oceania, and Antarctica
- **Geographic Info Display** - Scenery entries now show country/region information
  - Country or continent badge displayed on each scenery card
  - Hover to see full location details
- **Database Reset Option** - Handle incompatible database versions gracefully
  - When opening a database created by a newer app version, a reset option is now provided
  - One-click database reset with automatic index rebuild prompt

### Changed

- **Faster Navigation** - The Management page opens more quickly.

## [0.8.3] - 2026-02-01

### Added

- **Navdata Backup Toggle** - Users can now choose whether to backup existing navdata during clean install
  - When unchecked, existing backups for the same provider will also be deleted
  - Provides more control over disk space usage
- **Navdata Backup Restore** - New restore button for navdata entries in Management view
  - One-click restoration of previously backed up navdata
  - Shows backup cycle/AIRAC info and backup time
- **Unrecognized Scenery Category** - New category for scenery packages that cannot be automatically classified
  - Packages without Earth nav data, library.txt, or plugins are now shown in this category
  - Placed at the bottom of the scenery list with lowest priority

### Changed

- **App Data Location** - Logs, settings and scenery data now share a common app-data folder.
  - Linux: `~/.local/share/com.xfastmanager.tool/` (or `$XDG_DATA_HOME`)
- **Progress Bar Gradient Effect** - Installation progress bar now displays a smooth gradient animation
  - Gradient moves through blue to green as progress increases
  - More visually appealing and intuitive progress indication
- **Scenery Index Auto-Refresh** - Background scan now automatically detects scenery changes
  - Scenery list refreshes automatically when changes are detected
  - No manual refresh needed after external modifications

### Fixed

- **Linux Startup** - Fixed startup freezes with certain graphics configurations.
- **Long Text Truncation** - Fixed display issues with long addon names in confirmation modal
  - Names are now properly truncated with ellipsis
  - Prevents layout breaking with very long navdata provider names

## [0.8.2] - 2026-01-29

### Fixed

- **Aircraft with Embedded Plugins** - Plugins inside aircraft packages are no longer incorrectly detected as standalone plugins
  - When an aircraft contains internal plugins (e.g., for custom systems), they are now correctly identified as part of the aircraft
  - Only truly standalone plugins in the plugins folder will be detected

### Changed

- **Data Storage Location** - App data now stored in `%LOCALAPPDATA%` instead of `%APPDATA%` on Windows
  - Logs, database, and cache files are now stored locally rather than in roaming profiles
  - Provides faster access and avoids sync issues on domain-joined machines

## [0.8.1] - 2026-01-29

### Added

- **Updater Lock Sync** - Aircraft and plugin locks now stay in sync with SkunkCrafts updater settings.

### Changed

- **Saved Settings** - Improved the reliability of saved preferences across updates and platforms.

## [0.8.0] - 2026-01-28

### Added

#### Installation Progress
- **Adaptive Task List Size** - Installation progress view now adjusts task item sizes based on the number of tasks
  - Fewer tasks (1-3): Larger, more prominent display
  - Medium count (4-6): Standard size
  - Many tasks (7+): Compact view to fit more items
- **Add-on Icons** - Replaced emoji with a consistent set of icons for each add-on type.

#### Management View
- **Update & Outdated Filters** - New filter buttons to quickly find addons with available updates or outdated navdata

#### Livery Detection
- **Enhanced Livery Recognition** - Improved detection of livery names from folder structures
  - Better parsing of livery folder naming conventions
  - More accurate livery count display

### Changed

#### Installation Progress
- **Smoother Progress Animation** - Progress bar now animates more smoothly without regression
- **View Persistence** - Installation progress and completion view now persists correctly when switching between pages

#### User Interface
- **New Application Icon** - Updated app icon with modern design
- **Error Messages** - More specific and localized error messages for various failure scenarios

#### Update Notification
- **New Download Link** - Update notification now links to the official download page

### Fixed

- **Progress Bar Regression** - Fixed issue where progress percentage could temporarily decrease during task transitions
- **Password Modal** - Improved password input interface with better visual feedback

## [0.7.7] - 2026-01-25

### Fixed

#### Scenery Manager
- **Shortcut Conflict Fix** - Multiple shortcuts pointing to folders with the same name now work correctly
  - Previously, only one shortcut would be recognized when multiple shortcuts pointed to different folders with identical names
  - Now each shortcut is treated as a separate entry using its own name
- **Path Compatibility** - Windows shortcut target paths are now handled correctly in scenery_packs.ini
  - Backslashes are automatically converted to forward slashes as required by X-Plane

### Changed

#### Scenery Installation
- **Respect Index State** - Installing scenery via the software now respects whether the scenery index has been created
  - If you haven't built the scenery index yet, newly installed scenery will not be automatically added to the index or sorted
  - This gives you full control over when to start using the scenery manager feature
  - Once you build the index, future scenery installations will be automatically managed

## [0.7.6] - 2026-01-25

### Changed

- **Large Scenery Libraries** - Faster loading and saving, lower memory use, and smoother browsing and ordering.
- **Scenery Index Upgrade** - Rebuild the scenery index once after updating, from Settings or through the automatic first-launch rebuild.

## [0.7.5] - 2026-01-25

### Changed

#### Error Messages
- **Clearer Error Prompts** - Error messages are now more specific and easier to understand
  - Different error types display appropriate messages (permission denied, file not found, password error, etc.)
  - Full support for Chinese and English

#### Confirmation Dialog
- **Item Name Display** - Confirmation dialogs now clearly show the name of the item being operated on
- **Loading State** - Shows loading indicator during operations

#### Settings & Onboarding
- **Cleaner Interface** - Removed experimental labels, improved layout and visual organization
- **Smoother Setup Flow** - More intuitive first-time user experience

#### Scenery Manager
- **Category Rename** - "Orthophotos" category renamed to "Airport Mesh" for better accuracy
- **INI Sync Status** - Shows notification when scenery_packs.ini differs from the current list

## [0.7.0] - 2026-01-24

### Added

- **Unified Management** - Browse and search aircraft, plugins, navigation data and scenery from one page.
- **Aircraft and Plugins** - View versions, enable or disable items, delete them, and open their folders. Aircraft show livery counts; plugins show supported platforms.
- **Navigation Data** - View providers and AIRAC cycles, filter outdated data, delete packages, and open their folders.
- **Update Checks** - Check supported aircraft and plugins for updates, with current and latest versions shown on each card.
- **Livery Recognition** - More accurate aircraft livery counts.

### Changed

- **Navigation** - Smoother page and tab transitions.
- **Scenery Management** - Moved into the Scenery tab on the unified management page; automatic scenery sorting must be enabled to access it in this version.

## [0.6.0] - 2026-01-22

### Added

- **Guided Setup** - Configure the X-Plane path, installation preferences, Windows integration and scenery sorting on first launch.
- **Scenery Index Setup** - A first-time prompt helps create the scenery index and shows progress.

### Changed

- **Project Name** - Renamed XFastInstall to XFast Manager.
- **Archive Handling** - Improved 7z and RAR support.
- **Reliability** - Improved error handling and responsiveness during installation and scenery management.

## [0.5.0] - 2026-01-21

### Added

- **Scenery Management** - Browse, search, enable, disable and reorder scenery, with collapsible groups, dependency warnings and backups before applying changes.
- **Scenery Classification** - Recognize airports, libraries, overlays, orthophotos and terrain meshes, with appropriate loading priorities.
- **Scenery Index** - Faster repeat scans, automatic refresh of changed packages, and a rebuild option in Settings.
- **Scenery Ordering** - Sort by type, geographic coverage and name, with SAM libraries given priority.
- **Automatic Sorting (Experimental)** - Optionally place newly installed scenery while preserving existing manual order.
- **Sort on Demand** - Sort scenery from Settings, with a backup before changes.
- **Linked Scenery** - Recognize scenery reached through Windows shortcuts and symbolic links.
- **Scenery Search** - Search expands matching groups; group collapse states are remembered.
- **Themes** - Smoother switching between light and dark themes.
- **Large Scenery Packages** - Faster indexing, including Ortho4XP collections.
- **Log Detail** - Adjust the level of logging while the app is running.

### Changed

- **Scenery Installation** - Installation now follows the automatic scenery sorting preference.

## [0.4.1] - 2026-01-17

### Fixed
- **Conflicts between Scenery and SceneryLibrary** - Improve logic to process this situation

## [0.4.0] - 2026-01-17

### Added

- **App Updates** - Check for updates automatically at startup or manually from Settings, with download notifications on the home page.
- **Update Preferences** - Choose whether to check at startup and include pre-releases, and view the last check time.

### Changed

- **Settings Layout** - Update settings appear above logs.
- **Update Banner** - Consistent styling and a clearer Download button.

### Fixed

- **Update Checks** - Handle cases where no stable release is available.

## [0.3.0] - 2026-01-17

### Changed

- **Faster Analysis** - Improved detection and scanning for large add-on collections.
- **Installation Speed** - Faster archive handling and aircraft backups; disabling verification now skips unnecessary file checks.
- **Installation Results** - A clearer completion view separates failed tasks and their error details.
- **Password Entry** - Clearer feedback and a more compact password dialog.
- **Error Messages** - More specific explanations in English and Chinese.
- **Platform Support** - Interface options better reflect the current operating system.

### Added

- **RAR Retry** - Retry files that fail verification, including password-protected packages.

### Fixed

- **Navigation Stability** - Reduced memory use when switching pages.

## [0.2.0] - 2026-01-16

### Added

- **Source Cleanup** - Optionally delete source files after successful installation. Failed or unrecognized sources are kept, and overlapping source folders are protected.
- **Disk Root Protection** - Prevent accidental installation from an entire drive.
- **Settings Help** - Expand installation and Windows integration options for explanations.
- **App Version** - Settings now displays the current app version accurately.

### Fixed

- **Task Controls** - Skip and Cancel buttons respond correctly.

### Changed

- **Installation Guidance** - Clearer descriptions of new installation, clean installation and direct overwrite.
- **Settings Layout** - Moved atomic installation below Windows integration.

## [0.1.1] - 2026-01-16

### Initial Release

XFast Manager installs and manages X-Plane aircraft, scenery, plugins and navigation data on Windows, macOS and Linux.

### Added

- **Add-on Detection** - Identify supported add-on types and avoid duplicate detection of components bundled inside other add-ons.
- **Batch Installation** - Drag in archives or folders, pass paths from the command line, or use the Windows right-click menu. Supports ZIP, 7z and RAR.
- **Encrypted Archives** - Open password-protected ZIP, 7z and RAR packages with shared or individual passwords and retry when needed.
- **Installation Modes** - Install new packages, replace an existing installation, or overwrite matching files while keeping other content.
- **Aircraft Backups** - Preserve liveries and selected configuration files during clean installation.
- **Atomic Installation** - Use temporary installation and rollback to recover from failures, with progress and automatic cleanup. Requires at least 1 GB of free space.
- **Installation Preferences** - Choose which add-on types to install and remember the selection.
- **Archive Safety** - Protect against unsafe extraction paths and unusually large archives, with a 20 GB size limit and a 100:1 compression-ratio limit.
- **Installation Verification** - Check installed files, retry failures and choose which source formats to verify. RAR verification is not supported in this version.
- **Conflict Warnings** - Review existing installations, navigation-data cycles, source and target paths, and estimated sizes before confirming.
- **Progress and Task Controls** - Track each installation phase, speed and elapsed time; skip a task or cancel all tasks with cleanup and warnings about possible data loss.
- **Interface Options** - English and Chinese, light and dark themes, keyboard shortcuts and saved preferences.
- **Logs and Error Details** - View, refresh, copy or export logs, open the log folder, and choose the level of detail.
- **Performance** - Faster scanning, extraction and copying, with a responsive interface during long operations.
- **Help** - In-app guidance explains installation modes, backups, passwords and common errors.
