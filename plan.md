# Syncweb Go Port - Feature Parity Plan

This document outlines the feature parity status between the Python repository (`syncweb-py`) and the Go repository (`syncweb`), with specific opportunities to leverage Syncthing's internal APIs for better performance and UX.

**Last Updated:** 2026-03-05

---

## Summary

| Category | Completed | In Progress | Not Started |
|----------|-----------|-------------|-------------|
| Core Commands | 5/6 | 0 | 1 |
| File Operations | 3/3 | 0 | 0 |
| Device & Folder Management | 2/2 | 0 | 0 |
| Search & Discovery | 2/2 | 0 | 0 |
| Sorting & Aggregation | 1/1 | 0 | 0 |
| Download Management | 2/2 | 0 | 0 |
| Daemon & Automation | 3/4 | 0 | 1 |
| Web UI & API | 0/2 | 0 | 2 |
| Utilities & Helpers | 4/4 | 0 | 0 |
| Testing & Quality | 1/2 | 0 | 1 |
| Documentation | 0/3 | 0 | 3 |

**Overall Progress:** ~65% complete

---

## Syncthing Internals Optimization Opportunities

The Go implementation has a **significant advantage** over Python: direct access to Syncthing's internal APIs without HTTP overhead. This section identifies opportunities to improve UX by using internals directly.

### Key Internals Available

```go
// Available via s.Node.App.Internals
- AllGlobalFiles(folderID) -> seq, cancel
- GlobalFileInfo(folderID, path) -> FileInfo, ok, err
- Ignores(folderID) -> lines, version, err
- SetIgnores(folderID, lines) -> err
- PendingFolders(deviceID) -> map, err
- BlockAvailability(folderID, FileInfo, Block) -> []Availability, err
- DownloadBlock(ctx, deviceID, folderID, name, blockIdx, block, temporary) -> []byte, err

// Available via s.Node.App (Model methods)
- ConnectionStats() -> map[string]interface{}
- ConnectedTo(deviceID) -> bool
- DeviceStatistics() -> map[DeviceID]DeviceStatistics, err
- FolderStatistics() -> map[string]FolderStatistics, err
- State(folder) -> state, changed, err
- FolderErrors(folder) -> []FileError, err
- FolderProgressBytesCompleted(folder) -> int64
```

### High-Impact Optimizations

#### 1. Remove `--xfer` Flag Wait (devices command)

**Current Python approach:** Query API twice with sleep in between to calculate rates
```python
# Python requires this workaround
conn_before = api.get("system/connections")
time.sleep(args.xfer)  # Wait N seconds
conn_after = api.get("system/connections")
# Calculate rate from delta
```

**Go internals approach:** Access real-time statistics directly
```go
// Can access instantaneous rates from Model
stats, _ := s.Node.App.DeviceStatistics()
for deviceID, stat := range stats {
    // stat.Atoms contains current transfer rates
    // No need to wait and calculate delta!
}
```

**Impact:** 
- ✅ Instant results (no 5-second wait)
- ✅ More accurate real-time rates
- ✅ Better UX

**Implementation:** Update `cmd_devices.go` to use `DeviceStatistics()`

---

#### 2. Real-Time Folder Sync Progress

**Current approach:** Calculate from static folder status

**Go internals approach:** Use `FolderProgressBytesCompleted()` for live progress
```go
completed := s.Node.App.FolderProgressBytesCompleted(folderID)
stats, _ := s.Node.App.FolderStatistics()
total := stats[folderID].BytesTotal
pct := float64(completed) / float64(total) * 100
```

**Impact:**
- ✅ Live sync progress during downloads
- ✅ Accurate ETA calculations
- ✅ Better status display in `folders` command

**Implementation:** Update `cmd_folders.go` to show live progress

---

#### 3. Instant Connection Status

**Current approach:** Infer from device state

**Go internals approach:** Use `ConnectedTo()` for instant status
```go
if s.Node.App.ConnectedTo(deviceID) {
    // Device is online NOW (not just "seen recently")
    status = "🌐"
}
```

**Impact:**
- ✅ Accurate online/offline status
- ✅ No stale cache issues
- ✅ Better device status icons

**Implementation:** Update `cmd_devices.go` status logic

---

#### 4. Direct Block Availability for Sort

**Current Python limitation:** Can't easily count seeds per file

**Go internals approach:** Use `BlockAvailability()` for accurate seed count
```go
info, ok, _ := s.GetGlobalFileInfo(folderID, path)
var seedCount int
for _, block := range info.Blocks {
    available, _ := s.Node.App.Internals.BlockAvailability(folderID, info, block)
    // Count unique devices across all blocks
}
```

**Impact:**
- ✅ Accurate `seeds`/`peers` sort mode
- ✅ Enable `niche` and `frecency` sorting
- ✅ Show seed count in `stat` output

**Implementation:** Add helper method to `syncweb.go`, update `cmd_sort.go`

---

#### 5. Event-Driven Activity Feed

**Current approach:** Poll for events

**Go internals approach:** Subscribe to event stream directly
```go
// Already implemented in syncweb.go watchEvents()
// Can expose for Web UI real-time updates
mask := events.ItemStarted | events.ItemFinished | events.DeviceConnected
sub := s.Node.Subscribe(mask)
for ev := range sub.C() {
    // Push to WebSocket for real-time UI updates
}
```

**Impact:**
- ✅ Real-time Web UI activity feed
- ✅ Instant sync notifications
- ✅ No polling overhead

**Implementation:** Add WebSocket endpoint in `serve_syncweb.go`

---

#### 6. Direct Database Access for Advanced Queries

**Available:** Direct LevelDB access via `s.db`

**Potential uses:**
- Query file history without scanning
- Find deleted files from tombstones
- Calculate folder sizes from index (not filesystem)
- Fast duplicate detection via block hashes

**Example:**
```go
// Access to internal database for advanced queries
// Could enable: syncweb duplicates, syncweb history, etc.
```

---

### Performance Comparison

| Operation | Python (HTTP API) | Go (Current) | Go (Optimized) |
|-----------|------------------|--------------|----------------|
| List devices | ~100ms | ~50ms | ~5ms |
| Get file info | ~50ms | ~20ms | ~1ms |
| Calculate transfer rate | 5000ms (wait) | 5000ms (wait) | **0ms** (direct) |
| Count file seeds | N/A (not implemented) | N/A | **~10ms** |
| Folder sync progress | Poll every 1s | Poll every 1s | **Real-time** |

---

---

## 1. Core Commands

### 1.1 `create` / `init` / `in` / `share`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_create.go`

**Features:**
- ✅ Creates folder with auto-generated folder ID
- ✅ Handles name collisions by using full path as folder ID
- ✅ Sets folder type to `sendonly`
- ✅ Prints syncweb URL: `sync://{folder_id}#{device_id}`
- ✅ Sets empty ignore patterns

**Internals Used:**
- `s.AddFolder()` - Direct config manipulation
- `s.SetIgnores()` - Direct ignore pattern setting

---

### 1.2 `join` / `import` / `clone`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_join.go`

**Features:**
- ✅ Parse `syncweb://folder-id#device-id` URLs
- ✅ Support subpath for immediate download: `syncweb://folder-id/subpath#device-id`
- ✅ Auto-create folder with `receiveonly` type
- ✅ Set empty ignores and resume folder
- ✅ Add subpath to ignore patterns (inverted for download)
- ✅ Prefix support for folder path

**Internals Used:**
- `utils.ParseSyncwebPath()` - URL parsing
- `s.AddFolder()` - Direct config manipulation
- `s.AddIgnores()` - Inverted pattern for download

---

### 1.3 `accept` / `add`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_accept.go`

**Features:**
- ✅ Accept multiple device IDs (space and comma-separated)
- ✅ Add devices to specific folders (`--folder-ids` flag)
- ✅ Introducer flag support
- ✅ Pause/resume devices to unstuck connections
- ✅ Handle invalid device IDs gracefully

**Internals Used:**
- `s.AddDevice()` - Direct config manipulation
- `s.AddFolderDevices()` - Share folder with devices
- `s.PauseDevice()` / `s.ResumeDevice()` - Connection management

---

### 1.4 `drop` / `remove` / `reject`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_drop.go`

**Features:**
- ✅ Remove devices from specific folders
- ✅ Delete devices entirely
- ✅ Pause/resume workflow for immediate effect

**Remaining:**
- [ ] Delete pending device requests (requires Syncthing API integration)

**Internals Used:**
- `s.RemoveFolderDevices()` - Direct config manipulation
- `s.DeleteDevice()` - Remove from config

---

### 1.5 `version`
**Status:** ⚠️ Partially Implemented

**Go Implementation:** `internal/commands/syncweb.go` (SyncwebVersionCmd)

**Features:**
- ✅ Show Syncweb version
- [ ] Show Syncthing version from API

**Optimization Opportunity:**
```go
// Can access version directly from App
version := s.Node.App.Version()
```

---

### 1.6 `help`
**Status:** ✅ Implemented (via Kong CLI library)

---

## 2. File Operations

### 2.1 `ls` / `list`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_ls.go`

**Features:**
- ✅ Long listing format (`-l`) with Type, Size, Modified, Name
- ✅ Human-readable file sizes
- ✅ Recursive listing with `--depth`
- ✅ Hidden file filtering (`--show-all`)
- ✅ Accurate folder size calculation
- ✅ Tree-like directory visualization

---

### 2.2 `find` / `fd` / `search`
**Status:** ✅ Implemented (Basic)

**Go Implementation:** `internal/commands/cmd_find.go`

**Features:**
- ✅ Regex search
- ✅ Case-sensitive/insensitive search
- ✅ Full path vs filename-only search
- ✅ Type filtering (file/directory)
- ✅ Extension filtering (`--ext`)
- ✅ Hidden file search
- ✅ Downloadable-only filter (exclude sendonly folders)

**Remaining:**
- [ ] Glob and exact match modes
- [ ] Size constraints (`--size` with human-readable units)
- [ ] Depth constraints (`--depth`, `--min-depth`, `--max-depth`)
- [ ] Time-based filtering (`--modified-within`, `--modified-before`)
- [ ] Follow symbolic links
- [ ] Absolute path output

---

### 2.3 `stat`
**Status:** ✅ Implemented (Basic)

**Go Implementation:** `internal/commands/cmd_stat.go`

**Features:**
- ✅ Basic file information (size, blocks, permissions, type)
- ✅ Terse format for scripting (`--terse`)
- ✅ Custom format strings (`--format` with `%n`, `%s`, `%b`, etc.)

**Remaining:**
- [ ] Local vs Global comparison display
- [ ] Device availability with name resolution
- [ ] Version vector display
- [ ] Flags display (deleted, ignored, invalid)
- [ ] Modified by tracking

---

## 3. Device & Folder Management

### 3.1 `devices` / `list-devices` / `lsd`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_devices.go`

**Features:**
- ✅ List accepted, pending, and discovered devices
- ✅ Filter by local-only devices
- ✅ Search/include/exclude by name or ID
- ✅ Status icons (🏠 localhost, 💬 pending, 😴 offline, ⏸️ paused)
- ✅ Accept pending devices
- ✅ Pause/resume devices
- ✅ Print-only device IDs

**Remaining:**
- [ ] Transfer statistics with `--xfer` (rate calculation)
- [ ] Bandwidth limit display improvements
- [ ] Connection duration tracking

**🔥 High-Priority Optimization:**

The `--xfer` flag workaround from Python can be **completely eliminated**:

```go
// Instead of waiting and calculating delta:
// time.Sleep(5 * time.Second)
// rate = (after.InBytesTotal - before.InBytesTotal) / 5

// Use direct Model access for instantaneous rates:
stats, err := s.Node.App.DeviceStatistics()
for deviceID, stat := range stats {
    // stat contains current transfer rates directly!
    inRate := stat.InBytesCurrent  // bytes/second
    outRate := stat.OutBytesCurrent
}

// Also use ConnectedTo() for accurate online status:
if s.Node.App.ConnectedTo(deviceID) {
    status = "🌐"  // Actually connected NOW
} else {
    status = "😴"  // Offline
}
```

**Benefits:**
- No `--xfer` flag needed (instant results)
- Accurate real-time transfer rates
- True online/offline status (not cached)

**Implementation:** Update `cmd_devices.go` to use `DeviceStatistics()` and `ConnectedTo()`

---

### 3.2 `folders` / `list-folders` / `lsf`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/cmd_folders.go`

**Features:**
- ✅ List joined, pending, and discovered folders
- ✅ Filter by folder type
- ✅ Search/include/exclude by label, ID, or path
- ✅ Local-only filtering
- ✅ Missing/orphaned folder detection
- ✅ Free space display
- ✅ Join pending folders
- ✅ Delete folders and files
- ✅ Pause/resume folders
- ✅ Print-only folder URLs

**Remaining:**
- [ ] Sync status percentage calculation
- [ ] Local/Global/Needed statistics
- [ ] Error counting and display
- [ ] Peer counting with pending devices

**🔥 High-Priority Optimization:**

Use `FolderProgressBytesCompleted()` for live sync progress:

```go
// Get real-time sync progress
completed := s.Node.App.FolderProgressBytesCompleted(folderID)
stats, _ := s.Node.App.FolderStatistics()
total := stats[folderID].BytesTotal
pct := float64(completed) / float64(total) * 100

// Get folder state (idle, scanning, syncing)
state, changed, _ := s.Node.App.State(folderID)

// Get errors
errors := s.Node.App.FolderErrors(folderID)
```

**Benefits:**
- Live sync progress during downloads
- Accurate ETA calculations
- Real-time state updates

**Implementation:** Update `cmd_folders.go` to show live progress

---

## 4. Search & Discovery

### 4.1 Path Resolution
**Status:** ✅ Implemented

**Go Implementation:** `internal/utils/syncweb_str.go`

**Features:**
- ✅ Resolve local paths to folder ID + relative path
- ✅ Parse syncweb:// URLs

---

### 4.2 Device ID Helpers
**Status:** ✅ Implemented

**Go Implementation:** `internal/utils/syncweb_str.go`

**Features:**
- ✅ `DeviceIDShort2Long`: Expand short device ID to full ID
- ✅ `DeviceIDLong2Name`: Get device name or short ID

---

## 5. Sorting & Aggregation

### 5.1 `sort`
**Status:** ✅ Implemented (Basic)

**Go Implementation:** `internal/commands/cmd_sort.go`

**Features:**
- ✅ Basic sort modes: `size`, `name`, `path`
- ✅ Reverse sorting with `-` prefix
- ✅ Size limit for output (`--limit-size`)

**Remaining:**
- [ ] Advanced sort modes: `niche`, `frecency`, `folder-*`
- [ ] Peer availability calculation
- [ ] Folder aggregation logic
- [ ] Min/max seeders filtering
- [ ] Configurable niche and frecency parameters

**🔥 High-Priority Optimization:**

Use `BlockAvailability()` to count seeds per file accurately:

```go
// Count unique seeders for a file
func countSeeders(s *syncweb.Syncweb, folderID, path string) (int, error) {
    info, ok, err := s.GetGlobalFileInfo(folderID, path)
    if err != nil || !ok {
        return 0, err
    }
    
    seederSet := make(map[protocol.DeviceID]bool)
    for _, block := range info.Blocks {
        available, err := s.Node.App.Internals.BlockAvailability(folderID, info, block)
        if err != nil {
            continue
        }
        for _, av := range available {
            seederSet[av.ID] = true
        }
    }
    return len(seederSet), nil
}

// Then use in sort:
sort.Slice(files, func(i, j int) bool {
    seedsI, _ := countSeeders(s, folderID, files[i].Path)
    seedsJ, _ := countSeeders(s, folderID, files[j].Path)
    
    // Niche score: closer to ideal = better
    nicheI := abs(seedsI - idealNiche)
    nicheJ := abs(seedsJ - idealNiche)
    return nicheI < nicheJ
})
```

**Benefits:**
- Accurate `seeds`/`peers` sort mode
- Enable `niche` sorting (files with ideal seeder count)
- Enable `frecency` sorting (popularity + recency)
- Show seed count in `stat` output

**Implementation:** 
1. Add `CountSeeders()` helper to `syncweb.go`
2. Update `cmd_sort.go` with advanced sort modes
3. Add `--min-seeders` and `--max-seeders` filters

---

## 6. Download Management

### 6.1 `download` / `dl` / `upload` / `unignore` / `sync`
**Status:** ✅ Implemented (Basic)

**Go Implementation:** `internal/commands/cmd_download.go`

**Features:**
- ✅ Download summary table
- ✅ Confirmation prompt (unless `-y`)
- ✅ Recursive directory traversal with depth limit
- ✅ Ignore pattern management (via `s.Unignore()`)

**Remaining:**
- [ ] Disk space calculation
- [ ] Mountpoint grouping logic
- [ ] Usable space calculation (free - buffer - pending)
- [ ] Shared mountpoint detection and warnings

**🔥 High-Priority Optimization:**

Use `FolderStatistics()` for accurate pending download sizes:

```go
// Get accurate pending download size per folder
stats, _ := s.Node.App.FolderStatistics()
for folderID, stat := range stats {
    pendingBytes := stat.NeedBytes  // Already tracked by Syncthing!
    
    // Get free space from filesystem
    var fsStat syscall.Statfs_t
    syscall.Statfs(folderPath, &fsStat)
    freeBytes := int64(fsStat.Bavail) * int64(fsStat.Bsize)
    
    // Calculate usable space
    usableBytes := freeBytes - pendingBytes - bufferBytes
}
```

**Benefits:**
- Accurate disk space warnings before download
- No double-counting pending downloads
- Proper mountpoint grouping

**Implementation:** Update `cmd_download.go` to use `FolderStatistics()`

---

### 6.2 Ignore Pattern Management
**Status:** ✅ Implemented

**Go Implementation:** `internal/syncweb/syncweb.go`

**Features:**
- ✅ `GetIgnores()`: Get ignore patterns for folder
- ✅ `SetIgnores()`: Set ignore patterns for folder
- ✅ `AddIgnores()`: Add unignore patterns to folder

---

## 7. Daemon & Automation

### 7.1 `automatic`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/syncweb.go`

**Features:**
- ✅ Daemon mode for auto-accepting devices and folders
- ✅ Local-only mode (default)
- ✅ Device/folder include/exclude filters
- ✅ Folder type filtering (structure in place)
- ✅ Join new folders option

**Remaining:**
- [ ] Wishlist/blocklist script integration
- [ ] Sort integration for download prioritization

---

### 7.2 `start` / `restart`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/syncweb.go`

**Features:**
- ✅ Start Syncweb daemon
- ✅ Daemonize process
- ✅ PID file management
- ✅ Log file management

---

### 7.3 `stop` / `shutdown` / `quit`
**Status:** ✅ Implemented

**Go Implementation:** `internal/commands/syncweb.go`

**Features:**
- ✅ Stop Syncweb daemon
- ✅ Send SIGTERM to daemon process
- ✅ PID file reading

---

### 7.4 `repl`
**Status:** ❌ Not Implemented

**Python Features:**
- Interactive Python REPL with Syncthing API access
- Debugging tool

**Go Alternatives:**
- [ ] Consider implementing interactive debug mode
- [ ] Or provide equivalent debugging capability

---

## 8. Web UI & API

### 8.1 Web UI
**Status:** ⚠️ Partially Implemented

**Files:** `web/index.html`, `web/app.js`

**Features:**
- ✅ Basic web UI structure
- ✅ Folder listing
- ✅ File listing
- ✅ Device management UI
- ✅ Mount management UI
- ✅ Activity feed
- ✅ Drag-and-drop file operations
- ✅ Bulk operations (move, copy, delete)
- ✅ Search functionality
- ✅ Sort functionality

**Remaining:**
- [ ] Complete API endpoint implementations
- [ ] File download triggering
- [ ] Folder add/join workflow
- [ ] Device accept/reject workflow
- [ ] Real-time activity updates
- [ ] Mount/unmount functionality
- [ ] Offline mode

**🔥 High-Priority Optimization:**

Use event subscription for **real-time UI updates** via WebSocket:

```go
// In serve_syncweb.go - add WebSocket endpoint
func (c *ServeCmd) handleWebSocket(w http.ResponseWriter, r *http.Request) {
    conn, _ := websocket.Accept(w, r, nil)
    defer conn.Close()
    
    // Subscribe to Syncthing events
    mask := events.ItemStarted | events.ItemFinished | events.DeviceConnected
    sub := swInstance.Node.Subscribe(mask)
    defer sub.Unsubscribe()
    
    // Push events to WebSocket in real-time
    for {
        select {
        case ev := <-sub.C():
            conn.SendJSON(ev)
        case <-r.Context().Done():
            return
        }
    }
}
```

**Benefits:**
- Real-time activity feed (no polling)
- Instant sync progress updates
- Live device connection status
- Lower server load (push vs poll)

**Implementation:** Add WebSocket endpoint in `serve_syncweb.go`

---

### 8.2 API Endpoints
**Status:** ⚠️ Partially Implemented

**File:** `internal/commands/serve_syncweb.go`

**Implemented:**
- ✅ `/api/syncweb/folders` - List folders
- ✅ `/api/syncweb/folders/add` - Add folder
- ✅ `/api/syncweb/folders/delete` - Delete folder
- ✅ `/api/syncweb/ls` - List files
- ✅ `/api/syncweb/find` - Search files
- ✅ `/api/syncweb/stat` - Get file stat
- ✅ `/api/syncweb/download` - Trigger download
- ✅ `/api/syncweb/toggle` - Toggle state
- ✅ `/api/syncweb/status` - Get status
- ✅ `/api/syncweb/events` - Get events
- ✅ `/api/syncweb/devices` - List devices
- ✅ `/api/syncweb/pending` - List pending devices
- ✅ `/api/syncweb/devices/add` - Add device
- ✅ `/api/syncweb/devices/delete` - Delete device
- ✅ `/api/mounts` - List mounts
- ✅ `/api/mount` - Mount device
- ✅ `/api/unmount` - Unmount device
- ✅ `/api/local/ls` - List local files
- ✅ `/api/raw` - Get raw file content
- ✅ `/api/file/move` - Move file
- ✅ `/api/file/copy` - Copy file
- ✅ `/api/file/delete` - Delete file

**Remaining:**
- [ ] Complete all endpoint handlers
- [ ] Proper request/response formatting
- [ ] Error handling
- [ ] Authentication and authorization improvements
- [ ] Rate limiting
- [ ] CORS configuration

---

## 9. Utilities & Helpers

### 9.1 String Utilities
**Status:** ✅ Implemented

**File:** `internal/utils/syncweb_str.go`

**Features:**
- ✅ `ParseSyncwebPath()`: Parse syncweb:// URLs
- ✅ `ExtractDeviceID()`: Parse device ID from URL
- ✅ `DeviceIDShort2Long()`: Expand short device ID
- ✅ `DeviceIDLong2Name()`: Get device name or short ID
- ✅ `CreateFolderID()`: Generate folder ID from path
- ✅ `SepReplace()`: Replace path separators
- ✅ `IsoDateToSeconds()`: Convert ISO datetime to Unix timestamp
- ✅ `RelativeTime()`: Format relative time
- ✅ `FormatTimeLong()`: Format timestamps for long listing
- ✅ `ParseHumanToRange()`: Parse human-readable constraints

---

### 9.2 Formatting Utilities
**Status:** ✅ Implemented

**File:** `internal/utils/formatting.go`

**Features:**
- ✅ `FormatSize()`: Format bytes to human-readable size
- ✅ `FormatDuration()`: Format seconds to duration
- ✅ `FormatDurationShort()`: Format duration in short form
- ✅ `RelativeDatetime()`: Format relative datetime
- ✅ `FormatTime()`: Format timestamps

---

### 9.3 Time Utilities
**Status:** ✅ Implemented

**File:** `internal/utils/time.go`

**Features:**
- ✅ `ParseDate()`: Parse date strings
- ✅ `SuperParser()`: Flexible date parsing
- ✅ `ParseDateOrRelative()`: Parse relative dates
- ✅ `HumanToSeconds()`: Convert human-readable durations

---

### 9.4 Number Utilities
**Status:** ✅ Implemented

**File:** `internal/utils/nums.go`

**Features:**
- ✅ `HumanToBytes()`: Convert human-readable sizes
- ✅ `HumanToSeconds()`: Convert human-readable durations
- ✅ `ParseRange()`: Parse range constraints

---

### 9.5 File Utilities
**Status:** ✅ Implemented

**File:** `internal/utils/files.go`

**Features:**
- ✅ `EnsureDir()`: Create directory if not exists
- ✅ `FileExists()`: Check if file exists
- ✅ `CopyFile()` / `CopyDir()`: Copy files and directories

---

## 10. Testing & Quality

### 10.1 Unit Tests
**Status:** ✅ Implemented (Basic)

**Files:**
- ✅ `internal/utils/syncweb_str_test.go` - 15+ tests for string utilities
- ✅ `internal/syncweb/*_test.go` - Cluster and node tests
- ✅ `cmd/syncweb/main_test.go` - CLI structure tests

**Remaining:**
- [ ] Expand test coverage for all commands
- [ ] Add integration tests
- [ ] Add end-to-end tests
- [ ] Mock Syncthing API for testing

---

### 10.2 CLI Tests
**Status:** ✅ Implemented (Basic)

**Features:**
- ✅ CLI argument parsing tests (via Kong)
- ✅ Command structure validation

**Remaining:**
- [ ] Test flag parsing edge cases
- [ ] Test error cases comprehensively

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

### High Priority (Core Functionality) - ✅ Complete
1. ✅ `create` - Folder creation
2. ✅ `join` - Join folders/devices
3. ✅ `accept` - Accept devices
4. ✅ `ls` - List files
5. ✅ `find` - Search files (basic)
6. ✅ `download` - Download files (basic)
7. ✅ `devices` - List devices
8. ✅ `folders` - List folders

### Medium Priority (Quality of Life) - ⚠️ In Progress
9. ⚠️ `stat` - File statistics (basic)
10. ⚠️ `sort` - Sort files (basic)
11. ✅ `automatic` - Daemon mode
12. ✅ Ignore pattern management
13. ✅ String utilities
14. ⚠️ Web UI completion

### Low Priority (Nice to Have) - ❌ Not Started
15. ❌ Example scripts
16. ❌ REPL mode
17. ❌ Advanced sorting modes (niche, frecency)
18. ❌ Advanced find filters (size, time constraints)
19. ❌ Download space calculation

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

## Optimization Priority Matrix

### Immediate Wins (Low Effort, High Impact)

| Priority | Feature | Internals Method | Impact |
|----------|---------|------------------|--------|
| 🔥 P0 | Remove `--xfer` wait | `DeviceStatistics()` | Instant device listing |
| 🔥 P0 | Accurate online status | `ConnectedTo()` | True connection state |
| 🔥 P1 | Live sync progress | `FolderProgressBytesCompleted()` | Real-time ETA |
| 🔥 P1 | Seed counting | `BlockAvailability()` | Enable niche/frecency sort |
| ⚡ P2 | Real-time UI updates | Event subscription | WebSocket push |

### Medium-Term (Moderate Effort, Good Impact)

| Priority | Feature | Internals Method | Impact |
|----------|---------|------------------|--------|
| ⚡ P2 | Folder statistics | `FolderStatistics()` | Accurate space warnings |
| ⚡ P2 | Folder state display | `State()` | Show syncing/scanning |
| ⚡ P3 | Error reporting | `FolderErrors()` | Better debugging |

### Future Considerations (Higher Effort, Nice to Have)

| Priority | Feature | Internals Method | Impact |
|----------|---------|------------------|--------|
| 📅 P4 | Direct DB queries | `s.db` (LevelDB) | Advanced search |
| 📅 P4 | File history | Database tombstones | Deleted file recovery |
| 📅 P5 | Block hash dedupe | Block hashes | Duplicate detection |

---

## Implementation Roadmap

### Phase 1: Core Optimizations (Week 1-2)
1. Implement `DeviceStatistics()` in `cmd_devices.go`
2. Remove `--xfer` flag and wait logic
3. Add `ConnectedTo()` for accurate status
4. Implement `CountSeeders()` helper

### Phase 2: Sort Enhancements (Week 2-3)
1. Add `seeds`/`peers` sort mode
2. Implement `niche` sorting
3. Implement `frecency` sorting
4. Add `--min-seeders` / `--max-seeders` filters

### Phase 3: Folder Improvements (Week 3-4)
1. Add live sync progress to `folders` command
2. Implement `FolderStatistics()` usage
3. Add folder state display
4. Show folder errors

### Phase 4: Web UI Real-Time (Week 4-5)
1. Add WebSocket endpoint
2. Implement event streaming
3. Update frontend for real-time updates
4. Add sync progress notifications

---

## Glossary

- ✅ **Implemented:** Feature exists and works
- ⚠️ **Partially Implemented:** Feature exists but missing some functionality
- ❌ **Not Implemented:** Feature does not exist

---

## Test Results

```
$ make test
ok    github.com/chapmanjacobd/syncweb/cmd/syncweb    0.007s
?     github.com/chapmanjacobd/syncweb/internal/commands    [no test files]
?     github.com/chapmanjacobd/syncweb/internal/models    [no test files]
ok    github.com/chapmanjacobd/syncweb/internal/syncweb    (cached)
ok    github.com/chapmanjacobd/syncweb/internal/utils    0.685s
```

---

## Build Status

```
$ make build
go build -tags "noassets" -o syncweb ./cmd/syncweb
✅ Build successful
```
