use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show syncweb version information")]
    Version,
    #[command(about = "Start an interactive command shell")]
    Repl,
}
