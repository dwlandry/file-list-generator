# File List Generator v1.1

A lightning-fast Windows file explorer with context menu integration, built with Rust for maximum performance.

**Developer:** David Landry  
**Organization:** Scallon Controls

## What's New in Version 1.1

- **Column Sorting**: Click any column header to sort files
- **Open Folder Button**: Quick access to containing folders
- **Shortcut Support**: Recognizes and follows .lnk files
- **Improved Drag & Drop**: Drop folders or shortcuts to scan
- **Better UI**: Cleaner colors, no keyboard shortcut conflicts
- **Copy List Button**: Export file lists to clipboard

## Features

- **Instant Launch**: Right-click any folder to instantly view its contents
- **Blazing Fast**: Multi-threaded file scanning using Rust and parallel processing
- **Real-time Filtering**: Search and filter files as you type
- **Native Windows GUI**: Uses Win32 API for minimal overhead
- **Interactive**: Double-click to open files or explore folders
- **Smart Search**:
  - Text search across names, paths, and extensions
  - Size filters (e.g., `>10mb`, `<1kb`)
- **Column Management**: Show/hide columns with checkboxes
- **No Installation Required**: Portable application

## Quick Start

### Using Pre-Built Package (Recommended)

1. Extract `FileListGenerator_v1.1.zip` to a permanent location (e.g., `C:\Tools\FileListGenerator\`)
2. Run `Install.bat` (no admin rights needed)
3. Right-click any folder and select "Generate File List"

That's it! The tool is ready to use.

### Building from Source

Prerequisites:
- Windows 10/11
- [Rust](https://rustup.rs/)

```cmd
# Clone the repository
git clone [repository-url]
cd file-list-generator

# Build release version
cargo build --release

# The executable will be in target/release/
```

## Usage

### Launching the Application

**Method 1: Right-Click Menu**
- Right-click on a folder → Select "Generate File List"
- Right-click inside a folder → Select "Generate File List"
- Right-click on a drive → Select "Generate File List"

**Method 2: Drag & Drop**
- Open File List Generator from any folder
- Drag another folder into the window to scan it
- Works with shortcut files (.lnk) too!

### Keyboard Shortcuts

- **Ctrl+F**: Focus the search box
- **Ctrl+O**: Open the containing folder of selected file
- **Enter**: Open selected file or folder
- **Escape**: Clear search and show all files
- **Tab**: Switch between search box and file list

### Search Filters

**Text Search:**
- Type any text to filter by name, path, or extension
- Examples: `report`, `.pdf`, `2024`

**Size Filters:**
- `>10mb` - Files larger than 10 MB
- `<1kb` - Files smaller than 1 KB
- `>100k` - Files larger than 100 KB
- Supports: kb/k, mb/m, gb/g

### Features

**Main Controls:**
- **Search Box**: Start typing to filter files instantly
- **Copy List**: Copy all visible files to clipboard (paste into Excel)
- **Open Folder**: Open the folder containing selected file
- **Help (?)**: Show keyboard shortcuts and tips

**Column Options:**
- Toggle visibility with checkboxes
- Click headers to sort ascending/descending
- Available columns:
  - Name (always visible)
  - Type (file extension)
  - Size (human-readable format)
  - Modified (date and time)
  - Path (parent directory)

**Right-Click Menu:**
- Open file/folder
- Open containing folder
- Copy full path
- Copy file name

## Uninstallation

1. Run `Uninstall.bat` from the installation folder
2. Delete the FileListGenerator folder

No traces are left in the system.

## Technical Details

### Architecture
- **Language**: Rust
- **GUI Framework**: Native Windows API (Win32)
- **Parallelization**: Rayon for multi-threading
- **File Traversal**: Optimized walkdir with parallel processing

### Performance
- Release build with Link-Time Optimization (LTO)
- Multi-threaded directory traversal
- Virtual list view for handling millions of files
- Lazy loading and intelligent batching
- ~20MB RAM usage, scales with directory size

### Security
- No network connectivity
- Read-only file access
- User-level registry entries only (HKEY_CURRENT_USER)
- No background services or startup entries
- Completely portable and removable

## Distribution

For Teams/organizational deployment:
1. Share `FileListGenerator_v1.1.zip`
2. Users extract and run `Install.bat`
3. No IT admin privileges required

See `TEAMS_DISTRIBUTION_MESSAGE.txt` for sharing template.

## Support

For technical questions or issues, contact David Landry.

For IT security review, see `IT_ADMIN_REPORT.html`.

## License

This project is provided as-is for Scallon Controls internal use.

---

*Version 1.1.0 - © 2025 David Landry*