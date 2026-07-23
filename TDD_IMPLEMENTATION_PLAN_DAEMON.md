# TDD Implementation Plan for Daemon Mode

Following the strict TDD workflow from TDD_IMPLEMENTATION_PLAN.md:
1. Write failing test first → Watch it fail (Red)
2. Write minimal implementation → Make test pass (Green)
3. Refactor while keeping tests green (Refactor)
4. Run `cargo test --all-targets --all-features` after each module
5. Run `cargo clippy --all-targets --all-features -- -D warnings` after each phase
6. Run `cargo fmt --all` before commit

---

## Motivation

The current `Automatic` command (`syncweb automatic`) is the closest thing to a daemon: it opens the node, starts continuous sync on every managed folder, and waits on Ctrl-C. But it has no process lifecycle management, no IPC control channel, no restart-on-failure, no schedule-aware long-running loop, and no filesystem-watching integration.

**The fundamental constraint:** Iroh's blobstore and docs engine are backed by `redb`, which enforces exclusive write access via OS-level file locks. Only one process can hold the redb `Database` open for writing at a time. A second process attempting to open the same data directory fails with `DatabaseAlreadyOpen`. This means every CLI command that needs the node (`download`, `import`, `export`, `join`, `publish`, etc.) currently competes for the same lock — and two simultaneous invocations race or fail.

Phase 10 (this plan) solves this by making the daemon the **sole owner** of the Iroh node:
- The daemon acquires an `fs2` exclusive lock on startup — the single point of coordination that prevents two processes from colliding on redb
- All CLI commands that need the node route through the daemon's Unix socket IPC rather than opening the node themselves
- Commands detect a running daemon via the lock file; if no daemon is running, they either auto-start one or fail with a clear message
- Archive import/export, folder management, sync, and download all become IPC requests handled by the daemon
- The daemon integrates filesystem watching, schedule-aware sync, and Rayon thread pools for CPU-bound archive work

**Architecture:**
```
┌─────────────────────────────────────────────────┐
│  Daemon process (single redb writer)            │
│  ┌─────────┐ ┌──────────┐ ┌───────────────────┐ │
│  │ IrohNode│ │ SyncLoop │ │ Rayon archive pool│ │
│  └────┬────┘ └────┬─────┘ └────────┬──────────┘ │
│       │           │                │             │
│  ┌────┴───────────┴────────────────┴──────────┐  │
│  │          Unix Socket IPC Server            │  │
│  └───────────────────┬───────────────────────┘  │
└──────────────────────┼──────────────────────────┘
                       │ JSON over Unix socket
┌──────────┬───────────┼──────────┬───────────────┐
│ CLI      │ CLI       │ CLI      │ CLI           │
│ download │ import    │ export   │ join/sync     │
│ (IPC)    │ (IPC)     │ (IPC)    │ (IPC)         │
└──────────┴───────────┴──────────┴───────────────┘
```

**Default Daemon Mode (New Behavior):**
- **All `syncweb` commands now run in daemon mode by default** — they route through the daemon's IPC socket
- **`--no-daemon` / `--embedded` flag** bypasses the daemon and opens the node directly (for embedded/one-shot use cases)
- This is a behavioral change from earlier phases (10.1-10.7) which were committed with the old routing logic. The modifications to CLI routing (originally in 10.3-10.4) will be implemented as **rework in Phase 10C (10.8+)** since phases 10.1-10.7 are already committed.

## Identified Gaps (Current State → Daemon Mode)

| Gap | Current State | Daemon Mode Fix |
|-----|---------------|-----------------|
| **No exclusive access guard** | Any two `syncweb` processes can race on redb | `fs2` exclusive lock — single writer enforced at OS level |
| **CLI opens node directly** | Every command calls `open_node()` independently | CLI routes through daemon IPC; daemon is sole node owner |
| **No process lifecycle** | `Automatic` runs in foreground, Ctrl-C exits | PID file + lock file + `--daemon`/`--foreground` flag |
| **No IPC control** | Once started, no external control | Unix socket listener accepting JSON commands |
| **No status reporting** | No structured status output | Status JSON file + `syncweb status` command |
| Watch not integrated | `Watch` and `Automatic` are separate commands | Existing `FsWatcher` runs inside the daemon loop |
| No retry on failure | Failed sync intents stay dead | Supervision loop wraps the existing sync intents and restarts them with backoff |
| Schedule not applied to sync | `ScheduleManager` parses and displays configuration but does not control running intents | Daemon applies the existing schedule evaluation to pause/resume sync |
| No graceful signal handling | Only Ctrl-C | SIGINT/SIGTERM for shutdown; IPC commands for reload and force sync |
| Archive ops block CLI | Import/export are foreground-only | Rayon thread pool for CPU-bound archive work via IPC |
| No dynamic folder add/remove | Folders fixed at startup | IPC commands to add/remove folders at runtime |

## Existing Implementation to Reuse (Not Recreate)

Phase 10 integrates the following existing components:

- `SyncEngine` and `IntentHandle` remain the synchronization primitives. The
  daemon adds ownership, supervision, and IPC around them; it does not create a
  second sync engine.
- `Automatic` already starts continuous filtered sync intents. Its daemon
  implementation should move that behavior into the daemon loop.
- `FsWatcher` in `syncweb-core/src/fs/watcher.rs` and the existing `Watch`
  command provide filesystem event primitives, debounce inputs, and exclusion
  handling. The daemon owns their lifecycle and connects events to importing
  and sync.
- `ScheduleManager` already parses global/per-folder active windows and
  bandwidth limits. Phase 10 adds runtime application of those limits rather
  than another schedule parser.
- `ActiveSession` in `syncweb-core/src/sync/sessions.rs` already provides
  registration, cancellation, and activity tracking. `IntentSupervisor` must
  reuse it.
- `ParallelScanner`, `ParallelImporter`, and `ParallelExporter` already
  provide bounded filesystem parallelism. `ManagedPool` is only for the
  daemon's shared archive CPU work and must not duplicate or replace these
  utilities.
- Existing CLI commands currently call `open_node()` directly. Daemon routing
  must replace those call sites; it must not add a second node-opening path.

The daemon state, PID lock, IPC protocol, supervision layer, shared archive
pool, and `TryReference` import mode remain new functionality.

---

## Phase 10A: Daemon Lifecycle & PID Management — Weeks 29-30

### 10.1 Daemon State File (TDD)

**New type** in `syncweb-core/src/daemon/state.rs`:

```rust
pub struct DaemonState {
    pub pid: u32,
    pub node_id: String,
    pub started_at: u64,
    pub data_dir: PathBuf,
    pub status: DaemonStatus,
}

pub enum DaemonStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
}

pub struct StateFile {
    path: PathBuf,
}

impl StateFile {
    pub fn new(data_dir: &Path) -> Self;
    pub fn save(&self, state: &DaemonState) -> Result<()>;
    pub fn load(&self) -> Result<Option<DaemonState>>;
    pub fn exists(&self) -> bool;
    pub fn remove(&self) -> Result<()>;
    pub fn is_running(&self) -> Result<bool>;
}
```

**Unit Tests** (`tests/unit/daemon/state_test.rs`):
- [ ] `test_state_file_save_and_load` - round-trip serialize/deserialize
- [ ] `test_state_file_missing_returns_none` - nonexistent file returns None
- [ ] `test_state_file_corrupted_returns_error` - invalid JSON returns error
- [ ] `test_state_file_is_running_stale` - PID liveness is only an initial hint
- [ ] `test_state_file_pid_collision_requires_ipc_status` - a live unrelated PID does not prevent recovery when the daemon status ping fails
- [ ] `test_state_file_remove` - file deleted after stop
- [ ] `test_daemon_status_serialization` - serde round-trip

### 10.2 PID Lock File (TDD)

**This is the most critical component.** The `fs2` exclusive lock prevents two processes from simultaneously opening the redb-backed Iroh node. Every CLI command that needs node access must check this lock first.

**New type** in `syncweb-core/src/daemon/state.rs`:

```rust
pub struct PidLock {
    lock_path: PathBuf,
    state_path: PathBuf,
    _lock: Option<File>,  // fs2 File handle held for duration
}

impl PidLock {
    pub fn new(data_dir: &Path) -> Self;
    pub fn try_acquire(&self) -> Result<bool>;
    pub fn release(&self) -> Result<()>;
}
```

Uses `fs2::FileExt::try_lock_exclusive` (the `fs2` crate is already a workspace dependency but currently unused — this is its first use).

The lock protocol:
1. Open (or create) `<data_dir>/daemon.lock`
2. Call `try_lock_exclusive()` — returns `Ok(true)` if acquired, `Ok(false)` if another process holds it
3. If acquired, write PID to the file and proceed
4. If not acquired, read the PID from the file and use it only as an initial liveness hint
5. If the PID is alive, issue a lightweight `Status` IPC request with a short bounded timeout
6. If the status ping succeeds, return `Err(DaemonAlreadyRunning)`; if the socket is missing, refusing connections, or times out, treat the lock/state as stale and re-acquire
7. The lock handle is held for the daemon's lifetime — dropping it releases the lock

**Every CLI command** that needs the node (download, import, export, join, publish, etc.) calls `PidLock::try_acquire()` first. If the lock is held by a live daemon, the command routes through IPC instead of opening the node directly. If no daemon is running, the command either auto-starts one or fails with a clear message.

**Unit Tests** (`tests/unit/daemon/state_test.rs`):
- [ ] `test_pid_lock_acquire_success` - first lock succeeds
- [ ] `test_pid_lock_acquire_conflict` - second lock fails when held
- [ ] `test_pid_lock_release` - lock released after drop
- [ ] `test_pid_lock_stale_detection` - stale PID (dead process) allows re-acquire
- [ ] `test_pid_lock_state_file_consistency` - lock + state file written atomically
- [ ] `test_pid_lock_concurrent_start_race` - two simultaneous `try_acquire` calls: exactly one succeeds
- [ ] `test_pid_lock_held_across_fork` - lock survives daemon spawning child processes
- [ ] `test_pid_lock_live_pid_with_dead_socket` - dead IPC endpoint is recoverable
- [ ] `test_pid_lock_status_ping_timeout` - a hung endpoint is treated as stale after the bounded timeout

### 10.3 Daemon Command & Args (TDD)

**Modify the existing `Command` enum** in `syncweb-cli/src/cli/commands.rs`:

```rust
Command::Daemon(DaemonArgs),
Command::Status,
Command::DaemonShutdown(DaemonShutdownArgs),
Command::DaemonReload,
Command::DaemonSync,
Command::DaemonAdd(DaemonAddArgs),
Command::DaemonRemove(DaemonRemoveArgs),
```

`Start`, `Shutdown`, `Automatic`, and `Watch` already exist. Decide their
compatibility behavior as part of this change: `Start`/`Automatic` should
delegate to daemon startup where applicable, `Watch` should delegate to the
daemon watcher, and `Shutdown` should become the daemon shutdown command
without creating duplicate lifecycle handlers.

**Global CLI flags** (new file `syncweb-cli/src/cli/args.rs`):

```rust
pub struct GlobalArgs {
    pub data_dir: Option<PathBuf>,
    pub no_daemon: bool,        // --no-daemon / --embedded: bypass daemon, open node directly
    // ... other global flags
}
```

**New types** in `syncweb-cli/src/cli/commands.rs`:

```rust
pub struct DaemonArgs {
    pub foreground: bool,         // --foreground / -f: run in foreground
    pub data_dir: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub max_threads: Option<usize>,
    pub sync_interval: Option<u64>,  // seconds between sync cycles
}

pub struct DaemonShutdownArgs {
    pub force: bool,              // --force: skip graceful shutdown
}
```

**Unit Tests** (`tests/unit/cli/daemon_args_test.rs`):
- [ ] `test_daemon_args_defaults` - all optional fields are None/false
- [ ] `test_daemon_args_foreground` - --foreground flag parsed
- [ ] `test_daemon_args_log_file` - --log-file path parsed
- [ ] `test_daemon_args_max_threads` - --max-threads parsed
- [ ] `test_daemon_shutdown_args_force` - --force flag parsed
- [ ] `test_global_args_no_daemon` - --no-daemon flag parsed
- [ ] `test_global_args_embedded_alias` - --embedded flag parsed as alias for --no-daemon

### 10.4 CLI Node Access Routing (TDD)

**New file** `syncweb-core/src/daemon/route.rs`:

Every CLI command that currently calls `open_node()` must instead route through the daemon. This is the routing layer.

```rust
/// Check if a daemon is running and return an IPC client for it.
pub fn daemon_client(data_dir: &Path) -> Result<Option<IpcClient>>;

/// Route a node operation through the daemon, or start the daemon if needed.
pub async fn with_node<F, R>(data_dir: &Path, f: F) -> Result<R>
where
    F: FnOnce(IpcClient) -> R;

/// For one-shot commands: check if daemon is running, send IPC request.
/// If no daemon is running and --no-daemon/--embedded is NOT specified, auto-start the daemon.
/// If --no-daemon/--embedded IS specified, return None so the caller opens the node directly.
pub fn try_daemon(data_dir: &Path, no_daemon: bool) -> Result<Option<IpcClient>>;
```

**Routing logic in every CLI handler:**
1. Check for `--no-daemon` / `--embedded` flag (global flag, defaults to false → daemon mode enabled)
2. If `--no-daemon` / `--embedded` is NOT set: check `PidLock::try_acquire()` — if daemon is holding the lock, verify with IPC `Status` ping; if running, construct `IpcClient` and send command as `IpcCommand`; if not running, auto-start daemon then send IPC
3. If `--no-daemon` / `--embedded` IS set: return None so caller opens node directly (legacy one-shot mode)
4. **Daemon-aware commands** (`daemon`, `status`, `daemon-add`, `daemon-remove`, `daemon-shutdown`, `daemon-reload`, `daemon-sync`): operate on the daemon directly (never use `--no-daemon`)
5. **Node-access commands** (`download`, `import`, `export`, `join`, `publish`, `subscribe`, `automatic`, `watch`, `start`): default to daemon mode; only bypass with `--no-daemon`/`--embedded`

**New CLI commands** in `syncweb-cli/src/cli/commands.rs`:

```rust
Command::Daemon(DaemonArgs),        // start/manage daemon
Command::Status,                    // query daemon status
Command::DaemonShutdown(DaemonShutdownArgs), // signal daemon to stop
Command::DaemonReload,              // ask daemon to reload config over IPC
Command::DaemonSync,                // ask daemon to trigger sync over IPC
Command::DaemonAdd(DaemonAddArgs),  // add folder to daemon
Command::DaemonRemove(DaemonRemoveArgs), // remove folder from daemon
```

**Global CLI flag** (in `syncweb-cli/src/cli/args.rs`):

```rust
pub struct GlobalArgs {
    pub data_dir: Option<PathBuf>,
    pub no_daemon: bool,        // --no-daemon / --embedded: bypass daemon, open node directly
    // ... other global flags
}
```

**Modify existing commands** to route through IPC (default) with `--no-daemon`/`--embedded` bypass:
- `Command::Download` → send `IpcCommand::Download { ... }` (default); `--no-daemon` opens node directly
- `Command::Import` → send `IpcCommand::ImportArchive { ... }` (default); `--no-daemon` opens node directly
- `Command::Export` → send `IpcCommand::ExportArchive { ... }` (default); `--no-daemon` opens node directly
- `Command::Join` → send `IpcCommand::Join { ... }` (default); `--no-daemon` opens node directly
- `Command::Publish` → send `IpcCommand::Publish { ... }` (default); `--no-daemon` opens node directly
- `Command::Subscribe` → send `IpcCommand::Subscribe { ... }` (default); `--no-daemon` opens node directly
- `Command::Automatic` → replaced by `Command::Daemon` (deprecated)
- `Command::Watch` → folded into daemon's built-in watcher (deprecated)
- `Command::Start` → replaced by `Command::Daemon` (deprecated)

**Unit Tests** (`tests/unit/cli/daemon_args_test.rs`):
- [ ] `test_daemon_args_defaults` - all optional fields are None/false
- [ ] `test_daemon_args_foreground` - --foreground flag parsed
- [ ] `test_daemon_args_log_file` - --log-file path parsed
- [ ] `test_daemon_args_max_threads` - --max-threads parsed
- [ ] `test_daemon_shutdown_args_force` - --force flag parsed
- [ ] `test_routing_detects_running_daemon` - lock held → IPC client returned
- [ ] `test_routing_detects_no_daemon` - lock free → None returned
- [ ] `test_download_routes_through_ipc` - download sends IPC when daemon running
- [ ] `test_import_routes_through_ipc` - import sends IPC when daemon running
- [ ] `test_one_shot_without_daemon_fails_clearly` - "daemon not running" message
- [ ] `test_routing_requires_responsive_daemon` - a live PID without a responsive status endpoint is not treated as a running daemon
- [ ] `test_global_no_daemon_flag_bypasses_daemon` - --no-daemon flag makes commands open node directly
- [ ] `test_global_embedded_alias_bypasses_daemon` - --embedded flag is alias for --no-daemon
- [ ] `test_default_behavior_is_daemon_mode` - commands route to daemon by default without flags
- [ ] `test_daemon_commands_ignore_no_daemon` - daemon/status commands always use daemon

---

## Phase 10B: IPC Control Channel — Weeks 31-32

### 10.5 Unix Socket Listener (TDD)

**New file** `syncweb-core/src/daemon/ipc.rs`:

```rust
pub struct IpcListener {
    socket_path: PathBuf,
}

pub struct IpcRequest {
    pub command: IpcCommand,
}

pub enum IpcCommand {
    // Daemon management
    Status,
    ListFolders,
    AddFolder { namespace: String, path: PathBuf },
    RemoveFolder { namespace: String },
    TriggerSync { namespace: Option<String> },
    SetLogLevel { level: String },
    ReloadConfig,
    Shutdown { force: bool },

    // Node-access commands (replaces open_node() in CLI)
    Download { namespace: String, strategy: FetchStrategy },
    ImportArchive { input: PathBuf, target: PathBuf, filter: Option<FilterConfig> },
    ExportArchive { namespace: String, version: Option<String>, output: PathBuf },
    Join { ticket: String, path: PathBuf, mode: SessionMode },
    Publish { namespace: String, blob: Option<String> },
    Subscribe { namespace: String, params: SubscribeParams },
}

pub enum IpcResponse {
    Ok { message: String },
    Status(DaemonStatus),
    FolderList(Vec<FolderStatus>),
    DownloadComplete { bytes_transferred: u64 },
    ImportComplete(DropImportResult),
    ExportComplete(DropExportResult),
    Error { message: String },
}

pub struct FolderStatus {
    pub namespace: String,
    pub path: PathBuf,
    pub session_active: bool,
    pub last_sync_at: Option<u64>,
    pub sync_count: u64,
}
```

Uses `tokio::net::UnixListener` for the socket.

On Unix, create the socket with owner-only permissions (`0600`) before accepting
requests. The client and server must use the same data-directory ownership
assumption; other local user accounts must not be able to issue node-access IPC
commands. Non-Unix platforms use the platform's local IPC permission model.

**Unit Tests** (`tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_request_serialize_deserialize` - JSON round-trip
- [ ] `test_ipc_response_serialize_deserialize` - JSON round-trip
- [ ] `test_ipc_command_variants_all_serializable` - every variant serializes
- [ ] `test_ipc_download_command_serializes` - download variant round-trips
- [ ] `test_ipc_import_command_serializes` - import variant round-trips
- [ ] `test_ipc_export_command_serializes` - export variant round-trips
- [ ] `test_ipc_socket_permissions_owner_only` - Unix socket mode is `0600`

### 10.6 IPC Command Handler (TDD)

**Extend** `syncweb-core/src/daemon/ipc.rs`:

```rust
pub struct IpcServer {
    listener: IpcListener,
    daemon_handle: DaemonHandle,
}

impl IpcServer {
    pub fn new(socket_path: PathBuf, daemon_handle: DaemonHandle) -> Self;
    pub async fn serve(&self) -> Result<()>;
    pub async fn handle_request(&self, request: IpcRequest) -> IpcResponse;
}

pub struct DaemonHandle {
    state: Arc<RwLock<DaemonState>>,
    folder_registry: Arc<RwLock<FolderRegistry>>,
    shutdown_sender: broadcast::Sender<()>,
    sync_trigger: mpsc::UnboundedSender<Option<String>>,
}

pub struct FolderRegistry {
    folders: HashMap<String, FolderEntry>,
}

pub struct FolderEntry {
    pub namespace: NamespaceId,
    pub path: PathBuf,
    pub session: Option<ActiveSession>,
    pub last_sync_at: Option<u64>,
    pub sync_count: u64,
}
```

**Unit Tests** (`tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_handle_status` - returns current daemon state
- [ ] `test_ipc_handle_list_folders` - returns registered folders
- [ ] `test_ipc_handle_add_folder` - folder added to registry
- [ ] `test_ipc_handle_remove_folder` - folder removed from registry
- [ ] `test_ipc_handle_trigger_sync_all` - sync trigger sent for all folders
- [ ] `test_ipc_handle_trigger_sync_one` - sync trigger sent for specific namespace
- [ ] `test_ipc_handle_shutdown` - shutdown signal broadcast
- [ ] `test_ipc_handle_reload_config` - reload request is handled over IPC
- [ ] `test_ipc_handle_unknown_command` - error response

### 10.7 CLI IPC Client (TDD)

**Extend** `syncweb-core/src/daemon/ipc.rs`:

```rust
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new(data_dir: &Path) -> Self;
    pub async fn send(&self, request: IpcRequest) -> Result<IpcResponse>;
}
```

**Unit Tests** (`tests/unit/daemon/ipc_test.rs`):
- [ ] `test_ipc_client_send_and_receive` - client connects to server, round-trip
- [ ] `test_ipc_client_server_not_running` - returns descriptive error

---

## Phase 10C: Daemon Supervision Loop — Weeks 33-34

### 10.8 Intent Supervision with Restart (TDD)

**New file** `syncweb-core/src/daemon/supervisor.rs`:

```rust
pub struct IntentSupervisor {
    max_retries: u32,
    backoff_base: Duration,
    backoff_max: Duration,
}

pub struct SupervisedIntent {
    pub namespace: NamespaceId,
    pub handle: Option<IntentHandle>,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub last_started_at: Option<u64>,
}

impl IntentSupervisor {
    pub fn new(max_retries: u32, backoff_base: Duration, backoff_max: Duration) -> Self;
    pub async fn run_intent(&self, sync: &SyncEngine, namespace: NamespaceId, params: SubscribeParams) -> SupervisedIntent;
    pub async fn supervise(&self, sync: &SyncEngine, namespace: NamespaceId, params: SubscribeParams, mut shutdown: broadcast::Receiver<()>) -> Result<SupervisedIntent>;
}
```

Uses `sessions.rs`'s `ActiveSession::register()` pattern — each supervised intent registers an `ActiveSession` guard so that `cancel_session(namespace)` works from IPC.

**Unit Tests** (`tests/unit/daemon/supervisor_test.rs`):
- [ ] `test_supervisor_creates_intent` - intent starts successfully
- [ ] `test_supervisor_registers_session` - ActiveSession::is_active returns true after start
- [ ] `test_supervisor_handles_intent_failure` - failed intent triggers restart
- [ ] `test_supervisor_respects_max_retries` - stops after N failures
- [ ] `test_supervisor_backoff_increases` - retry delay doubles each attempt
- [ ] `test_supervisor_backoff_capped` - delay never exceeds max
- [ ] `test_supervisor_success_resets_retries` - successful run resets counter
- [ ] `test_supervisor_shutdown_cancels_intent` - broadcast shutdown cancels all intents
- [ ] `test_supervisor_deregisters_session_on_shutdown` - ActiveSession::is_active false after shutdown

### 10.9 Daemon Main Loop (TDD)

**New file** `syncweb-core/src/daemon/daemon.rs`:

```rust
pub struct Daemon {
    config: DaemonConfig,
    state_file: StateFile,
    pid_lock: PidLock,
    ipc_server: IpcServer,
    intent_supervisor: IntentSupervisor,
    folder_manager: FolderManager,
    sync_engine: SyncEngine,
    schedule_manager: Option<ScheduleManager>,
    archive_pool: Arc<ManagedPool>,
}

pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub foreground: bool,
    pub sync_interval: Duration,
    pub observation_ttl: Duration,
    pub max_retries: u32,
    pub backoff_base: Duration,
    pub backoff_max: Duration,
    pub rayon_threads: usize,
    pub log_level: String,
    pub log_file: Option<PathBuf>,
}

impl Daemon {
    pub async fn new(config: DaemonConfig) -> Result<Self>;
    pub async fn run(&self) -> Result<()>;
    async fn run_cycle(&self) -> Result<()>;
    async fn handle_signals(&self, shutdown: broadcast::Sender<()>) -> Result<()>;
}
```

The main loop:
1. Acquire PID lock + write state file
2. Start IPC listener in a background task
3. Spawn `IntentSupervisor::supervise()` for each managed folder
4. Enter cycle loop: wait `sync_interval`, check schedule, trigger `SyncEngine::sync_with_filter()` for each folder via the supervision layer
5. On SIGINT or SIGTERM: shut down gracefully. Reload and force-sync requests are handled only through IPC so the daemon remains cross-platform.
6. On exit: cancel all intents, release lock, remove state file

**Unit Tests** (`tests/unit/daemon/daemon_test.rs`):
- [ ] `test_daemon_new_creates_state` - state file written on creation
- [ ] `test_daemon_run_cycle_starts_intents` - folders get supervised intents
- [ ] `test_daemon_run_cycle_skips_during_inactive_hours` - schedule respected
- [ ] `test_daemon_run_cycle_resumes_during_active_hours` - schedule resumed
- [ ] `test_daemon_signal_term_triggers_shutdown` - SIGTERM stops daemon
- [ ] `test_daemon_signal_int_triggers_shutdown` - SIGINT stops daemon
- [ ] `test_daemon_ipc_reload_reloads_config` - `DaemonReload` applies new filter and schedule config
- [ ] `test_daemon_ipc_sync_triggers_immediate_sync` - `DaemonSync` forces an immediate sync

---

## Phase 10D: Rayon Integration for Archive Operations — Weeks 35-36

### 10.10 Managed Thread Pool (TDD)

**New file** `syncweb-core/src/daemon/pool.rs`:

The repository already uses Rayon in `ParallelScanner`, `ParallelImporter`,
and `ParallelExporter`, but those are per-operation filesystem utilities. This
pool is a daemon-owned archive pool shared by IPC export/import requests.

```rust
pub struct ManagedPool {
    pool: rayon::ThreadPool,
    name: String,
    thread_count: usize,
}

impl ManagedPool {
    pub fn new(name: impl Into<String>, thread_count: usize) -> Result<Self>;
    pub fn install<F, R>(&self, f: F) -> R where F: FnOnce() -> R + Send;
    pub fn spawn_fifo<F>(&self, f: F) where F: FnOnce() + Send + 'static;
    pub fn thread_count(&self) -> usize;
}

impl std::fmt::Debug for ManagedPool { ... }
```

**Unit Tests** (`tests/unit/daemon/pool_test.rs`):
- [ ] `test_pool_creates_with_custom_threads` - thread count matches
- [ ] `test_pool_install_executes_on_pool` - work runs on rayon threads
- [ ] `test_pool_spawn_fifo_executes` - spawned task runs
- [ ] `test_pool_size_is_fixed_for_daemon_lifetime` - pool size remains the startup configuration

The pool is sized once at daemon startup. Reloading configuration must not drop
the pool or block on active archive work; changing `rayon_threads` requires a
full daemon restart.

### 10.11 Archive Export with Rayon Pool (TDD)

**Modify `DropExporter`** to accept an optional `&ManagedPool`:

```rust
pub async fn export_drop_with_options(
    &self,
    manifests: &[CollectionManifest],
    output: impl AsRef<Path>,
    options: DropExportOptions,
    pool: Option<&ManagedPool>,
) -> Result<DropExportResult>
```

When `pool` is `Some`, the manifest serialization and blob verification steps (CPU-bound) are dispatched via `pool.install()`. The async I/O (file writes, CAR streaming) remains on the tokio runtime.

**Unit Tests** (`tests/unit/daemon/archive_test.rs`):
- [ ] `test_export_with_pool_matches_export_without` - same output regardless of pool
- [ ] `test_export_with_pool_uses_rayon_threads` - CPU work runs off tokio runtime
- [ ] `test_export_without_pool_still_works` - None pool is transparent

### 10.12 Archive Import with Rayon Pool (TDD)

**Modify `DropImporter`** to accept an optional `&ManagedPool`:

```rust
pub async fn import_archive(
    &self,
    input: impl AsRef<Path>,
    options: DropImportOptions,
    pool: Option<&ManagedPool>,
) -> Result<DropImportResult>
```

When `pool` is `Some`, the hash verification and manifest signature verification (CPU-bound) are dispatched via `pool.install()`. The async I/O (decompression, staging writes) remains on tokio.

**Unit Tests** (`tests/unit/daemon/archive_test.rs`):
- [ ] `test_import_with_pool_matches_import_without` - same result regardless of pool
- [ ] `test_import_with_pool_verifies_hashes_off_thread` - hash work on rayon
- [ ] `test_import_without_pool_still_works` - None pool is transparent

### 10.13 Daemon Pool Integration (TDD)

**Extend `DaemonConfig`** with `rayon_threads: usize` and wire `ManagedPool` into `Daemon`:

```rust
pub struct Daemon {
    // ... existing fields ...
    archive_pool: Arc<ManagedPool>,
}
```

Pass `Some(&self.archive_pool)` to `DropExporter` and `DropImporter` when the daemon handles archive operations via IPC.

**Unit Tests** (`tests/unit/daemon/daemon_test.rs`):
- [ ] `test_daemon_creates_pool_with_configured_threads` - pool size matches config
- [ ] `test_daemon_ipc_export_uses_pool` - export via IPC runs on rayon pool
- [ ] `test_daemon_ipc_import_uses_pool` - import via IPC runs on rayon pool

---

## Phase 10E: Zero-Copy Import via TryReference — Weeks 37-38

iroh-blobs v0.103 supports `ImportMode::TryReference` which references files at their original path instead of copying bytes into the blobstore. Today, syncweb always uses the default `ImportMode::Copy`, causing 2x disk usage for every imported folder. This phase switches the regular filesystem import pipeline to `TryReference`, eliminating that duplication.

### 10.22 BlobStore add_file_ref (TDD)

**Extend `BlobStore`** in `syncweb-core/src/node/blob_store.rs`:

```rust
/// Add a file using TryReference mode — the blobstore references the file
/// at its original path instead of copying bytes. The file must remain at
/// `path` for the lifetime of the blobstore.
///
/// # Errors
///
/// Returns an error if the file fails to be read or added to the store.
pub async fn add_file_ref(&self, path: impl AsRef<Path>) -> Result<Hash> {
    Ok(self
        .store
        .add_path_with_opts(AddPathOptions {
            path: path.as_ref().to_owned(),
            mode: ImportMode::TryReference,
            format: BlobFormat::Raw,
        })
        .await
        .map_err(|error| SyncwebError::operation("failed to add blob file (reference)", error))?
        .hash)
}
```

The existing `add_file` remains unchanged (still uses `Copy`) for callers that need guaranteed ownership.

**Unit Tests** (`tests/unit/blob_store_test.rs`):
- [ ] `test_add_file_ref_stores_hash` - blob is accessible after TryReference import
- [ ] `test_add_file_ref_same_hash_as_copy` - same content produces same hash regardless of mode
- [ ] `test_add_file_ref_does_not_copy_bytes` - blobstore data dir has no new owned file for large files
- [ ] `test_add_file_ref_fallback_for_small_files` - iroh inlines small files even with TryReference
- [ ] `test_add_file_ref_original_must_exist` - missing file returns error

### 10.23 Importer Switches to TryReference (TDD)

**Modify `Importer::import_one`** in `syncweb-core/src/fs/importer.rs`:

```rust
async fn import_one(&self, entry: FileEntry) -> Result<ImportEntry> {
    let hash = self.blob_store.add_file_ref(&entry.path).await?;
    // ... rest unchanged
}
```

This affects both `Importer` and `ParallelImporter` since `ParallelImporter` delegates to `Importer::import_one`.
Because the source file can change while iroh is hashing or referencing it,
capture the relevant file metadata before and after the read. If the file
changes during import, return the existing `SyncwebError` operation error,
avoid publishing a mismatched tree entry, and let the daemon intent continue
with its normal retry path rather than crashing.

**Unit Tests** (`tests/unit/fs/importer_test.rs`):
- [ ] `test_importer_uses_reference_mode` - imported files are referenced, not copied
- [ ] `test_importer_reference_file_stays_on_disk` - original file unchanged after import
- [ ] `test_importer_parallel_uses_reference_mode` - parallel importer also uses TryReference
- [ ] `test_importer_import_then_delete_original_still_readable` - blob survives if hash is pinned (inline/small)
- [ ] `test_importer_large_file_reference_requires_original` - large referenced file fails to read if deleted
- [ ] `test_importer_surfaces_file_change_during_hash` - mutation during hashing returns a read-consistency error
- [ ] `test_importer_consistency_error_does_not_publish_entry` - inconsistent content is not added to the collection tree

### 10.24 Watch Integration Uses TryReference (TDD)

The daemon's filesystem watch handler (Phase 10.28) calls `Importer::import_one` which now uses TryReference. No additional changes needed — it inherits the behavior.

**Unit Tests** (`tests/unit/daemon/watch_test.rs`):
- [ ] `test_daemon_watch_import_uses_reference` - files detected by FsWatcher are referenced

### 10.25 Archive Import Stays Copy (TDD)

Archive import extracts blocks to temporary staging paths — the files don't exist on disk beforehand, so `TryReference` would fail. `archive_import.rs:209` continues to call `blob_store.add_file()` (which uses `Copy`). This is correct.

**Unit Tests** (`tests/unit/folder/archive_import_test.rs`):
- [ ] `test_archive_import_uses_copy_mode` - archive blocks are owned by blobstore
- [ ] `test_archive_import_staging_deleted_after` - temp staging files cleaned up

### 10.26 Snapshot Stays Copy (TDD)

Snapshot creation (`snapshot.rs:254`) captures a point-in-time copy. Using `TryReference` would risk the snapshot being corrupted if the original file changes. `add_file` (Copy) is correct here.

**Unit Tests** (`tests/unit/snapshot_test.rs`):
- [ ] `test_snapshot_uses_copy_mode` - snapshot blobs are owned by blobstore
- [ ] `test_snapshot_survives_original_modification` - snapshot intact after source file changes

### 10.27 CLI Collection Publish Stays Copy (TDD)

Collection publish (`main.rs:1322`) imports package content blobs. These may be in a staging directory that gets cleaned up. `add_file` (Copy) is correct.

**Unit Tests** (`tests/integration/collection_publish_test.rs`):
- [ ] `test_collection_publish_uses_copy_mode` - package blobs are owned by blobstore

---

## Phase 10F: Watch + Sync Integration & Schedule — Weeks 39-40

### 10.28 Filesystem Watch Inside Daemon (TDD)

**Extend** `syncweb-core/src/daemon/daemon.rs`:

```rust
pub struct Daemon {
    // ... existing fields ...
    watchers: HashMap<String, FsWatcher>,
}

impl Daemon {
    async fn start_watching(&self) -> Result<()>;
    async fn handle_watch_events(&self) -> Result<()>;
}
```

The daemon starts an `FsWatcher` for each managed folder's root path. Watch events
use a sufficient debounce window to allow editor writes to settle, then trigger
an import via the existing `Importer`, followed by a sync cycle via `SyncEngine`.
Read-consistency errors from a file that is still changing are reported as
recoverable intent errors and retried; they must not crash the daemon.

**Unit Tests** (`tests/unit/daemon/watch_test.rs`):
- [ ] `test_daemon_starts_watcher_per_folder` - one watcher per managed folder
- [ ] `test_daemon_file_change_triggers_import` - new file detected and imported
- [ ] `test_daemon_file_delete_triggers_delete` - removed file triggers doc delete
- [ ] `test_daemon_watch_debounce_coalesces` - rapid changes produce one import
- [ ] `test_daemon_watch_respects_exclude_patterns` - excluded files ignored
- [ ] `test_daemon_watch_debounce_allows_writes_to_settle` - hashing starts after the configured settle window
- [ ] `test_daemon_watch_recovers_from_file_read_inconsistency` - a changing file is retried without stopping the daemon

### 10.29 Schedule-Aware Sync (TDD)

**Extend** the daemon cycle to check `ScheduleManager`:

```rust
impl Daemon {
    async fn is_in_active_window(&self) -> bool;
    async fn current_bandwidth_limit(&self) -> Option<u64>;
}
```

During inactive hours, the daemon pauses all sync intents (sends `SyncCommand::Pause`). During active hours, it resumes them (sends `SyncCommand::Resume`). Bandwidth limits are applied via the existing `BandwidthWindowConfig`.

**Unit Tests** (`tests/unit/daemon/schedule_test.rs`):
- [ ] `test_daemon_pauses_during_inactive_hours` - intents paused outside window
- [ ] `test_daemon_resumes_during_active_hours` - intents resumed inside window
- [ ] `test_daemon_bandwidth_limit_applied` - sync throttled to configured rate
- [ ] `test_daemon_no_schedule_runs_always` - no schedule config means always active
- [ ] `test_daemon_per_folder_schedule_override` - folder-specific schedules respected

### 10.30 Daemon Reload (TDD)

On the `ReloadConfig` IPC command, the daemon:
1. Reloads `filters.toml` from disk
2. Reloads `config.toml` schedule settings
3. Recalculates which folders need watching
4. Applies new filter to running sync intents

**Unit Tests** (`tests/unit/daemon/daemon_test.rs`):
- [ ] `test_daemon_reload_updates_filters` - new filter rules take effect
- [ ] `test_daemon_reload_updates_schedule` - new schedule windows take effect
- [ ] `test_daemon_reload_adds_new_folder` - new folder in config starts syncing
- [ ] `test_daemon_reload_removes_deleted_folder` - removed folder stops syncing
- [ ] `test_daemon_reload_keeps_rayon_pool_running` - reload does not rebuild or drop the active pool

---

## Phase 10G: Status Reporting & CLI Integration — Week 41

### 10.31 Status File (TDD)

**New type** in `syncweb-core/src/daemon/state.rs`:

```rust
pub struct DaemonStatusReport {
    pub pid: u32,
    pub node_id: String,
    pub started_at: u64,
    pub uptime_seconds: u64,
    pub folders: Vec<FolderStatusReport>,
    pub bandwidth: BandwidthSnapshot,
    pub schedule: Option<ScheduleStatus>,
    pub rayon_threads: usize,
}

pub struct FolderStatusReport {
    pub namespace: String,
    pub path: PathBuf,
    pub session_active: bool,
    pub last_sync_at: Option<u64>,
    pub entries_synced: u64,
    pub errors: Vec<String>,
}

pub struct BandwidthSnapshot {
    pub upload_total: u64,
    pub download_total: u64,
    pub upload_rate: u64,
    pub download_rate: u64,
}

pub struct ScheduleStatus {
    pub in_active_window: bool,
    pub next_window_start: Option<u64>,
}
```

**Extend `StateFile`** with:
- `save_status(&self, report: &DaemonStatusReport) -> Result<()>`
- `load_status(&self) -> Result<Option<DaemonStatusReport>>`

**Unit Tests** (`tests/unit/daemon/state_test.rs`):
- [ ] `test_status_report_save_and_load` - round-trip
- [ ] `test_status_report_serialization_json` - valid JSON output
- [ ] `test_status_report_includes_folders` - folder list present

### 10.32 `syncweb status` Command (TDD)

**Extend** CLI dispatch:

```rust
// syncweb-cli/src/cli/daemon.rs
pub async fn handle_status(data_dir: &Path, output_json: bool) -> Result<()>;
```

Reads the status file written by the daemon and formats it as a table (text) or JSON.

**Unit Tests** (`tests/unit/cli/daemon_args_test.rs`):
- [ ] `test_status_command_reads_state_file` - parses daemon status
- [ ] `test_status_command_daemon_not_running` - descriptive message when no state file
- [ ] `test_status_command_json_output` - valid JSON format
- [ ] `test_status_command_text_output` - human-readable table

### 10.33 IPC Folder Management Commands (TDD)

**Extend CLI** with:

```rust
// syncweb-cli/src/cli/commands.rs
Command::DaemonAdd(DaemonAddArgs),   // syncweb daemon-add <namespace> <path>
Command::DaemonRemove(DaemonRemoveArgs), // syncweb daemon-remove <namespace>
```

These send `IpcCommand::AddFolder` / `IpcCommand::RemoveFolder` to the running daemon via `IpcClient`.

**Unit Tests** (`tests/unit/cli/daemon_args_test.rs`):
- [ ] `test_daemon_add_sends_ipc` - add command reaches daemon
- [ ] `test_daemon_remove_sends_ipc` - remove command reaches daemon

---

## Phase 10H: Integration Tests & Benchmarks — Week 42

### 10.34 Integration Tests

**Integration Tests** (`tests/integration/daemon_test.rs`):
- [ ] `test_full_daemon_start_stop_lifecycle` - start → status check → shutdown → clean
- [ ] `test_daemon_syncs_folder_end_to_end` - start daemon, add folder, verify sync
- [ ] `test_daemon_watches_and_syncs_new_file` - create file → daemon imports + syncs
- [ ] `test_daemon_restart_picks_up_existing_state` - restart reads state file
- [ ] `test_daemon_concurrent_ipc_commands` - multiple IPC commands in parallel
- [ ] `test_daemon_export_archive_via_ipc` - export command through daemon
- [ ] `test_daemon_import_archive_via_ipc` - import command through daemon
- [ ] `test_daemon_schedule_pause_resume` - schedule transition pauses/resumes
- [ ] `test_daemon_intent_restart_on_failure` - crashed intent restarts with backoff
- [ ] `test_daemon_reload_over_ipc` - config changes applied live without Unix signals
- [ ] `test_daemon_sync_over_ipc` - force-sync command works without Unix signals
- [ ] `test_daemon_sessions_reuse_pattern` - ActiveSession guards work across restart
- [ ] `test_two_daemons_cannot_start_simultaneously` - race: two processes try acquire, exactly one wins
- [ ] `test_stale_pid_with_reused_pid_is_recovered` - unrelated process PID does not mask a dead daemon
- [ ] `test_unix_socket_rejects_non_owner_access` - IPC socket is not group/world accessible
- [ ] `test_try_reference_change_does_not_crash_daemon` - file mutation during hashing yields a recoverable error
- [ ] `test_cli_routes_through_running_daemon` - download/import use IPC when daemon alive
- [ ] `test_cli_fails_clearly_without_daemon` - download fails with "daemon not running" when no daemon

### 10.35 Benchmarks

Add to `benches/` with `criterion`:
- [ ] `bench_ipc_round_trip` - Target < 1ms per request
- [ ] `bench_supervisor_restart_latency` - Target < 500ms to relaunch intent
- [ ] `bench_state_file_write_read` - Target < 10ms
- [ ] `bench_archive_export_with_pool` - Compare with/without Rayon pool
- [ ] `bench_archive_import_with_pool` - Compare with/without Rayon pool

Run with: `cargo bench --all-features`

---

## New Files

| File | Purpose |
|------|---------|
| `syncweb-core/src/daemon.rs` | Module root, re-exports |
| `syncweb-core/src/daemon/state.rs` | `DaemonState`, `StateFile`, `PidLock`, `DaemonStatusReport` |
| `syncweb-core/src/daemon/ipc.rs` | `IpcListener`, `IpcServer`, `IpcClient`, `IpcCommand`, `IpcResponse` |
| `syncweb-core/src/daemon/supervisor.rs` | `IntentSupervisor`, `SupervisedIntent` |
| `syncweb-core/src/daemon/daemon.rs` | `Daemon`, `DaemonConfig` |
| `syncweb-core/src/daemon/pool.rs` | `ManagedPool` |
| `syncweb-cli/src/cli/daemon.rs` | CLI handlers for `daemon`, `status`, `daemon-add`, `daemon-remove` |
| `tests/unit/daemon/state_test.rs` | Unit tests for state/lock |
| `tests/unit/daemon/ipc_test.rs` | Unit tests for IPC |
| `tests/unit/daemon/supervisor_test.rs` | Unit tests for supervision |
| `tests/unit/daemon/daemon_test.rs` | Unit tests for daemon main loop |
| `tests/unit/daemon/pool_test.rs` | Unit tests for managed pool |
| `tests/unit/daemon/archive_test.rs` | Unit tests for pool-integrated archive ops |
| `tests/unit/daemon/watch_test.rs` | Unit tests for watch integration |
| `tests/unit/daemon/schedule_test.rs` | Unit tests for schedule integration |
| `tests/unit/cli/daemon_args_test.rs` | Unit tests for CLI arg parsing |
| `tests/unit/blob_store_test.rs` | Unit tests for `add_file_ref` (TryReference) |
| `tests/unit/fs/importer_test.rs` | Unit tests for importer using TryReference |
| `tests/unit/folder/archive_import_test.rs` | Unit tests confirming archive import stays Copy |
| `tests/unit/snapshot_test.rs` | Unit tests confirming snapshot stays Copy |
| `tests/integration/collection_publish_test.rs` | Integration tests confirming publish stays Copy |
| `tests/integration/daemon_test.rs` | End-to-end daemon lifecycle tests |

## Modified Files

| File | Changes |
|------|---------|
| `syncweb-cli/src/main.rs` | Replace existing `open_node()` call sites with IPC routing in `download`, `import`, `subscribe`, `publish`, `join`, `automatic`, and related node-access handlers; wire new daemon commands into dispatch |
| `syncweb-cli/src/cli/commands.rs` | Add `Daemon`, `Status`, `DaemonShutdown`, `DaemonReload`, `DaemonSync`, `DaemonAdd`, `DaemonRemove` variants + arg structs; modify `Download`, `Import`, `Export` to support IPC mode |
| `syncweb-cli/src/cli/args.rs` | **NEW** - Add `GlobalArgs` struct with `no_daemon`/`embedded` flag for daemon bypass |
| `syncweb-core/src/lib.rs` | Add `pub mod daemon;` |
| `syncweb-core/src/folder/archive_export.rs` | Accept optional `&ManagedPool` for CPU-bound work |
| `syncweb-core/src/folder/archive_import.rs` | Accept optional `&ManagedPool` for CPU-bound work |
| `syncweb-core/src/sync/sessions.rs` | (No structural changes — reused as-is via `ActiveSession::register()`) |
| `syncweb-core/src/fs/watcher.rs` | (Reuse existing watcher; only modify if daemon-specific event/debounce support cannot be composed at the daemon layer) |
| `syncweb-core/src/schedule.rs` | (Reuse existing parsing/evaluation; only add runtime integration hooks if required) |

---

## Phase Gates

| Sub-Phase | Gate |
|-----------|------|
| 10.1-10.4 | PID lock acquired/released, state file written/read, daemon starts and stops cleanly |
| 10.5-10.7 | IPC server accepts connections, CLI client can send commands, status query works |
| 10.8-10.9 | Supervised intents restart on failure, main loop runs continuously, SIGINT/SIGTERM shutdown works |
| 10.10-10.13 | Rayon pool managed with configurable threads, archive export/import use pool transparently |
| 10.22-10.27 | `add_file_ref` works, importer uses TryReference, archive/snapshot/publish stay Copy |
| 10.28-10.30 | Filesystem watching triggers imports, schedule pauses/resumes sync, reload applies config |
| 10.31-10.33 | Status file written, `syncweb status` reads it, folder management via IPC |
| 10.34-10.35 | All integration tests pass, benchmarks meet targets |

---

## TDD Checklist Per Module (MANDATORY)

For EVERY module/file created:
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

## Dependencies to Add

```toml
# syncweb-core/Cargo.toml
# fs2 = "0.4" — already declared, currently unused; first used for PidLock
# tokio net feature — add "net" to tokio features for UnixListener
# iroh-blobs — already present; add "ImportMode" and "AddPathOptions" to use TryReference

# No other new external dependencies required.
# All daemon features build on existing deps:
# - fs2 (already present, unused → now used for PID lock)
# - tokio (already present, add "net" feature for UnixListener)
# - rayon (already present, now managed via ManagedPool)
# - notify (already present, already used in FsWatcher)
# - serde/serde_json (already present)
# - tracing (already present)
# - blake3 (already present)
# - ed25519-dalek (already present)
# - iroh-blobs (already present, ImportMode::TryReference is in v0.103)
```

---

## Design Decisions

### Why the daemon is the sole Iroh node owner (not optional)

`redb` (the storage engine behind `iroh-blobs` and `iroh-docs`) enforces exclusive write access via OS-level file locks. Only one process can hold the redb `Database` open for writing at a time. If two processes try to open the same `<data_dir>/blobs` file simultaneously, the second gets `DatabaseAlreadyOpen`. This is not a limitation we can work around — it's a fundamental property of the storage layer.

The daemon solves this by being the **single point of node ownership**:
- It acquires the `fs2` lock and opens the Iroh node once
- All other processes communicate via Unix socket IPC
- The lock file (`daemon.lock`) is the coordination point that prevents race conditions between two simultaneously-started `syncweb` processes
- If process A and process B both try to start the daemon, exactly one wins the `fs2` lock; the other sees the lock and routes through IPC

This is the same pattern used by PostgreSQL (postmaster holds the lock, psql connects via IPC), Docker (dockerd holds the socket, CLI sends requests), and other database/service architectures.

### Why Unix socket IPC instead of HTTP?

A Unix socket is lighter-weight than an HTTP server, requires no port management, and is naturally restricted to the local machine — appropriate for a daemon control channel. The JSON-over-socket protocol is simple to implement and test. A future phase could add an HTTP endpoint if remote management is needed.

### Why `fs2` for PID locking instead of `fd-lock`?

`fs2` is already a workspace dependency (declared in `syncweb-core/Cargo.toml`) but unused. It provides `try_lock_exclusive` on `File`, which is exactly what a PID lock needs. Using an already-declared dependency avoids adding new crates.

### Why a separate `IntentSupervisor` instead of modifying `SyncEngine`?

`SyncEngine` is a clean, focused coordination layer. Adding restart logic, backoff, and retry tracking would bloat it. The `IntentSupervisor` wraps `SyncEngine::sync_with_filter()` and manages the lifecycle externally, keeping the sync engine testable in isolation.

### Why a `ManagedPool` instead of using Rayon's global pool?

The global Rayon pool has a default thread count derived from `num_cpus`. In a daemon context, the archive-import CPU work competes with tokio async I/O for CPU time. A managed pool with a configurable thread count lets the daemon size its CPU pool independently of the system. The pool is intentionally fixed for the daemon lifetime because dropping a Rayon pool waits for active work; changing the size requires a daemon restart.

### Why `ActiveSession` reuse?

The `ActiveSession` pattern in `sessions.rs` is elegant: a `register()` call returns a guard that auto-deregisters on drop, backed by a static `OnceLock<Mutex<Vec>>`. The daemon's `IntentSupervisor` uses this same pattern — each supervised intent registers an `ActiveSession` so that `cancel_session(namespace)` works from IPC and signal handlers without the daemon needing its own parallel tracking mechanism.

### Why schedule integration in the daemon rather than in `SyncEngine`?

The `SyncEngine` is a coordination layer that doesn't know about wall-clock time or policy. Schedule awareness is a daemon-level policy decision — different deployments may want different scheduling behavior. Keeping it in the daemon loop (via `ScheduleManager`) lets the daemon pause/resume intents without modifying the sync engine's contract.

### Why archive pool is `Option<&ManagedPool>` in export/import?

Not every caller needs a managed pool. The CLI's one-shot `package export` / `package import` commands work fine with Rayon's global pool. Making the pool optional means the archive APIs remain backward-compatible — the daemon passes `Some(pool)` while CLI commands pass `None`.

### Why `TryReference` for regular imports but not archive/snapshot/publish?

`TryReference` avoids 2x disk usage by referencing files at their original path. This is ideal for regular folder imports where the user's files are the source of truth. But three callers need guaranteed ownership:

- **Archive import** extracts blocks from a `.car.zst` to temporary staging paths. The staging files are deleted after ingestion. `TryReference` would fail because the referenced file would vanish.
- **Snapshot creation** captures a point-in-time copy. If the original file changes after import, the snapshot must remain intact. `Copy` ensures the blobstore owns its own bytes.
- **Collection publish** imports package content blobs that may come from a staging directory. Same rationale as archive import.

The `add_file` (Copy) and `add_file_ref` (TryReference) methods coexist on `BlobStore` so each caller picks the right mode. This is a one-line change per call site — no API breakage.

---

*This plan follows strict TDD: no implementation code before a failing test exists. Each module follows Red-Green-Refactor cycle with clippy/fmt gates.*
