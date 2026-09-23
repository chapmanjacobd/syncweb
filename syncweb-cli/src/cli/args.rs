use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser};

use super::commands::Command;

#[derive(Debug, Clone, Copy)]
pub struct CliContext<'a> {
    pub data_dir: &'a Path,
    pub output_json: bool,
    pub no_daemon: bool,
    pub network: Option<&'a str>,
    pub yes: bool,
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
        Command::Stop(_) => "stop",
        Command::Status => "status",
        Command::Reload => "reload",
        Command::Sync(_) => "sync",
        Command::Devices => "devices",
    ];
    "Folders" => [
        Command::Folders { .. } => "folders",
    ];
    "Files" => [
        Command::Ls(_) => "ls",
        Command::Stat(_) => "stat",
        Command::Find(_) => "find",
        Command::Search(_) => "search",
        Command::Sort(_) => "sort",
        Command::Download(_) => "download",
        Command::Verify(_) => "verify",
        Command::Transfer { .. } => "transfer",
    ];
    "Sharing & Access" => [
        Command::Share(_) => "share",
        Command::Access(_) => "access",
        Command::Link { .. } => "link",
        Command::Package { .. } => "package",
    ];
    "Network" => [
        Command::Network { .. } => "network",
    ];
    "Automation" => [
        Command::Watch(_) => "watch",
        Command::Snapshot { .. } => "snapshot",
    ];
    "Indexing" => [
        Command::Indexing { .. } => "indexing",
    ];
    "Statistics" => [
        Command::Stats { .. } => "stats",
    ];
    "Maintenance" => [
        Command::Db { .. } => "db",
    ];
    "Configuration" => [
        Command::Config { .. } => "config",
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

/// Build the top-level help with subcommands grouped by category.
fn build_grouped_help() -> String {
    use std::fmt::Write as _;
    let mut cmd = Cli::command();
    cmd.build();
    let subcommands: std::collections::HashMap<&str, &clap::Command> =
        cmd.get_subcommands().map(|sc| (sc.get_name(), sc)).collect();

    let mut out = String::new();
    if let Some(about) = cmd.get_about() {
        let _ = writeln!(out, "{about}");
        out.push('\n');
    }
    let _ = writeln!(out, "Usage: {} [OPTIONS] <COMMAND>", cmd.get_name());
    out.push('\n');

    for (heading, names) in COMMAND_CATEGORIES {
        let visible: Vec<(&str, &clap::Command)> = names
            .iter()
            .filter_map(|name| subcommands.get(name).map(|sc| (*name, *sc)))
            .filter(|(_, sc)| !sc.is_hide_set())
            .collect();
        if visible.is_empty() {
            continue;
        }
        let _ = writeln!(out, "{heading}:");
        for (name, sc) in visible {
            let about = sc.get_about().unwrap_or_default();
            let _ = writeln!(out, "  {name:<16} {about}");
        }
        out.push('\n');
    }

    out.push_str("Options:\n");
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
        let _ = writeln!(out, "  {spec:<width$} {help}");
    }
    out
}

/// Print the top-level help with subcommands grouped by category.
pub fn print_grouped_help() {
    print!("{}", build_grouped_help());
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

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

    /// Extract the subcommand names rendered in the grouped help body (before
    /// the `Options:` block), one `  name ...` line at a time.
    fn grouped_help_command_names() -> HashSet<String> {
        let help = build_grouped_help();
        let body = help.split("Options:").next().unwrap_or(&help);
        body.lines()
            .filter_map(|line| line.strip_prefix("  "))
            .map(|line| line.split_whitespace().next().unwrap_or_default().to_owned())
            .collect()
    }

    #[test]
    fn grouped_help_lists_the_whole_surface() {
        let surface: HashSet<&str> = [
            "start",
            "stop",
            "status",
            "reload",
            "sync",
            "devices",
            "folders",
            "ls",
            "stat",
            "find",
            "search",
            "sort",
            "download",
            "verify",
            "transfer",
            "share",
            "access",
            "link",
            "package",
            "network",
            "watch",
            "snapshot",
            "indexing",
            "stats",
            "db",
            "config",
            "version",
            "completions",
            "manpages",
            "help",
        ]
        .into_iter()
        .collect();
        let visible = grouped_help_command_names();
        assert_eq!(
            visible,
            surface.iter().map(|s| (*s).to_owned()).collect::<HashSet<_>>(),
            "grouped help should show exactly the major-release surface"
        );
    }

    #[test]
    fn grouped_help_omits_rehomed_verbs() {
        let visible = grouped_help_command_names();
        for removed in [
            "create", "join", "leave", "import", "networks", "publish", "unshare", "provider",
        ] {
            assert!(!visible.contains(removed), "re-homed verb '{removed}' should be gone");
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

    #[arg(
        long,
        global = true,
        help = "Emit machine-readable JSON. Each command prints a single JSON object (arrays only inside a named key); streaming commands (stats network --follow) print one JSON object per line (NDJSON)"
    )]
    pub json: bool,

    #[arg(long, global = true, help = "Assume yes to every destructive-operation prompt")]
    pub yes: bool,

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
