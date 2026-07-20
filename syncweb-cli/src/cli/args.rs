use clap::Parser;

use super::commands::Command;

#[derive(Debug, Parser)]
#[command(name = "syncweb", about = "Delay-tolerant web surfing")]
pub struct Cli {
    #[arg(long, global = true, help = "Enable verbose structured logging")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}
