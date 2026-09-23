use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use comfy_table::Table;
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
    table.set_header(["Folder", "Mode", "Write?", "Shared with", "Networks"]);
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
        let networks = if row.networks.is_empty() {
            "-".to_owned()
        } else {
            row.networks.join(", ")
        };
        table.add_row([folder_label, mode, write, shared, networks]);
    }
    table.to_string()
}

/// Render the `access --json` array. `pinned` is intentionally omitted: pin
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
                "networks": row.networks,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&values).unwrap_or_else(|_| "[]".to_owned())
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
        assert!(!json.contains("pinned"), "json must not guess pin status: {json}");
    }
}
