use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use comfy_table::Table;
use dialoguer::Confirm;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Install the JSON logger. When `log_file` is set, records are additionally
/// appended to that exact path, which is what `start --log-file` promises.
/// The returned guard flushes the non-blocking writer and must be held for the
/// lifetime of the process.
///
/// # Errors
///
/// Returns an error if the log file cannot be opened or the global subscriber
/// is already installed.
pub fn init_tracing_with_log_file(verbose: bool, log_file: Option<&Path>) -> Result<Option<WorkerGuard>> {
    let default_filter = if verbose { "syncweb=debug" } else { "syncweb=info" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    let stderr_layer = fmt::layer().json().with_writer(std::io::stderr);
    if let Some(path) = log_file {
        let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty());
        let directory = parent.unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().unwrap_or_else(|| OsStr::new("syncweb.log"));
        std::fs::create_dir_all(directory)
            .map_err(|err| anyhow!("failed to create log directory {}: {err}", directory.display()))?;
        // The appender always takes (directory, file name). Splitting the
        // requested path keeps the name exactly as asked for: passing the whole
        // path as the directory would create `<path>/<name>.YYYY-MM-DD` with
        // rotation, or a stray directory without it.
        let appender = tracing_appender::rolling::never(directory, file_name);
        let (writer, guard) = tracing_appender::non_blocking(appender);
        tracing_subscriber::registry()
            .with(filter)
            .with(stderr_layer)
            .with(fmt::layer().json().with_writer(writer))
            .try_init()
            .map_err(|err| anyhow!("failed to initialize structured logging: {err}"))?;
        Ok(Some(guard))
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(stderr_layer)
            .try_init()
            .map_err(|err| anyhow!("failed to initialize structured logging: {err}"))?;
        Ok(None)
    }
}

pub fn print_version() {
    println!("syncweb {}", env!("CARGO_PKG_VERSION"));
}

/// Serialize a list of rows under a single named key.
///
/// `--json` promises one JSON object per command with arrays only inside a
/// named key, so list-shaped output is wrapped instead of printed bare.
pub fn json_list<T: serde::Serialize>(key: &str, rows: T) -> Result<String> {
    let mut map = serde_json::Map::with_capacity(1);
    map.insert(key.to_owned(), serde_json::to_value(rows)?);
    Ok(serde_json::to_string_pretty(&serde_json::Value::Object(map))?)
}

/// Require interactive confirmation for destructive operations. Auto-approves
/// only when the caller passes `--yes`, and aborts the operation when stdin is
/// not interactive, so non-interactive automation cannot run a destructive
/// command silently (not even with `--json`). When `no_color` is set, the
/// prompt is rendered without ANSI styling.
pub fn confirm_destructive(operation: &str, assume_yes: bool, no_color: bool) -> Result<bool> {
    if assume_yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Ok(false);
    }
    let prompt = format!("Are you sure you want to {operation}?");
    let interact = if no_color {
        Confirm::with_theme(&dialoguer::theme::SimpleTheme)
            .with_prompt(prompt)
            .default(false)
            .show_default(true)
            .interact()?
    } else {
        Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .show_default(true)
            .interact()?
    };
    Ok(interact)
}

/// Render the shared post-command guidance block.
///
/// `folders create`, `folders join`, `share`, and `package install` end with
/// the same compact shape so the core setup flow reads identically everywhere.
/// Each step states plainly whether content moved eagerly or stays lazy.
pub fn print_next_steps(steps: &[String]) {
    if steps.is_empty() {
        return;
    }
    println!();
    println!("next steps:");
    for step in steps {
        println!("  - {step}");
    }
}

/// Attach the shared next-steps list to a `--json` object envelope.
///
/// The field is additive so existing script consumers keep working.
#[must_use]
pub fn with_next_steps(mut value: serde_json::Value, steps: &[String]) -> serde_json::Value {
    if steps.is_empty() {
        return value;
    }
    if let serde_json::Value::Object(map) = &mut value {
        map.insert(
            "next_steps".to_owned(),
            serde_json::Value::Array(steps.iter().cloned().map(serde_json::Value::String).collect()),
        );
    }
    value
}

/// How a `folders join` handled content, for the shared next-steps wording.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JoinFetch {
    /// `--metadata-only`: entries tracked, no blob bytes fetched.
    MetadataOnly,
    /// `--download-existing`: bulk fetch ran; counts when known.
    Eager(Option<(usize, u64)>),
    /// Default: nothing fetched; content stays lazy until `download`.
    Lazy,
}

/// Shared next steps after `folders create`.
pub fn create_next_steps(shared: bool) -> Vec<String> {
    let mut steps = Vec::new();
    if shared {
        steps.push("send the link above to a device you trust; there it runs `syncweb folders join <link>`".to_owned());
    } else {
        steps.push("no link yet: `syncweb share <path>` mints a join link for other devices".to_owned());
    }
    steps.push(
        "content is local-first: files here are imported on this device now; peers fetch them on demand".to_owned(),
    );
    steps.push(
        "keep a daemon running (`syncweb start`) so changes sync automatically; `syncweb status` shows health"
            .to_owned(),
    );
    steps
}

/// Shared next steps after `folders join`, explaining the fetch model.
pub fn join_next_steps(subscribe: bool, fetch: JoinFetch) -> Vec<String> {
    let mut steps = Vec::new();
    match fetch {
        JoinFetch::MetadataOnly => steps.push(
            "metadata-only join: no file content was fetched (lazy) — `syncweb download <path>` pulls files on demand"
                .to_owned(),
        ),
        JoinFetch::Eager(Some((files, bytes))) => steps.push(format!(
            "content was fetched eagerly: {files} files ({bytes} bytes); later changes follow live sync or `syncweb sync`"
        )),
        JoinFetch::Eager(None) => steps.push(
            "content was fetched eagerly on join; later changes follow live sync or `syncweb sync`".to_owned(),
        ),
        JoinFetch::Lazy => steps.push(
            "content stays lazy: nothing was downloaded — `syncweb download <path>` fetches files on demand"
                .to_owned(),
        ),
    }
    if subscribe {
        steps.push("live sync is on: this folder stays up to date while a daemon runs".to_owned());
    } else {
        steps.push("live sync is off: `syncweb folders join <folder> --subscribe` enables it".to_owned());
    }
    steps
}

/// Shared next steps after creating a named-network invitation.
pub fn network_invite_next_steps(name: &str) -> Vec<String> {
    vec![
        format!(
            "this invitation grants membership in the named network `{name}`; it is not a folder ticket, package ticket, or offline community enrollment"
        ),
        "send the ticket to the recipient; they join with `syncweb network join <ticket>`".to_owned(),
        "share folders separately with `syncweb share <path>`; install packages with `syncweb package install <ticket>`"
            .to_owned(),
    ]
}

/// Shared next steps after joining a named network.
pub fn network_join_next_steps() -> Vec<String> {
    vec![
        "network membership is the peer-access boundary; it does not create or join a folder".to_owned(),
        "join folders with `syncweb folders join <folder-ticket>` or install packages with `syncweb package install <ticket>`"
            .to_owned(),
    ]
}

/// Shared next steps after `share`.
pub fn share_next_steps(write: bool, pinned: bool) -> Vec<String> {
    let mut steps = Vec::new();
    if write {
        steps.push("WRITE link: anyone holding it can change this folder — treat it like a password".to_owned());
    } else {
        steps.push("read-only link: receivers see changes but cannot edit this folder".to_owned());
    }
    if pinned {
        steps.push("content is pinned here and seeded eagerly while the daemon runs".to_owned());
    } else {
        steps.push("content is not pinned: only files this device already has are served (lazy)".to_owned());
    }
    steps.push("revoke the link later with `syncweb access --revoke <folder>`".to_owned());
    steps
}

/// Shared next steps after `package install`.
pub fn package_install_next_steps(collection: &str, version: &str) -> Vec<String> {
    vec![
        format!(
            "content is eager: {collection} {version} was fetched, verified, and materialized under the install root"
        ),
        format!("re-check integrity with `syncweb package verify {collection}`"),
        "new versions install with `syncweb package upgrade <ticket>`; `syncweb package list` shows what is installed"
            .to_owned(),
    ]
}

/// Maximum number of share rows rendered per folder before the list is
/// summarized with an "and N more" line. `access --full` lifts the cap.
pub const ACCESS_SHARE_ROW_CAP: usize = 12;

/// One folder's aggregated access state for `syncweb access`.
#[derive(Default)]
pub struct AccessRow {
    pub path: Option<PathBuf>,
    pub mode: String,
    pub shares: Vec<(String, String)>,
    pub networks: Vec<String>,
    /// Inbound peers (who joined) for the folder; empty when not surfaced.
    pub devices: Vec<String>,
}

/// Render a stored share ticket as a `syncweb://folder/...` URL, falling back
/// to the raw ticket when it cannot be parsed.
#[must_use]
pub fn folder_share_url(namespace: &str, ticket: &str) -> String {
    match (
        namespace.parse::<iroh_docs::NamespaceId>(),
        syncweb_core::uri::parse_folder_ticket(ticket),
    ) {
        (Ok(namespace_id), Ok(doc_ticket)) => syncweb_core::uri::folder_url(namespace_id, &doc_ticket),
        _ => ticket.to_owned(),
    }
}

fn render_access_shares(namespace: &str, shares: &[(String, String)], full: bool) -> String {
    if shares.is_empty() {
        return "-".to_owned();
    }
    let cap = if full { shares.len() } else { ACCESS_SHARE_ROW_CAP };
    let mut lines = shares
        .iter()
        .take(cap)
        .map(|(access, ticket)| format!("{access}: {}", folder_share_url(namespace, ticket)))
        .collect::<Vec<_>>();
    let shown = lines.len();
    if shares.len() > shown {
        lines.push(format!("and {} more", shares.len().saturating_sub(shown)));
    }
    lines.join("\n")
}

/// Render the `access` table: one row per folder with its mode, whether any
/// share grants write, the outbound share tickets, and network membership.
#[must_use]
pub fn render_access_table(rows: &BTreeMap<String, AccessRow>, full: bool) -> String {
    let mut table = Table::new();
    table.set_header(["Folder", "Mode", "Write?", "Shared with", "Devices", "Networks"]);
    for (namespace, row) in rows {
        let folder_label = row
            .path
            .as_ref()
            .map_or_else(|| namespace.clone(), |path| path.display().to_string());
        let mode = if row.mode.is_empty() {
            "-".to_owned()
        } else {
            row.mode.clone()
        };
        let write = if row.shares.iter().any(|(access, _)| access == "write") {
            "yes".to_owned()
        } else {
            "no".to_owned()
        };
        let shared = render_access_shares(namespace, &row.shares, full);
        let devices = if row.devices.is_empty() {
            "-".to_owned()
        } else {
            row.devices.join(", ")
        };
        let networks = if row.networks.is_empty() {
            "-".to_owned()
        } else {
            row.networks.join(", ")
        };
        table.add_row([folder_label, mode, write, shared, devices, networks]);
    }
    table.to_string()
}

/// Render the `access --json` list. `pinned` is intentionally omitted: pin
/// status needs a live local node and guessing `false` would mislead.
#[must_use]
pub fn render_access_json(rows: &BTreeMap<String, AccessRow>) -> String {
    let values = rows
        .iter()
        .map(|(namespace, row)| {
            let shares = row
                .shares
                .iter()
                .map(|(access, ticket)| {
                    serde_json::json!({
                        "access": access,
                        "url": folder_share_url(namespace, ticket),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "folder": namespace,
                "mode": row.mode,
                "write": row.shares.iter().any(|(access, _)| access == "write"),
                "shares": shares,
                "devices": row.devices,
                "networks": row.networks,
            })
        })
        .collect::<Vec<_>>();
    json_list("folders", values).unwrap_or_else(|_| "{\"folders\":[]}".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rows() -> BTreeMap<String, AccessRow> {
        let mut rows = BTreeMap::new();
        rows.insert(
            "namespace-one".to_owned(),
            AccessRow {
                path: Some(PathBuf::from("/tmp/documents")),
                mode: "sendreceive".to_owned(),
                shares: vec![
                    ("read".to_owned(), "read-ticket".to_owned()),
                    ("write".to_owned(), "write-ticket".to_owned()),
                ],
                networks: vec!["home".to_owned()],
                devices: vec!["ABCD-EFGH".to_owned()],
            },
        );
        rows
    }

    #[test]
    fn access_table_includes_mode_and_write_markers() {
        let table = render_access_table(&sample_rows(), false);
        assert!(table.contains("Mode"), "table should have a Mode column: {table}");
        assert!(table.contains("Write?"), "table should have a Write? column: {table}");
        assert!(table.contains("sendreceive"), "table should show the mode: {table}");
        assert!(table.contains("home"), "table should show network membership: {table}");
        assert!(table.contains("ABCD-EFGH"), "table should show inbound peers: {table}");
    }

    #[test]
    fn access_json_includes_mode_and_write_markers() {
        let json = render_access_json(&sample_rows());
        assert!(json.contains("\"mode\""), "json should carry mode: {json}");
        assert!(json.contains("\"write\""), "json should carry write: {json}");
        assert!(
            json.contains("\"sendreceive\""),
            "json should carry the mode value: {json}"
        );
        assert!(json.contains("\"devices\""), "json should carry devices: {json}");
        assert!(json.contains("ABCD-EFGH"), "json should carry the inbound peer: {json}");
        assert!(!json.contains("pinned"), "json must not guess pin status: {json}");
    }

    #[test]
    fn next_steps_json_adds_additive_field() {
        let value = with_next_steps(serde_json::json!({"status": "ok"}), &["step one".to_owned()]);
        assert_eq!(value.get("status"), Some(&serde_json::Value::from("ok")));
        let steps = value
            .get("next_steps")
            .and_then(serde_json::Value::as_array)
            .expect("next_steps should be an array");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps.first(), Some(&serde_json::Value::from("step one")));
    }

    #[test]
    fn next_steps_json_leaves_other_shapes_untouched() {
        let value = with_next_steps(serde_json::json!([1, 2]), &["step".to_owned()]);
        assert_eq!(value, serde_json::json!([1, 2]), "non-object envelopes stay unchanged");
        let empty = with_next_steps(serde_json::json!({"status": "ok"}), &[]);
        assert_eq!(empty.get("next_steps"), None, "no empty next_steps field");
    }

    #[test]
    fn create_next_steps_distinguish_shared_and_unshared() {
        let shared = create_next_steps(true);
        assert!(
            shared.iter().any(|step| step.contains("join <link>")),
            "shared create should point at the join link: {shared:?}"
        );
        let unshared = create_next_steps(false);
        assert!(
            unshared.iter().any(|step| step.contains("syncweb share")),
            "unshared create should point at share: {unshared:?}"
        );
        assert!(
            shared.iter().any(|step| step.contains("on demand")),
            "create should explain the fetch model: {shared:?}"
        );
    }

    #[test]
    fn join_next_steps_state_eager_vs_lazy() {
        let lazy = join_next_steps(false, JoinFetch::Lazy);
        assert!(
            lazy.iter()
                .any(|step| step.contains("lazy") && step.contains("syncweb download")),
            "lazy join should explain on-demand fetch: {lazy:?}"
        );
        assert!(
            lazy.iter().any(|step| step.contains("live sync is off")),
            "bare join should point at --subscribe: {lazy:?}"
        );
        let eager = join_next_steps(true, JoinFetch::Eager(Some((3, 42))));
        assert!(
            eager
                .iter()
                .any(|step| step.contains("eagerly") && step.contains("3 files")),
            "eager join should report what was fetched: {eager:?}"
        );
        assert!(
            eager.iter().any(|step| step.contains("live sync is on")),
            "--subscribe join should say live sync is on: {eager:?}"
        );
        let metadata = join_next_steps(false, JoinFetch::MetadataOnly);
        assert!(
            metadata.iter().any(|step| step.contains("metadata-only")),
            "metadata-only join should say so: {metadata:?}"
        );
    }

    #[test]
    fn share_next_steps_flag_write_and_pin_state() {
        let read_pinned = share_next_steps(false, true);
        assert!(read_pinned.iter().any(|step| step.contains("read-only")));
        assert!(
            read_pinned
                .iter()
                .any(|step| step.contains("pinned") && step.contains("eager"))
        );
        let write_unpinned = share_next_steps(true, false);
        assert!(write_unpinned.iter().any(|step| step.contains("WRITE")));
        assert!(write_unpinned.iter().any(|step| step.contains("not pinned")));
        assert!(
            read_pinned.iter().any(|step| step.contains("access --revoke")),
            "share should explain how to revoke: {read_pinned:?}"
        );
    }

    #[test]
    fn package_install_next_steps_state_eager_materialization() {
        let steps = package_install_next_steps("abc", "1.0.0");
        assert!(
            steps
                .iter()
                .any(|step| step.contains("eager") && step.contains("1.0.0")),
            "install should state eager materialization: {steps:?}"
        );
        assert!(
            steps.iter().any(|step| step.contains("package verify")),
            "install should point at verify: {steps:?}"
        );
    }

    #[test]
    fn network_next_steps_distinguish_membership_from_content() {
        let invite = network_invite_next_steps("team");
        assert!(
            invite
                .iter()
                .any(|step| step.contains("not a folder ticket, package ticket")),
            "network invite should explain ticket scope: {invite:?}"
        );
        assert!(
            invite.iter().any(|step| step.contains("network join <ticket>")),
            "network invite should explain how to provision the recipient: {invite:?}"
        );
        let join = network_join_next_steps();
        assert!(
            join.iter().any(|step| step.contains("folders join")),
            "network join should point at folder provisioning: {join:?}"
        );
    }
}
