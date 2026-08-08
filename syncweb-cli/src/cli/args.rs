use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser};

use super::commands::Command;

#[derive(Debug, Clone, Copy)]
pub struct CliContext<'a> {
    pub data_dir: &'a Path,
    pub output_json: bool,
    pub no_daemon: bool,
    pub network: Option<&'a str>,
}

pub fn effective_data_dir(data_dir: &Path, network: Option<&str>) -> PathBuf {
    let net = network.unwrap_or("default");
    data_dir.join(net)
}

/// Declares the top-level help categories: the display order, the subcommand
/// names shown under each heading, and the `Command` variant backing each one.
///
/// Generates `COMMAND_CATEGORIES`, the grouped help layout used by
/// `print_grouped_help`, and `category_of`, an exhaustive `match` over
/// `Command` mapping every variant to its category heading. Rust's
/// exhaustiveness check makes adding a `Command` variant without an entry here
/// a compile-time error, so a new subcommand cannot land in the help unless it
/// is categorized.
macro_rules! help_categories {
    (
        $(
            $heading:literal => [
                $( $pat:pat => $name:literal ),* $(,)?
            ];
        )*
    ) => {
        pub const COMMAND_CATEGORIES: &[(&str, &[&str])] = &[
            $( ($heading, &[ $($name),* ]) ),*
        ];

        pub const fn category_of(command: &Command) -> &'static str {
            match command {
                $( $( $pat => $heading, )* )*
            }
        }
    };
}

help_categories! {
    "Daemon" => [
        Command::Start(_) => "start",
        Command::Shutdown(_) => "shutdown",
        Command::Status => "status",
        Command::Reload => "reload",
        Command::DaemonSync => "daemon-sync",
    ];
    "Folders" => [
        Command::Create(_) => "create",
        Command::Join(_) => "join",
        Command::Leave(_) => "leave",
        Command::Folders => "folders",
    ];
    "Files" => [
        Command::Ls(_) => "ls",
        Command::Find(_) => "find",
        Command::Sort(_) => "sort",
        Command::Stat(_) => "stat",
        Command::Download(_) => "download",
        Command::Import(_) => "import",
        Command::Verify(_) => "verify",
        Command::Health(_) => "health",
    ];
    "Automation" => [
        Command::Watch(_) => "watch",
        Command::Automatic(_) => "automatic",
    ];
    "Sharing & Publishing" => [
        Command::Publish(_) => "publish",
        Command::Unpublish(_) => "unpublish",
        Command::Mirror(_) => "mirror",
        Command::Provider { .. } => "provider",
        Command::Link { .. } => "link",
    ];
    "Content" => [
        Command::Snapshot { .. } => "snapshot",
        Command::Transfer { .. } => "transfer",
        Command::Collection { .. } => "collection",
        Command::Package { .. } => "package",
    ];
    "Network" => [
        Command::Network { .. } => "network",
        Command::Media(_) => "media",
        Command::Devices => "devices",
    ];
    "Statistics" => [
        Command::Stats(_) => "stats",
        Command::FileStats(_) => "filestats",
    ];
    "Configuration" => [
        Command::Config { .. } => "config",
    ];
    "Maintenance" => [
        Command::Db { .. } => "db",
    ];
    "Indexing" => [
        Command::Indexing { .. } => "indexing",
    ];
    "Trust & Moderation" => [
        Command::Trust { .. } => "trust",
        Command::Attest { .. } => "attest",
        Command::Moderation { .. } => "moderation",
    ];
    "Tooling" => [
        Command::Version => "version",
        Command::Completions { .. } => "completions",
        Command::Manpages { .. } => "manpages",
        Command::Help { .. } => "help",
    ];
}

fn spec_string(arg: &clap::Arg) -> String {
    use std::fmt::Write as _;
    let mut spec = String::new();
    if let Some(short) = arg.get_short() {
        let _ = write!(spec, "-{short}");
    }
    if let Some(long) = arg.get_long() {
        if !spec.is_empty() {
            spec.push_str(", ");
        }
        let _ = write!(spec, "--{long}");
        if arg.get_action().takes_values()
            && let Some(names) = arg.get_value_names()
        {
            let _ = write!(spec, " <{}>", names.join("> <"));
        }
    }
    let aliases = arg.get_visible_aliases().unwrap_or_default();
    if !aliases.is_empty() {
        let plural = if aliases.len() == 1 { "" } else { "es" };
        let _ = write!(
            spec,
            " [alias{plural}: {}]",
            aliases.iter().map(|a| format!("--{a}")).collect::<Vec<_>>().join(", ")
        );
    }
    spec
}

fn spec_tail(arg: &clap::Arg) -> String {
    if !arg.get_action().takes_values() {
        return String::new();
    }
    let mut tail = String::new();
    let defaults = arg.get_default_values();
    if !defaults.is_empty() {
        use std::fmt::Write as _;
        let vals = defaults
            .iter()
            .map(|v| v.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        let _ = write!(tail, " [default: {vals}]");
    }
    tail
}

/// Print the top-level help with subcommands grouped by category.
pub fn print_grouped_help() {
    let mut cmd = Cli::command();
    cmd.build();
    let subcommands: std::collections::HashMap<&str, &clap::Command> =
        cmd.get_subcommands().map(|sc| (sc.get_name(), sc)).collect();

    if let Some(about) = cmd.get_about() {
        println!("{about}");
        println!();
    }
    println!("Usage: {} [OPTIONS] <COMMAND>", cmd.get_name());
    println!();

    for (heading, names) in COMMAND_CATEGORIES {
        println!("{heading}:");
        for name in *names {
            if let Some(sc) = subcommands.get(name) {
                let about = sc.get_about().unwrap_or_default();
                println!("  {name:<16} {about}");
            }
        }
        println!();
    }

    println!("Options:");
    let options: Vec<(String, String)> = cmd
        .get_arguments()
        .map(|arg| {
            let spec = format!("{}{}", spec_string(arg), spec_tail(arg));
            let help = arg.get_help().map(std::string::ToString::to_string).unwrap_or_default();
            (spec, help)
        })
        .collect();
    let width = options.iter().map(|(s, _)| s.len()).max().unwrap_or(0);
    for (spec, help) in options {
        println!("  {spec:<width$} {help}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_subcommands_are_categorized() {
        let mut cmd = Cli::command();
        cmd.build();
        let subcommand_names: std::collections::HashSet<&str> =
            cmd.get_subcommands().map(clap::Command::get_name).collect();
        let mut mapped: Vec<&str> = COMMAND_CATEGORIES
            .iter()
            .flat_map(|(_, names)| names.iter().copied())
            .collect();
        for name in &mapped {
            assert!(
                subcommand_names.contains(name),
                "COMMAND_CATEGORIES lists '{name}' which is not a subcommand"
            );
        }
        mapped.sort_unstable();
        mapped.dedup();
        assert_eq!(
            mapped.len(),
            subcommand_names.len(),
            "COMMAND_CATEGORIES does not cover every Command variant"
        );
        for sc in cmd.get_subcommands() {
            assert!(
                mapped.iter().any(|n| *n == sc.get_name()),
                "subcommand '{}' is missing from COMMAND_CATEGORIES",
                sc.get_name()
            );
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "syncweb",
    about = "Delay-tolerant web surfing",
    disable_help_subcommand = true
)]
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

    #[arg(
        long,
        help = "Network name for scoped operations (uses data_dir/<network>/). Defaults to 'default' if absent."
    )]
    pub network: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}
