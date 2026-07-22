use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show syncweb version information")]
    Version,
    #[command(about = "Start an interactive command shell")]
    Repl,
    #[command(about = "Start the local syncweb node for one command invocation")]
    Start,
    #[command(about = "Stop the local syncweb node")]
    Shutdown,
    #[command(about = "Start and manage the local syncweb daemon")]
    Daemon(DaemonArgs),
    #[command(about = "Show the local daemon status")]
    Status,
    #[command(about = "Ask the local daemon to stop")]
    DaemonShutdown(DaemonShutdownArgs),
    #[command(about = "Ask the local daemon to reload configuration")]
    DaemonReload,
    #[command(about = "Ask the local daemon to trigger synchronization")]
    DaemonSync,
    #[command(about = "Add a folder to the running daemon")]
    DaemonAdd(DaemonAddArgs),
    #[command(about = "Remove a folder from the running daemon")]
    DaemonRemove(DaemonRemoveArgs),
    #[command(about = "Create a synchronized folder")]
    Create(FolderCreate),
    #[command(about = "Join a folder from an Iroh document ticket")]
    Join(FolderJoin),
    #[command(about = "Leave and remove a synchronized folder")]
    Leave(FolderSelector),
    #[command(about = "Unsubscribe from a folder's live sync loop")]
    Unsubscribe(FolderSelector),
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
    #[command(about = "Initialize a folder and print a shareable URL")]
    Init(InitArgs),
    #[command(about = "Run rules-based automatic synchronization")]
    Automatic(AutomaticArgs),
    #[command(about = "Watch a folder and import filesystem changes")]
    Watch(WatchArgs),
    #[command(about = "Show persisted bandwidth accounting")]
    Stats(StatsArgs),
    #[command(about = "Re-check local folder blob integrity")]
    Verify(VerifyArgs),
    #[command(about = "Show or update synchronization schedules")]
    Schedule {
        #[command(subcommand)]
        command: Option<ScheduleCommand>,
    },
    #[command(about = "Subscribe to a folder with event filters")]
    Subscribe(SubscribeArgs),
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
    #[command(about = "Register alternate content providers")]
    Mirror {
        #[command(subcommand)]
        command: MirrorCommand,
    },
    #[command(about = "Inspect and delegate local trust")]
    Trust {
        #[command(subcommand)]
        command: TrustCommand,
    },
    #[command(about = "Sign content provenance attestations")]
    Attest(AttestArgs),
    #[command(about = "Submit a local moderation report")]
    Report(ReportArgs),
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
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Set a configuration value")]
    Set { key: String, value: String },
    #[command(about = "Show configuration, optionally limited to a section")]
    Show { section: Option<String> },
}

#[derive(Debug, Args)]
pub struct FolderCreate {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "sendreceive")]
    pub mode: String,
    #[arg(long, help = "Enable Syncthing relay fallback for this folder")]
    pub relay_fallback: bool,
    #[arg(long, help = "Add the created folder to a named network")]
    pub network: Option<String>,
}

#[derive(Debug, Args)]
pub struct FolderJoin {
    pub ticket: String,
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "receiveonly")]
    pub mode: String,
    #[arg(long, help = "Enable Syncthing relay fallback for this folder")]
    pub relay_fallback: bool,
    #[arg(long, help = "Add the joined folder to a named network")]
    pub network: Option<String>,
    #[arg(long, help = "Exit after joining without entering the sync loop")]
    pub once: bool,
    #[arg(long, help = "Only deliver entries ingested after subscription")]
    pub ingest_only: bool,
    #[arg(long, help = "Ignore events emitted by this subscription session")]
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
        help = "Exclude sendonly/publicreadonly folders from search"
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
pub struct FolderSelector {
    #[arg(help = "Namespace ID or path to a managed folder")]
    pub folder: String,
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    #[arg(long, help = "Fetch only blobs with at most N observed peers")]
    pub max_peers: Option<usize>,
    #[arg(long, help = "Fetch only blobs with at least N observed peers")]
    pub min_peers: Option<usize>,
    #[arg(long)]
    pub min_count: Option<usize>,
    #[arg(long)]
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
    #[arg(long, help = "Folder namespace; defaults to the only managed folder")]
    pub folder: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Import threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
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
pub struct DaemonArgs {
    #[arg(short = 'f', long, help = "Run in the foreground")]
    pub foreground: bool,
    #[arg(long, help = "Override the global persistent data directory")]
    pub data_dir: Option<PathBuf>,
    #[arg(long, help = "Write daemon logs to this file")]
    pub log_file: Option<PathBuf>,
    #[arg(long, value_parser = clap::value_parser!(usize))]
    pub max_threads: Option<usize>,
    #[arg(long, value_parser = clap::value_parser!(u64))]
    pub sync_interval: Option<u64>,
}

#[derive(Debug, Args)]
pub struct DaemonShutdownArgs {
    #[arg(long, help = "Skip graceful shutdown")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct DaemonAddArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub namespace: Option<String>,
}

#[derive(Debug, Args)]
pub struct DaemonRemoveArgs {
    pub namespace: String,
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
pub struct VerifyArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum ScheduleCommand {
    #[command(about = "Update the global schedule")]
    Set {
        #[arg(long)]
        active: Option<String>,
        #[arg(long)]
        bandwidth: Option<String>,
        #[arg(long, requires = "bandwidth")]
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
pub struct SubscribeArgs {
    #[arg(help = "Namespace ID or path to a managed folder")]
    pub folder: String,
    #[arg(long, help = "Only deliver entries ingested after subscription")]
    pub ingest_only: bool,
    #[arg(long, help = "Ignore events emitted by this subscription session")]
    pub ignore_self: bool,
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
pub struct PublishArgs {
    pub namespace: String,
    #[arg(long, help = "Publish this content hash as an unauthenticated blob ticket")]
    pub blob: Option<String>,
}

#[derive(Debug, Args)]
pub struct UnpublishArgs {
    pub namespace: String,
    #[arg(long)]
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
    #[command(about = "Show a collection manifest")]
    Info {
        #[arg(required_unless_present = "ticket", conflicts_with = "ticket")]
        manifest: Option<PathBuf>,
        #[arg(long, conflicts_with = "manifest")]
        ticket: Option<String>,
    },
    #[command(about = "Verify, stage, and atomically install a collection version")]
    Install {
        #[arg(required_unless_present = "ticket", conflicts_with = "ticket")]
        manifest: Option<PathBuf>,
        #[arg(required_unless_present = "ticket", conflicts_with = "ticket")]
        source: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["manifest", "source"])]
        ticket: Option<String>,
    },
    #[command(about = "Install a newer collection manifest version")]
    Upgrade {
        #[arg(required_unless_present = "ticket", conflicts_with = "ticket")]
        manifest: Option<PathBuf>,
        #[arg(required_unless_present = "ticket", conflicts_with = "ticket")]
        source: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["manifest", "source"])]
        ticket: Option<String>,
    },
    #[command(about = "Remove a non-current installed collection version")]
    Remove { collection: String, version: String },
    #[command(about = "Verify an installed collection version against its manifest")]
    Verify { manifest: PathBuf },
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
    },
    #[command(about = "Resolve a stable link")]
    Resolve {
        link: String,
        #[arg(long)]
        version: Option<String>,
    },
    #[command(about = "Revoke a private capability link")]
    Revoke { link: String },
}

#[derive(Debug, Subcommand)]
pub enum MirrorCommand {
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
    },
    #[command(about = "Distrust a provider")]
    Distrust {
        provider: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value = "locally distrusted provider")]
        reason: String,
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
    pub content: String,
    #[arg(long, conflicts_with_all = ["provenance", "derivative"])]
    pub license: Option<String>,
    #[arg(long, conflicts_with_all = ["license", "derivative"])]
    pub provenance: Option<String>,
    #[arg(long, conflicts_with_all = ["license", "provenance"])]
    pub derivative: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub sequence: u64,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    pub record: String,
    #[arg(long)]
    pub reason: String,
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
}
