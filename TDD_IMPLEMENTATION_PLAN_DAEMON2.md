# TDD Implementation Plan for Daemon Mode — Phase 2 (Remaining Work)

Following the strict TDD workflow:
1. Write failing test first → Watch it fail (Red)
2. Write minimal implementation → Make test pass (Green)
3. Refactor while keeping tests green (Refactor)
4. Run `cargo test --all-targets --all-features` after each module
5. Run `cargo clippy --all-targets --all-features -- -D warnings` after each phase
6. Run `cargo fmt --all` before commit

---

## Motivation

Phase 1 (TDD_IMPLEMENTATION_PLAN_DAEMON.md) established the daemon architecture:
PID lock, state file, Unix socket IPC server/client, supervision loop, Rayon
archive pool, TryReference import mode, watch integration, schedule-aware sync,
status reporting, and folder management — all working and committed.

**Remaining work falls into three categories:**

1. **Command-parity gap**: ~20 CLI commands (`leave`, `unsubscribe`, `unpublish`,
   `snapshot *`, `collection *`, `package *`, `network *`, `indexing *`,
   `link *`, `mirror *`, `trust *`, `attest`, `report`, `moderation *`) still
   open an embedded `IrohNode` directly instead of routing through the daemon.
   Every one of these touches the blobstore or docs engine and risks a redb
   lock conflict when a daemon is already running.

2. **IPC handler test gap**: The new IPC handlers (`CreateFolder`, `HealthCheck`,
   `VerifyIntegrity`, and the newly-enabled `Join`, `Publish`, `Subscribe`)
   lack unit tests. The old "disabled" handlers (`Join`, `Publish`,
   `Subscribe`) previously returned `"node-access IPC command is not available"`
   and now execute real operations — their test coverage is zero.

3. **Integration & benchmark gap**: No end-to-end daemon lifecycle integration
   tests exist. No daemon-specific benchmarks exist. The existing
   `core_benchmarks.rs` covers only non-daemon paths.

---

## Current Architecture

```
┌─────────────────────────────────────────────────┐
│  Daemon process                                 │
│  ┌─────────┐ ┌──────────┐ ┌───────────────────┐│
│  │ IrohNode│ │ SyncLoop │ │ Rayon archive pool││
│  └────┬────┘ └────┬─────┘ └────────┬──────────┘│
│       │           │                │            │
│  ┌────┴───────────┴────────────────┴──────────┐ │
│  │          Unix Socket IPC Server            │ │
│  └───────────────────┬───────────────────────┘ │
└──────────────────────┼─────────────────────────┘
                       │ JSON over Unix socket
┌──────────┬───────────┬───────────┬──────────────┐
│ download │ import    │ folders   │ create       │
│ (daemon) │ (daemon)  │ (daemon)  │ (daemon)     │
├──────────┼───────────┼───────────┼──────────────┤
│ health   │ verify    │ init      │ join --once  │
│ (daemon) │ (daemon)  │ (daemon)  │ (daemon)     │
├──────────┼───────────┼───────────┼──────────────┤
│ publish  │ subscribe │ leave     │ unsubscribe  │
│ (daemon) │ (daemon)  │ EMBEDDED  │ EMBEDDED     │
├──────────┼───────────┼───────────┼──────────────┤
│ snapshot*│ collection│ package*  │ network*     │
│ EMBEDDED │ * EMBEDDED│ EMBEDDED  │ EMBEDDED     │
├──────────┼───────────┼───────────┼──────────────┤
│ indexing*│ link*     │ trust*    │ moderation*  │
│ EMBEDDED │ EMBEDDED  │ EMBEDDED  │ EMBEDDED     │
└──────────┴───────────┴───────────┴──────────────┘
```

**Legend:** `(daemon)` = routes through daemon IPC by default; `EMBEDDED` = opens `IrohNode` directly (needs conversion).

---

## Identified Gaps

| Gap | Current State | Fix |
|-----|---------------|-----|
| **No daemon IPC for 20+ blobstore commands** | `leave`, `unsubscribe`, `snapshot`, etc. open their own `IrohNode` | Add `IpcCommand` variants + daemon handlers for each; update CLI to route through daemon first |
| **No test coverage for new IPC handlers** | `CreateFolder`, `HealthCheck`, `VerifyIntegrity`, `Join`, `Publish`, `Subscribe` have zero unit tests | Add unit tests to existing `ipc.rs` test module |
| **No daemon integration tests** | No end-to-end start/stop/sync/ipc tests | Create `tests/integration/daemon_test.rs` |
| **No daemon benchmarks** | `core_benchmarks.rs` skips all daemon paths | Add IPC round-trip, supervision restart, state file benchmarks |
| **Disabled Join/Publish/Subscribe IPC now live** | These three IPC commands previously returned an error; they now execute real operations | Add both unit and integration tests to verify they work |
| **Lock contention risk** | Users running embedded commands while daemon is active get redb `DatabaseAlreadyOpen` | All blobstore commands must daemon-route by default; `--no-daemon` is the explicit opt-out |

---

## Phase 11A: IPC Command Parity — Remaining CLI Commands (Weeks 1-4)

Each command needs:
1. New `IpcCommand` variant in `syncweb-core/src/daemon/ipc.rs`
2. Handler method on `IpcServer` in `ipc.rs`
3. Response type (or reuse `IpcResponse::Ok { message }`)
4. CLI handler update in `syncweb-cli/src/main.rs` to route through daemon first
5. Unit tests for the IPC handler
6. Unit/integration tests for the CLI routing

### 11.1 `unsubscribe` via IPC (TDD)

**IPC Command:**
```rust
IpcCommand::Unsubscribe { namespace: String },
```

**Daemon handler:** Calls `cancel_session(namespace_id)`. Cancels the active
sync session and removes any supervised intent for that namespace.

**CLI update:** `handle_unsubscribe()` currently opens a node, resolves the
namespace, and calls `cancel_session`. Route through daemon by default.

**Tests** (in `tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_unsubscribe_cancels_session` — sends unsubscribe, session is cancelled
- [ ] `test_ipc_unsubscribe_nonexistent_namespace` — returns error for unknown namespace

**Tests** (in `tests/integration/daemon_test.rs`):
- [ ] `test_cli_unsubscribe_routes_through_daemon` — CLI sends IPC when daemon running

### 11.2 `leave` via IPC (TDD)

**IPC Command:**
```rust
IpcCommand::LeaveFolder { namespace: String },
```

**Daemon handler:** Uses `FolderManager::drop(namespace)` to remove the folder
from the Iroh namespace and clean up blobstore references. Also removes from
`FolderRegistry` and cancels any active session.

**CLI update:** `handle_leave()` currently opens a node, resolves namespace,
drops folder. Route through daemon by default.

**Tests** (in `tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_leave_folder_drops_from_node` — folder removed from FolderManager
- [ ] `test_ipc_leave_folder_removes_from_registry` — folder removed from FolderRegistry
- [ ] `test_ipc_leave_folder_cancels_session` — active session cancelled
- [ ] `test_ipc_leave_folder_nonexistent` — returns error

**Tests** (in `tests/integration/daemon_test.rs`):
- [ ] `test_cli_leave_routes_through_daemon` — CLI sends IPC when daemon running

### 11.3 `unpublish` via IPC (TDD)

**IPC Command:**
```rust
IpcCommand::Unpublish { namespace: String, blob: String },
```

**Daemon handler:** Gets folder from `FolderManager`, calls
`folder.unpublish_blob(hash)` to remove the blob pin.

**CLI update:** `handle_unpublish()` currently opens a node. Route through daemon.

**Tests** (in `tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_unpublish_removes_blob_pin` — blob unpinned from folder
- [ ] `test_ipc_unpublish_invalid_hash` — returns error for bad hash

### 11.4 `snapshot` subcommands via IPC (TDD)

**Snapshot operations that touch blobstore/docs engine:**
- `snapshot create [PATH]` — reads files, adds to blobstore, writes snapshot metadata
- `snapshot list [PATH]` — reads snapshot store
- `snapshot delete <PATH> <ID>` — removes snapshot, releases pins
- `snapshot restore <PATH> <ID>` — reads snapshot, writes files (file I/O, not just blobstore)
- `snapshot diff <PATH> <A> <B>` — reads snapshot metadata, compares entries

**For `create`, `list`, `delete`:** Add IPC commands and handlers. These are
pure blobstore/docs-engine operations that must go through the daemon.

**For `restore` and `diff`:** These involve filesystem I/O (writing files,
comparing paths) in addition to blobstore reads. The blobstore reads should go
through the daemon; the filesystem work happens locally. Split: daemon returns
snapshot data, CLI writes files / computes diffs.

**IPC Commands:**
```rust
IpcCommand::SnapshotCreate { path: PathBuf, description: Option<String>, threads: usize },
IpcCommand::SnapshotList { path: PathBuf },
IpcCommand::SnapshotDelete { id: String },
```

**Daemon handlers:** Use `SnapshotStore::with_docs(node.blob_store(), node.docs_engine())`.

**CLI update:** `handle_snapshot()` dispatches to sub-handlers. Route `create`,
`list`, `delete` through daemon by default. `restore` and `diff` remain
embedded (they do local filesystem work after reading data).

**Tests** (in `tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_snapshot_create` — creates snapshot via daemon
- [ ] `test_ipc_snapshot_list` — lists snapshots via daemon
- [ ] `test_ipc_snapshot_delete` — deletes snapshot via daemon
- [ ] `test_ipc_snapshot_create_from_folder` — snapshot from managed folder

### 11.5 `collection publish` via IPC (TDD)

**IPC Command:**
```rust
IpcCommand::CollectionPublish {
    path: PathBuf,
    namespace: String,
    sequence: u64,
    bootstrap: Vec<String>,
},
```

**Daemon handler:** Reads manifest, adds files to blobstore, gets folder,
publishes with `CollectionStore`, creates announcement via `PackageCatalog`.

**CLI update:** The existing `handle_collection` dispatches to `Init`, `Add`,
`Versions`, `Publish`. Only `Publish` touches blobstore. Route `Publish` through
daemon by default.

**Tests** (in `tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_collection_publish` — collection manifest published via daemon

### 11.6 `package import` and `package export` via IPC (TDD)

These already have daemon IPC commands (`ImportArchive`, `ExportArchive`) and
handlers. The CLI already routes `handle_package_archive_import` and
`handle_package_archive_export` through the daemon when `archive_context` is
available.

**Remaining work:** Verify the CLI's `handle_package` dispatch in `main.rs`
routes `PackageCommand::Import` and `PackageCommand::Export` through daemon IPC
by default. Currently these commands open `open_node()` directly within their
handler helpers.

**Tests** (in `tests/integration/daemon_test.rs`):
- [ ] `test_package_import_via_daemon` — package import routes through IPC
- [ ] `test_package_export_via_daemon` — package export routes through IPC

### 11.7 `package install`/`upgrade`/`info` with `--ticket` via IPC (TDD)

When `install`, `upgrade`, or `info` is called with `--ticket`, they fetch a
blob from the network via `node.blob_store().fetch()`. This requires the node.

**No new IPC command needed:** These can use the existing `ExportArchive` or a
generic `FetchBlob` IPC, or simply open an embedded node with `--no-daemon`
semantics.

**For now:** Document that `install/upgrade/info --ticket` should be run with
`--no-daemon` or when daemon is stopped, to avoid lock contention.

### 11.8 Remaining always-embedded commands (deferred)

These commands touch blobstore indirectly or not at all:

| Command | Blobstore? | Notes |
|---------|-----------|-------|
| `network test-relay` | No | Opens relay connection only |
| `package search` | No | Uses gossip service, not blobstore |
| `indexing *` | No | Uses SQLite, not redb |
| `link *` | Yes | Reads/writes blobstore via LinkStore |
| `mirror add` | Yes | Registers provider reference |
| `trust *` | No | Local file/index operations |
| `attest` | No | Local file/index operations |
| `report` | No | Local file/index operations |
| `moderation *` | No | Local file/index operations |

**For `link` and `mirror`:** These touch the blobstore/index but have no
daemon IPC yet. Deferred to a future phase.

**For all others:** These are local file/index operations that don't open
an `IrohNode`. No daemon IPC needed.

---

## Phase 11B: IPC Handler Unit Tests — Weeks 5-6

### 11.9 `CreateFolder` IPC handler tests (TDD)

**Test file:** `syncweb-core/src/daemon/ipc.rs` (existing test module)

**New tests:**
- [ ] `test_ipc_create_folder_creates_and_returns_message` — sends `CreateFolder`,
      receives `Ok` with namespace + ticket in message
- [ ] `test_ipc_create_folder_invalid_mode` — invalid sync mode returns error
- [ ] `test_ipc_create_folder_no_archive_context` — returns error when daemon
      has no node context
- [ ] `test_ipc_create_folder_duplicate_namespace` — second create returns error
      (NamespaceId collision is probabilistic; test reflects error handling)

### 11.10 `HealthCheck` IPC handler tests (TDD)

- [ ] `test_ipc_health_check_returns_report` — sends `HealthCheck`, receives `Ok`
- [ ] `test_ipc_health_check_no_archive_context` — returns error without node
- [ ] `test_ipc_health_check_unknown_folder` — nonexistent path returns error

### 11.11 `VerifyIntegrity` IPC handler tests (TDD)

- [ ] `test_ipc_verify_integrity_returns_result` — sends `VerifyIntegrity`,
      receives `Ok` with total/verified/corrupted/missing counts
- [ ] `test_ipc_verify_integrity_no_archive_context` — returns error without node
- [ ] `test_ipc_verify_integrity_unknown_folder` — nonexistent path returns error

### 11.12 `Join` IPC handler tests (TDD)

- [ ] `test_ipc_join_folder_joins_and_returns_message` — sends `Join`, receives `Ok`
- [ ] `test_ipc_join_folder_invalid_ticket` — bad ticket returns error
- [ ] `test_ipc_join_folder_no_archive_context` — returns error without node

### 11.13 `Publish` IPC handler tests (TDD)

- [ ] `test_ipc_publish_folder_ticket` — sends `Publish` without blob, gets ticket
- [ ] `test_ipc_publish_blob_ticket` — sends `Publish` with blob hash, gets blob ticket
- [ ] `test_ipc_publish_invalid_namespace` — bad namespace returns error
- [ ] `test_ipc_publish_no_archive_context` — returns error without node

### 11.14 `Subscribe` IPC handler tests (TDD)

- [ ] `test_ipc_subscribe_returns_ok` — sends `Subscribe`, receives `Ok`
- [ ] `test_ipc_subscribe_with_params` — params are forwarded correctly
- [ ] `test_ipc_subscribe_no_archive_context` — returns error without node

---

## Phase 11C: Integration Tests — Week 7

### 11.15 Daemon lifecycle integration tests

**New file:** `syncweb-cli/tests/daemon_integration_test.rs`

These tests spawn the actual `syncweb` binary (via `CARGO_BIN_EXE_syncweb`),
manage a daemon process, and exercise IPC commands end-to-end.

**Test infrastructure:**
```rust
fn daemon_test_dir(name: &str) -> PathBuf;
fn start_daemon(data_dir: &Path) -> Result<Child>;
fn stop_daemon(child: &mut Child) -> Result<()>;
fn wait_for_daemon_ready(data_dir: &Path, timeout: Duration) -> Result<()>;
fn daemon_run(args: &[&str]) -> Result<Output>;
```

- [ ] `test_daemon_start_stop_lifecycle` — start daemon, verify status, stop cleanly,
      verify state file removed, verify socket removed
- [ ] `test_daemon_auto_start_on_download` — run `download` without `--no-daemon`
      and without a running daemon; daemon auto-starts
- [ ] `test_daemon_auto_start_on_create` — run `create` without `--no-daemon`;
      daemon auto-starts and handles the create
- [ ] `test_daemon_auto_start_on_folders` — run `folders` without `--no-daemon`;
      daemon auto-starts
- [ ] `test_daemon_routes_create_through_ipc` — daemon running, run `create --no-daemon`
      vs without `--no-daemon`; both produce a folder but via different paths
- [ ] `test_daemon_routes_health_through_ipc` — daemon running, `health` routes
      through IPC
- [ ] `test_daemon_routes_verify_through_ipc` — daemon running, `verify` routes
      through IPC
- [ ] `test_daemon_routes_init_through_ipc` — daemon running, `init` routes
      through IPC
- [ ] `test_daemon_routes_publish_through_ipc` — daemon running, `publish` routes
      through IPC
- [ ] `test_daemon_routes_subscribe_through_ipc` — daemon running, `subscribe`
      routes through IPC
- [ ] `test_daemon_routes_join_once_through_ipc` — daemon running, `join --once`
      routes through IPC
- [ ] `test_daemon_multiple_ipc_commands` — run several IPC commands in sequence
      against a running daemon; all succeed
- [ ] `test_daemon_shutdown_via_ipc` — run `daemon-shutdown`, verify daemon stops
      cleanly
- [ ] `test_daemon_reload_via_ipc` — run `daemon-reload`, verify daemon responds
- [ ] `test_daemon_sync_via_ipc` — run `daemon-sync`, verify daemon responds
- [ ] `test_daemon_unwatch_via_ipc` — register a folder, then `unwatch`, verify
      daemon removes it
- [ ] `test_daemon_two_instances_cannot_start` — start daemon, try to start
      a second; second fails with clear message
- [ ] `test_cli_no_daemon_flag_bypasses_daemon` — daemon running, `create --no-daemon`
      opens embedded node (no IPC); verify no lock conflict
- [ ] `test_cli_default_is_daemon_mode` — daemon running, `create` (no flags)
      routes through IPC; verify lock is held by daemon, not newly acquired
- [ ] `test_daemon_folders_list_differs_from_embedded` — daemon `folders` shows
      operational status; embedded `folders --no-daemon` shows config mode

### 11.16 `--no-daemon` flag routing tests

- [ ] `test_no_daemon_flag_works_for_all_commands` — for each selective command,
      verify `--no-daemon` executes embedded without error
- [ ] `test_no_daemon_flag_embedded_alias` — `--embedded` is alias for `--no-daemon`

---

## Phase 11D: Benchmarks — Week 8

### 11.17 Daemon performance benchmarks

**Existing file:** `syncweb-core/benches/core_benchmarks.rs`

**New benchmarks:**
- [ ] `bench_ipc_round_trip` — measure time to send an IPC request and receive
      a response. Target: < 1ms per round trip (local Unix socket).
      Setup: start daemon, measure `send(IpcCommand::Status)` → response.
- [ ] `bench_ipc_download_through_daemon` — measure `download` via IPC vs
      embedded node. Target: IPC overhead < 20% of total operation time.
- [ ] `bench_ipc_create_folder` — measure `create` via IPC vs embedded.
      Target: IPC overhead < 20%.
- [ ] `bench_ipc_health_check` — measure `health` via IPC vs embedded.
- [ ] `bench_ipc_verify_integrity` — measure `verify` via IPC vs embedded.
- [ ] `bench_supervisor_intent_restart` — measure time for
      `IntentSupervisor::supervise()` to detect a failed intent and restart it.
      Target: < 500ms.
- [ ] `bench_state_file_write_read` — measure `StateFile::save()` +
      `StateFile::load()`. Target: < 10ms.
- [ ] `bench_daemon_start_stop` — measure cold start + clean shutdown.
      Target: < 5s for start, < 1s for shutdown.

---

## Phase 11E: Documentation & Hardening — Week 9

### 11.18 Error message consistency

Audit all daemon IPC handlers and CLI fallback paths for consistent error
messages. Every "daemon not available" message should mention
`--no-daemon` / `--embedded` as the fallback option.

**Files:**
- `syncweb-core/src/daemon/ipc.rs` — all `response_from_error()` calls
- `syncweb-cli/src/main.rs` — `print_daemon_message()`, `daemon_client_or_start()`

- [ ] `test_daemon_unavailable_message_mentions_no_daemon_flag` — error message
      contains "--no-daemon" or "--embedded"
- [ ] `test_embedded_flag_works_without_daemon` — running with `--no-daemon`
      when no daemon exists succeeds without error

### 11.19 `--help` output verification

- [ ] `test_help_mentions_daemon_mode` — `syncweb --help` mentions daemon mode
- [ ] `test_help_mentions_no_daemon_flag` — `syncweb --help` lists `--no-daemon`
- [ ] `test_command_help_mentions_daemon_routing` — `syncweb create --help`
      mentions default daemon routing

### 11.20 `verify` command classification

The `verify` command was moved from auxiliary to non-auxiliary in Phase 1.
Ensure its `--help` output is correct and it appears in the right help section.

- [ ] `test_verify_help_lists_selector_arg` — `syncweb verify --help` shows path arg
- [ ] `test_verify_not_listed_as_daemon_subcommand` — `syncweb --help` does not
      list verify under daemon management section

---

## New / Modified Files

| File | Purpose |
|------|---------|
| `syncweb-core/src/daemon/ipc.rs` | Add `Unsubscribe`, `LeaveFolder`, `Unpublish`, `SnapshotCreate`, `SnapshotList`, `SnapshotDelete`, `CollectionPublish` variants + handlers + unit tests |
| `syncweb-cli/src/main.rs` | Update `handle_unsubscribe`, `handle_leave`, `handle_unpublish`, `handle_snapshot`, `handle_collection` to route through daemon by default |
| `syncweb-cli/tests/daemon_integration_test.rs` | **NEW** — End-to-end daemon lifecycle and IPC integration tests |
| `syncweb-core/benches/core_benchmarks.rs` | Add daemon-specific benchmarks |

---

## Detailed Implementation: IPC Commands

### `Unsubscribe`

```rust
// In IpcCommand enum:
Unsubscribe { namespace: String },

// In IpcServer::handle_request():
IpcCommand::Unsubscribe { namespace } => self.handle_unsubscribe(namespace).await,

// Handler:
async fn handle_unsubscribe(&self, namespace: String) -> IpcResponse {
    let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
        Ok(id) => id,
        Err(error) => return IpcResponse::Error { message: format!("invalid namespace: {error}") },
    };
    if cancel_session(namespace_id) {
        IpcResponse::Ok { message: format!("unsubscribed: {namespace}") }
    } else {
        IpcResponse::Error { message: format!("no active session for {namespace}") }
    }
}
```

### `LeaveFolder`

```rust
// In IpcCommand enum:
LeaveFolder { namespace: String },

// Handler:
async fn handle_leave_folder(&self, namespace: String) -> IpcResponse {
    let context = match &self.archive_context {
        Some(ctx) => ctx.clone(),
        None => return IpcResponse::Error { message: "daemon has no node context".to_owned() },
    };
    let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
        Ok(id) => id,
        Err(error) => return IpcResponse::Error { message: format!("invalid namespace: {error}") },
    };
    let _ = cancel_session(namespace_id);
    let manager = FolderManager::new(&context.node);
    match manager.drop(namespace_id).await {
        Ok(()) => {
            let _ = self.daemon_handle.folder_registry.write().await.remove(&namespace_id);
            IpcResponse::Ok { message: format!("left: {namespace}") }
        }
        Err(error) => response_from_error(error),
    }
}
```

### `Unpublish`

```rust
// In IpcCommand enum:
Unpublish { namespace: String, blob: String },

// Handler:
async fn handle_unpublish(&self, namespace: String, blob: String) -> IpcResponse {
    let context = match &self.archive_context {
        Some(ctx) => ctx.clone(),
        None => return IpcResponse::Error { message: "daemon has no node context".to_owned() },
    };
    let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
        Ok(id) => id,
        Err(error) => return IpcResponse::Error { message: format!("invalid namespace: {error}") },
    };
    let hash = match blob.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(error) => return IpcResponse::Error { message: format!("invalid blob hash: {error}") },
    };
    let manager = FolderManager::new(&context.node);
    let folder = match manager.get(namespace_id).await {
        Ok(f) => f,
        Err(error) => return response_from_error(error),
    };
    match folder.unpublish_blob(hash).await {
        Ok(()) => IpcResponse::Ok { message: format!("unpublished: {blob}") },
        Err(error) => response_from_error(error),
    }
}
```

### `SnapshotCreate`

```rust
// In IpcCommand enum:
SnapshotCreate { path: PathBuf, description: Option<String>, threads: usize },

// Handler:
async fn handle_snapshot_create(&self, path: PathBuf, description: Option<String>, threads: usize) -> IpcResponse {
    let context = match &self.archive_context {
        Some(ctx) => ctx.clone(),
        None => return IpcResponse::Error { message: "daemon has no node context".to_owned() },
    };
    let snapshots = SnapshotStore::with_docs(
        context.node.blob_store().clone(),
        context.node.docs_engine().clone(),
    );
    let result = if path.exists() {
        snapshots.create_from_path(&path, threads, description).await
    } else {
        let manager = FolderManager::new(&context.node);
        let folder = match resolve_folder_for_daemon(&manager, &path).await {
            Ok(f) => f,
            Err(error) => return error,
        };
        snapshots.create_for_folder(&folder, description).await
    };
    match result {
        Ok(snapshot) => IpcResponse::Ok {
            message: format!("snapshot: {}\nroot_hash: {}\nfiles: {}\nsize: {}",
                snapshot.id, snapshot.root_hash, snapshot.file_count, snapshot.total_size),
        },
        Err(error) => response_from_error(error),
    }
}
```

### `SnapshotList`

```rust
// In IpcCommand enum:
SnapshotList { path: PathBuf },

// Handler:
async fn handle_snapshot_list(&self, path: PathBuf) -> IpcResponse {
    let context = match &self.archive_context {
        Some(ctx) => ctx.clone(),
        None => return IpcResponse::Error { message: "daemon has no node context".to_owned() },
    };
    let snapshots = SnapshotStore::with_docs(
        context.node.blob_store().clone(),
        context.node.docs_engine().clone(),
    );
    let namespace = path.to_string_lossy().parse::<iroh_docs::NamespaceId>().ok();
    match snapshots.list().await {
        Ok(all) => {
            let matching: Vec<_> = all.into_iter()
                .filter(|s| namespace.is_none_or(|id| s.namespace_id == Some(id)))
                .collect();
            let count = matching.len();
            IpcResponse::Ok { message: format!("snapshots: {count}") }
        }
        Err(error) => response_from_error(error),
    }
}
```

### `SnapshotDelete`

```rust
// In IpcCommand enum:
SnapshotDelete { id: String },

// Handler:
async fn handle_snapshot_delete(&self, id: String) -> IpcResponse {
    let context = match &self.archive_context {
        Some(ctx) => ctx.clone(),
        None => return IpcResponse::Error { message: "daemon has no node context".to_owned() },
    };
    let hash = match id.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(error) => return IpcResponse::Error { message: format!("invalid snapshot id: {error}") },
    };
    let snapshots = SnapshotStore::with_docs(
        context.node.blob_store().clone(),
        context.node.docs_engine().clone(),
    );
    match snapshots.delete(hash).await {
        Ok(()) => IpcResponse::Ok { message: format!("deleted: {id}") },
        Err(error) => response_from_error(error),
    }
}
```

---

## CLI Routing Pattern (for each converted command)

Each CLI handler follows this structure:

```rust
// 1. Pre-work that doesn't need the node (param validation, etc.)
// 2. Try daemon
if let Some(client) = daemon_client_or_start(data_dir, no_daemon).await? {
    let response = client
        .send(IpcRequest::new(IpcCommand::Xxx { /* command fields */ }))
        .await?;
    return print_daemon_message(response, output_json);
}
// 3. Embedded fallback (original logic, unchanged)
let node = open_node(data_dir).await?;
// ... original logic ...
node.stop().await?;
```

The `daemon_client_or_start()` function handles all three cases:
- `no_daemon == true` → returns `Ok(None)` → embedded fallback
- `no_daemon == false`, daemon running → returns `Ok(Some(client))` → IPC path
- `no_daemon == false`, no daemon → auto-starts daemon → retry → IPC path or error

---

## Remaining Commands Summary

### Selective (daemon-by-default):

| Command | Phase | Added as IPC? |
|---------|-------|---------------|
| `create` | ✅ Phase 1 | `CreateFolder` |
| `init` | ✅ Phase 1 | `CreateFolder` |
| `folders` | ✅ Phase 1 | `ListFolders` |
| `health` | ✅ Phase 1 | `HealthCheck` |
| `verify` | ✅ Phase 1 | `VerifyIntegrity` |
| `download` | ✅ Phase 1 | `Download` |
| `import` | ✅ Phase 1 | `ImportFiles` |
| `watch` | ✅ Phase 1 | `AddFolder` |
| `join --once` | ✅ Phase 1 | `Join` |
| `publish` | ✅ Phase 1 | `Publish` |
| `subscribe` | ✅ Phase 1 | `Subscribe` |
| `leave` | **⚡ 11.2** | `LeaveFolder` |
| `unsubscribe` | **⚡ 11.1** | `Unsubscribe` |
| `unpublish` | **⚡ 11.3** | `Unpublish` |
| `snapshot create` | **⚡ 11.4** | `SnapshotCreate` |
| `snapshot list` | **⚡ 11.4** | `SnapshotList` |
| `snapshot delete` | **⚡ 11.4** | `SnapshotDelete` |
| `collection publish` | **⚡ 11.5** | `CollectionPublish` |
| `package import` | **⚡ 11.6** | `ImportArchive` (exists) |
| `package export` | **⚡ 11.6** | `ExportArchive` (exists) |

### Always-embedded (local file ops / no blobstore):

| Command | Reason |
|---------|--------|
| `version` | No node needed |
| `repl` | Interactive shell |
| `ls` | Local filesystem scan |
| `find` | Local filesystem search |
| `sort` | Local filesystem sort |
| `stat` | Local file metadata |
| `devices` | Reads identity file only |
| `config` | Reads/writes TOML file |
| `schedule` | Reads/writes TOML file |
| `stats` | Reads/writes JSON stats file |
| `status` | Reads daemon state file |
| `completions` | Shell completion generation |
| `manpages` | Man page generation |
| `automatic --dry-run` | Local filter evaluation |
| `automatic --show-filters` | Config display |
| `collection init / add / versions` | Local manifest files |
| `package list / search / remove / verify / versions / switch` | Local package manager |
| `network create / ls / join / leave / invite / kick` | JSON file operations |
| `indexing *` | SQLite file operations |
| `link create / resolve / revoke` | (touches blobstore — deferred) |
| `mirror add` | (touches blobstore — deferred) |
| `trust *` | Local index operations |
| `attest` | Local file/index operations |
| `report` | Local file operations |
| `moderation *` | Local file/index operations |
| `daemon` / `start` | Spawns/manages daemon process |
| `shutdown` | Sends IPC to daemon |
| `daemon-shutdown` | Sends IPC to daemon |
| `daemon-reload` | Sends IPC to daemon |
| `daemon-sync` | Sends IPC to daemon |
| `unwatch` | Sends IPC to daemon |

### Deferred (touches blobstore, needs IPC handlers in future):

| Command | Reason Deferred |
|---------|-----------------|
| `link create / resolve / revoke` | Reads/writes blobstore, but part of indexing subsystem with separate data paths |
| `mirror add` | Writes blobstore ticket reference, used infrequently |
| `snapshot restore` | Local file I/O + blobstore read — split design needed |
| `snapshot diff` | Local file I/O + blobstore read — split design needed |
| `collection add` | Scans files + checks hashes against local state |
| `package install / upgrade --ticket` | Fetches blob from network |

---

## Phase Gates

| Sub-Phase | Gate |
|-----------|------|
| 11.1-11.8 | All blobstore-touching CLI commands route through daemon by default; `--no-daemon` bypass works |
| 11.9-11.14 | All new IPC handlers have unit tests covering success, error, and edge cases |
| 11.15-11.16 | Full daemon lifecycle integration tests pass; CLI daemon-routing tests pass |
| 11.17 | All benchmarks compile and produce meaningful measurements |
| 11.18-11.20 | Error messages mention `--no-daemon`; help output is accurate |

---

## TDD Checklist Per Module (MANDATORY)

For EVERY module/file created or modified:
- [ ] Write failing unit tests first
- [ ] Run tests → confirm RED
- [ ] Write minimal implementation
- [ ] Run tests → confirm GREEN
- [ ] Refactor with tests passing
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo fmt --all`
- [ ] Add integration test if involves multiple components
- [ ] Add benchmarks for performance-critical paths

---

## Dependencies

No new external dependencies required. All additions build on existing deps:
- `serde/serde_json` — already present (IPC serialization)
- `tokio` — already present (async IPC, daemon loop)
- `iroh-blobs` — already present (snapshot, blob operations)
- `iroh-docs` — already present (folder management)
- `SnapshotStore` — already exists in `syncweb-core/src/snapshot.rs`
- `cancel_session` — already exists in `syncweb-core/src/sync/sessions.rs`
