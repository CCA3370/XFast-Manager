# XFast Manager

<p align="center">
  <img src="public/icon.png" width="128" height="128" alt="XFast Manager">
</p>

<p align="center">
  <strong>The Modern X-Plane Addon Manager</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#download">Download</a> •
  <a href="#usage">Usage</a> •
  <a href="#developer-guide">Developer Guide</a> •
  <a href="#license">License</a>
</p>

---

XFast Manager makes installing and managing X-Plane addons effortless. Drag and drop any addon file, and it's automatically installed to the right place. Available for Windows, macOS, and Linux.

---

## Features

### Drag & Drop Installation

Simply drag addon files into the window. XFast Manager automatically detects what you're installing and puts it in the correct location:

- **Aircraft** - Goes to `Aircraft/` folder
- **Scenery** - Goes to `Custom Scenery/` folder
- **Scenery Libraries** - Resource libraries for other scenery
- **Plugins** - Goes to `Resources/plugins/` folder
- **Navigation Data** - Supports GNS430 and Navigraph formats
- **Liveries** - Matched with the correct aircraft automatically

### Archive Support

Install directly from compressed files without manual extraction:

| Format | Encrypted Archives |
|--------|-------------------|
| ZIP | Fully supported |
| 7z | Fully supported |
| RAR | Fully supported |
| Folders | Direct installation |

### Unified Management

View and manage everything you've installed in one place:

- **Aircraft** - See all aircraft with version info and livery counts. Enable, disable, or remove with one click.
- **Plugins** - View platform compatibility at a glance. Quickly enable or disable any plugin.
- **Navigation Data** - See AIRAC cycle status. Get notified when data is outdated.
- **Scenery** - Visual list of all scenery packages. Drag to reorder loading priority.

### Smart Scenery Sorting

Keep your scenery loading in the optimal order:

- Automatically organizes scenery by type (airports, libraries, overlays, ortho, mesh)
- SAM libraries always load first for proper animations
- Drag and drop to fine-tune the order
- Changes saved directly to `scenery_packs.ini`

### Installation Options

Choose how to handle existing addons:

- **Fresh Install** - Quick installation when no conflicts exist
- **Clean Install** - Removes old version first, automatically backs up your liveries and settings
- **Overwrite** - Keeps existing files, only updates changed ones

Additional options:
- Automatically delete source files after successful installation
- Verify file integrity after installation
- Atomic installation mode for maximum safety with automatic rollback on failure

### Update Detection

Stay up to date without manual checking:

- See available updates for aircraft and plugins
- Navigation data shows current vs outdated status
- Filter views to show only items with updates
- App notifies you when a new version is available

### Safety Features

Install with confidence:

- Warning when an addon already exists at the target location
- Protection against malicious archive contents
- Large file warnings before extraction
- Confirmation required before overwriting

### Interface

A clean, modern experience:

- Dark and light themes
- English and Chinese languages (auto-detected)
- Real-time installation progress
- Skip or cancel individual tasks during batch installation
- Detailed logs for troubleshooting

### Windows Integration

Right-click any file or folder and select "Install to X-Plane" to install instantly. No administrator privileges required.

---

## Download

Get the latest version from [Releases](https://github.com/CCA3370/XFast-Manager/releases).

### System Requirements

- Windows 10/11 (x64)
- macOS 10.15+ (Intel & Apple Silicon)
- Linux (x64, requires GTK3)

### First Launch

1. Run the installer
2. Follow the setup wizard to set your X-Plane path
3. Configure your preferences (optional)

---

## Usage

### Installing Addons

**Drag & Drop:**
1. Drag a ZIP, 7z, RAR file, or folder into the window
2. Review detected addons and adjust options if needed
3. Click "Install"

**Right-Click Menu (Windows):**
1. Enable "Context Menu Integration" in Settings
2. Right-click any addon file
3. Select "Install to X-Plane"

### Managing Installed Content

Click the "Management" tab to:
- Browse all installed aircraft, plugins, and navigation data
- Check for available updates
- Enable, disable, or delete items
- Open the folder location

### Scenery Organization

1. Enable "Auto-sort Scenery" in Settings
2. Click "Build Index" on first use
3. Go to the "Scenery" tab in Management
4. Drag items to reorder
5. Click "Apply Changes" to save

---

## Developer Guide

### Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | Vue 3 + TypeScript + Tailwind CSS v4 |
| Backend | Rust + Tauri 2 |
| Build | Vite + Cargo |
| State | Pinia |
| i18n | vue-i18n |

### Project Structure

```
XFast-Manager/
├── src/                    # Frontend source
│   ├── components/         # Vue components
│   ├── views/              # Page views
│   ├── stores/             # Pinia state management
│   ├── services/           # API and logging services
│   ├── i18n/               # Localization files
│   ├── types/              # TypeScript definitions
│   └── utils/              # Utility functions
├── src-tauri/              # Rust backend
│   ├── src/                # Rust source code
│   ├── icons/              # Application icons
│   └── capabilities/       # Tauri permissions
├── public/                 # Static assets
└── scripts/                # Build scripts
```

### Building

**Requirements:**
- Node.js 18+
- Rust 1.70+
- Platform-specific dependencies (see [Tauri Prerequisites](https://tauri.app/start/prerequisites/))

```bash
# Install dependencies
npm install

# Development mode
npm run tauri:dev

# Production build
npm run tauri:build
```

### Key Backend Modules

| Module | Purpose |
|--------|---------|
| `analyzer.rs` | Addon detection and classification |
| `installer.rs` | File installation and backup |
| `scanner.rs` | Scanning installed content |
| `scenery.rs` | Scenery classification and sorting |
| `updater.rs` | Update checking |
| `archive/` | Archive handling (ZIP/7z/RAR) |

---

## Notes

### Data Storage

| Platform | Location |
|----------|----------|
| Windows | `%LOCALAPPDATA%\XFast Manager\` |
| macOS | `~/Library/Application Support/XFast Manager/` |
| Linux | `~/.local/share/XFast Manager/` |

Stores:
- `scenery.db` - Scenery index database
- `logs/` - Application logs

### Known Limitations

- RAR archives do not support file integrity verification
- Windows shortcuts (.lnk) only resolved on Windows

---

## Contributors

**Development** - [CCA3370](https://forums.x-plane.org/profile/1288218-3370/)

**Testing** - SINO1660, enenY, 🍊, Tong Wu

---

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

```
XFast Manager - X-Plane Addon Manager
Copyright (C) 2026 3370Tech

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
```

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

---

## Feedback

Report issues at [GitHub Issues](https://github.com/CCA3370/XFast-Manager/issues).
