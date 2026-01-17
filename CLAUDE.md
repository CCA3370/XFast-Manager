# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

XFastInstall is a cross-platform X-Plane addon installer built with Tauri 2, Rust, and Vue 3. It provides intelligent detection and installation of Aircraft, Scenery, Plugins, and Navdata addons through drag-and-drop or Windows context menu integration.

## Prerequisites

- Node.js 20+
- Rust (latest stable)
- Platform-specific build dependencies:
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools

## Development Commands

```bash
# Install dependencies
npm install

# Development mode (hot reload)
npm run tauri:dev

# Production build
npm run tauri:build

# Frontend only (Vite dev server)
npm run dev

# Generate icon (if needed)
npm run generate:icon

# Rust development commands
cd src-tauri

# Run Rust tests
cargo test

# Run Rust tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Format Rust code
cargo fmt

# Check code without building
cargo check

# Run Clippy linter
cargo clippy --all-targets --all-features

# Build documentation
cargo doc --open

# Check for unused dependencies (requires cargo-udeps)
cargo +nightly udeps

# View dependency tree
cargo tree
```

## Architecture Overview

### Three-Layer Architecture

1. **Rust Backend (src-tauri/src/)**: Handles file system operations, addon detection, and installation
2. **Tauri Bridge**: Command-based RPC between Rust and JavaScript
3. **Vue 3 Frontend (src/)**: UI layer with Pinia state management and vue-i18n

### Performance Optimizations

The backend includes several performance optimizations:

- **Archive Metadata Caching** (cache.rs): Thread-safe DashMap cache with 5-minute TTL
  - Caches uncompressed sizes
  - Reduces repeated archive scanning
  - Tracks cache hit/miss rates
- **Optimized File I/O** (installer.rs): 4MB buffer size for maximum throughput
- **Parallel ZIP Extraction** (installer.rs): Multi-threaded ZIP decompression
  - Each thread opens its own ZipArchive instance for parallel file extraction
  - Significantly speeds up large ZIP files on multi-core systems
  - Works with both encrypted and unencrypted ZIP files
- **Parallel File Copying** (installer.rs): Uses rayon to copy multiple files simultaneously on multi-core systems
  - Significantly speeds up 7z/RAR extraction (which extracts to temp then copies)
  - Parallel directory copying for direct folder installations
- **Async Command Execution** (lib.rs): All Tauri commands run in background thread pool via tokio
  - Prevents UI blocking during long operations
  - Uses `tokio::task::spawn_blocking` for CPU-intensive tasks
- **Directory Scan Limits** (scanner.rs): Max depth of 15 levels to prevent excessive recursion
- **Parallel Path Scanning** (analyzer.rs): Uses rayon for concurrent path analysis

### Rust Backend Structure

The backend follows a modular pipeline architecture:

- **scanner.rs**: File system traversal and marker detection
  - Scans directories and archives (.zip, .7z, .rar) recursively
  - Detects addon types by marker files:
    - Aircraft: `*.acf` files
    - Scenery: `library.txt` or `*.dsf` files
    - Plugins: `*.xpl` files (handles platform-specific subdirectories)
    - Navdata: `cycle.json` files with GNS430 detection
  - Handles password-protected archives (returns PasswordRequiredError if password needed)

- **analyzer.rs**: Deduplication and task generation
  - Receives `DetectedItem[]` from scanner
  - Uses parallel processing (rayon) for scanning multiple paths
  - Deduplicates nested addons (e.g., plugin inside aircraft folder)
  - Creates `InstallTask[]` with target paths and conflict detection
  - Handles password-protected archives (tracks passwords per archive)
  - Determines target directories:
    - Aircraft → `X-Plane/Aircraft/`
    - Scenery → `X-Plane/Custom Scenery/`
    - Plugin → `X-Plane/Resources/plugins/`
    - Navdata → `X-Plane/Custom Data/` or `X-Plane/Custom Data/GNS430/`

- **installer.rs**: File operations execution
  - Copies directories or extracts archives to target paths
  - Handles .zip, .7z, and .rar extraction with full password support
  - ZIP: Supports both ZipCrypto and AES encryption
  - Preserves Unix file permissions
  - Implements safety checks: max extraction size (20GB) and compression ratio (100:1)
  - Supports overwrite mode (deletes existing directory before install using robust removal)

- **registry.rs**: Windows-specific context menu registration (Windows only)
  - Uses HKEY_CURRENT_USER for non-admin access (changed from HKEY_CLASSES_ROOT)

- **models.rs**: Shared data types between all modules
  - `InstallTask`: Includes fields for password, overwrite, size warnings, and Navdata cycle info
  - `AnalysisResult`: Returns tasks, errors, and password_required list

- **cache.rs**: Archive metadata caching system
  - Thread-safe concurrent cache using DashMap
  - 5-minute TTL for cached entries
  - Automatic cleanup of expired entries
  - Tracks cache statistics

- **performance.rs**: Performance monitoring and metrics
  - Tracks bytes and files processed
  - Measures operation throughput (MB/s)
  - Records cache hit/miss rates
  - Provides performance statistics

- **logger.rs**: Logging system with i18n support
  - Thread-safe file logging with automatic rotation (3MB max, trims to 2MB)
  - Bilingual support (English/Chinese) with locale switching
  - Log location: `%LOCALAPPDATA%/XFastInstall/logs/xfastinstall.log` (Windows)
  - Provides log reading and folder opening functionality

### Frontend Structure

- **Views**:
  - `Home.vue`: Main drag-and-drop interface, handles CLI args event
  - `Settings.vue`: X-Plane path configuration, context menu registration

- **State Management (Pinia stores)**:
  - `app.ts`: X-Plane path, current tasks, loading states
  - `toast.ts`: Toast notification queue
  - `theme.ts`: Dark/light theme persistence
  - `modal.ts`: Modal state management

- **Internationalization (i18n/)**:
  - Auto-detects system language (zh/en)
  - Language files: `zh.ts`, `en.ts`

### Data Flow

1. User drops files → Frontend validates X-Plane path
2. Frontend calls `analyze_addons` command → Rust Scanner scans paths
3. Scanner returns `DetectedItem[]` → Analyzer deduplicates and generates `InstallTask[]`
4. Frontend displays tasks with conflict warnings → User confirms
5. Frontend calls `install_addons` command → Installer executes file operations

### Tauri Commands

All Rust functions exposed to frontend (defined in lib.rs):
- `get_cli_args()`: Returns command-line arguments (for context menu integration)
- `get_platform()`: Returns OS name
- `analyze_addons(paths, xplane_path, passwords)`: Analyzes dropped files and returns install tasks (with optional password map)
- `install_addons(tasks)`: Executes installation
- `register_context_menu()`: Registers Windows context menu (Windows only)
- `unregister_context_menu()`: Unregisters context menu (Windows only)
- `is_context_menu_registered()`: Checks if context menu is registered
- `log_from_frontend(level, message, context)`: Logs messages from frontend
- `get_recent_logs(lines)`: Returns recent log entries
- `get_log_path()`: Returns log file path
- `get_all_logs()`: Returns all log content
- `open_log_folder()`: Opens log folder in system file explorer
- `set_log_locale(locale)`: Sets logging locale (en/zh)
- `check_path_exists(path)`: Checks if a path exists
- `validate_xplane_path(path)`: Validates X-Plane installation directory

### CLI Args Integration

When launched via Windows context menu, arguments are passed via CLI:
- Rust emits `cli-args` event in setup hook (lib.rs:67)
- Frontend listens in Home.vue and auto-triggers analysis

## Important Implementation Notes

### Addon Detection Logic

**Deduplication Algorithm** (analyzer.rs:46-84):
- If a detected addon is a subdirectory of another, the parent wins
- Example: If both `A330/` (Aircraft) and `A330/plugins/fms/` (Plugin) are detected, only `A330/` is kept
- Prevents installing nested components separately

**Archive Handling**:
- ZIP, 7z, and RAR files are scanned without extraction during analysis
- Password-protected archives return a `password_required` list in AnalysisResult
  - All three formats (ZIP, 7z, RAR) support password-protected archives
  - ZIP supports both ZipCrypto (legacy) and AES encryption
- During installation, archives are extracted to target directories
- Archive filename becomes the addon folder name
- Safety limits: 20GB max extraction size, 100:1 max compression ratio

**Plugin Platform Detection** (scanner.rs:308-316):
- Detects platform folders: `32`, `64`, `win`, `lin`, `mac`, `win_x64`, `mac_x64`, `lin_x64`
- Goes up one directory level to find the actual plugin root
- Example: `MyPlugin/win_x64/MyPlugin.xpl` → installs `MyPlugin/`

**Scenery DSF Detection** (scanner.rs:226-246):
- `.dsf` files are inside `Earth nav data/` subdirectories
- Goes up 3 levels from `.dsf` file to find scenery root
- Example: `Scenery/Earth nav data/+50+120/file.dsf` → installs `Scenery/`

### Type System

TypeScript types (src/types/index.ts) mirror Rust types (src-tauri/src/models.rs):
- Use `#[serde(rename_all = "camelCase")]` in Rust to match JS naming
- `AddonType` enum uses PascalCase serialization on both sides
- All Tauri command parameters/returns must be Serialize/Deserialize

### State Persistence

- X-Plane path: localStorage (`xplanePath` key)
- Theme: localStorage (handled by theme store)
- Language: Auto-detected on each launch, not persisted

## Testing

- Rust unit tests exist in analyzer.rs (deduplication test)
- Run Rust tests: `cd src-tauri && cargo test`
- Run with output: `cd src-tauri && cargo test -- --nocapture`
- No frontend tests currently exist

## Changelog Requirements

**IMPORTANT**: After completing any changes, you MUST update CHANGELOG.md following the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

### Guidelines

- Add entries under the `[Unreleased]` section
- Use appropriate categories:
  - `Added` - New features
  - `Changed` - Changes to existing functionality
  - `Deprecated` - Soon-to-be removed features
  - `Removed` - Removed features
  - `Fixed` - Bug fixes
  - `Security` - Security improvements
- Describe the **user-facing effect**, not implementation details
- Focus on **what changed** and **why it matters** to users
- Use clear, concise language
- Include relevant context (e.g., which component, which setting)

### Examples

**Good entries:**
- `Added automatic update check with 24-hour cache`
- `Fixed memory leak in event listener cleanup`
- `Changed context menu registration to use HKEY_CURRENT_USER for non-admin access`
- `Removed outdated documentation files`

**Bad entries (too technical):**
- `Refactored deduplication algorithm to use O(n log n) complexity`
- `Added tokio::task::spawn_blocking for CPU-intensive tasks`
- `Fixed variable name from toastStore to toast`

### When to Update

Update CHANGELOG.md for:
- New features or functionality
- Bug fixes
- UI/UX improvements
- Breaking changes
- Dependency updates (if significant)
- Performance improvements (if user-noticeable)

Do NOT update for:
- Code refactoring without user-visible changes
- Internal code cleanup
- Comment or documentation updates in code
- Minor code style changes

## Key Dependencies

**Rust**:
- `tauri`: v2 framework
- `walkdir`: Directory traversal
- `zip`: ZIP archive handling
- `sevenz-rust2`: 7z archive handling
- `unrar`: RAR archive handling
- `rayon`: Parallel processing
- `winreg`: Windows registry (Windows only)

**Frontend**:
- `vue`: v3.5+ with Composition API
- `pinia`: State management
- `vue-i18n`: Internationalization
- `@vueuse/core`: Vue utilities
- `tailwindcss`: v4 styling

## Platform-Specific Notes

**Windows**:
- Context menu registration uses HKEY_CURRENT_USER (no admin required)
- Uses winreg crate (registry.rs)

**Linux**:
- Requires libwebkit2gtk-4.1-dev build dependency
- Unix file permissions preserved during extraction

**macOS**:
- Standard Tauri requirements (Xcode Command Line Tools)

## Recent Important Changes

- **Context Menu Registration**: Changed from HKEY_CLASSES_ROOT to HKEY_CURRENT_USER for non-admin access
- **Archive Support**: Added RAR support and full password handling for all archive formats (ZIP, 7z, RAR)
  - ZIP now supports both ZipCrypto and AES encryption with password
- **Safety Features**: Added extraction size limits and compression ratio checks
- **Task Management**: Added `enabled` state tracking for tasks (can be toggled on/off)
- **Directory Removal**: Implemented robust directory removal with retry logic for overwrite mode
- **Navdata Enhancement**: Added support for Navdata cycle information display
- **Security Improvements**: Fixed path traversal vulnerabilities in 7z and RAR extraction
