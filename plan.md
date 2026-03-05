# Syncweb Go Port - Feature Parity Plan

This document outlines all features from the Python repository (`syncweb-py`) that still need to be ported to the Go repository (`syncweb`) for production readiness.

---

## Table of Contents

1. [Core Commands](#1-core-commands)
2. [File Operations](#2-file-operations)
3. [Device & Folder Management](#3-device--folder-management)
4. [Search & Discovery](#4-search--discovery)
5. [Sorting & Aggregation](#5-sorting--aggregation)
6. [Download Management](#6-download-management)
7. [Daemon & Automation](#7-daemon--automation)
8. [Web UI & API](#8-web-ui--api)
9. [Utilities & Helpers](#9-utilities--helpers)
10. [Testing & Quality](#10-testing--quality)
11. [Documentation & Examples](#11-documentation--examples)

---

## 1. Core Commands

### 1.1 `create` / `init` / `in` / `share`
**Status:** ✅ Partially Implemented

**Python Features:**
- Creates folder with auto-generated folder ID
- Handles name collisions by using full path as folder ID
- Sets folder type to `sendonly`
- Prints syncweb URL: `sync://{folder_id}#{device_id}`
- Sets empty ignore patterns

**Go Implementation Gaps:**
- [ ] Folder ID collision handling (use full path when basename exists)
- [ ] Set folder type to `sendonly` (currently uses `sendreceive`)
- [ ] Print syncweb URL output
- [ ] Initialize with empty ignore patterns

---

### 1.2 `join` / `import` / `clone`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Parse `syncweb://folder-id#device-id` URLs
- Support subpath for immediate download: `syncweb://folder-id/subpath#device-id`
- Auto-create folder with `receiveonly` type
- Pause/resume folder workflow
- Add subpath to ignore patterns (inverted for download)
- Prefix support for folder path

**Go Implementation Gaps:**
- [ ] URL parsing with subpath extraction
- [ ] Set folder type to `receiveonly`
- [ ] Ignore pattern management (`add_ignores` function)
- [ ] Subpath unignore for selective download
- [ ] Proper prefix handling

---

### 1.3 `accept` / `add`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Accept multiple device IDs
- Add devices to specific folders
- Introducer flag support
- Pause/resume devices to unstuck connections
- Handle invalid device IDs gracefully

**Go Implementation Gaps:**
- [ ] Folder-specific device assignment (`--folder-ids` flag)
- [ ] Pause/resume workflow after adding devices to folders
- [ ] Better error handling for invalid device IDs
- [ ] Support space and comma-separated device IDs

---

### 1.4 `drop` / `remove` / `reject`
**Status:** ❌ Not Implemented

**Python Features:**
- Remove devices from specific folders
- Delete devices entirely
- Delete pending device requests
- Pause/resume workflow for immediate effect

**Go Implementation Gaps:**
- [ ] Implement `Drop` command fully
- [ ] Support `--folder-ids` flag
- [ ] Delete pending device functionality
- [ ] Pause/resume workflow

---

### 1.5 `version`
**Status:** ❌ Not Implemented

**Python Features:**
- Show Syncweb version
- Show Syncthing version from API

**Go Implementation Gaps:**
- [ ] Fetch and display Syncthing version from API
- [ ] Proper version formatting

---

## 2. File Operations

### 2.1 `ls` / `list`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Long listing format (`-l`) with Type, Size, Modified, Name
- Human-readable file sizes
- Recursive listing with `--depth`
- Hidden file filtering (`--show-all`)
- Accurate folder size calculation
- File stat for non-directory paths
- Header suppression option
- Tree-like directory visualization

**Go Implementation Gaps:**
- [ ] Implement recursive depth-limited listing
- [ ] Folder size calculation (recursive sum of children)
- [ ] Hidden file filtering
- [ ] File stat for individual files
- [ ] Tree-like visualization with indentation
- [ ] Header suppression (`--no-header`)
- [ ] Modified time formatting

---

### 2.2 `find` / `fd` / `search`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Regex, glob, and exact match modes
- Case-sensitive/insensitive search
- Full path vs filename-only search
- Type filtering (file/directory)
- Size constraints (`--size` with human-readable units)
- Depth constraints (`--depth`, `--min-depth`, `--max-depth`)
- Time-based filtering (`--modified-within`, `--modified-before`)
- Extension filtering (`--ext`)
- Hidden file search
- Follow symbolic links
- Absolute path output
- Downloadable-only filter (exclude sendonly folders)
- Search outside Syncthing folders

**Go Implementation Gaps:**
- [ ] Glob and exact match modes
- [ ] Case sensitivity control
- [ ] Size constraint parsing and filtering
- [ ] Depth constraint parsing
- [ ] Time-based filtering with human-readable durations
- [ ] Extension filtering
- [ ] Hidden file toggle
- [ ] Downloadable folder filtering
- [ ] Path resolution for searches outside Syncthing folders
- [ ] Proper depth calculation and filtering

---

### 2.3 `stat`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Detailed file information (size, blocks, permissions, type)
- Local vs Global state comparison
- Device availability with device names
- Version vector display
- Flags (deleted, ignored, invalid)
- Modified by device tracking
- Inode change time
- Terse format for scripting (`--terse`)
- Custom format strings (`--format` with `%n`, `%s`, `%b`, etc.)
- Timestamp formatting (human, unix, iso)
- Symlink dereference option

**Go Implementation Gaps:**
- [ ] Local vs Global comparison display
- [ ] Device availability with name resolution
- [ ] Version vector display
- [ ] Flags display (deleted, ignored, invalid)
- [ ] Modified by tracking
- [ ] Terse format output
- [ ] Custom format string support
- [ ] Multiple timestamp formats

---

## 3. Device & Folder Management

### 3.1 `devices` / `list-devices` / `lsd`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- List accepted, pending, and discovered devices
- Filter by local-only devices
- Search/include/exclude by name or ID
- Transfer statistics with `--xfer` (rate calculation)
- Bandwidth limit display
- Last seen time with relative formatting
- Connection duration
- Status icons (🏠 localhost, 🗨️ discovered, 💬 pending, 😴 offline, 🌐 online)
- Accept pending devices
- Pause/resume devices
- Delete devices
- Print-only device IDs

**Go Implementation Gaps:**
- [ ] Discovered devices listing
- [ ] Pending devices with proper metadata
- [ ] Transfer rate calculation (`--xfer` flag)
- [ ] Bandwidth limit display
- [ ] Relative datetime formatting
- [ ] Status icons/indicators
- [ ] Include/exclude filtering
- [ ] Accept action for pending devices
- [ ] Print-only mode

---

### 3.2 `folders` / `list-folders` / `lsf`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- List joined, pending, and discovered folders
- Filter by folder type (sendreceive, sendonly, receiveonly, receiveencrypted)
- Search/include/exclude by label, ID, or path
- Local-only filtering
- Missing/orphaned folder detection
- Free space display
- Sync status with percentage
- Local vs Global file counts and sizes
- Needed files/bytes
- Error and pull error counts
- Peer count with pending device count
- Join pending folders
- Delete folders and files
- Pause/resume folders
- Introduce devices to folders
- Print-only folder URLs

**Go Implementation Gaps:**
- [ ] Pending folder discovery and display
- [ ] Discovered folder detection
- [ ] Folder type filtering
- [ ] Include/exclude/search filtering
- [ ] Missing folder detection
- [ ] Free space calculation
- [ ] Sync status percentage calculation
- [ ] Local/Global/Needed statistics
- [ ] Error counting and display
- [ ] Peer counting with pending devices
- [ ] Join pending folders workflow
- [ ] Delete files functionality
- [ ] Introduce devices functionality
- [ ] Print-only mode with syncweb URLs

---

## 4. Search & Discovery

### 4.1 Path Resolution
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Resolve local paths to folder ID + relative path
- Handle paths outside Syncthing folders
- Handle Syncthing folders inside search paths
- User-friendly prefix display

**Go Implementation Gaps:**
- [ ] Robust path-to-folder resolution
- [ ] Handle edge cases (outside folders, nested folders)
- [ ] User prefix calculation

---

### 4.2 Device ID Helpers
**Status:** ❌ Not Implemented

**Python Features:**
- `device_short2long`: Expand short device ID to full ID
- `device_long2name`: Get device name or short ID

**Go Implementation Gaps:**
- [ ] Short-to-long device ID expansion
- [ ] Device ID to name resolution
- [ ] Fallback formatting for unknown devices

---

## 5. Sorting & Aggregation

### 5.1 `sort`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Multiple sort modes:
  - `seeds`/`peers`/`copies`: Number of available peers
  - `time`/`date`/`week`/`month`/`year`: Time-based sorting
  - `size`: File size
  - `niche`: Deviation from ideal peer count
  - `frecency`: Popularity + recency combination
  - `folder-size`: Aggregate folder size
  - `folder-date`: Aggregate folder modification time
  - `file-count`: Number of files in folder
  - `random`: Stable random order
- Reverse sorting with `-` prefix
- Min/max seeders filtering
- Niche ideal peer count configuration
- Frecency weight configuration
- Size limit for output (`--limit-size`)
- Depth constraints for folder aggregation
- Folder aggregation functions (mean, median, sum, min, max, count)
- Read from stdin or arguments

**Go Implementation Gaps:**
- [ ] All advanced sort modes (niche, frecency, folder-*)
- [ ] Peer availability calculation
- [ ] Folder aggregation logic
- [ ] Min/max seeders filtering
- [ ] Configurable niche and frecency parameters
- [ ] Size limit enforcement
- [ ] Depth constraint parsing
- [ ] Stdin input support

---

## 6. Download Management

### 6.1 `download` / `dl` / `upload` / `unignore` / `sync`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Build download plan from paths
- Calculate disk space requirements
- Group folders by mountpoint
- Calculate usable space (free - buffer - pending)
- Download summary table with:
  - Folder ID, file count, total size
  - Usable space, pending downloads, buffer config
  - Status indicator (OK/LOW)
- Shared mountpoint detection and warnings
- Confirmation prompt (unless `--yes`)
- Recursive directory traversal with depth limit
- Sendonly folder exclusion
- Existing file detection
- Ignore pattern management (`add_ignores`)
- Download queuing via unignore

**Go Implementation Gaps:**
- [ ] Disk space calculation
- [ ] Mountpoint grouping logic
- [ ] Usable space calculation
- [ ] Pending download tracking
- [ ] Download summary table
- [ ] Shared mountpoint detection
- [ ] Warning system for low space
- [ ] Confirmation prompt
- [ ] Recursive file collection
- [ ] Sendonly folder filtering
- [ ] Existing file detection
- [ ] Ignore pattern management (`add_ignores` function)
- [ ] Proper unignore pattern formatting

---

### 6.2 Ignore Pattern Management
**Status:** ❌ Not Implemented

**Python Features:**
- `add_ignores`: Add unignore patterns to folder
- Maintain Syncweb-managed section
- Pattern ordering (unignores first, then ignores)
- Wildcard default (`*`)
- Preserve existing patterns

**Go Implementation Gaps:**
- [ ] Implement `SetIgnores` wrapper
- [ ] Pattern ordering logic
- [ ] Syncweb-managed section tracking
- [ ] Unignore pattern formatting (`!/path`)

---

## 7. Daemon & Automation

### 7.1 `automatic`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Daemon mode for auto-accepting devices and folders
- Local-only mode (default)
- Global mode (accept from anywhere)
- Device auto-accept with filters
- Folder auto-join with filters
- Folder type filtering
- Device/folder include/exclude lists
- Wishlist integration (`syncweb-wishlist.sh`)
- Blocklist integration (`syncweb-blocklist.sh`)
- Sort integration for download prioritization
- Signal handling (SIGTERM, SIGINT)
- Configurable sleep intervals

**Go Implementation Gaps:**
- [ ] Full filter support (include/exclude, folder types)
- [ ] Wishlist/blocklist script integration
- [ ] Sort integration for download prioritization
- [ ] Proper signal handling
- [ ] Configurable intervals
- [ ] Devices and folders action flags
- [ ] Join new folders option

---

### 7.2 `start` / `restart`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Start Syncweb daemon
- Daemonize process
- PID file management
- Log file management

**Go Implementation Gaps:**
- [ ] Proper daemon process handling
- [ ] Ensure child process continues correctly

---

### 7.3 `stop` / `shutdown` / `quit`
**Status:** ⚠️ Partially Implemented

**Python Features:**
- Stop Syncweb daemon
- Send SIGTERM to daemon process
- PID file reading

**Go Implementation Gaps:**
- [ ] Verify daemon shutdown
- [ ] Better error messages

---

### 7.4 `repl`
**Status:** ❌ Not Implemented

**Python Features:**
- Interactive Python REPL with Syncthing API access
- Debugging tool

**Go Implementation Gaps:**
- [ ] Consider implementing interactive debug mode
- [ ] Or provide equivalent debugging capability

---

## 8. Web UI & API

### 8.1 Web UI
**Status:** ⚠️ Partially Implemented

**Python Features:**
- N/A (Python doesn't have web UI)

**Go Implementation Status:**
- ✅ Basic web UI structure exists
- ✅ Folder listing
- ✅ File listing
- ✅ Device management UI
- ✅ Mount management UI
- ✅ Activity feed
- ✅ Drag-and-drop file operations
- ✅ Bulk operations (move, copy, delete)
- ✅ Search functionality
- ✅ Sort functionality

**Go Implementation Gaps:**
- [ ] Complete API endpoint implementations
- [ ] File download triggering
- [ ] Folder add/join workflow
- [ ] Device accept/reject workflow
- [ ] Real-time activity updates
- [ ] Mount/unmount functionality
- [ ] Offline mode
- [ ] Proper error handling and notifications

---

### 8.2 API Endpoints
**Status:** ⚠️ Partially Implemented

**Implemented:**
- `/api/syncweb/folders` - List folders
- `/api/syncweb/folders/add` - Add folder
- `/api/syncweb/folders/delete` - Delete folder
- `/api/syncweb/ls` - List files
- `/api/syncweb/find` - Search files
- `/api/syncweb/stat` - Get file stat
- `/api/syncweb/download` - Trigger download
- `/api/syncweb/toggle` - Toggle state
- `/api/syncweb/status` - Get status
- `/api/syncweb/events` - Get events
- `/api/syncweb/devices` - List devices
- `/api/syncweb/pending` - List pending devices
- `/api/syncweb/devices/add` - Add device
- `/api/syncweb/devices/delete` - Delete device
- `/api/mounts` - List mounts
- `/api/mount` - Mount device
- `/api/unmount` - Unmount device
- `/api/local/ls` - List local files
- `/api/raw` - Get raw file content
- `/api/file/move` - Move file
- `/api/file/copy` - Copy file
- `/api/file/delete` - Delete file

**Go Implementation Gaps:**
- [ ] Implement all endpoint handlers
- [ ] Proper request/response formatting
- [ ] Error handling
- [ ] Authentication and authorization
- [ ] Rate limiting
- [ ] CORS configuration

---

## 9. Utilities & Helpers

### 9.1 String Utilities
**Status:** ❌ Not Implemented

**Python Features (`str_utils.py`):**
- `basename`: Cross-platform basename
- `sep_replace`: Replace path separators
- `extract_device_id`: Parse device ID from URL
- `parse_syncweb_path`: Parse syncweb:// URLs
- `human_to_bytes`: Convert human-readable sizes to bytes
- `human_to_seconds`: Convert human-readable durations to seconds
- `isodate2seconds`: Convert ISO datetime to Unix timestamp
- `parse_human_to_lambda`: Parse human-readable constraints to lambda
- `file_size`: Format bytes to human-readable size
- `format_time`: Format timestamps
- `relative_datetime`: Format relative time (e.g., "3 days ago")
- `duration_short`: Format duration in short form
- `pipe_print`: Print for piping (no quotes)

**Go Implementation Gaps:**
- [ ] Move existing utils to dedicated package
- [ ] Implement all string utilities
- [ ] Implement time formatting utilities
- [ ] Implement size formatting utilities

---

### 9.2 Configuration Utilities
**Status:** ❌ Not Implemented

**Python Features (`config.py`):**
- Default state directory detection
- Platform-specific paths

**Go Implementation Gaps:**
- [ ] Already partially in `utils.GetConfigDir()`
- [ ] Verify cross-platform compatibility

---

### 9.3 Logging Utilities
**Status:** ⚠️ Partially Implemented

**Python Features (`log_utils.py`):**
- Terminal detection
- Colored logging
- Log level management

**Go Implementation Gaps:**
- [ ] Terminal detection for logging
- [ ] Consider colored output for CLI

---

### 9.4 Syncthing API Client
**Status:** ⚠️ Partially Implemented

**Python Features (`syncthing.py`):**
- Full Syncthing REST API wrapper
- Device management
- Folder management
- Ignore pattern management
- File/folder queries
- Connection statistics
- Discovery cache
- Pending devices/folders
- Folder status
- Device stats
- Version info

**Go Implementation Gaps:**
- [ ] Using Syncthing lib directly (better approach)
- [ ] Ensure all API functionality is accessible
- [ ] Pending folders API
- [ ] Discovery cache access
- [ ] Connection statistics with rate calculation

---

## 10. Testing & Quality

### 10.1 Unit Tests
**Status:** ⚠️ Partially Implemented

**Python Test Coverage:**
- URL parsing tests
- Syncthing API tests
- File system tree generation
- Database tests

**Go Implementation Gaps:**
- [ ] Expand test coverage for all commands
- [ ] Add integration tests
- [ ] Add end-to-end tests
- [ ] Mock Syncthing API for testing
- [ ] Test utilities (string utils, size formatting, etc.)

---

### 10.2 CLI Tests
**Status:** ❌ Not Implemented

**Python Features:**
- CLI argument parsing tests
- Command execution tests

**Go Implementation Gaps:**
- [ ] Add CLI tests for all commands
- [ ] Test flag parsing
- [ ] Test error cases

---

## 11. Documentation & Examples

### 11.1 Example Scripts
**Status:** ❌ Not Implemented

**Python Examples:**
- `install.sh`: Installation script for syncweb-automatic
- `simple_wishlist.sh`: Simple wishlist generator
- `syncweb-blocklist.sh`: Blocklist script
- `syncweb-wishlist.sh`: Wishlist script
- `syncweb-automatic.service`: Systemd service file

**Go Implementation Gaps:**
- [ ] Create example scripts for Go version
- [ ] Update systemd service file
- [ ] Create installation script

---

### 11.2 README Updates
**Status:** ❌ Not Implemented

**Gaps:**
- [ ] Update README with Go-specific installation
- [ ] Update usage examples
- [ ] Document differences from Python version
- [ ] Add migration guide

---

### 11.3 FAQ Updates
**Status:** ❌ Not Implemented

**Gaps:**
- [ ] Update FAQ for Go version
- [ ] Document known issues
- [ ] Troubleshooting guide

---

## Priority Matrix

### High Priority (Core Functionality)
1. ✅ `create` - Folder creation
2. ✅ `join` - Join folders/devices
3. ✅ `accept` - Accept devices
4. ⚠️ `ls` - List files
5. ⚠️ `find` - Search files
6. ⚠️ `download` - Download files
7. ⚠️ `devices` - List devices
8. ⚠️ `folders` - List folders

### Medium Priority (Quality of Life)
9. ⚠️ `stat` - File statistics
10. ⚠️ `sort` - Sort files
11. ⚠️ `automatic` - Daemon mode
12. ❌ Ignore pattern management
13. ❌ String utilities
14. ⚠️ Web UI completion

### Low Priority (Nice to Have)
15. ❌ Example scripts
16. ❌ REPL mode
17. ❌ Advanced sorting modes
18. ❌ Folder aggregation

---

## Implementation Notes

### Key Architectural Differences

**Python:**
- Uses Syncthing REST API
- HTTP client for all operations
- External process management

**Go:**
- Uses Syncthing libraries directly
- Embedded Syncthing node
- Direct configuration manipulation
- More efficient, fewer network calls

### Migration Considerations

1. **Configuration Format:** Go uses Syncthing's native config structures
2. **Event Handling:** Go uses Syncthing's event system directly
3. **File Operations:** Go can use native file operations instead of API calls
4. **Concurrency:** Go can leverage goroutines for parallel operations

---

## Glossary

- ✅ **Implemented:** Feature exists and works
- ⚠️ **Partially Implemented:** Feature exists but missing functionality
- ❌ **Not Implemented:** Feature does not exist

---

## Last Updated

2026-03-05
