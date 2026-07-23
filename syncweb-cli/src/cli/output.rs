use anyhow::{Result, anyhow};
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing(verbose: bool) -> Result<()> {
    let default_filter = if verbose { "syncweb=debug" } else { "syncweb=info" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    fmt()
        .json()
        .with_writer(std::io::stdout)
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow!("failed to initialize structured logging: {err}"))?;
    Ok(())
}

pub fn print_version() {
    println!("syncweb {}", env!("CARGO_PKG_VERSION"));
}
