use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show syncweb version information")]
    Version,
    #[command(about = "Start the local syncweb daemon")]
    Start(StartArgs),
    #[command(about = "Stop the local syncweb node")]
    Shutdown(ShutdownArgs),
    #[command(about = "Show local daemon status")]
    Status,
    #[command(about = "Show this device's Iroh and Syncthing identities")]
    Devices,
    #[command(about = "Show networks and their health")]
    Networks(NetworkListArgs),
    #[command(about = "Ask the local daemon to reload configuration")]
    Reload,
    #[command(about = "Ask the local daemon to trigger synchronization")]
    DaemonSync(DaemonSyncArgs),
    #[command(
        about = "Create a synchronized folder and print a read-only join ticket/URL (--write for write access, --no-share to skip)"
    )]
    Create(FolderCreate),
    #[command(about = "Join a folder from an Iroh document ticket")]
    Join(FolderJoin),
    #[command(about = "Leave a synchronized folder, optionally deleting its local files")]
    Leave(LeaveArgs),
    #[command(about = "List managed folders")]
    Folders,
    #[command(about = "Show or update local configuration")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    #[command(about = "List files in a local folder")]
    Ls(LocalPathArgs),
    #[command(about = "Search local files")]
    Find(FindArgs),
    #[command(about = "Search catalog content, packages, and editorial channels")]
    Search(SearchArgs),
    #[command(about = "Sort local files by discovery criteria")]
    Sort(SortArgs),
    #[command(about = "Show detailed metadata for a local file")]
    Stat(StatArgs),
    #[command(about = "Download folder content or copy a local file")]
    Download(DownloadArgs),
    #[command(about = "Import local files into a synchronized folder")]
    Import(ImportArgs),
    #[command(about = "Manage content-addressed snapshots", alias = "snapshots")]
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    #[command(about = "Inspect and control durable transfer jobs")]
    Transfer {
        #[command(subcommand)]
        command: TransferCommand,
    },
    #[command(about = "Watch a folder and import filesystem changes")]
    Watch(WatchArgs),
    #[command(about = "Show statistics for folders and files")]
    Stats {
        #[command(subcommand)]
        command: StatsCommand,
    },
    #[command(about = "Re-check local folder blob integrity")]
    Verify(VerifyArgs),
    #[command(about = "Publish folder metadata to a catalog")]
    Publish {
        #[command(subcommand)]
        command: PublishCommand,
    },
    #[command(about = "Share a folder, printing a ticket (read-only by default, --write for write access)")]
    Share(ShareArgs),
    #[command(about = "Stop sharing a folder or blob (removes pins and announcements)")]
    Unshare(UnshareArgs),
    #[command(about = "Create, version, publish, and manage collection packages")]
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
    #[command(about = "Manage blob provider registrations")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
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
    #[command(about = "Print this message or the help of the given subcommand(s)")]
    Help {
        #[arg(value_name = "COMMAND")]
        command: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct DaemonSyncArgs {
    #[arg(help = "Namespace of a live folder to sync now; omit it to sync every enabled folder")]
    pub namespace: Option<String>,
}

#[derive(Debug, Args)]
pub struct NetworkListArgs {
    #[arg(value_name = "NAME", help = "Limit to a single network by name or ID")]
    pub name: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Set a configuration value")]
    Set { key: String, value: String },
    #[command(about = "Show configuration, optionally limited to a section")]
    Show { section: Option<String> },
    #[command(about = "Show or update synchronization schedules")]
    Schedule {
        #[command(subcommand)]
        command: Option<ScheduleCommand>,
    },
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
    pub hash: Option<String>,
    #[arg(
        long,
        help = "Local file to read content from; computes the blob hash and size automatically"
    )]
    pub source: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Blob size in bytes (ignored when --source is given)"
    )]
    pub size: u64,
    #[arg(
        long,
        help = "Allocate and materialize the job immediately instead of leaving it queued"
    )]
    pub now: bool,
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
    #[arg(
        long,
        default_value_t = true,
        help = "Scan and import existing files in the directory"
    )]
    pub import: bool,
    #[arg(
        long,
        conflicts_with = "import",
        help = "Skip scanning existing files in the directory"
    )]
    pub no_import: bool,
    #[arg(long, help = "Grant write access on the share ticket (default: read-only)")]
    pub write: bool,
    #[arg(long, help = "Create the folder without sharing it (no ticket/URL printed)")]
    pub no_share: bool,
    #[arg(
        long,
        help = "Do not opt the folder into local indexing (indexing is enabled by default)"
    )]
    pub no_indexing: bool,
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
    #[arg(
        long,
        help = "Download matching existing content to the local folder after joining (one-shot; uses the same prefix/glob/max filters)"
    )]
    pub download_all: bool,
    #[arg(
        long,
        help = "Do not opt the folder into local indexing (indexing is enabled by default)"
    )]
    pub no_indexing: bool,
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
    #[command(flatten)]
    pub listing: ListingFlags,
}

/// Shared metadata-vs-disk listing switches for `ls`, `find`, and `sort`.
#[derive(Debug, Args, Clone, Default)]
pub struct ListingFlags {
    #[arg(
        long,
        conflicts_with = "local_only",
        help = "Show only entries not yet downloaded (State == remote)"
    )]
    pub remote_only: bool,
    #[arg(
        long,
        conflicts_with = "remote_only",
        help = "Scan the local disk instead of the metadata index (works on any path, even outside a Syncweb folder)"
    )]
    pub local_only: bool,
    #[arg(long, help = "Only entries whose path starts with this prefix")]
    pub path_prefix: Option<String>,
    #[arg(long, alias = "glob", help = "Only entries whose path matches this glob pattern")]
    pub path_glob: Option<String>,
    #[arg(long, help = "Skip per-file disk metadata lookup (pure metadata listing)")]
    pub no_enrich: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SearchKind {
    /// Search the local catalog index (FTS) and, when requested, editorial channels and gossip.
    All,
    /// Search only the local catalog index (FTS over subscribed catalog docs).
    Catalog,
    /// Search installed packages and, when requested, the gossip package catalog.
    Package,
    /// Search only an editorial channel.
    Channel,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[arg(help = "Search query; omit to list everything")]
    pub query: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value = "all",
        help = "Which backend to search: all, catalog, package, or channel"
    )]
    pub kind: SearchKind,
    #[arg(long, help = "Restrict results to an editorial channel")]
    pub channel: Option<String>,
    #[arg(long, default_value_t = 20, help = "Maximum number of results")]
    pub limit: usize,
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
    #[command(flatten)]
    pub listing: ListingFlags,
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
    #[command(flatten)]
    pub listing: ListingFlags,
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

#[derive(Debug, Args, Clone)]
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
pub struct ImportArgs {
    pub path: PathBuf,
    #[arg(
        long,
        visible_alias = "namespace",
        help = "Folder namespace or managed folder path; defaults to the only managed folder"
    )]
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

#[derive(Debug, Subcommand)]
pub enum StatsCommand {
    #[command(about = "Show persisted bandwidth accounting")]
    Network(StatsNetworkArgs),
    #[command(about = "Show file-level statistics for synced folder content")]
    Files(StatsFilesArgs),
}

#[derive(Debug, Args)]
pub struct StatsNetworkArgs {
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
pub struct StatsFilesArgs {
    #[arg(default_value = ".", help = "Namespace ID or path to a managed folder")]
    pub path: PathBuf,
    #[arg(
        long,
        default_value = "extension",
        value_parser = ["extension", "size", "all", "time"]
    )]
    pub by: String,
    #[arg(long, help = "Top N largest files by size")]
    pub top_largest: Option<usize>,
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
pub struct StartArgs {
    #[arg(long, alias = "background", help = "Run in the background (daemon mode)")]
    pub bg: bool,
    #[arg(long, help = "Run only the media HTTP server (standalone) and exit")]
    pub media_only: bool,
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
    #[arg(long, help = "Print the active filter configuration and exit")]
    pub show_filters: bool,
    #[arg(long, help = "Evaluate paths against the filter rules without importing")]
    pub dry_run: bool,
    #[arg(long, num_args = 1.., help = "Paths evaluated by --dry-run")]
    pub paths: Vec<PathBuf>,
    #[arg(long, help = "Filter configuration (defaults to DATA_DIR/filters.toml)")]
    pub filters: Option<PathBuf>,
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

#[derive(Debug, Subcommand)]
pub enum PublishCommand {
    #[command(about = "Publish folder metadata to a catalog")]
    Catalog(PublishCatalogArgs),
}

#[derive(Debug, Args)]
pub struct PublishCatalogArgs {
    pub folder: PathBuf,
    #[arg(long)]
    pub catalog: String,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ShareArgs {
    #[arg(default_value = ".", help = "Folder path or namespace")]
    pub path: PathBuf,
    #[arg(
        long,
        help = "Share a single content hash as an unauthenticated blob ticket (blobs are immutable; always pinned, never persisted)"
    )]
    pub blob: Option<String>,
    #[arg(long = "write", alias = "writable", help = "Grant write access (default: read-only)")]
    pub write: bool,
    #[arg(long, help = "Skip pinning the shared folder's blobs")]
    pub no_pin: bool,
    #[arg(long, help = "Skip persisting the share record")]
    pub no_persist: bool,
    #[arg(
        long = "list",
        help = "List persisted shares, optionally filtered by the positional path"
    )]
    pub list: bool,
}

#[derive(Debug, Args)]
pub struct UnshareArgs {
    #[arg(default_value = ".", help = "Folder path or namespace")]
    pub path: PathBuf,
    #[arg(long, help = "Stop sharing a single content hash (unpins and unannounces the blob)")]
    pub blob: Option<String>,
    #[arg(
        long = "write",
        alias = "writable",
        help = "Remove the write share (default: read-only)"
    )]
    pub write: bool,
}

#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    #[command(about = "Scan one or more paths into a package manifest (creates it if missing)")]
    Add {
        #[arg(required = true, num_args = 1.., value_name = "PATH", default_value = ".")]
        paths: Vec<PathBuf>,
        #[arg(long, default_value = "1.0.0")]
        version: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(
            long,
            value_name = "PATH",
            help = "Override the common root for logical path rebasing"
        )]
        root: Option<PathBuf>,
    },
    #[command(about = "Create a new package manifest version")]
    Bump {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        changelog: Option<String>,
    },
    #[command(about = "Publish a package manifest ticket and announce it to the catalog")]
    Publish {
        #[arg(required = true, num_args = 1.., value_name = "PATH")]
        paths: Vec<PathBuf>,
        #[arg(
            long,
            help = "Folder namespace, managed folder path, or omitted to default to the only managed folder"
        )]
        namespace: Option<String>,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, value_name = "NODE_ID")]
        bootstrap: Vec<String>,
        #[arg(
            long,
            value_name = "PATH",
            help = "Override the common root for logical path rebasing"
        )]
        root: Option<PathBuf>,
    },
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
    #[command(about = "Join a network from an invitation")]
    Join { ticket: String },
    #[command(about = "Leave a network")]
    Leave { name: String },
    #[command(
        about = "List networks, optionally limited to a single network by name",
        alias = "ls"
    )]
    List {
        #[arg(help = "Optional network name to inspect")]
        name: Option<String>,
    },
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
    #[command(about = "Manage local and federated denylists")]
    Filter {
        #[command(subcommand)]
        command: FilterCommand,
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
        #[arg(long, help = "Print the resolution without fetching or pinning the resolved content")]
        no_fetch: bool,
    },
    #[command(about = "Revoke a private capability link")]
    Revoke { link: String },
}

#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    #[command(about = "Register a blob ticket as an alternate provider")]
    Add { collection: String, provider: String },
}
