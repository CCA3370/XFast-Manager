# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

XFastInstall is a modern, intelligent X-Plane addon installer that makes installing aircraft, scenery, plugins, and navigation data effortless.

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
- **Log Location**: `%LOCALAPPDATA%/XFastInstall/logs/xfastinstall.log` (Windows)
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

[unreleased]: https://github.com/CCA3370/XFastInstall-Tauri/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/CCA3370/XFastInstall-Tauri/releases/tag/v0.1.0
