use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show syncweb version information")]
    Version,
    #[command(about = "Start the local syncweb daemon", alias = "daemon")]
    Start(StartArgs),
    #[command(about = "Stop the local syncweb node", alias = "daemon-shutdown")]
    Shutdown(ShutdownArgs),
    #[command(about = "Show the local daemon status")]
    Status,
    #[command(about = "Ask the local daemon to reload configuration", alias = "daemon-reload")]
    Reload,
    #[command(about = "Ask the local daemon to trigger synchronization")]
    DaemonSync,
    #[command(about = "Create a synchronized folder")]
    Create(FolderCreate),
    #[command(about = "Join a folder from an Iroh document ticket")]
    Join(FolderJoin),
    #[command(about = "Leave a synchronized folder, optionally deleting its local files")]
    Leave(LeaveArgs),
    #[command(about = "List managed folders")]
    Folders,
    #[command(about = "Show this device's Iroh and Syncthing identities")]
    Devices,
    #[command(about = "Show or update local configuration")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    #[command(about = "List files in a local folder")]
    Ls(LocalPathArgs),
    #[command(about = "Search local files")]
    Find(FindArgs),
    #[command(about = "Sort local files by discovery criteria")]
    Sort(SortArgs),
    #[command(about = "Show detailed metadata for a local file")]
    Stat(StatArgs),
    #[command(about = "Download folder content or copy a local file")]
    Download(DownloadArgs),
    #[command(about = "Import local files into a synchronized folder")]
    Import(ImportArgs),
    #[command(about = "Manage content-addressed snapshots")]
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    #[command(about = "Show seeding status per folder blob")]
    Health(HealthArgs),
    #[command(about = "Inspect and control durable transfer jobs")]
    Transfer {
        #[command(subcommand)]
        command: TransferCommand,
    },
    #[command(about = "Run rules-based automatic synchronization")]
    Automatic(AutomaticArgs),
    #[command(about = "Watch a folder and import filesystem changes")]
    Watch(WatchArgs),
    #[command(about = "Show persisted bandwidth accounting")]
    Stats(StatsArgs),
    #[command(name = "filestats", about = "Show file-level statistics for synced folder content")]
    FileStats(FileStatsArgs),
    #[command(about = "Re-check local folder blob integrity")]
    Verify(VerifyArgs),
    #[command(about = "Show or update synchronization schedules")]
    Schedule {
        #[command(subcommand)]
        command: Option<ScheduleCommand>,
    },
    #[command(about = "Publish a folder or blob for public read access")]
    Publish(PublishArgs),
    #[command(about = "Remove a public blob pin")]
    Unpublish(UnpublishArgs),
    #[command(about = "Create and publish versioned content collections")]
    Collection {
        #[command(subcommand)]
        command: CollectionCommand,
    },
    #[command(about = "Manage locally installed collection packages")]
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    #[command(about = "Network connectivity utilities")]
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    #[command(about = "Database maintenance: check, vacuum, stats, backup")]
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },
    #[command(about = "Manage opt-in indexing, catalogs, and metadata")]
    Indexing {
        #[command(subcommand)]
        command: IndexingCommand,
    },
    #[command(about = "Create and resolve stable syncweb links")]
    Link {
        #[command(subcommand)]
        command: LinkCommand,
    },
    #[command(about = "Mirror all blobs from a provider or network")]
    Mirror(MirrorArgs),
    #[command(about = "Manage blob provider registrations")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },
    #[command(about = "Inspect and delegate local trust")]
    Trust {
        #[command(subcommand)]
        command: TrustCommand,
    },
    #[command(about = "Sign content provenance attestations")]
    Attest {
        #[command(subcommand)]
        command: AttestCommand,
    },
    #[command(about = "Manage local moderation decisions")]
    Moderation {
        #[command(subcommand)]
        command: ModerationCommand,
    },
    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    #[command(about = "Generate manpages")]
    Manpages {
        #[arg(default_value = "man")]
        dir: PathBuf,
    },
    #[command(about = "Serve media blobs via HTTP (standalone media server)")]
    Media(MediaArgs),
    #[command(about = "Print this message or the help of the given subcommand(s)")]
    Help {
        #[arg(value_name = "COMMAND")]
        command: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct MediaArgs {
    #[arg(long, default_value = "127.0.0.1:9193")]
    pub listen: std::net::SocketAddr,
    #[arg(long, help = "Override the global persistent data directory")]
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Set a configuration value")]
    Set { key: String, value: String },
    #[command(about = "Show configuration, optionally limited to a section")]
    Show { section: Option<String> },
}

#[derive(Debug, Subcommand)]
pub enum TransferCommand {
    #[command(about = "List durable transfer jobs")]
    Info(TransferInfoArgs),
    #[command(about = "Show configured roots and remaining capacity")]
    Remaining,
    #[command(about = "Add or update a materialization root")]
    Root(TransferRootArgs),
    #[command(about = "Enqueue an individually addressable file job")]
    Enqueue(TransferEnqueueArgs),
    #[command(about = "Allocate queued jobs to configured roots")]
    Allocate(TransferAllocateArgs),
    #[command(about = "Fetch and materialize assigned jobs through the daemon")]
    Materialize(TransferMaterializeArgs),
    #[command(about = "Pause a transfer job")]
    Pause(TransferJobArgs),
    #[command(about = "Resume a paused transfer job")]
    Resume(TransferJobArgs),
    #[command(about = "Cancel a transfer job")]
    Cancel(TransferJobArgs),
    #[command(about = "Retry a failed transfer job")]
    Retry(TransferJobArgs),
}

#[derive(Debug, Args)]
pub struct TransferInfoArgs {
    #[arg(long, help = "Limit display to a namespace")]
    pub namespace: Option<String>,
    #[arg(long, help = "Limit display to a lifecycle state")]
    pub state: Option<String>,
    #[arg(long, value_parser = ["created", "updated", "size", "peers", "path"], default_value = "created")]
    pub sort: String,
    #[arg(long, value_parser = ["namespace", "root", "state"])]
    pub group_by: Option<String>,
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct TransferRootArgs {
    pub id: String,
    pub path: PathBuf,
    #[arg(long, default_value_t = 0, help = "Free bytes to preserve on this root")]
    pub min_free: u64,
    #[arg(long, help = "Disable this root for allocation")]
    pub disabled: bool,
}

#[derive(Debug, Args)]
pub struct TransferEnqueueArgs {
    #[arg(long)]
    pub namespace: String,
    #[arg(long, help = "Relative materialization path")]
    pub path: PathBuf,
    #[arg(long, help = "32-byte blob hash in hexadecimal")]
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Args)]
pub struct TransferAllocateArgs {
    #[arg(long, help = "Report allocations without persisting them")]
    pub dry_run: bool,
    #[arg(long, help = "Limit allocation to a namespace")]
    pub namespace: Option<String>,
    #[arg(long, help = "Only allocate paths below this relative prefix")]
    pub path_prefix: Option<PathBuf>,
    #[arg(long)]
    pub min_size: Option<u64>,
    #[arg(long)]
    pub max_size: Option<u64>,
}

#[derive(Debug, Args)]
pub struct TransferMaterializeArgs {
    #[arg(long, help = "Limit processing to a namespace")]
    pub namespace: Option<String>,
}

#[derive(Debug, Args)]
pub struct TransferJobArgs {
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    #[command(about = "Run integrity check on all databases")]
    Check,
    #[command(about = "Run VACUUM to reclaim space in all databases")]
    Vacuum,
    #[command(about = "Show database sizes and table statistics")]
    Stats,
    #[command(about = "Back up all databases to a directory")]
    Backup {
        #[arg(long, default_value = ".")]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct FolderCreate {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(
        long,
        default_value = "sendreceive",
        help = "Sync mode: sendreceive, receiveonly, or sendonly"
    )]
    pub mode: String,
    #[arg(long, help = "Enable Syncthing relay fallback for this folder")]
    pub relay_fallback: bool,
    #[arg(long, help = "Add the created folder to a named network")]
    pub network: Option<String>,
}

#[derive(Debug, Args)]
pub struct FolderJoin {
    #[arg(help = "Iroh document ticket for a new folder, or a folder selector when using --subscribe")]
    pub ticket: String,
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "receiveonly")]
    pub mode: String,
    #[arg(long, help = "Enable Syncthing relay fallback for this folder")]
    pub relay_fallback: bool,
    #[arg(long, help = "Add the joined folder to a named network")]
    pub network: Option<String>,
    #[arg(
        long,
        help = "Track + enable live syncing (persisted subscribe-changes); idempotent on an existing folder"
    )]
    pub subscribe: bool,
    #[arg(long, help = "Only deliver entries ingested after live syncing is enabled")]
    pub ingest_only: bool,
    #[arg(long, help = "Ignore events emitted by this device's own writes")]
    pub ignore_self: bool,
    #[arg(long, help = "Parent directory prepended to the path argument")]
    pub prefix: Option<PathBuf>,
    #[arg(long, help = "Area prefix filter for subscription entries", conflicts_with = "glob")]
    pub sync_prefix: Option<PathBuf>,
    #[arg(long, conflicts_with = "sync_prefix")]
    pub glob: Option<String>,
    #[arg(long)]
    pub max_count: Option<u64>,
    #[arg(long)]
    pub max_size: Option<u64>,
}

#[derive(Debug, Args)]
pub struct LocalPathArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, help = "Collect and sort output instead of streaming it")]
    pub sort: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Scanner threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct FindArgs {
    pub pattern: String,
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "glob", value_parser = ["exact", "glob", "regex"])]
    pub kind: String,
    #[arg(
        short = 'i',
        long,
        help = "Case insensitive search",
        conflicts_with = "case_sensitive"
    )]
    pub ignore_case: bool,
    #[arg(short = 's', long, help = "Case sensitive search", conflicts_with = "ignore_case")]
    pub case_sensitive: bool,
    #[arg(short = 'F', long, help = "Treat patterns as literal strings")]
    pub fixed_strings: bool,
    #[arg(short = 'p', long, help = "Search full path (default: filename only)")]
    pub full_path: bool,
    #[arg(short = 'H', long, help = "Search hidden files and directories")]
    pub hidden: bool,
    #[arg(short = 'L', long, help = "Follow symbolic links")]
    pub follow_links: bool,
    #[arg(short = 'a', long, help = "Print absolute paths")]
    pub absolute_path: bool,
    #[arg(
        short = 'd',
        long = "download",
        alias = "dl",
        alias = "downloadable",
        help = "Exclude sendonly folders from search"
    )]
    pub downloadable: bool,
    #[arg(
        long,
        alias = "depth",
        alias = "levels",
        action = clap::ArgAction::Append,
        help = "Depth constraints: N, +N (min), -N (max)"
    )]
    pub depth: Vec<String>,
    #[arg(long, help = "Alternative min depth notation")]
    pub min_depth: Option<usize>,
    #[arg(long, help = "Alternative max depth notation")]
    pub max_depth: Option<usize>,
    #[arg(
        long,
        alias = "size",
        alias = "S",
        action = clap::ArgAction::Append,
        help = "Size constraints: N, -N, +N, N%10, +5GB, etc."
    )]
    pub sizes: Vec<String>,
    #[arg(
        long,
        alias = "changed-within",
        action = clap::ArgAction::Append,
        help = "Newer than: '3 days', '2 weeks'"
    )]
    pub modified_within: Vec<String>,
    #[arg(
        long,
        alias = "changed-before",
        action = clap::ArgAction::Append,
        help = "Older than: '3 years', '1 month'"
    )]
    pub modified_before: Vec<String>,
    #[arg(
        long,
        action = clap::ArgAction::Append,
        help = "Time modified: '-3 days' (newer), '+3 days' (older)"
    )]
    pub time_modified: Vec<String>,
    #[arg(
        short = 'e',
        long,
        alias = "ext",
        alias = "exts",
        alias = "extensions",
        action = clap::ArgAction::Append,
        help = "File extensions to include"
    )]
    pub extension: Vec<String>,
    #[arg(
        long = "type",
        value_parser = ["f", "d", "l"],
        help = "Filter by type: f=file, d=dir, l=symlink"
    )]
    pub file_type: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Scanner threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct SortArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(
        long = "by",
        alias = "sort",
        alias = "u",
        default_value = "niche",
        value_parser = [
            "niche", "frecency", "peers", "random", "folder",
            "time", "date", "week", "month", "year", "size",
            "folder-size", "folder-avg-size", "folder-date", "folder-time", "count"
        ]
    )]
    pub by: String,
    #[arg(long, help = "Filter files with fewer than N seeders")]
    pub min_seeders: Option<usize>,
    #[arg(long, help = "Filter files with more than N seeders")]
    pub max_seeders: Option<usize>,
    #[arg(long, help = "Ideal popularity (peer count) for niche scoring")]
    pub niche: Option<usize>,
    #[arg(long, help = "Divisor for recency weighting in frecency calculation")]
    pub frecency_weight: Option<u64>,
    #[arg(long, alias = "TS", alias = "LS", help = "Quit after printing N bytes of files")]
    pub limit_size: Option<String>,
    #[arg(
        long,
        alias = "d",
        alias = "levels",
        action = clap::ArgAction::Append,
        help = "Constrain folder aggregates by depth: N, +N (min), -N (max)"
    )]
    pub depth: Vec<String>,
    #[arg(long, help = "Alternative min depth notation")]
    pub min_depth: Option<usize>,
    #[arg(long, help = "Alternative max depth notation")]
    pub max_depth: Option<usize>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Scanner threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
    #[arg(
        long,
        help = "Query daemon for peer counts and frequency data to enrich niche/frecency/peers sorting"
    )]
    pub enrich: bool,
}

#[derive(Debug, Args)]
pub struct StatArgs {
    pub path: PathBuf,
    #[arg(long, conflicts_with = "format")]
    pub terse: bool,
    #[arg(long, conflicts_with = "terse")]
    pub format: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Scanner threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct LeaveArgs {
    #[arg(help = "Namespace ID or path to a managed folder")]
    pub folder: String,
    #[arg(long, help = "Also delete the folder's local files")]
    pub delete_files: bool,
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    #[command(flatten)]
    pub filter: super::filter::ContentFilter,
    #[command(flatten)]
    pub providers: super::filter::ProviderSelector,
    #[arg(long, help = "Fetch only blobs with at most N observed peers")]
    pub max_peers: Option<usize>,
    #[arg(long, help = "Fetch only blobs with at least N observed peers")]
    pub min_peers: Option<usize>,
    #[arg(long, help = "Minimum number of blobs to fetch")]
    pub min_count: Option<usize>,
    #[arg(long, help = "Maximum number of blobs to fetch")]
    pub max_count: Option<usize>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Copy threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct MirrorArgs {
    #[arg(help = "Provider ID (PublicKey hex) to mirror blobs from")]
    pub provider: Option<String>,
    #[arg(long, help = "Network name or ID to mirror all blobs across")]
    pub network: Option<String>,
    #[arg(long, default_value_t = 3, help = "Minimum replication budget per blob (default 3)")]
    pub min_providers: usize,
    #[arg(
        long,
        visible_alias = "no-seeding",
        help = "Skip lease announcements after mirroring"
    )]
    pub no_sharing: bool,
    #[arg(long, help = "Report what would be mirrored without fetching")]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    pub path: PathBuf,
    #[arg(long, help = "Folder namespace; defaults to the only managed folder")]
    pub folder: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Scanner threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
    #[arg(
        long,
        help = "Query daemon for peer counts and frequency data to enrich niche/frecency/peers sorting"
    )]
    pub enrich: bool,
}

#[derive(Debug, Args)]
pub struct SnapshotCreateArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Scanner threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct SnapshotRestoreArgs {
    pub path: PathBuf,
    pub snapshot: String,
}

#[derive(Debug, Args)]
pub struct HealthArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[command(flatten)]
    pub filter: super::filter::ContentFilter,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    #[command(about = "Create a content-addressed snapshot")]
    Create(SnapshotCreateArgs),
    #[command(about = "Restore a snapshot to a folder or directory")]
    Restore(SnapshotRestoreArgs),
    #[command(name = "list", about = "List local snapshots")]
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    #[command(about = "Compare two snapshots")]
    Diff {
        path: PathBuf,
        first: String,
        second: String,
    },
    #[command(about = "Delete a snapshot and release its pins")]
    Delete { path: PathBuf, snapshot: String },
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "sendreceive")]
    pub mode: String,
}

#[derive(Debug, Args)]
pub struct AutomaticArgs {
    #[arg(long, help = "Print the active filter configuration and exit")]
    pub show_filters: bool,
    #[arg(long, help = "Evaluate paths without starting the daemon")]
    pub dry_run: bool,
    #[arg(long, num_args = 1.., help = "Paths evaluated by --dry-run")]
    pub paths: Vec<PathBuf>,
    #[arg(long, help = "Filter configuration (defaults to DATA_DIR/filters.toml)")]
    pub filters: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct StartArgs {
    #[arg(long, alias = "background", help = "Run in the background (daemon mode)")]
    pub bg: bool,
    #[arg(long, help = "Override the global persistent data directory")]
    pub data_dir: Option<PathBuf>,
    #[arg(long, help = "Write daemon logs to this file")]
    pub log_file: Option<PathBuf>,
    #[arg(long, value_parser = clap::value_parser!(usize))]
    pub max_threads: Option<usize>,
    #[arg(long, value_parser = clap::value_parser!(u64))]
    pub sync_interval: Option<u64>,
    #[arg(long, help = "Disable Iroh relay mode (no relay server connections)")]
    pub no_relay: bool,
    #[arg(long, help = "Disable mDNS local peer discovery")]
    pub no_mdns: bool,
    #[arg(long, help = "Disable the UDP beacon local peer discovery")]
    pub no_beacon: bool,
    #[arg(long, help = "Base UDP port the beacon spreads network scopes over")]
    pub beacon_port: Option<u16>,
    #[arg(long, help = "Restrict the beacon to a single network interface by name")]
    pub discovery_interface: Option<String>,
    #[arg(long, help = "Media HTTP server listen address (e.g. 127.0.0.1:9193)")]
    pub media_listen: Option<std::net::SocketAddr>,
}

#[derive(Debug, Args)]
pub struct ShutdownArgs {
    #[arg(long, help = "Skip graceful shutdown")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value_t = 500, help = "Debounce changes in milliseconds")]
    pub debounce_ms: u64,
    #[arg(long, value_name = "GLOB", help = "Ignore a path glob; may be repeated")]
    pub exclude: Vec<String>,
    #[arg(long, help = "Process one event and exit")]
    pub once: bool,
}

#[derive(Debug, Args)]
pub struct StatsArgs {
    #[arg(long, help = "Limit display to a folder or namespace")]
    pub folder: Option<PathBuf>,
    #[arg(long, help = "Limit display to a peer node ID")]
    pub peer: Option<String>,
    #[arg(long, help = "Reset persisted counters before displaying them")]
    pub reset: bool,
    #[arg(long, help = "Retained for compatibility; counters are persisted since period start")]
    pub period: Option<String>,
}

#[derive(Debug, Args)]
pub struct FileStatsArgs {
    #[arg(help = "Namespace ID or path to a managed folder")]
    pub folder: String,
    #[arg(
        long,
        default_value = "extension",
        value_parser = ["extension", "size", "all", "time"]
    )]
    pub by: String,
    #[arg(long, help = "Top N largest files by size")]
    pub top_largest: Option<usize>,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[command(flatten)]
    pub filter: super::filter::ContentFilter,
    #[arg(long, help = "Attempt to repair corrupted blobs by re-downloading from peers")]
    pub fix: bool,
    #[command(flatten)]
    pub providers: super::filter::ProviderSelector,
}

#[derive(Debug, Subcommand)]
pub enum ScheduleCommand {
    #[command(about = "Update the global schedule")]
    Set {
        #[arg(long)]
        active: Option<String>,
        #[arg(long, help = "Bandwidth rate (e.g. '500K', '2M')")]
        bandwidth: Option<String>,
        #[arg(
            long,
            requires = "bandwidth",
            help = "Time window for the bandwidth limit (e.g. '08:00-18:00')"
        )]
        period: Option<String>,
    },
    #[command(about = "Set schedule overrides for a named folder")]
    Folder {
        name: String,
        #[arg(long)]
        active: Option<String>,
        #[arg(long)]
        max_upload: Option<String>,
        #[arg(long)]
        max_download: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    #[arg(help = "Namespace ID or managed folder path")]
    pub namespace: String,
    #[arg(long, help = "Publish this content hash as an unauthenticated blob ticket")]
    pub blob: Option<String>,
}

#[derive(Debug, Args)]
pub struct UnpublishArgs {
    #[arg(help = "Namespace ID or managed folder path")]
    pub namespace: String,
    #[arg(long, help = "Blob content hash to unpublish")]
    pub blob: String,
}

#[derive(Debug, Subcommand)]
pub enum CollectionCommand {
    #[command(about = "Initialize a directory as a versioned collection")]
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "1.0.0")]
        version: String,
        #[arg(long)]
        name: Option<String>,
    },
    #[command(about = "Scan files and update the local collection manifest")]
    Add {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    #[command(about = "Create a new collection manifest version")]
    Versions {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        changelog: Option<String>,
    },
    #[command(about = "Store a collection manifest and mutable head in a folder")]
    Publish {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, value_name = "NODE_ID")]
        bootstrap: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    #[command(about = "Export one or more package directories as compressed CAR archive files")]
    Export {
        #[arg(required = true, num_args = 1.., value_name = "PACKAGE_OR_OUTPUT")]
        paths: Vec<PathBuf>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long, value_name = "EXPRESSION")]
        filter: Vec<String>,
    },
    #[command(about = "Import and install a compressed CAR archive file")]
    Import {
        #[arg(required = true, num_args = 1.., value_name = "ARCHIVE")]
        archives: Vec<PathBuf>,
        #[arg(long, value_name = "EXPRESSION")]
        filter: Vec<String>,
    },
    #[command(about = "List locally installed packages, optionally filtering by text")]
    Search {
        query: Option<String>,
        #[arg(long, value_name = "NODE_ID")]
        bootstrap: Vec<String>,
        #[arg(long, default_value_t = 250)]
        timeout_ms: u64,
    },
    #[command(about = "Show a collection manifest from a ticket or blob hash")]
    Info {
        ticket: Option<String>,
        #[arg(long, help = "Blob hash of the manifest (requires --node-id)")]
        hash: Option<String>,
        #[arg(long, help = "Node ID hosting the manifest blob")]
        node_id: Option<String>,
    },
    #[command(about = "Verify, stage, and atomically install a collection version")]
    Install {
        ticket: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
    #[command(about = "Install a newer collection manifest version via ticket")]
    Upgrade {
        ticket: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
    #[command(about = "Remove a non-current installed collection version")]
    Remove { collection: String, version: String },
    #[command(about = "Verify an installed collection version")]
    Verify {
        collection: String,
        #[arg(long)]
        version: Option<String>,
    },
    #[command(name = "list", about = "List locally installed collections")]
    List,
    #[command(about = "List installed versions for a collection")]
    Versions { collection: String },
    #[command(about = "Switch the active installed collection version")]
    Switch { collection: String, version: String },
}

#[derive(Debug, Subcommand)]
pub enum NetworkCommand {
    #[command(about = "Create a named network")]
    Create {
        name: String,
        #[arg(long, default_value = "")]
        label: String,
        #[arg(long)]
        invite_only: bool,
    },
    #[command(name = "ls", about = "List networks or inspect one")]
    List { name: Option<String> },
    #[command(about = "Join a network from an invitation")]
    Join { ticket: String },
    #[command(about = "Leave a network")]
    Leave { name: String },
    #[command(about = "Generate a network invitation")]
    Invite {
        name: String,
        #[arg(help = "Optional Iroh node ID to bind the invitation to")]
        device: Option<String>,
    },
    #[command(about = "Remove a device from a network")]
    Kick { name: String, device: String },
    #[command(about = "Show recent network events")]
    Events {
        network_id: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    #[command(about = "Show network connectivity health")]
    Health {
        #[arg(long)]
        network: Option<String>,
    },
    #[command(about = "Test a Syncthing relay TCP connection")]
    TestRelay {
        #[arg(long = "relay-url")]
        relay_url: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum IndexingCommand {
    #[command(about = "Opt a synchronized folder into indexing")]
    Enable { folder: PathBuf },
    #[command(about = "Remove a folder from the local index")]
    Disable { folder: PathBuf },
    #[command(about = "Publish folder metadata to a catalog")]
    Publish {
        folder: PathBuf,
        #[arg(long)]
        catalog: String,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    #[command(about = "Search subscribed catalogs")]
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    #[command(about = "Show verified provider health for a content hash")]
    Health { hash: String },
    #[command(about = "Manage signed metadata")]
    Meta {
        #[command(subcommand)]
        command: MetaCommand,
    },
    #[command(about = "Manage local and federated denylists")]
    Filter {
        #[command(subcommand)]
        command: FilterCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum MetaCommand {
    #[command(about = "Append signed metadata to a content hash")]
    Add {
        hash: String,
        key: String,
        value: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
    },
}

#[derive(Debug, Subcommand)]
pub enum FilterCommand {
    #[command(about = "Add a device, file, or hash denylist rule")]
    Add {
        #[arg(value_parser = ["device", "file", "hash"])]
        rule_type: String,
        value: String,
    },
    #[command(about = "Import a signed federated filter list")]
    Subscribe { source: String },
}

#[derive(Debug, Subcommand)]
pub enum LinkCommand {
    #[command(about = "Create an immutable, private, or mutable link")]
    Create {
        source: PathBuf,
        #[arg(long, alias = "alias", conflicts_with = "private")]
        name: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long, default_value_t = 0)]
        sequence: u64,
        #[arg(long, conflicts_with = "name")]
        private: bool,
        #[arg(long, help = "Private-link expiration as a Unix timestamp")]
        expires: Option<u64>,
        #[arg(long, help = "Namespace (folder) to publish the link into")]
        publish: Option<String>,
    },
    #[command(about = "Resolve a stable link")]
    Resolve {
        link: String,
        #[arg(long)]
        version: Option<String>,
    },
    #[command(about = "Revoke a private capability link")]
    Revoke {
        link: String,
        #[arg(long, help = "Broadcast revocation to peers via gossip")]
        broadcast: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    #[command(about = "Register a blob ticket as an alternate provider")]
    Add { collection: String, provider: String },
}

#[derive(Debug, Subcommand)]
pub enum TrustCommand {
    #[command(about = "Show trust and moderation state")]
    Show { subject: String },
    #[command(about = "Delegate trust to a publisher identity")]
    Delegate {
        publisher: String,
        #[arg(long)]
        expires: Option<u64>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, help = "Maximum delegation chain depth (1 = delegate only)")]
        max_depth: Option<u32>,
    },
    #[command(about = "Revoke a trust delegation")]
    RevokeDelegation {
        publisher: String,
        #[arg(long)]
        scope: Option<String>,
    },
    #[command(about = "Manage provider trust and bans")]
    Provider {
        #[command(subcommand)]
        command: ProviderTrustCommand,
    },
    #[command(about = "Publish or subscribe to provider trust signals")]
    Stream {
        #[command(subcommand)]
        command: TrustStreamCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProviderTrustCommand {
    #[command(about = "Show provider reputation, bans, and trust records")]
    Show {
        provider: String,
        #[arg(long, help = "Evaluate content-scoped trust for this hash")]
        hash: Option<String>,
    },
    #[command(name = "list", about = "List providers known to the local index")]
    List {
        #[arg(long, help = "Evaluate content-scoped trust for this hash")]
        hash: Option<String>,
    },
    #[command(about = "Ban a provider globally or for one content hash")]
    Ban {
        provider: String,
        #[arg(long)]
        hash: Option<String>,
        #[arg(long, default_value = "manual provider ban")]
        reason: String,
        #[arg(long, help = "Ban duration in seconds")]
        duration: Option<u64>,
    },
    #[command(about = "Remove a provider's global and scoped bans")]
    Unban { provider: String },
    #[command(about = "Vouch for a provider")]
    Vouch {
        provider: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value = "locally vouched provider")]
        reason: String,
        #[arg(long, help = "Broadcast vouch via gossip trust stream")]
        broadcast: bool,
    },
    #[command(about = "Distrust a provider")]
    Distrust {
        provider: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value = "locally distrusted provider")]
        reason: String,
        #[arg(long, help = "Broadcast distrust via gossip trust stream")]
        broadcast: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum TrustStreamCommand {
    #[command(about = "Subscribe to a provider trust stream ticket or file")]
    Subscribe { ticket: String },
    #[command(about = "Publish a signed provider trust signal")]
    Publish {
        #[arg(long)]
        provider: String,
        #[arg(long)]
        signal: String,
        #[arg(long)]
        hash: Option<String>,
        #[arg(long)]
        sequence: Option<u64>,
    },
}

#[derive(Debug, Args)]
pub struct AttestArgs {
    #[arg(help = "Content hash to attest")]
    pub content: String,
    #[arg(long, conflicts_with_all = ["provenance", "derivative"])]
    pub license: Option<String>,
    #[arg(long, conflicts_with_all = ["license", "derivative"], help = "Provenance attestation type")]
    pub provenance: Option<String>,
    #[arg(long, conflicts_with_all = ["license", "provenance"], help = "Derivative work attestation type")]
    pub derivative: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub sequence: u64,
}

#[derive(Debug, Subcommand)]
pub enum AttestCommand {
    #[command(about = "Sign and optionally broadcast a content attestation")]
    Create {
        content: String,
        #[arg(long, conflicts_with_all = ["provenance", "derivative"])]
        license: Option<String>,
        #[arg(long, conflicts_with_all = ["license", "derivative"], help = "Provenance attestation type")]
        provenance: Option<String>,
        #[arg(long, conflicts_with_all = ["license", "provenance"], help = "Derivative work attestation type")]
        derivative: Option<String>,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, help = "Broadcast attestation via gossip")]
        broadcast: bool,
    },
    #[command(about = "Verify attestations for content from the network")]
    Verify {
        hash: String,
        #[arg(long, help = "Timeout in seconds for gossip collection")]
        timeout: Option<u64>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ModerationCommand {
    #[command(name = "ls", about = "List local moderation records")]
    List { content: Option<String> },
    #[command(about = "Hide a content record locally")]
    Hide {
        record: String,
        #[arg(long, default_value = "hidden by local policy")]
        reason: String,
    },
    #[command(about = "Sign and submit a moderation report (broadcasts via gossip)")]
    Report {
        #[arg(help = "Content hash to report")]
        record: String,
        #[arg(long, help = "Reason for the report")]
        reason: String,
        #[arg(long, help = "Also broadcast to peers via gossip")]
        broadcast: bool,
    },
}
