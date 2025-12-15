# XFastInstall User Guide

## What is XFastInstall?

XFastInstall is a smart installer for X-Plane addons that automatically detects what type of addon you're trying to install (Aircraft, Scenery, Plugin, or Navdata) and puts it in the right place.

## Getting Started

### First Time Setup

1. **Launch XFastInstall**
2. **Go to Settings** (click the Settings link in the top right)
3. **Set your X-Plane Path**:
   - Click "Browse" to select your X-Plane installation folder
   - Example: `C:\X-Plane\` or `/Applications/X-Plane/`
   - Click "Save"

### Installing Addons

#### Method 1: Drag and Drop
1. Go to the Home page
2. Drag your addon file (.zip, .7z) or folder onto the drop zone
3. Wait for the analysis to complete
4. Review the detected addon(s) in the confirmation dialog
5. Click "Install" to proceed

#### Method 2: Windows Context Menu (Windows Only)
1. Go to Settings and click "Register Context Menu"
2. Now you can right-click any file or folder
3. Select "Install to X-Plane" from the menu
4. The application will launch and analyze the addon

## What Gets Detected?

### Aircraft
- **Identified by**: `.acf` files
- **Installs to**: `X-Plane/Aircraft/`
- **Example**: An Airbus A330 package with an `.acf` file

### Scenery
- **Identified by**: `library.txt` or `.dsf` files
- **Installs to**: `X-Plane/Custom Scenery/`
- **Example**: Airport scenery packages

### Plugins
- **Identified by**: `.xpl` files
- **Installs to**: `X-Plane/Resources/plugins/`
- **Example**: Better Pushback, AutoGate, etc.

### Navigation Data
- **Identified by**: `cycle.json` files
- **Installs to**: `X-Plane/Custom Data/` or `X-Plane/Custom Data/GNS430/`
- **Example**: Navigraph cycle updates

## Smart Features

### Automatic Detection
The app scans your addon deeply and finds the right folder to install, even if it's nested inside multiple folders.

### Deduplication
If an aircraft package contains plugins, XFastInstall is smart enough to install the whole aircraft folder (which includes the plugins) rather than trying to install each part separately.

### Conflict Detection
Before installing, the app checks if something already exists at the destination. You'll see a warning icon (⚠️) if there's a conflict.

## Tips

- **Archive Files**: The app can handle ZIP and 7z archives. You don't need to extract them first!
- **Folders**: You can also drag and drop folders directly
- **Multiple Files**: Drag multiple addons at once to batch install
- **Errors**: If something goes wrong, you'll see error notifications at the top right

## Troubleshooting

### "No valid addons detected"
Your file/folder might not contain recognizable addon markers. Make sure it contains:
- `.acf` files for aircraft
- `library.txt` or `.dsf` files for scenery
- `.xpl` files for plugins
- `cycle.json` for navdata

### "Please set X-Plane path in Settings first"
You need to configure where X-Plane is installed before you can install addons.

### Installation Failed
- Check that X-Plane is not running
- Make sure you have write permissions to the X-Plane folder
- Check that the archive is not corrupted

## Support

For issues and feature requests, please visit the GitHub repository.

## License

See LICENSE file for details.
