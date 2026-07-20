use anyhow::{Result, anyhow};
use std::io::{self, BufRead, Write};
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing(verbose: bool) -> Result<()> {
    let default_filter = if verbose {
        "syncweb=debug"
    } else {
        "syncweb=info"
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
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

pub fn run_repl() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    writeln!(stdout, "syncweb repl")?;
    writeln!(stdout, "Type 'help' for help or 'exit' to quit.")?;
    stdout.flush()?;

    for line in stdin.lock().lines() {
        let line = line?;
        match line.trim() {
            "" => {}
            "exit" | "quit" => break,
            "help" => println!("Commands: help, exit, quit"),
            command => println!("Unknown command: {command}"),
        }
    }
    Ok(())
}
