# Syncweb Decision Tables

This document describes all states, transitions, and outputs in the Syncweb system.

---

## 1. Device States

### State Definitions

| State | Symbol | Condition | Priority |
|-------|--------|-----------|----------|
| Local | 🏠 | `DeviceID == localDeviceID` | 1 (highest) |
| Paused | ⏸️ | `config.Paused == true` | 2 |
| Connected | 🌐 | `IsConnectedTo(DeviceID) == true` | 3 |
| Offline | 😴 | `IsConnectedTo(DeviceID) == false` | 4 |
| Pending | 💬 | In `pendingDevices` map | N/A |
| Discovered | 🗨️ | In discovery cache | N/A |

### State Decision Logic

```
IF DeviceID == localDeviceID        → 🏠 Local
ELSE IF config.Paused == true       → ⏸️ Paused
ELSE IF IsConnectedTo(DeviceID)     → 🌐 Connected
ELSE                                → 😴 Offline
```

### State Transitions

| From | Event/Action | To |
|------|--------------|-----|
| (none) | `join` command | 💬 Pending |
| 💬 Pending | `accept` command | 😴 Offline |
| 😴 Offline | Device connects | 🌐 Connected |
| 🌐 Connected | Device disconnects | 😴 Offline |
| Any | `pause` command | ⏸️ Paused |
| ⏸️ Paused | `resume` command | Previous state |
| Any | `drop` command | (removed) |

---

## 2. Folder States

### State Definitions

| State | Condition | Source |
|-------|-----------|--------|
| `idle` | No activity, `FolderState == ""` | Default |
| `syncing` | `FolderState == "syncing"` | Syncthing API |
| `scanning` | `FolderState == "scanning"` | Syncthing API |
| `paused` | `config.Paused == true` | Overrides API |

### State Decision Logic

```
IF config.Paused == true            → paused
ELSE IF FolderState != ""           → FolderState (from API)
ELSE                                → idle
```

### Folder Types

| Type | Constant | Behavior |
|------|----------|----------|
| Send-Receive | `FolderTypeSendReceive` | Two-way sync (default) |
| Send-Only | `FolderTypeSendOnly` | Upload only |
| Receive-Only | `FolderTypeReceiveOnly` | Download only |

### State Transitions

| From | Event/Action | To |
|------|--------------|-----|
| (none) | `create` command | idle |
| idle | `pause` command | paused |
| paused | `resume` command | idle |
| idle | Sync starts | syncing |
| syncing | Sync completes | idle |
| Any | `delete` command | (deleted) |

---

## 3. Syncweb System States

### State Definitions

| State | Condition |
|-------|-----------|
| Not Configured | Syncweb instance not initialized |
| Online | `IsRunning() == true` |
| Offline | `IsRunning() == false` (configured but stopped) |

### State Transitions

| From | Event/Action | To |
|------|--------------|-----|
| Not Configured | `serve` command | Online |
| Online | `toggle(offline=true)` | Offline |
| Offline | `toggle(offline=false)` | Online |
| Online | `stop` command | Offline |
| Offline | `start` command | Online |

### Toggle Decision Logic

```
IF req.Offline == true:
    IF IsRunning() → Stop()  (Online → Offline)
ELSE:
    IF !IsRunning() → Start()  (Offline → Online)
```

---

## 4. File/Item States

### State Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `Local` | bool | File exists locally |
| `IsDir` | bool | Is a directory |
| `Deleted` | flag | Marked for deletion |

### Download Status

| Status | Condition |
|--------|-----------|
| `OK` | `TotalDownload <= Usable` |
| `LOW` | `TotalDownload > Usable` |
| `?` | Unknown (no mountpoint info) |

### Space Calculation

```
Usable = Free - MinFree - PendingDownloads
Status = (TotalDownload > Usable) ? "LOW" : "OK"
```

---

## 5. Search/Filter Decision Logic

### Search Mode Selection

| Flag | Mode | Behavior |
|------|------|----------|
| `-exact` | Exact match | Case-sensitive string comparison |
| `-exact -i` | Exact match (CI) | Case-insensitive string comparison |
| `-glob` | Glob pattern | Convert glob→regex, then match |
| `-fixed-strings` | Literal | Treat as literal string in regex |
| (none) | Regex | Raw regex matching |

### Filter Chain (ALL must pass)

```
IF Type filter passes AND
   Hidden filter passes AND
   Extension filter passes AND
   Size filter passes AND
   Time filter passes AND
   Depth filter passes AND
   Pattern match passes
THEN include in results
```

---

## 6. Auto-Accept Decision Logic

### Device Auto-Accept

```
FOR each pending device:
    IF matches(DevicesInclude) AND NOT matches(DevicesExclude):
        AddDevice(deviceID)
```

### Folder Auto-Join

```
FOR each pending folder:
    IF matches(FoldersInclude) AND NOT matches(FoldersExclude):
        IF (folder exists OR JoinNewFolders == true):
            CreateFolder() + ShareWithDevice()
```

---

## 7. Peer Selection Decision Logic

### Peer Scoring

```
Score = avgTime * (1.0 + errorRate * 10.0)
```

| Metric | Impact |
|--------|--------|
| Lower `avgTime` | Lower score (better) |
| Higher `errorRate` | Higher score (worse) |

### Selection Logic

```
1. Get all peers with block availability
2. Sort by Score (ascending)
3. Try best peer first
4. On failure, try next best
5. Record measurement for future scoring
```

---

## 8. API Request Decision Logic

### Authentication

```
IF request from localhost OR
   valid X-API-Key token provided
THEN allow request
ELSE return 403 Forbidden
```

### CSRF Protection

```
IF method == GET OR
   request from localhost
THEN allow
ELSE check CSRF token
```

### Path Security

```
IF path contains ".." OR
   path is absolute
THEN reject with error
ELSE process request
```

---

## 9. Output Types Summary

### API Response Types

| Endpoint | Response Type | Fields |
|----------|---------------|--------|
| `/api/syncweb/folders` | `[]FolderInfo` | ID, Label, Path, Type, Paused, Devices |
| `/api/syncweb/ls` | `[]LsEntry` | Name, Path, IsDir, Local, Size, Type, Modified |
| `/api/syncweb/find` | `[]LsEntry` | Same as ls + filter metadata |
| `/api/syncweb/stat` | `FileInfo` | Name, Size, Modified, Hash, Version |
| `/api/syncweb/download` | `DownloadResponse` | Status (OK/LOW), Space info |
| `/api/syncweb/toggle` | `ToggleResponse` | Offline (bool) |
| `/api/syncweb/status` | `StatusResponse` | Running, Configured, Version |
| `/api/syncweb/events` | `[]SyncEvent` | Time, Type, Message, Data |
| `/api/syncweb/devices` | `[]DeviceInfo` | ID, Name, Addresses, Introducer, Paused |
| `/api/syncweb/pending` | `PendingResponse` | Devices, Folders |

### Error Response

```json
{
  "error": "error message"
}
```

---

## 10. Event Types

| Event | Trigger | Handler |
|-------|---------|---------|
| `DeviceRejected` | Unknown device connects | Add to `pendingDevices` |
| `DeviceConnected` | Device establishes connection | Remove from `pendingDevices` |
| `ItemStarted` | File sync begins | Log sync start |
| `ItemFinished` | File sync completes | Log sync completion, update stats |
| `FolderSummary` | Folder state changes | Log folder stats |

---

## 11. Command Reference

| Command | Input | Output/Effect |
|---------|-------|---------------|
| `create` | folderID, label | New SendOnly folder |
| `join` | folderID, label | New ReceiveOnly folder (paused) |
| `accept` | deviceID | Device moves from Pending → Offline |
| `drop` | deviceID | Device removed from config |
| `folders` | (none) | List of FolderInfo |
| `devices` | (none) | List of DeviceInfo with status |
| `ls` | path | List of LsEntry |
| `find` | pattern, filters | Filtered list of LsEntry |
| `download` | paths... | Download marked, Status returned |
| `automatic` | (daemon) | Auto-accepts matching devices/folders |
| `serve` | (daemon) | Start web UI |
| `start` | (none) | Syncweb → Online |
| `stop` | (none) | Syncweb → Offline |

---

## 12. Complete State Machine Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        DEVICE LIFECYCLE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  (unknown) ──join──> 💬 Pending ──accept──> 😴 Offline         │
│                                               │                 │
│                    ┌──────────────────────────┘                 │
│                    │ connect                                    │
│                    ▼                                            │
│                   🌐 Connected <──────┐                         │
│                    │                  │ disconnect               │
│                    └──────────────────┘                         │
│                    │                                            │
│              pause │ resume                                     │
│                    ▼                                            │
│                  ⏸️ Paused                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                        FOLDER LIFECYCLE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  (none) ──create──> idle <──────┐                              │
│                    │            │                              │
│                    │ sync start │ sync complete                │
│                    ▼            │                              │
│                 syncing ────────┘                              │
│                    │                                            │
│              pause │ resume                                     │
│                    ▼                                            │
│                  paused                                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      SYNCWEB LIFECYCLE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  (none) ──serve──> Online <──────┐                             │
│                    │             │                             │
│              stop  │ start       │ toggle                      │
│                    ▼             │                             │
│                 Offline ─────────┘                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## File References

| Component | Source Files |
|-----------|--------------|
| Device States | `internal/commands/cmd_devices.go` |
| Folder States | `internal/commands/cmd_folders.go` |
| System States | `internal/commands/serve_syncweb.go` |
| API Types | `internal/models/api.go` |
| Syncweb Logic | `internal/syncweb/syncweb.go` |
| Node Logic | `internal/syncweb/node.go` |
| Download Logic | `internal/commands/cmd_download.go` |
| Search Logic | `internal/commands/cmd_find.go` |
