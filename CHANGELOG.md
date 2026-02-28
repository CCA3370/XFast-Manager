# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.1] - 2026-02-28

### Added

- **Lua Companion Bundle Support** - Lua scripts now detect `SCRIPT_DIRECTORY` companion files/folders and install them together; companion items are shown in the confirmation list with expandable details.
- **Lua Lock Controls in Scripts Page** - Lua entries now support lock/unlock directly in the Lua management page.
- **Batch Mode for Aircraft/Plugin Management** - Added multi-select with full-row "Select All" behavior and batch enable/disable actions.
- **Drop-Zone File Picker** - Clicking the drag-and-drop area now opens a file picker for manual file selection.
- **UFMC Navdata Support** - Added installation support for UFMC-type navdata (providers `Universal FMC (UFMC)` and `Modern UFMC`); installs to `Custom Data/UFMC`.

### Changed

- **7z Analysis and Install Performance** - Improved 7z scanning speed (including nested archive handling) and password/encryption detection responsiveness.
- **7z Size Estimation Accuracy** - Installation size display for 7z sources is now based on archive metadata instead of rough multipliers.
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

- **Linux File Manager** - Fixed `open_in_explorer` on Linux opening folders in a browser instead of the file manager; now tries common file managers (nautilus, dolphin, thunar, nemo, pcmanfm) before falling back to xdg-open
- **Linux X-Plane Detection** - Path validation, executable launch, and process detection now use `X-Plane*` glob matching instead of a hardcoded `X-Plane` filename

### Changed

- **Update Download URL** - "Download" button now selects URL based on current language
- **Error Messages** - Updated invalid path error messages to remove platform-specific executable suffix

## [1.0.0] - 2026-02-24

This is the first official public release of XFast Manager.

### Fixed

- **Toggle-Enabled Event Propagation** - Fixed the enable/disable toggle button in management cards triggering unintended parent click events
- **Auto-Lock Sync on Disable/Enable** - Disabling an aircraft or plugin now immediately locks it in the UI and syncs to `skunkcrafts_updater.cfg`; re-enabling auto-unlocks it (manual locks are preserved)

### Changed

- **Livery Pattern Detection** - Refined detection rules for Felis B742 Classic and B742 Freighter
  - More precise file patterns to reduce false positives
  - Fixed typo in wing texture pattern (`mings_main` → `wings_main`)
- **Update Download URL** - "Download" button now links to GitHub Releases

## [0.9.2] - 2026-02-23

### Added

- **Archive Version Detection** - Aircraft and plugin versions are now detected directly from archive files during analysis
  - No longer requires extraction to read version information
  - Supports ZIP, 7z, and RAR archives
  - Reads `skunkcrafts_updater.cfg` and version files within archives
- **Per-Task Progress Details** - Installation progress overlay now shows per-task size information
  - Active task displays real-time processed/total MB
  - Completed and pending tasks show estimated size

### Changed

- **Faster Installation** - Significantly reduced installation time through inline verification
  - ZIP archives: CRC32 hash is computed while writing files, eliminating the separate verification re-read pass
  - 7z archives: SHA256 hash is computed while extracting, enabling verification without re-reading all files
  - RAR archives: Direct extraction to target directory when no internal root stripping is needed, removing the temp directory copy step
  - Larger I/O buffer (4 MB → 8 MB) for faster file operations
- **Per-Task Progress Bar** - The progress bar for the current task now shows task-specific percentage instead of overall percentage

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

- **Bug Report Submission** - New integrated bug reporting feature
  - Submit bug reports directly from the app with system information automatically included
  - Accessible from Settings page
  - Streamlined issue creation workflow
- **Sponsor Modal** - New donation support interface
  - WeChat and Alipay QR codes for supporting development
  - Accessible from the app menu
- **Enhanced Livery Pattern Detection** - Improved aircraft livery recognition
  - Added support for X-Crafts aircraft (E-Jets family, ERJ family)
  - Added Felis B742 variants (Classic and Freighter)
  - Remote livery pattern loading with automatic updates on startup
  - Wildcard pattern support for more flexible matching
- **Airport Duplicate Detection** - Identifies scenery packages with overlapping airports
  - Automatically detects duplicate airport IDs across packages
  - Helps identify potential conflicts in scenery library
- **Task List Component** - Enhanced installation confirmation interface
  - Dedicated TaskListSection component for better task visualization
  - Improved layout and user experience during installation confirmation

### Changed

- **Performance Optimizations** - Enhanced UI responsiveness
  - Optimized transitions across multiple components
  - Improved component loading and rendering performance
- **Code Architecture** - Major backend refactoring for maintainability
  - Reorganized Rust modules into logical groups (analysis, core, data, install, management, scenery, services)
  - Split large files into focused modules (scanner split into zip/rar/sevenz handlers)
  - Enhanced database structure with new entity models
  - Improved separation of concerns across the codebase
- **Scenery Packs Manager** - Enhanced scenery_packs.ini management
  - Better change tracking with detailed modification logs
  - Improved UI feedback for scenery index operations
- **Library Link Management** - Improved missing library workflow
  - Force refresh option for library link lookup
  - Enhanced error handling in approval workflow
  - Updated library links database

### Fixed

- **Context Menu Display** - Fixed context menus appearing outside viewport boundaries
- **Log Rotation** - Fixed unnecessary log rotation on first log entry
- **Disk Space Calculation** - Fixed type casting issues on macOS for available disk space checks
- **Error Handling** - Improved lock error handling and X-Plane path validation

### Technical

- Added ESLint configuration for better code quality
- Added Prettier configuration for consistent code formatting
- Enhanced database schema with new migration system
- Improved geo-region detection with binary continent map
- Added build workflow for automated CI/CD

## [0.8.11] - 2026-02-13

### Fixed

- Fixed minor bugs

## [0.8.10] - 2026-02-13

### Added

- **Missing Library Download Links** - Missing library dialog now supports direct download links for many common libraries.
- **Link Contribution Flow** - You can now submit missing library download links directly from the Missing Libraries dialog.
  - Added a contribution form with copy/search helpers for the selected library name.
  - Added automatic review pipeline integration for submitted links.

### Changed

- **Smarter Link Source Updates** - Missing library links now load immediately from bundled data, then refresh in the background from the latest online list.

### Fixed

- **Missing Library False Positives** - Improved library matching logic to handle case differences.
- **Library Export Parsing Reliability** - Improved `library.txt` parsing for non-UTF8 files so installed libraries are less likely to be incorrectly marked as missing.
- **Incremental Scenery Scan Accuracy** - Entering Scenery Manager now better re-evaluates library packages with incomplete cached export data.

## [0.8.9] - 2026-02-09

### Added

- **Aircraft Livery Management** - Browse and manage aircraft liveries
  - View all liveries for each aircraft with thumbnail icons
  - Preview livery images with zoom and pan controls
  - Open livery folders in file explorer
  - Delete individual liveries
- **Lua Script Management** - Manage FlyWithLua scripts
  - View all installed FlyWithLua scripts with enable/disable status
  - Toggle scripts between enabled (.lua) and disabled (.xfml) states
  - Delete scripts directly from the manager
  - Right-click context menu support for quick actions
- **DSF Tile Overlap Detection** - Identify conflicting scenery packages
  - Automatically detect packages with overlapping terrain tiles
  - Show conflicting packages in detail view
  - New filter to show only packages with conflicts
  - XPME-aware conflict detection (respects XPME load order)
- **Enhanced Filtering System** - More control over scenery display
  - New filter dropdown with checkbox-based controls
  - Filter by enabled/disabled status
  - Improved filter UX with visual indicators

### Changed

- **Release Artifacts** - All platform installers now packaged as .zip files for consistency
- **README** - Updated tagline to "The Modern X-Plane Addon Installer & Manager"
- **Plugin Display** - Show script count for FlyWithLua plugin

### Technical

- Database schema updated to v4 with original_category tracking for better package classification
- Improved asset protocol support for local file access
- Optimized CI/CD artifacts handling with compression

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

- **Faster Page Navigation** - Improved transition speed when switching to Management page
  - Views are preloaded in the background after app startup
  - Heavy components loaded asynchronously to reduce initial render time

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

- **Data Storage Location** - All app data (logs, database, cache) now stored alongside Tauri store
  - Linux: `~/.local/share/com.xfastmanager.tool/` (or `$XDG_DATA_HOME`)
- **Progress Bar Gradient Effect** - Installation progress bar now displays a smooth gradient animation
  - Gradient moves through blue to green as progress increases
  - More visually appealing and intuitive progress indication
- **Scenery Index Auto-Refresh** - Background scan now automatically detects scenery changes
  - Scenery list refreshes automatically when changes are detected
  - No manual refresh needed after external modifications

### Fixed

- **Linux Rendering Freeze** - Fixed app freezing on startup with certain GPU configurations
  - Automatically disables DMA-BUF renderer on Linux to prevent WebKitGTK rendering hangs
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

#### Lock System Integration
- **Config File Synchronization** - Lock state now syncs with `skunkcrafts_updater.cfg` files for aircraft and plugins
  - When scanning, items with `disabled|true` in their config file are automatically locked
  - When toggling lock state, the `disabled|` field is updated in the config file (if it exists)
  - Provides seamless integration with external updater tools

### Changed

#### Data Storage
- **Tauri Store Migration** - All persistent data now uses Tauri's secure store plugin instead of localStorage
  - More reliable data persistence across app updates
  - Better cross-platform compatibility
  - Improved data security

## [0.8.0] - 2026-01-28

### Added

#### Installation Progress
- **Adaptive Task List Size** - Installation progress view now adjusts task item sizes based on the number of tasks
  - Fewer tasks (1-3): Larger, more prominent display
  - Medium count (4-6): Standard size
  - Many tasks (7+): Compact view to fit more items
- **Professional Icons** - Replaced emoji icons with professional SVG icons for each addon type
  - Aircraft: Plane icon
  - Scenery: Globe icon
  - Scenery Library: Layers icon
  - Plugin: Lightning bolt icon
  - Navdata: Map icon
  - Livery: Palette icon

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

#### Scenery Index Performance
- **Faster Scenery Index** - Significantly improved performance for large scenery libraries (1000+ packages)
  - Faster loading and saving of scenery index data
  - Reduced memory usage when managing scenery
  - Smoother experience when browsing and editing scenery order

#### Data Storage
- **New Index Location** - Scenery index now stored in `scenery.db` instead of `scenery_index.json`
  - Existing users will need to rebuild their scenery index once after updating
  - Click "Rebuild Index" in Settings or wait for automatic rebuild on first launch

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

#### Unified Management Page
- **Integrated Management Interface** - New unified page for managing aircraft, plugins, navdata, and scenery
  - Tab-based navigation with sliding indicator animation
  - Page transitions adapt to navigation direction
  - Shared search functionality across all tabs

#### Aircraft Management
- **Aircraft Overview** - View and manage all installed aircraft
  - Display aircraft name, version, and livery count
  - Enable/disable aircraft
  - Delete aircraft with confirmation dialog
  - Open aircraft folder in file explorer
  - Search and filter aircraft list

#### Plugin Management
- **Plugin Overview** - View and manage all installed plugins
  - Display plugin name, version, and platform compatibility (Win/Mac/Lin/Multi)
  - Enable/disable plugins
  - Delete plugins with confirmation dialog
  - Open plugin folder in file explorer
  - Search and filter plugin list

#### Navdata Management
- **Navdata Overview** - View installed navigation data providers
  - Display provider name and AIRAC cycle information
  - Cycle status indicators: current (green), outdated (red), unknown (gray)
  - Smart AIRAC cycle detection
  - Filter to show only outdated navdata
  - Delete navdata with confirmation dialog
  - Open navdata folder in file explorer

#### Update Checking
- **Automatic Update Detection** - Check for aircraft and plugin updates
  - Detect versions and update URLs from skunkcrafts_updater.cfg
  - Parallel update checking for all supported addons
  - Visual update indicators on addon cards (current → latest version)
  - Update count badges in management tabs
  - 1-hour cache to avoid redundant checks
  - System proxy support

#### Livery Detection
- **Smart Livery Recognition** - Improved livery folder detection
  - Accurate livery count display for each aircraft

### Changed

- **Page Transitions** - Animation direction adapts to navigation direction
- **Tab Switching** - Smooth tab transition animations without loading state blocking
- **Scenery Manager** - Integrated into unified management page as "Scenery" tab (requires auto-sort enabled)

## [0.6.0] - 2026-01-22

### Added

#### First-Time User Onboarding
- **Guided Setup Wizard** - New onboarding experience for first-time users
  - Step-by-step configuration of X-Plane path
  - Installation preferences setup (addon type toggles)
  - Context menu registration option (Windows)
  - Auto-sort scenery preference
  - Delete source after install preference
  - Worry-Free installation mode preference
  - Clean, modern UI with smooth transitions

#### Scenery Index Improvements
- **Index Status Detection** - Automatically detects if scenery index needs to be created
  - Shows prompt when index doesn't exist
  - One-click index creation from home page
  - Progress feedback during index building

### Changed

- **Project Rebranding** - Renamed from "XFastInstall" to "XFast Manager"
  - Updated all references throughout the application
  - New application identifier: com.xfastmanager.app
  - Updated window title and descriptions

- **Dependency Upgrades**
  - Upgraded sevenz-rust2 from 0.10 to 0.20 for improved 7z archive handling with safer Rust implementation
  - Upgraded unrar from 0.5 to 0.5.8 for bug fixes and improved RAR archive handling

- **Code Quality Improvements**
  - Refactored scenery packs manager for better maintainability
  - Streamlined entry management and sorting mechanism
  - Enhanced error handling in updater and task control
  - Performance optimizations in multiple components

## [0.5.0] - 2026-01-21

### Added

#### Scenery Manager
- **Complete Scenery Management Interface** - New dedicated page for managing X-Plane scenery packages
  - Visual interface for viewing and organizing all scenery in Custom Scenery folder
  - Real-time sync with scenery_packs.ini file
  - Enable/disable scenery packages with toggle switches
  - Drag-and-drop reordering within and across categories
  - Search functionality to quickly find scenery packages
  - Collapsible category groups with entry counts (e.g., "机场 (162)")
  - Missing library dependency warnings with visual indicators
  - Apply changes button to save modifications to scenery_packs.ini
  - Automatic backup creation before applying changes

#### Scenery Auto-Sorting System
- **Intelligent Scenery Classification** - Automatically categorizes scenery packages
  - Analyzes DSF file headers and folder structure
  - 8 classification categories with priority-based sorting:
    1. FixedHighPriority - SAM libraries (SAM_Library, openSAM, etc.)
    2. Airport - Scenery with apt.dat files
    3. DefaultAirport - X-Plane's *GLOBAL_AIRPORTS* marker
    4. Library - Packages with library.txt but no Earth nav data
    5. Other - Unclassified scenery
    6. Overlay - Scenery with sim/overlay property
    7. Orthophotos - Ortho4XP imagery (detected by creation agent)
    8. Mesh - Terrain mesh packages
  - Parses DSF PROP section for sim/overlay, sim/creation_agent, sim/filter/aptid
  - Detects terrain references (TRET) vs object references (OBJT)
  - Recursive apt.dat search with depth limits for performance
  - Plugin detection for "Other" category classification

- **Persistent Scenery Index** - Fast caching system for scenery metadata
  - Stored at %LOCALAPPDATA%/XFast Manager/scenery_index.json (Windows)
  - Automatic cache invalidation based on directory modification time
  - Parallel scanning with rayon for large scenery collections
  - Tracks: category, sub-priority, file counts, library dependencies, tile coverage
  - Manual rebuild index button in settings

- **Earth Nav Tile Coverage Sorting** - Sorts scenery by geographic coverage
  - Counts 10-degree tile folders under Earth nav data (e.g., "+30+110")
  - Scenery with more tiles sorted before scenery with fewer tiles
  - Ensures large coverage areas load before small regional scenery

- **Alphabetical Sorting Within Categories** - Consistent ordering
  - After category priority, entries sorted by folder name (case-insensitive)
  - Provides predictable, organized scenery list

- **Auto-Sort After Installation** - Optional automatic sorting
  - Toggle in settings to enable/disable auto-sorting after scenery installation
  - Smart insertion: only inserts newly installed scenery at correct position
  - Preserves user's manual ordering of existing scenery
  - Marked as experimental feature in settings UI

- **Manual Sort Button** - Sort all scenery on demand
  - "Sort All Scenery Now" button in settings
  - Creates timestamped backup before modifying scenery_packs.ini
  - Atomic file write (temp file + rename) for safety

#### Enhanced Scenery Detection
- **SAM Library Priority** - SAM libraries automatically promoted to highest priority
  - Enhanced pattern matching: "SAM_Library", "open_SAM_library", "openSAM", "mySAM"
  - Correctly excludes airport codes (e.g., "ZSAM") and unrelated words (e.g., "sample")
  - Ensures SAM libraries load first for proper airport animations

- **Windows Shortcut Support** - Full support for .lnk files in Custom Scenery
  - Automatically resolves shortcuts to target directories
  - Shortcut target folder name used for sorting and classification
  - Works alongside symbolic links (mklink) for maximum compatibility
  - Proper handling in index updates and sorting operations

- **Symbolic Link Support** - Enhanced symlink handling
  - All file system traversal explicitly follows symbolic links
  - Applies to: apt.dat search, DSF file search, texture counting, validation
  - Debug logging shows symlink targets for troubleshooting

#### UI/UX Improvements
- **Enhanced Theme Transitions** - Smoother theme switching experience
  - Improved animation timing and easing
  - Better visual feedback during theme changes

- **Search Functionality** - Quick scenery filtering
  - Real-time search across all scenery packages
  - Automatically expands collapsed groups when search matches found
  - Clear search button for quick reset

- **Grouped Display** - Organized category view
  - 8 collapsible category groups with rotating chevron icons
  - Group collapse state persists to localStorage
  - Smooth expand/collapse animations
  - Visual separation between categories

#### Performance Optimizations
- **Runtime Log Level Control** - Dynamic logging configuration
  - New set_log_level command (Debug/Info/Error)
  - Parallel processing enabled when not in debug mode
  - Sequential processing only for debug builds (ordered logs)
  - Significantly faster indexing in production

- **Scenery Classification Performance** - Major speed improvements
  - Only scans first DSF file found (not all DSF files)
  - apt.dat search limited to Earth nav data folder with max depth 5
  - Texture counting stops after finding 5 files (sufficient for classification)
  - Dramatically faster indexing for large scenery packages (Ortho4XP, etc.)

### Changed
- **Scenery Installation Flow** - Optional auto-sorting integration
  - install_addons command now accepts auto_sort_scenery parameter
  - Newly installed scenery automatically sorted if enabled in settings

## [0.4.1] - 2026-01-17

### Fixed
- **Conflicts between Scenery and SceneryLibrary** - Improve logic to process this situation

## [0.4.0] - 2026-01-17

### Added
- **Automatic Update Check** - GitHub-based version checking with 24-hour cache
  - Auto-check on app startup (can be disabled in settings)
  - Manual check button in settings page
  - Support for stable releases and pre-releases (Beta/RC)
  - Update notification banner on home page with download link
- **Update Settings** - Comprehensive update configuration in settings page
  - Toggle auto-check on startup
  - Toggle pre-release inclusion
  - View last check time
  - Expandable section with feature explanation

### Changed
- **Settings Layout** - Moved auto update check section above logs section
- **Update Banner Design** - Unified styling with other notification banners
- **Button Labels** - Changed "View Details" to "Download" for clarity

### Fixed
- **Compilation Warnings** - Resolved all Rust compiler warnings
- **Update Check Errors** - Graceful handling when no stable releases exist

### Dependencies
- Added `semver`, `reqwest`, `opener`, `tauri-plugin-http`

## [0.3.0] - 2026-01-17

### ⚡ Performance Improvements

#### 🚀 Backend Optimizations (Phase 1 - High Priority)
- **Deduplication Algorithm** - Optimized from O(n²) to O(n log n) complexity
  - Sorts items by path depth before processing
  - Uses single-pass algorithm with efficient path checking
  - Significant speedup for large addon collections (30-50% faster for large collections)
- **Directory Skip Checking** - Improved from O(n*m) to O(d) average case
  - Uses path ancestors with HashSet lookups
  - Reduces redundant string comparisons during directory traversal
- **Plugin Directory Detection** - Optimized 7 locations with O(n*m) complexity
  - Added helper functions for efficient path checking
  - Uses ancestor-based lookups instead of iteration
- **Reduced String Cloning** - Minimized unnecessary heap allocations
  - Uses references and helper functions where possible
  - Lower memory pressure during scanning

#### 🗄️ Caching Enhancements (Phase 2 - Medium Priority)
- **Directory Size Caching** - Avoid repeated directory traversals
  - Thread-safe cache with 5-minute TTL
  - Validates cache using directory modification time
  - Significantly faster for repeated size checks
- **Extended Archive Metadata Cache** - Store file count alongside size
  - Enhanced ArchiveMetadata structure
  - Better cache utilization for archive analysis
- **Pre-compiled Glob Patterns** - Eliminate repeated pattern compilation
  - CompiledPatterns struct for efficient reuse
  - Patterns compiled once per operation instead of per file
  - Faster config file matching during backup/restore

#### 📊 Monitoring & Observability (Phase 3 - Optional)
- **Enhanced Performance Metrics** - Comprehensive operation tracking
  - Cache hit/miss rates with atomic counters
  - Files scanned and archives processed counters
  - Bytes processed tracking
  - PerformanceStats structure for statistics export
- **Operation Timing** - Built-in timer for performance profiling
  - OperationTimer with automatic logging
  - Duration tracking for debugging
  - Optional auto-logging in debug builds

#### 🔍 Hash Verification Optimization
- **Smart Hash Collection** - Verification preferences now truly control hash operations
  - When verification is disabled for a file type, hash collection is completely skipped
  - No file reading or hash computation for disabled types
  - Significant time savings for large files and directories
  - Clear logging to indicate when hash collection is skipped
- **Optimized Verification Flow** - Removed unnecessary hash computation for 7z archives
  - Simplified logic when verification is disabled
  - No hash computation attempts when verification is turned off
  - Better performance for installations with verification disabled

**Expected Impact**: 30-50% faster addon analysis for large collections, reduced I/O overhead, better observability, significant time savings when verification is disabled

### ✨ Added

#### 🔧 Feature Completions
- **RAR Single-File Retry** - Implemented file verification retry for RAR archives
  - Full re-extraction to temp directory when verification fails
  - Password support for encrypted RAR files
  - Proper internal_root handling
  - Automatic cleanup of temporary files

#### 🎨 UI/UX Improvements
- **Failed Tasks Modal** - New dedicated modal for viewing failed installation tasks
  - Detailed error information for each failed task
  - Categorized error messages with icons
  - Expandable error details
  - Better error message categorization (20+ error types)
  - Smooth animations and transitions
- **Improved Completion View** - Redesigned installation completion interface
  - Cleaner layout with better visual hierarchy
  - "View Details" button for failed tasks instead of inline list
  - More compact and professional appearance
  - Better handling of partial failures
- **Password Modal Refinement** - Improved password input interface
  - More compact design with better spacing
  - Improved visual feedback
  - Better button states and interactions
  - Cleaner typography and layout
- **Platform Detection** - Automatic platform detection at app startup
  - Detects Windows/macOS/Linux
  - Checks context menu registration status (Windows only)
  - Stores platform state in app store
  - Better conditional UI rendering

#### 🌐 Internationalization
- **Enhanced Error Messages** - Expanded error message translations
  - 20+ categorized error types in both English and Chinese
  - More specific error reasons (password, path traversal, corruption, etc.)
  - Better user understanding of failure causes

### 🐛 Fixed

#### 🔧 Bug Fixes
- **Event Listener Cleanup** - Fixed memory leak in Home.vue
  - Properly unregister all event listeners on component unmount
  - Added cleanup for source-deletion-skipped listener
  - Prevents memory leaks during navigation

### 🧹 Maintenance
- **Documentation Cleanup** - Removed outdated documentation files
  - Removed DEVELOPER_GUIDE.md (content moved to CLAUDE.md)
  - Removed IMPLEMENTATION_STATUS.md (completed features)
  - Removed PLAN.md (completed tasks)
  - Removed USER_GUIDE.md (will be replaced with online documentation)
  - Streamlined project documentation structure
- **Test Code Refactoring** - Improved test code maintainability
  - Added helper functions for creating test objects
  - Reduced code duplication in tests
  - Cleaner and more readable test cases

## [0.2.0] - 2026-01-16

### ✨ Added

#### 🗑️ Delete Source After Install
- **Automatic Source Cleanup** - New option to automatically delete source files after successful installation
  - Only deletes files from successfully completed tasks
  - Only deletes files where addons were detected
  - Files/folders with no detected addons are preserved
  - Failed installation tasks won't trigger deletion
- **Smart Parent Directory Detection** - Prevents deletion when detected addon root is a parent of the input path
  - Example: Dragging `PLUGIN/win_x64` but detecting `PLUGIN` as root won't delete the directory
  - Shows notification when deletion is skipped for safety
- **Settings UI** - New expandable section in Settings with detailed feature explanation
  - Toggle switch to enable/disable the feature
  - Four-point feature explanation with benefits
  - Only visible on Windows systems

#### 🛡️ Enhanced Safety Features
- **Disk Root Protection** - Automatically ignores and filters out addons detected at disk root directories
  - Prevents accidental installation from `C:\`, `D:\`, `/`, etc.
  - Logs warning and adds to error list
  - Cross-platform support (Windows and Unix/Linux)

#### 📋 Windows Integration Improvements
- **Expandable Details** - Windows Integration section now shows detailed feature explanation
  - Click to expand/collapse
  - Four-point benefit list
  - Better user understanding of the feature

#### 🔧 UI/UX Improvements
- **Dynamic Version Display** - Settings page now shows app version from `Cargo.toml`
  - Automatically synced with build version
  - No more hardcoded version numbers
- **Better Installation Mode Description** - Clarified installation modes in CHANGELOG
  - Separated "New Installation" from existing target scenarios
  - Clear distinction between Clean Install and Direct Overwrite

### 🐛 Fixed
- **Button Click Issues** - Fixed skip and cancel task buttons not responding
  - Corrected variable names (`toastStore` → `toast`, `modalStore` → `modal`)
  - Proper toast notification methods (`toast.info()`, `toast.error()`)
- **TypeScript Warnings** - Resolved all TypeScript type warnings
  - Fixed `import.meta.env` type issues by adding `vite-env.d.ts`
  - Removed unused variables in `ConfirmModal.vue`
  - Fixed drag-drop event type checking in `Home.vue`
  - Fixed unused imports in `Settings.vue`
  - Fixed log level type assertion

### 🔄 Changed
- **Production Environment Check** - Updated from `import.meta.env.PROD` to `import.meta.env.MODE === 'production'`
- **Atomic Installation Position** - Moved atomic installation toggle below Windows Integration section for better organization

### 🏗️ Technical
- **Backend Architecture**:
  - Added `original_input_path` field to `InstallTask` and `DetectedItem` models
  - Implemented `delete_source_file()` method in installer with parent directory checking
  - Added `is_disk_root()` helper function in analyzer
  - Enhanced `install_addons` command to accept `delete_source_after_install` parameter
- **Frontend Architecture**:
  - Added `deleteSourceAfterInstall` state to app store with localStorage persistence
  - Implemented event listener for `source-deletion-skipped` notifications
  - Added internationalization support for new features (English and Chinese)
- **Type Safety**:
  - Created `vite-env.d.ts` for proper Vite type definitions
  - Fixed all TypeScript compilation warnings

## [0.1.1] - 2026-01-16

### 🎉 Initial Release

XFast Manager is a modern, intelligent X-Plane Addon Auto Manager that makes installing aircraft, scenery, plugins, and navigation data effortless.

---

### ✨ Core Features

#### 🎯 Smart Addon Detection
Automatically identifies and categorizes X-Plane addons:
- **Aircraft** - Detects `.acf` files
- **Scenery** - Recognizes `library.txt` or `.dsf` files with proper Earth nav data structure
- **Plugins** - Finds `.xpl` files with platform-specific subdirectory support (win_x64, mac_x64, lin_x64, etc.)
- **Navigation Data** - Identifies `cycle.json` files with GNS430 detection
- **Intelligent Deduplication** - Automatically removes nested duplicates (e.g., plugin inside aircraft folder)

#### 📦 Drag & Drop Installation
- **Simple Interface** - Just drag files or folders into the window
- **Batch Processing** - Install multiple addons at once
- **Archive Support** - Works with `.zip`, `.7z`, and `.rar` files
- **Folder Support** - Directly install from uncompressed folders
- **Windows Context Menu** - Right-click any file/folder and select "Install to X-Plane" (Windows only)
- **Command Line Support** - Launch with file paths as arguments

#### 🔐 Password-Protected Archives
- **Full Encryption Support**:
  - ZIP: Both ZipCrypto and AES encryption
  - 7z: Full password support
  - RAR: Full password support
- **Unified Password Mode** - Use one password for all archives
- **Individual Passwords** - Set different passwords for each archive
- **Smart Retry** - Automatically retry with correct password

---

### 🛠️ Installation Options

#### 📋 Installation Modes

**New Installation** (Target doesn't exist)
- Direct installation to target directory
- No conflicts or overwrites
- Fastest installation method
- Automatically selected when target folder doesn't exist

**When Target Already Exists** - Choose how to handle existing addons:

**Clean Install** (Recommended)
- Deletes old folder and installs fresh copy
- **Aircraft Backup Features**:
  - Automatic livery backup and restoration
  - Configuration file backup with customizable patterns (e.g., `*_prefs.txt`, `*.cfg`)
  - Skips existing liveries to preserve new ones
  - Pattern-based config file matching with glob support

**Direct Overwrite**
- Keeps existing files
- Only overwrites matching files
- Preserves files not in the new addon

#### ⚛️ Atomic Installation Mode
Advanced installation mode for maximum safety:
- **Three Scenarios**:
  - Fresh Install: Direct atomic move to target
  - Clean Install: Backup → Move → Restore → Cleanup
  - Overwrite Install: File-by-file atomic merge
- **Safety Features**:
  - Temporary staging directory on same drive
  - Automatic rollback on failure
  - Disk space validation (minimum 1GB required)
  - Symbolic link preservation (Unix and Windows)
  - Automatic cleanup of temporary files
- **Progress Reporting**: Real-time updates for each phase (backup, move, restore, cleanup)

#### 🎛️ Addon Type Filtering
- **Selective Installation** - Choose which addon types to auto-install
- **Type Toggles** - Enable/disable Aircraft, Scenery, Plugins, Navdata individually
- **Quick Toggle** - Enable/disable all types at once
- **Persistent Settings** - Preferences saved between sessions

---

### 🔍 Intelligence & Safety

#### 🛡️ Security Features
- **Path Traversal Protection** - Prevents malicious archives from extracting outside target directory
- **Compression Bomb Detection**:
  - Maximum extraction size: 20GB per archive
  - Maximum compression ratio: 100:1
  - Warning dialog for suspicious archives
- **Size Validation** - Alerts for unusually large archives
- **Safe Extraction** - All paths sanitized before extraction

#### ✅ File Integrity Verification
Post-installation verification ensures files are correctly installed:
- **Multiple Hash Algorithms**: MD5, SHA-1, SHA-256, CRC32
- **Configurable by Format**:
  - ZIP archives verification
  - 7z archives verification
  - RAR archives verification (note: not supported due to library limitation)
  - Directory verification
- **Smart Retry** - Automatically retries failed files
- **Detailed Statistics** - Shows verification progress and results
- **Progress Tracking** - Real-time verification progress (0-100%)

#### ⚠️ Conflict Detection
- **Existing Installation Warnings** - Alerts when target folder already exists
- **Navdata Cycle Display** - Shows existing and new cycle information
- **Size Warnings** - Displays estimated extraction size
- **Confirmation Required** - User must confirm before overwriting

---

### 📊 Progress & Monitoring

#### 📈 Real-Time Progress Tracking
Comprehensive progress information during installation:
- **Overall Progress** - Percentage complete (0-100%)
- **Current File** - Name of file being processed
- **Bytes Processed** - Data transfer progress
- **Installation Phases**:
  - Calculating: Analyzing files and preparing tasks
  - Installing: Copying/extracting files
  - Verifying: Checking file integrity (with sub-progress 0-100%)
  - Finalizing: Completing installation
- **Task Counter** - Current task number and total tasks
- **Atomic Installation Phases** - Detailed progress for backup, move, restore, cleanup

#### 🎮 Task Control
Full control over the installation process:
- **Cancel All Tasks** - Stop entire installation process
- **Skip Current Task** - Skip problematic addon and continue with next
- **Automatic Cleanup** - Removes partially installed files when cancelled/skipped
- **Confirmation Dialogs** - Warns about data loss before cancelling/skipping
- **Clean Install Warning** - Special warning when original files may be lost

#### 📉 Performance Metrics
Monitor installation performance:
- **Installation Speed** - MB/s throughput
- **Cache Hit Rate** - Archive metadata cache efficiency
- **Files Processed** - Total number of files installed
- **Time Elapsed** - Installation duration

---

### 🎨 User Interface

#### 🌍 Multi-Language Support
- **Languages**: English and Chinese
- **Auto-Detection** - Automatically detects system language
- **Manual Switch** - Change language anytime via language switcher
- **Bilingual Logs** - Log messages in selected language

#### 🌓 Theme Support
- **Light Theme** - Clean, bright interface
- **Dark Theme** - Easy on the eyes
- **Persistent** - Theme preference saved between sessions
- **Smooth Transitions** - Animated theme switching

#### 💬 Notifications & Dialogs
- **Toast Notifications** - Non-intrusive status messages
- **Confirmation Modals** - For critical operations (install, cancel, skip)
- **Error Modals** - Detailed error information with copy button
- **Password Modals** - Secure password input for encrypted archives
- **Completion View** - Summary of successful and failed installations

#### 📋 Task List Display
Comprehensive task information before installation:
- **Addon Type** - Visual icon and label
- **Addon Name** - Display name
- **Source Path** - Original file/folder location
- **Target Path** - Installation destination
- **File Size** - Estimated size after extraction
- **Conflict Warnings** - Existing installation alerts
- **Navdata Cycle Info** - Shows existing and new cycle numbers
- **Enable/Disable Toggle** - Skip individual tasks
- **Install Mode Selection** - Choose Clean Install or Direct Overwrite per task
- **Backup Options** - Configure livery and config file backup (for aircraft)

---

### 📝 Logging & Debugging

#### 📄 Comprehensive Logging System
- **Thread-Safe Logging** - Concurrent logging from multiple threads
- **Automatic Rotation** - Logs rotate at 3MB, trim to 2MB
- **Bilingual Messages** - Logs in English or Chinese based on setting
- **Log Levels**:
  - Basic: Errors and warnings only
  - Full: Includes info messages
  - Debug: Verbose debugging information
- **Log Location**: `%LOCALAPPDATA%/XFast Manager/logs/xfastmanager.log` (Windows)
- **Frontend Integration** - Frontend logs sent to backend

#### 🔧 Log Management
- **View Recent Logs** - Display last 50 log entries in UI
- **Refresh Logs** - Update log display
- **Copy Logs** - Copy all logs to clipboard
- **Open Log Folder** - Open log directory in file explorer
- **Export All Logs** - Get complete log file content
- **Log Path Display** - Shows current log file location

---

### ⚡ Performance Optimizations

#### 🚀 Speed Enhancements
- **Archive Metadata Caching**:
  - Thread-safe DashMap cache
  - 5-minute TTL (Time To Live)
  - Caches uncompressed sizes
  - Automatic cleanup of expired entries
  - Tracks cache hit/miss rates
- **Parallel Processing**:
  - Multi-threaded ZIP extraction (each thread has own ZipArchive instance)
  - Parallel file copying using rayon (for 7z/RAR and directories)
  - Parallel path scanning during analysis
  - Concurrent addon detection
- **Optimized File I/O**:
  - 4MB buffer size for maximum throughput
  - Memory-efficient streaming
- **Async Command Execution**:
  - All Tauri commands run in background thread pool via tokio
  - Non-blocking UI during long operations
  - Uses `tokio::task::spawn_blocking` for CPU-intensive tasks
- **Smart Scanning**:
  - Maximum directory depth: 15 levels
  - Prevents excessive recursion
  - Early termination for invalid paths

---

### 🖥️ Platform Support

#### 🌐 Cross-Platform
- **Windows** - Full support with context menu integration
- **macOS** - Full support with native file dialogs
- **Linux** - Full support with GTK integration

#### 🔧 Platform-Specific Features

**Windows**
- Context menu registration (uses HKEY_CURRENT_USER, no admin required)
- GetDiskFreeSpaceExW API for accurate disk space check
- Windows-specific symlink handling (file vs directory)

**Unix/Linux/macOS**
- statvfs-based disk space check
- Unix file permission preservation
- Unix symlink handling
- Native file dialogs

---

### 🏗️ Technical Architecture

#### 📐 Three-Layer Architecture
1. **Rust Backend** - File system operations, addon detection, installation
2. **Tauri Bridge** - Command-based RPC between Rust and JavaScript
3. **Vue 3 Frontend** - UI layer with Pinia state management

#### 🛠️ Technology Stack
- **Backend**: Rust with Tauri 2 framework
- **Frontend**: Vue 3 with Composition API
- **State Management**: Pinia stores
- **Internationalization**: vue-i18n
- **Styling**: Tailwind CSS v4
- **Build System**: Vite (frontend), Cargo (backend)

#### 📦 Key Dependencies
- **Archive Handling**: zip, sevenz-rust2, unrar
- **Parallel Processing**: rayon, tokio
- **Caching**: DashMap
- **Logging**: Custom logger with rotation
- **File Operations**: walkdir, tempfile
- **Hashing**: crc32fast, sha2

---

### 🔒 Security

- **Path Traversal Protection** - Sanitizes all file paths during extraction
- **Compression Bomb Detection** - Validates extraction size (20GB max) and compression ratio (100:1 max)
- **Safe Archive Extraction** - Prevents extraction outside target directory
- **Input Validation** - All user inputs validated before processing
- **Error Handling** - Comprehensive error handling with user-friendly messages

---

### 📚 Documentation

- **In-App Help** - Tooltips and descriptions throughout the UI
- **Pattern Help** - Glob pattern examples for config file backup
- **Mode Descriptions** - Clear explanations of Clean Install vs Direct Overwrite
- **Atomic Install Explanation** - Detailed benefits and notes
- **Error Messages** - Clear, actionable error descriptions

---

### 🎯 User Experience

- **Intuitive Interface** - Clean, modern design
- **Minimal Configuration** - Works out of the box with sensible defaults
- **Smart Defaults** - Recommended settings pre-selected
- **Visual Feedback** - Animated text, progress bars, status indicators
- **Responsive Design** - Adapts to different window sizes
- **Keyboard Shortcuts** - Quick access to common actions
- **Persistent Settings** - All preferences saved automatically
