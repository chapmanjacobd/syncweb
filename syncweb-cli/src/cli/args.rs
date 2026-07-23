use std::path::{Path, PathBuf};

use clap::Parser;

use super::commands::Command;

#[derive(Debug, Clone, Copy)]
pub struct CliContext<'a> {
    pub data_dir: &'a Path,
    pub output_json: bool,
    pub no_daemon: bool,
}

#[derive(Debug, Parser)]
#[command(name = "syncweb", about = "Delay-tolerant web surfing")]
pub struct Cli {
    #[arg(long, global = true, help = "Enable verbose structured logging")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Emit machine-readable JSON where supported")]
    pub json: bool,

    #[arg(
        long,
        visible_alias = "embedded",
        global = true,
        help = "Bypass the daemon and use an embedded node for supported commands"
    )]
    pub no_daemon: bool,

    #[arg(
        long,
        global = true,
        default_value = ".syncweb",
        help = "Directory used for persistent node identity and data"
    )]
    pub data_dir: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}
