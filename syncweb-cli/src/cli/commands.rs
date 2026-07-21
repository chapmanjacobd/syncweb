use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show syncweb version information")]
    Version,
    #[command(about = "Start an interactive command shell")]
    Repl,
    #[command(about = "Create a synchronized folder")]
    Create(FolderCreate),
    #[command(about = "Join a folder from an Iroh document ticket")]
    Join(FolderJoin),
    #[command(about = "Accept a locally available folder")]
    Accept { namespace: String },
    #[command(about = "Remove a local folder replica")]
    Drop { namespace: String },
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
    #[command(about = "Create a content-addressed snapshot")]
    Backup(BackupArgs),
    #[command(about = "Restore a snapshot to a folder or directory")]
    Restore(RestoreArgs),
    #[command(about = "List, diff, or delete snapshots")]
    Snapshots(SnapshotsArgs),
    #[command(about = "Show seeding status per folder blob")]
    Health(HealthArgs),
    #[command(about = "Initialize a folder and print a shareable URL")]
    Init(InitArgs),
    #[command(about = "Run rules-based automatic synchronization")]
    Automatic(AutomaticArgs),
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
    #[arg(long, alias = "depth")]
    pub max_depth: Option<usize>,
    #[arg(long)]
    pub min_size: Option<u64>,
    #[arg(long)]
    pub max_size: Option<u64>,
    #[arg(long, alias = "ext")]
    pub extension: Option<String>,
    #[arg(long = "type", value_parser = ["f", "d", "l"])]
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
    #[arg(long = "by", alias = "sort", default_value = "niche", value_parser = ["niche", "frecency", "peers", "random", "folder"])]
    pub by: String,
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
pub struct BackupArgs {
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
pub struct RestoreArgs {
    pub path: PathBuf,
    pub snapshot: String,
}

#[derive(Debug, Args)]
pub struct HealthArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct SnapshotsArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[command(subcommand)]
    pub command: Option<SnapshotCommand>,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
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
pub struct SubscribeArgs {
    pub ticket: String,
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, help = "Only deliver entries ingested after subscription")]
    pub ingest_only: bool,
    #[arg(long, help = "Ignore events emitted by this subscription session")]
    pub ignore_self: bool,
    #[arg(long, conflicts_with = "glob")]
    pub prefix: Option<PathBuf>,
    #[arg(long, conflicts_with = "prefix")]
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
