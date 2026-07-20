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
    #[command(about = "Download a local file to a destination")]
    Download(DownloadArgs),
    #[command(about = "Initialize a folder and print a shareable URL")]
    Init(InitArgs),
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
    pub destination: PathBuf,
    #[arg(
        long,
        default_value_t = 0,
        help = "Copy threads (1 disables parallelism, 0 uses all available CPUs)"
    )]
    pub threads: usize,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(long, default_value = "sendreceive")]
    pub mode: String,
}

#[derive(Debug, Subcommand)]
pub enum NetworkCommand {
    #[command(about = "Test a Syncthing relay TCP connection")]
    TestRelay {
        #[arg(long = "relay-url")]
        relay_url: String,
    },
}
