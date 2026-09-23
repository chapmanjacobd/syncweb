use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use dialoguer::Confirm;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing(verbose: bool) -> Result<()> {
    let default_filter = if verbose { "syncweb=debug" } else { "syncweb=info" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    fmt()
        .json()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow!("failed to initialize structured logging: {err}"))?;
    Ok(())
}

pub fn print_version() {
    println!("syncweb {}", env!("CARGO_PKG_VERSION"));
}

/// Require interactive confirmation for destructive operations. Auto-approves
/// only when the caller passes `--yes`, and aborts the operation when stdin is
/// not interactive, so non-interactive automation cannot run a destructive
/// command silently (not even with `--json`).
pub fn confirm_destructive(operation: &str, assume_yes: bool) -> Result<bool> {
    if assume_yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Ok(false);
    }
    Ok(Confirm::new()
        .with_prompt(format!("Are you sure you want to {operation}?"))
        .default(false)
        .show_default(true)
        .interact()?)
}
