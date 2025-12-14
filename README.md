# XFastInstall - X-Plane 12 Addon Installer

A modern, cross-platform X-Plane 12 addon installer built with Tauri 2, Rust, and Vue 3.

## Features

- 🎯 **Smart Detection**: Automatically detects Aircraft, Scenery, Plugins, and Navdata
- 🚀 **Drag & Drop**: Simple drag-and-drop interface for easy installation
- 📦 **Archive Support**: Supports .zip, .7z, and .rar files
- 🔍 **Intelligent Analysis**: Deep scan and deduplication of nested addons
- ⚠️ **Conflict Detection**: Warns about existing installations
- 🖱️ **Windows Integration**: Optional right-click context menu (Windows only)
- 🌙 **Dark Theme**: Aviation-inspired modern UI

## Installation

### Prerequisites

- Node.js 20+
- Rust (latest stable)
- Platform-specific requirements:
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools

### Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

## Usage

1. **Configure X-Plane Path**: Go to Settings and select your X-Plane 12 installation directory
2. **Install Addons**: Drag and drop addon files or folders onto the home screen
3. **Review & Confirm**: Review detected addons and their install locations
4. **Install**: Click Install to complete the installation

### Windows Context Menu

On Windows, you can register a right-click context menu:
1. Go to Settings
2. Click "Register Context Menu"
3. Now you can right-click any file or folder and select "Install to X-Plane"

## Detection Logic

### Aircraft (Type A)
- **Marker**: `*.acf` file
- **Installation**: `X-Plane 12/Aircraft/`

### Scenery (Type B)
- **Marker**: `library.txt` or `*.dsf` file
- **Installation**: `X-Plane 12/Custom Scenery/`

### Plugins (Type C)
- **Marker**: `*.xpl` file
- **Installation**: `X-Plane 12/Resources/plugins/`

### Navdata (Type D)
- **Marker**: `cycle.json` file
- **Installation**: `X-Plane 12/Custom Data/` or `X-Plane 12/Custom Data/GNS430/`

## Tech Stack

- **Frontend**: Vue 3, TypeScript, TailwindCSS, Pinia
- **Backend**: Rust, Tauri 2
- **Build**: Vite, GitHub Actions

## License

See LICENSE file for details.