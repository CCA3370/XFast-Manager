# XFastInstall Developer Guide

## Project Structure

```
├── src/                    # Vue 3 frontend source
│   ├── components/        # Reusable Vue components
│   ├── stores/           # Pinia state management
│   ├── types/            # TypeScript type definitions
│   ├── views/            # Page components
│   ├── App.vue           # Main application component
│   ├── main.ts           # Application entry point
│   └── style.css         # Global styles
├── src-tauri/             # Rust backend
│   ├── src/              # Rust source files
│   │   ├── analyzer.rs   # Addon analysis and deduplication
│   │   ├── installer.rs  # File installation logic
│   │   ├── lib.rs        # Tauri commands and setup
│   │   ├── main.rs       # Application entry point
│   │   ├── models.rs     # Data structures
│   │   ├── registry.rs   # Windows registry integration
│   │   └── scanner.rs    # File/archive scanning logic
│   ├── Cargo.toml        # Rust dependencies
│   └── tauri.conf.json   # Tauri configuration
```

## Development Setup

### Prerequisites
- Node.js 20+
- Rust (latest stable)
- Platform-specific:
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools

### Quick Start

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

## Architecture

### Frontend (Vue 3)
- **State Management**: Pinia stores for app state and toast notifications
- **Routing**: Vue Router for navigation
- **Styling**: TailwindCSS v4 with custom aviation theme
- **Communication**: Tauri API for backend communication

### Backend (Rust)
- **Scanner**: Detects addon types by scanning for marker files
- **Analyzer**: Deduplicates and creates installation tasks
- **Installer**: Extracts archives and copies files to X-Plane
- **Registry**: Windows context menu integration

## Addon Detection Logic

### Type A: Aircraft
- **Marker**: `*.acf` file
- **Logic**: Install parent folder of `.acf` file
- **Target**: `X-Plane 12/Aircraft/`

### Type B: Scenery
- **Marker 1**: `library.txt` → Install immediate parent
- **Marker 2**: `*.dsf` → Go up 2 levels from `.dsf`
- **Target**: `X-Plane 12/Custom Scenery/`

### Type C: Plugins
- **Marker**: `*.xpl` file
- **Logic**: Check for platform folders (win_x64, etc.), go up if found
- **Target**: `X-Plane 12/Resources/plugins/`

### Type D: Navdata
- **Marker**: `cycle.json`
- **Logic**: Parse JSON to determine X-Plane version
- **Target**: `X-Plane 12/Custom Data/` or `X-Plane 12/Custom Data/GNS430/`

## Deduplication Algorithm

The analyzer implements smart deduplication:
1. If a detected addon is a subdirectory of another, keep only the parent
2. Example: Aircraft at `A330/` with Plugin at `A330/plugins/fms/` → Only install `A330/`

## Adding New Features

### Adding a New Addon Type
1. Add new enum variant to `AddonType` in `src-tauri/src/models.rs`
2. Implement detection logic in `src-tauri/src/scanner.rs`
3. Add target path logic in `src-tauri/src/analyzer.rs`
4. Update frontend types in `src/types/index.ts`

### Adding New Archive Format
1. Add crate to `src-tauri/Cargo.toml`
2. Implement extraction in `src-tauri/src/installer.rs`
3. Add scanning logic in `src-tauri/src/scanner.rs`

## Testing

### Frontend
```bash
npm run build  # Test Vite build
```

### Backend
```bash
cd src-tauri
cargo check   # Check for compilation errors
cargo test    # Run unit tests
```

## CI/CD

GitHub Actions workflow (`.github/workflows/build.yml`) builds for:
- Windows (x86_64)
- macOS (x86_64 and ARM64)
- Linux (x86_64)

Artifacts are uploaded for each platform.

## Troubleshooting

### Linux Build Fails
Install required dependencies:
```bash
sudo apt-get install libwebkit2gtk-4.1-dev build-essential libssl-dev
```

### Icon Not Found
Ensure `src-tauri/icons/icon.png` is a valid RGBA PNG file.

### Archive Extraction Fails
Check that the archive format is supported (ZIP and 7z currently).

## Future Enhancements

- [ ] RAR archive support
- [ ] Advanced conflict resolution (version comparison)
- [ ] Installation history/rollback
- [ ] Batch installation queue
- [ ] Automatic X-Plane path detection
- [ ] Multi-language support
- [ ] Installation progress tracking
