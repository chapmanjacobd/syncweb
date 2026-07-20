mod cli;

use anyhow::Result;
use clap::Parser;
use cli::{
    args::Cli,
    commands::Command,
    output::{init_tracing, print_version, run_repl},
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    tracing::debug!(command = ?cli.command, "cli initialized");

    match cli.command {
        Command::Version => print_version(),
        Command::Repl => run_repl()?,
    }
    Ok(())
}
