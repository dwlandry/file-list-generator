# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

File List Generator v1.1 is a Windows desktop application written in Rust that provides fast file searching with Windows Explorer context menu integration. It uses native Win32 APIs for the GUI and multi-threaded scanning for performance.

**Developer:** David Landry  
**Organization:** Scallon Controls  
**Version:** 1.1.0

## Build Commands

```bash
# Debug build
cargo build

# Release build (optimized for performance)
cargo build --release

# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests (if any are added)
cargo test
```

## Installation

The application is distributed as a pre-built package in `FileListGenerator_v1.1/`:
- Run `Install.bat` to add to context menu (user-level, no admin required)
- Run `Uninstall.bat` to remove from context menu

## Architecture

The application follows a multi-threaded architecture with clear separation of concerns:

### Core Components

1. **Main Thread Coordinator** (`src/main.rs`): Manages GUI and scanner threads, handles command-line arguments, and coordinates communication via crossbeam channels.

2. **File Scanner** (`src/scanner.rs`): 
   - Implements parallel directory traversal using `walkdir` and `rayon`
   - Processes files in batches of 100 for optimal performance
   - Automatically skips problematic directories (node_modules, .git, Windows system folders)
   - Recognizes shortcut (.lnk) files with special icon

3. **GUI Layer** (`src/gui.rs`): 
   - Direct Win32 API implementation for minimal overhead
   - Updates list view in real-time as files are discovered
   - Handles all user interactions including search, filtering, and context menu operations
   - Column sorting by clicking headers (v1.1)
   - Open Folder button for quick navigation (v1.1)
   - Drag & drop support with shortcut resolution (v1.1)

4. **Filter System** (`src/filter.rs`): 
   - Supports text-based search (names, paths, extensions)
   - Size-based queries (>10mb, <1kb)
   - Filters are applied in real-time as the user types

### Thread Communication

- Uses `crossbeam-channel` for sending file data from scanner to GUI
- GUI thread processes messages via Windows message pump
- Scanner sends `ScanUpdate` messages containing batches of `FileInfo` structs

### Performance Considerations

- Release builds use aggressive optimizations (LTO, single codegen unit)
- Parallel scanning with rayon for large directories
- Batch processing to reduce GUI update overhead
- Smart directory filtering to avoid system/dependency folders
- Virtual list view for handling large directories efficiently

## Key Dependencies

- `windows` (0.58) - Windows API bindings
- `rayon` (1.10) - Data parallelism
- `walkdir` (2.5) - Recursive directory traversal
- `crossbeam-channel` (0.5) - Thread-safe message passing
- `chrono` (0.4) - Date/time handling for file metadata

## Windows Integration

The application integrates with Windows Explorer through registry entries:
- Adds context menu entries for folders, folder backgrounds, and drives
- Launches with selected directory as argument
- Registry paths (all in HKEY_CURRENT_USER, no admin required):
  - `HKEY_CURRENT_USER\Software\Classes\Directory\shell\FileListGenerator`
  - `HKEY_CURRENT_USER\Software\Classes\Directory\Background\shell\FileListGenerator`
  - `HKEY_CURRENT_USER\Software\Classes\Drive\shell\FileListGenerator`

## Version 1.1 Features

- **Column Sorting**: Click any column header to sort ascending/descending
- **Open Folder Button**: Quick access to containing folder
- **Shortcut Support**: Recognizes and resolves .lnk files
- **Improved Drag & Drop**: Drop folders or shortcuts to scan
- **Better Colors**: Removed confusing file type colors, kept alternating rows
- **No Keyboard Hijacking**: Removed global Ctrl+C/V shortcuts

## Distribution

The project includes a complete distribution package:
- `FileListGenerator_v1.1/` - Contains executable and installers
- `FileListGenerator_v1.1.zip` - Ready for Teams/email distribution
- `IT_ADMIN_REPORT.html` - Security documentation for IT approval
- `TEAMS_DISTRIBUTION_MESSAGE.txt` - Template for sharing

## Security Notes

- **No network access** - Application is 100% offline
- **Read-only file access** - Cannot modify or delete files
- **User-level registry only** - No system-wide changes
- **No background services** - Runs only when explicitly opened
- **Clean uninstall** - Removes all registry entries