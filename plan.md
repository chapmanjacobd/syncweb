# Syncweb Go Port - Feature Parity Plan

This document outlines the remaining work to achieve feature parity between the Python repository (`syncweb-py`) and the Go repository (`syncweb`).

**Last Updated:** 2026-03-05

---

## Summary

| Category | Remaining Work |
|----------|----------------|
| Core Commands | 1 (version command) |
| File Operations | 1 (stat enhancements) |
| Download Management | 1 (disk space calculation) |
| Daemon & Automation | 1 (repl) |
| Web UI & API | 2 (UI completion, API polish) |
| Testing & Quality | 1 (test coverage) |
| Documentation | 3 (examples, README, FAQ) |

**Overall Progress:** ~85% complete

---

## Completed Optimizations

The following optimizations have been implemented:

1. ✅ Remove `--xfer` flag wait - Uses `IsConnectedTo()` for instant connection status
2. ✅ Real-time folder sync progress - Uses `FolderProgressBytesCompleted()`
3. ✅ Instant connection status - Uses `ConnectedTo()` for accurate online/offline
4. ✅ Direct block availability for sort - Uses `BlockAvailability()` for seed counting
5. ✅ Folder statistics - Uses `GlobalSize()`, `LocalSize()`, `NeedSize()`
6. ✅ Folder state display - Uses `FolderState()`
7. ✅ Advanced sort modes - `seeds`, `peers`, `niche`, `frecency`

---

## Remaining Work

### 1. Core Commands

#### 1.5 `version`
**Status:** ⚠️ Partially Implemented

**Go Implementation:** `internal/commands/syncweb.go` (SyncwebVersionCmd)

**Remaining:**
- [ ] Show Syncthing version from API

---

### 2. File Operations

#### 2.2 `find` / `fd` / `search`
**Status:** ✅ Implemented

#### 2.3 `stat`
**Status:** ⚠️ Partially Implemented

**Remaining:**
- [ ] Local vs Global comparison display
- [ ] Device availability with name resolution
- [ ] Version vector display
- [ ] Flags display (deleted, ignored, invalid)
- [ ] Modified by tracking

---

### 3. Download Management

#### 3.1 `download` / `dl` / `upload` / `unignore` / `sync`
**Status:** ✅ Implemented (Basic)

**Remaining:**
- [ ] Disk space calculation
- [ ] Mountpoint grouping logic
- [ ] Usable space calculation (free - buffer - pending)
- [ ] Shared mountpoint detection and warnings

**Implementation:** Update `cmd_download.go` to use `FolderStatistics()`

---

### 4. Daemon & Automation

#### 4.1 `repl`
**Status:** ❌ Not Implemented

**Python Features:**
- Interactive Python REPL with Syncthing API access
- Debugging tool

**Go Alternatives:**
- [ ] Consider implementing interactive debug mode
- [ ] Or provide equivalent debugging capability

---

### 5. Web UI & API

#### 5.1 Web UI
**Status:** ⚠️ Partially Implemented

**Files:** `web/index.html`, `web/app.js`

**Remaining:**
- [ ] Complete API endpoint implementations
- [ ] File download triggering
- [ ] Folder add/join workflow
- [ ] Device accept/reject workflow
- [ ] Real-time activity updates
- [ ] Mount/unmount functionality
- [ ] Offline mode

**High-Priority:** Add WebSocket endpoint for real-time UI updates

#### 5.2 API Endpoints
**Status:** ⚠️ Partially Implemented

**File:** `internal/commands/serve_syncweb.go`

**Remaining:**
- [ ] Complete all endpoint handlers
- [ ] Proper request/response formatting
- [ ] Error handling
- [ ] Authentication and authorization improvements
- [ ] Rate limiting
- [ ] CORS configuration

---

### 6. Testing & Quality

#### 6.1 Unit Tests
**Status:** ✅ Implemented (Basic)

**Remaining:**
- [ ] Expand test coverage for all commands
- [ ] Add integration tests
- [ ] Add end-to-end tests
- [ ] Mock Syncthing API for testing

#### 6.2 CLI Tests
**Status:** ✅ Implemented (Basic)

**Remaining:**
- [ ] Test flag parsing edge cases
- [ ] Test error cases comprehensively

---

### 7. Documentation & Examples

#### 7.1 Example Scripts
**Status:** ❌ Not Implemented

**Python Examples (to port):**
- `install.sh`: Installation script for syncweb-automatic
- `simple_wishlist.sh`: Simple wishlist generator
- `syncweb-blocklist.sh`: Blocklist script
- `syncweb-wishlist.sh`: Wishlist script
- `syncweb-automatic.service`: Systemd service file

**Remaining:**
- [ ] Create example scripts for Go version
- [ ] Update systemd service file
- [ ] Create installation script

#### 7.2 README Updates
**Status:** ❌ Not Implemented

**Remaining:**
- [ ] Update README with Go-specific installation
- [ ] Update usage examples
- [ ] Document differences from Python version
- [ ] Add migration guide

#### 7.3 FAQ Updates
**Status:** ❌ Not Implemented

**Remaining:**
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

### Medium Priority (Quality of Life) - In Progress
9. ⚠️ `stat` - File statistics (5 remaining features)
10. ⚠️ `find` - Search enhancements (6 remaining features)
11. ⚠️ Web UI completion (7 remaining features)
12. ⚠️ API endpoint polish (6 remaining tasks)

### Low Priority (Nice to Have) - Not Started
13. ❌ Example scripts (5 scripts to port)
14. ❌ REPL mode
15. ❌ Download space calculation (4 remaining features)
16. ❌ Documentation updates (README, FAQ)

---

## Future Optimization Opportunities

### Event-Driven Activity Feed
Use event subscription for real-time UI updates via WebSocket:
- Subscribe to Syncthing events
- Push to WebSocket for real-time UI updates
- Benefits: No polling overhead, instant updates

### Direct Database Access
Direct LevelDB access via `s.db` for advanced queries:
- Query file history without scanning
- Find deleted files from tombstones
- Calculate folder sizes from index
- Fast duplicate detection via block hashes

---

## Implementation Roadmap

### Phase 1: File Operations Polish (Next)
1. [ ] Add glob/exact match to `find`
2. [ ] Add size constraints to `find`
3. [ ] Add time-based filtering to `find`
4. [ ] Enhance `stat` with device availability
5. [ ] Add version vector display to `stat`

### Phase 2: Download Management
1. [ ] Implement disk space calculation using `FolderStatistics()`
2. [ ] Add mountpoint grouping logic
3. [ ] Add usable space calculation
4. [ ] Add shared mountpoint warnings

### Phase 3: Web UI Completion
1. [ ] Add WebSocket endpoint for real-time updates
2. [ ] Complete file download triggering
3. [ ] Implement folder add/join workflow
4. [ ] Implement device accept/reject workflow
5. [ ] Add mount/unmount functionality

### Phase 4: Documentation
1. [ ] Create example scripts
2. [ ] Update README
3. [ ] Update FAQ
4. [ ] Add migration guide

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
