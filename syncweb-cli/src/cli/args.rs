use std::path::PathBuf;

use clap::Parser;

use super::commands::Command;

#[derive(Debug, Parser)]
#[command(name = "syncweb", about = "Delay-tolerant web surfing")]
pub struct Cli {
    #[arg(long, global = true, help = "Enable verbose structured logging")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Emit machine-readable JSON where supported")]
    pub json: bool,

    #[arg(long, global = true, help = "Disable colored output")]
    pub no_color: bool,

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
