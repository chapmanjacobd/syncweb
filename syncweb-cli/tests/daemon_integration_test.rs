#![cfg(unix)]

use anyhow::{Context, ensure};
use std::process::Command;

fn cli_test_dir(name: &str) -> anyhow::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("syncweb-daemon-test-{name}-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).context("create test dir")?;
    Ok(dir)
}

fn syncweb(args: &[&str]) -> anyhow::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(args)
        .output()
        .context("run syncweb")
}

/// Parse a folder URL out of a share command's human output. Write shares now
/// carry a `WRITE ticket:` prefix (user story 2), so strip it when present.
fn ticket_from_share_output(out: &str) -> String {
    let trimmed = out.trim();
    trimmed.strip_prefix("WRITE ticket: ").unwrap_or(trimmed).to_owned()
}

fn stdout_contains(output: &std::process::Output, needle: &str) -> bool {
    String::from_utf8(output.stdout.clone()).is_ok_and(|s| s.contains(needle))
}

fn daemon_start_bg(data_dir_arg: &str) -> anyhow::Result<std::process::Output> {
    syncweb(&["--data-dir", data_dir_arg, "start", "--bg", "--no-relay"])
}

fn wait_for_daemon_ready(data_dir_arg: &str) -> anyhow::Result<()> {
    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut last_diagnostic = String::new();

    for _ in 0..150 {
        let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon: running") {
            return Ok(());
        }
        if !status.stderr.is_empty() {
            last_diagnostic = String::from_utf8_lossy(&status.stderr).to_string();
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    anyhow::bail!("timed out waiting for daemon to become ready. last stderr: {last_diagnostic}");
}

#[test]
fn test_help_mentions_daemon_commands() -> anyhow::Result<()> {
    let output = syncweb(&["--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("daemon"));
    ensure!(help.contains("start"));
    Ok(())
}

#[test]
fn test_no_daemon_flag_is_listed_in_help() -> anyhow::Result<()> {
    let output = syncweb(&["--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon"));
    Ok(())
}

#[test]
fn test_embedded_flag_works_without_daemon() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("embedded-flag")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let output = syncweb(&["--data-dir", data_dir_arg, "--no-daemon", "version"])?;
    ensure!(output.status.success());
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_no_daemon_create_routes_embedded() -> anyhow::Result<()> {
    let dir = cli_test_dir("no-daemon-create")?;
    let data_dir = cli_test_dir("no-daemon-create-data")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let output = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "--no-daemon",
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("syncweb://"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_start_and_stop() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-lifecycle")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");

    let mut daemon_ready = false;
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_secs_f64(0.25));
        let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon: running") {
            daemon_ready = true;
            break;
        }
    }
    ensure!(daemon_ready, "daemon should be running after start");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success(), "daemon shutdown should succeed");

    let mut daemon_stopped = false;
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_secs_f64(0.25));
        let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon not running") {
            daemon_stopped = true;
            break;
        }
    }
    ensure!(daemon_stopped, "daemon should be stopped after shutdown");

    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_create_routes_through_daemon() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-create")?;
    let dir = cli_test_dir("daemon-create-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_no_daemon_fails_fast_when_daemon_owns_store() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("no-daemon-guard")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(data_dir_arg)?;

    // Embedded commands must fail fast instead of blocking on the store locks
    // the daemon holds. `access` previously hung indefinitely.
    for command in [vec!["access"], vec!["folders"], vec!["ls", "."]] {
        let mut args = vec!["--data-dir", data_dir_arg, "--no-daemon"];
        args.extend(command);
        let output = syncweb(&args)?;
        ensure!(
            !output.status.success(),
            "embedded command should fail while a daemon owns the store"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure!(
            stderr.contains("a daemon owns"),
            "expected the daemon-owns error, got: {stderr}"
        );
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_folders_create_is_idempotent_per_path() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("create-idempotent")?;
    let dir = cli_test_dir("create-idempotent-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let dir_arg = dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(data_dir_arg)?;

    let first = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        "--no-indexing",
        dir_arg,
    ])?;
    ensure!(first.status.success(), "first create should succeed");
    let second = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        "--no-indexing",
        dir_arg,
    ])?;
    ensure!(second.status.success(), "second create should succeed");

    let folders = syncweb(&["--data-dir", data_dir_arg, "--json", "folders"])?;
    ensure!(folders.status.success());
    let value: serde_json::Value = serde_json::from_slice(&folders.stdout).context("parse folders json")?;
    let list = value
        .get("folders")
        .and_then(serde_json::Value::as_array)
        .context("folders array")?;
    ensure!(
        list.len() == 1,
        "repeated create for one path must not register duplicate namespaces, got {}",
        list.len()
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_embedded_folders_create_is_idempotent_per_path() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("embedded-create-idempotent")?;
    let dir = cli_test_dir("embedded-create-idempotent-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let dir_arg = dir.to_str().context("UTF-8 path")?;

    for _ in 0..2 {
        let output = syncweb(&[
            "--data-dir",
            data_dir_arg,
            "--no-daemon",
            "folders",
            "create",
            "--no-indexing",
            dir_arg,
        ])?;
        ensure!(output.status.success(), "embedded create should succeed");
    }

    let folders = syncweb(&["--data-dir", data_dir_arg, "--no-daemon", "--json", "folders"])?;
    ensure!(folders.status.success());
    let value: serde_json::Value = serde_json::from_slice(&folders.stdout).context("parse folders json")?;
    let list = value
        .get("folders")
        .and_then(serde_json::Value::as_array)
        .context("folders array")?;
    ensure!(
        list.len() == 1,
        "repeated embedded create for one path must not register duplicates, got {}",
        list.len()
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_shutdown() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-shutdown")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes"])?;
    ensure!(shutdown.status.success(), "shutdown should succeed");

    let mut stopped = false;
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_secs_f64(0.25));
        let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon not running") {
            stopped = true;
            break;
        }
    }
    ensure!(stopped, "daemon should be stopped after shutdown");
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_embedded_alias_is_listed_in_help() -> anyhow::Result<()> {
    let output = syncweb(&["--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("embedded") || help.contains("--no-daemon"));
    Ok(())
}

#[test]
fn test_embedded_flag_alias_works() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("embedded-alias")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let output = syncweb(&["--data-dir", data_dir_arg, "--embedded", "version"])?;
    ensure!(output.status.success());
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_reload_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-reload")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let reload = syncweb(&["--data-dir", data_dir_arg, "reload"])?;
    ensure!(reload.status.success(), "reload should succeed");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_sync_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("sync")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let sync = syncweb(&["--data-dir", data_dir_arg, "sync"])?;
    ensure!(sync.status.success(), "daemon-sync should succeed");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_folders_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-folders")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let folders = syncweb(&["--data-dir", data_dir_arg, "folders"])?;
    ensure!(folders.status.success(), "folders should succeed via daemon");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_create_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-create")?;
    let dir = cli_test_dir("daemon-create-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "create should succeed via daemon");
    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("syncweb://"));

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_health_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-health")?;
    let dir = cli_test_dir("daemon-health-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);

    if let Some(ns) = namespace {
        let files = syncweb(&["--data-dir", data_dir_arg, "stats", "files", ns])?;
        ensure!(files.status.success(), "stats files should succeed via daemon");
        let output = String::from_utf8(files.stdout).context("UTF-8 output")?;
        ensure!(
            output.contains("total_files:"),
            "stats files output should include total_files"
        );
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_multiple_ipc_commands() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-multi-ipc")?;
    let dir = cli_test_dir("daemon-multi-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let folders = syncweb(&["--data-dir", data_dir_arg, "folders"])?;
    ensure!(folders.status.success());

    let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
    ensure!(status.status.success());

    let reload = syncweb(&["--data-dir", data_dir_arg, "reload"])?;
    ensure!(reload.status.success());

    let sync = syncweb(&["--data-dir", data_dir_arg, "sync"])?;
    ensure!(sync.status.success());

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_cli_default_is_daemon_mode() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-default")?;
    let dir = cli_test_dir("daemon-default-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let folders = syncweb(&["--data-dir", data_dir_arg, "folders"])?;
    ensure!(folders.status.success());
    let stdout = String::from_utf8(folders.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("Namespace") || stdout.contains("namespace"));

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_subscribe_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-subscribe")?;
    let dir = cli_test_dir("daemon-subscribe-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);

    if let Some(ns) = namespace {
        let subscribe = syncweb(&[
            "--data-dir",
            data_dir_arg,
            "folders",
            "join",
            "--subscribe",
            "--ingest-only",
            ns,
        ])?;
        ensure!(subscribe.status.success(), "join --subscribe should succeed via daemon");
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_publish_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-publish")?;
    let dir = cli_test_dir("daemon-publish-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);

    if let Some(ns) = namespace {
        let publish = syncweb(&["--data-dir", data_dir_arg, "share", ns])?;
        ensure!(publish.status.success(), "share should succeed via daemon");
        let pub_stdout = String::from_utf8(publish.stdout).context("UTF-8 output")?;
        ensure!(
            pub_stdout.contains("syncweb://"),
            "share should emit a URL: {pub_stdout}"
        );
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_leave_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-leave")?;
    let dir = cli_test_dir("daemon-leave-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);

    if let Some(ns) = namespace {
        let leave = syncweb(&["--data-dir", data_dir_arg, "folders", "leave", ns])?;
        ensure!(leave.status.success(), "leave should succeed via daemon");
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_leave_delete_files_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-leave-delete-files")?;
    let dir = cli_test_dir("daemon-leave-delete-files-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);
    ensure!(namespace.is_some(), "create should output a namespace");
    let ns = namespace.unwrap();

    std::fs::write(dir.join("file.txt"), b"content")?;
    ensure!(dir.exists(), "folder directory should exist before leave");

    let leave = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "leave",
        "--delete-files",
        "--yes",
        ns,
    ])?;
    ensure!(
        leave.status.success(),
        "leave --delete-files should succeed, got: {}",
        String::from_utf8_lossy(&leave.stderr)
    );

    ensure!(
        !dir.exists(),
        "folder directory should be deleted after leave --delete-files"
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_verify_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-verify")?;
    let dir = cli_test_dir("daemon-verify-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);

    if let Some(ns) = namespace {
        let verify = syncweb(&["--data-dir", data_dir_arg, "verify", ns])?;
        ensure!(verify.status.success(), "verify should succeed via daemon");
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_snapshot_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-snapshot")?;
    let dir = cli_test_dir("daemon-snapshot-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);

    if let Some(ns) = namespace {
        let snapshot_list = syncweb(&["--data-dir", data_dir_arg, "snapshot", "list", ns])?;
        ensure!(
            snapshot_list.status.success(),
            "snapshot list should succeed via daemon"
        );
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_help_mentions_daemon_mode() -> anyhow::Result<()> {
    let output = syncweb(&["--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("daemon") || help.contains("Daemon"));
    Ok(())
}

#[test]
fn test_create_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["folders", "create", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_verify_help_lists_selector_arg() -> anyhow::Result<()> {
    let output = syncweb(&["verify", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("PATH") || help.contains("path") || help.contains("folder"));
    Ok(())
}

#[test]
fn test_folders_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["folders", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_download_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["download", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_subscribe_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["folders", "join", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_publish_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["indexing", "publish", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_import_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["folders", "import", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_daemon_leave_untracks_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-leave-untracks")?;
    let dir = cli_test_dir("daemon-leave-untracks-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim);
    ensure!(namespace.is_some(), "create should output a namespace");
    let ns = namespace.unwrap();

    let sync = syncweb(&["--data-dir", data_dir_arg, "sync"])?;
    ensure!(sync.status.success(), "triggering daemon-sync should succeed");

    let leave = syncweb(&["--data-dir", data_dir_arg, "folders", "leave", ns])?;
    ensure!(
        leave.status.success(),
        "leave via namespace ID should succeed, got: {}",
        String::from_utf8_lossy(&leave.stderr)
    );

    let folders = syncweb(&["--data-dir", data_dir_arg, "folders"])?;
    ensure!(folders.status.success());
    let folder_stdout = String::from_utf8(folders.stdout).context("UTF-8 output")?;
    ensure!(
        !folder_stdout.contains(ns),
        "left namespace should not appear in folder list"
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_two_instances_cannot_start() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-dual-start")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let second_start = syncweb(&["--data-dir", data_dir_arg, "start"])?;
    ensure!(
        !second_start.status.success(),
        "second syncweb start without --bg should fail when daemon is already running"
    );
    let stderr = String::from_utf8_lossy(&second_start.stderr);
    ensure!(
        stderr.contains("already running") || stderr.contains("daemon"),
        "second start should report daemon already running, got: {stderr}"
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_cli_no_daemon_flag_bypasses_daemon() -> anyhow::Result<()> {
    let dir = cli_test_dir("no-daemon-bypass")?;
    let data_dir = cli_test_dir("no-daemon-bypass-data")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let output = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "--no-daemon",
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        output.status.success(),
        "embedded create with --no-daemon should succeed without daemon running"
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("syncweb://"));

    let folders = syncweb(&["--data-dir", data_dir_arg, "status"])?;
    let status_stdout = String::from_utf8(folders.stdout).context("UTF-8 output")?;
    ensure!(
        !status_stdout.contains("daemon: running"),
        "no daemon should be running after embedded create"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_global_network_flag_scopes_data_dir() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("network-scoped")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "--network",
        "home",
        "start",
        "--bg",
        "--no-relay",
    ])?;
    ensure!(start.status.success(), "daemon start with --network should succeed");

    let mut daemon_ready = false;
    for _ in 0..150 {
        std::thread::sleep(std::time::Duration::from_millis(400));
        let status = syncweb(&["--data-dir", data_dir_arg, "--network", "home", "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon: running") {
            daemon_ready = true;
            break;
        }
    }
    ensure!(daemon_ready, "daemon should be running after start with --network");

    let network_subdir = data_dir.join("home");
    ensure!(
        network_subdir.exists(),
        "--network home should create a home/ subdirectory under data_dir"
    );

    let shutdown = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "--network",
        "home",
        "stop",
        "--yes",
        "--force",
    ])?;
    ensure!(shutdown.status.success(), "shutdown should succeed");
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_start_with_log_file_writes_log() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("log-file")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let log_file = data_dir.join("daemon.log");
    let log_file_arg = log_file.to_str().context("UTF-8 path")?;

    let start = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "start",
        "--bg",
        "--no-relay",
        "--log-file",
        log_file_arg,
    ])?;
    ensure!(start.status.success(), "daemon start with --log-file should succeed");

    wait_for_daemon_ready(data_dir_arg)?;

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success(), "shutdown should succeed");
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));

    ensure!(
        log_file.is_file(),
        "--log-file should create the exact file it was given: {}",
        log_file.display()
    );
    let contents = std::fs::read_to_string(&log_file).context("read the daemon log file")?;
    ensure!(
        contents.contains("daemon"),
        "the log file should carry daemon records, got: {contents}"
    );
    ensure!(
        contents.lines().all(|line| line.trim_start().starts_with('{')),
        "the log file should be JSON lines: {contents}"
    );

    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_start_bg_reports_a_daemon_that_cannot_start() -> anyhow::Result<()> {
    // A broken `filters.toml` kills the daemon during startup. `--bg` sends the
    // child's stderr to /dev/null, so the CLI has to notice the early exit
    // itself instead of printing a success message for a dead process.
    let data_dir = cli_test_dir("bg-start-failure")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    // `--data-dir` is the base; the daemon's own store lives in `default/`.
    let store_dir = data_dir.join("default");
    std::fs::create_dir_all(&store_dir)?;
    std::fs::write(store_dir.join("filters.toml"), "this is not valid toml [[[")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg", "--no-relay"])?;
    let stderr = String::from_utf8_lossy(&start.stderr).into_owned();
    ensure!(
        !start.status.success(),
        "--bg should fail when the daemon cannot start, stdout: {}",
        String::from_utf8_lossy(&start.stdout)
    );
    ensure!(
        stderr.contains("daemon exited before becoming ready"),
        "the error should say the daemon died, got: {stderr}"
    );

    let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
    ensure!(
        !stdout_contains(&status, "daemon: running"),
        "no daemon should be left behind: {}",
        String::from_utf8_lossy(&status.stdout)
    );

    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_start_bg_is_idempotent_when_a_daemon_is_already_running() -> anyhow::Result<()> {
    // The second `start --bg` must not spawn a daemon that dies on the store
    // lock and then report that dead pid as if it were running.
    let data_dir = cli_test_dir("bg-idempotent")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let log_file = data_dir.join("second.log");

    let first = syncweb(&["--data-dir", data_dir_arg, "start", "--bg", "--no-relay"])?;
    ensure!(first.status.success(), "the first start should succeed");
    wait_for_daemon_ready(data_dir_arg)?;
    let running_pid = first_pid(&syncweb(&["--data-dir", data_dir_arg, "status"])?);

    let second = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "start",
        "--bg",
        "--no-relay",
        "--log-file",
        log_file.to_str().context("UTF-8 log path")?,
    ])?;
    ensure!(
        second.status.success(),
        "starting an already-running daemon should succeed: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    ensure!(
        stdout_contains(&second, "already running"),
        "the second start should say the daemon was already up: {}",
        String::from_utf8_lossy(&second.stdout)
    );
    // The CLI creates the file when it installs the appender, but no daemon ran,
    // so it must stay empty.
    let log_contents = std::fs::read_to_string(&log_file).unwrap_or_default();
    ensure!(
        log_contents.is_empty(),
        "no second daemon should have written this log: {log_contents}"
    );

    ensure!(daemon_still_alive(
        &syncweb(&["--data-dir", data_dir_arg, "status"])?,
        running_pid
    ));

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success(), "shutdown should succeed");
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));

    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

/// Pull the `pid: <n>` line out of `syncweb status` output.
fn first_pid(status: &std::process::Output) -> u32 {
    String::from_utf8_lossy(&status.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("pid: "))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_default()
}

fn daemon_still_alive(status: &std::process::Output, expected: u32) -> bool {
    stdout_contains(status, "daemon: running") && first_pid(status) == expected
}

#[test]
fn test_start_media_only_exits() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("media-only")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg, "start", "--no-relay", "--media-only"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawn media-only process")?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
    let status_stdout = String::from_utf8(status.stdout).context("UTF-8 output")?;
    ensure!(
        !status_stdout.contains("daemon: running"),
        "--media-only should not leave a daemon running"
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_start_discovery_and_media_tuning_flags_accepted() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("tuning-flags")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "start",
        "--bg",
        "--no-relay",
        "--max-threads",
        "4",
        "--sync-interval",
        "120",
    ])?;
    ensure!(start.status.success(), "daemon start with tuning flags should succeed");

    wait_for_daemon_ready(data_dir_arg)?;

    let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
    ensure!(status.status.success(), "status should succeed");
    let stdout = String::from_utf8(status.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("daemon: running"), "daemon should be running");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_sync_scoped_to_namespace() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-sync-ns")?;
    let dir = cli_test_dir("daemon-sync-ns-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();

    let sync = syncweb(&["--data-dir", data_dir_arg, "sync", &namespace])?;
    ensure!(sync.status.success(), "daemon-sync --namespace should succeed");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_global_network_option_is_listed_in_help() -> anyhow::Result<()> {
    let output = syncweb(&["--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--network"), "--help should list the --network option");
    Ok(())
}

#[test]
fn test_join_download_materializes_content() -> anyhow::Result<()> {
    let alice_data = cli_test_dir("join-dl-alice")?;
    let alice_folder = cli_test_dir("join-dl-alice-folder")?;
    let bob_data = cli_test_dir("join-dl-bob")?;
    let bob_folder = cli_test_dir("join-dl-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon start should succeed");
    wait_for_daemon_ready(alice_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();
    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace, "--write"])?;
    ensure!(share.status.success(), "share should succeed");
    let share_out = String::from_utf8(share.stdout).context("UTF-8 output")?;
    let ticket = ticket_from_share_output(&share_out);
    ensure!(ticket.starts_with("syncweb://"), "share should output a URL: {ticket}");

    std::fs::write(alice_folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;
    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "--no-daemon",
        "folders",
        "join",
        "--download-existing",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        join.status.success(),
        "join --download-existing should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );
    let join_out = String::from_utf8(join.stdout).context("UTF-8 output")?;
    ensure!(
        join_out.contains("downloaded:"),
        "join should report a download count: {join_out}"
    );

    let content = std::fs::read_to_string(bob_folder.join("hello.txt")).context("read materialized file")?;
    ensure!(
        content == "hello world",
        "materialized content should match source, got: {content:?}"
    );

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&alice_folder);
    let _ = std::fs::remove_dir_all(&alice_data);
    let _ = std::fs::remove_dir_all(&bob_folder);
    let _ = std::fs::remove_dir_all(&bob_data);
    Ok(())
}

#[test]
fn test_join_default_does_not_subscribe_without_download() -> anyhow::Result<()> {
    let alice_data = cli_test_dir("join-default-alice")?;
    let alice_folder = cli_test_dir("join-default-alice-folder")?;
    let bob_data = cli_test_dir("join-default-bob")?;
    let bob_folder = cli_test_dir("join-default-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon start should succeed");
    wait_for_daemon_ready(alice_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();
    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace, "--write"])?;
    ensure!(share.status.success(), "share should succeed");
    let share_out = String::from_utf8(share.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();
    let ticket = ticket_from_share_output(&share_out);
    ensure!(ticket.starts_with("syncweb://"), "share should output a URL: {ticket}");

    std::fs::write(alice_folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;
    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "folders",
        "join",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        join.status.success(),
        "bob join should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );
    let join_out = String::from_utf8(join.stdout).context("UTF-8 output")?;
    ensure!(
        !join_out.contains("live sync on"),
        "bare join should not report live sync enabled: {join_out}"
    );
    ensure!(
        !join_out.contains("downloaded:"),
        "bare join should not report a bulk download by default: {join_out}"
    );
    ensure!(
        !bob_folder.join("hello.txt").exists(),
        "bare join should leave existing content off the disk until a download is requested"
    );

    let config = syncweb(&["--data-dir", bob_data_arg, "config", "show", "subscribe"])?;
    ensure!(config.status.success(), "config show subscribe should succeed");
    let config_out = String::from_utf8(config.stdout).context("UTF-8 output")?;
    ensure!(
        config_out.contains("enabled = false"),
        "bare join should persist live sync as disabled: {config_out}"
    );

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&alice_folder);
    let _ = std::fs::remove_dir_all(&alice_data);
    let _ = std::fs::remove_dir_all(&bob_folder);
    let _ = std::fs::remove_dir_all(&bob_data);
    Ok(())
}

#[test]
fn test_daemon_lists_remote_entries_before_download() -> anyhow::Result<()> {
    let alice_data = cli_test_dir("lazy-ls-alice")?;
    let alice_folder = cli_test_dir("lazy-ls-alice-folder")?;
    let bob_data = cli_test_dir("lazy-ls-bob")?;
    let bob_folder = cli_test_dir("lazy-ls-bob-folder")?;
    let plain = cli_test_dir("lazy-ls-plain")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;
    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon should start");
    wait_for_daemon_ready(alice_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        "--no-share",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let namespace = String::from_utf8(create.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();
    ensure!(!namespace.is_empty(), "create should print a namespace");

    std::fs::write(alice_folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace])?;
    ensure!(share.status.success(), "share should succeed");
    let ticket = String::from_utf8(share.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();
    ensure!(ticket.starts_with("syncweb://"), "share should output a URL: {ticket}");

    let bob_start = daemon_start_bg(bob_data_arg)?;
    ensure!(bob_start.status.success(), "bob daemon should start");
    wait_for_daemon_ready(bob_data_arg)?;

    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "folders",
        "join",
        "--subscribe",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        join.status.success(),
        "bob join --subscribe should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );

    // Bob's doc entries ingress while the mount directory stays empty: poll `ls`.
    let bob_folder_arg = bob_folder.to_str().context("UTF-8 path")?;
    let mut saw_entry = false;
    for _ in 0..60 {
        let ls = syncweb(&["--data-dir", bob_data_arg, "ls", bob_folder_arg])?;
        if ls.status.success() && String::from_utf8_lossy(&ls.stdout).contains("hello.txt") {
            saw_entry = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(saw_entry, "bob should list the entry before it is materialized to disk");

    let ls_json = syncweb(&["--json", "--data-dir", bob_data_arg, "ls", bob_folder_arg])?;
    ensure!(ls_json.status.success(), "bob ls --json should succeed");
    let value: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&ls_json.stdout))
        .context("bob ls --json should be valid JSON")?;
    ensure!(value.get("folder").is_some(), "envelope should carry folder: {value}");
    ensure!(value.get("path").is_some(), "envelope should carry path: {value}");
    let entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .context("entries array")?;
    let hello = entries
        .iter()
        .find(|entry| entry["path"] == "hello.txt")
        .context("hello.txt entry")?;
    ensure!(
        hello
            .get("hash")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|h| !h.is_empty()),
        "entry should carry a content hash: {hello:?}"
    );
    ensure!(
        hello.get("local").is_some() && hello.get("size").is_some(),
        "entry should carry local and size: {hello:?}"
    );

    let local = syncweb(&["--data-dir", bob_data_arg, "ls", "--local-only", bob_folder_arg])?;
    ensure!(local.status.success());
    ensure!(
        !String::from_utf8_lossy(&local.stdout).contains("hello.txt"),
        "--local-only should not list the still-unmaterialized entry: {}",
        String::from_utf8_lossy(&local.stdout)
    );

    let ls_plain = syncweb(&["--data-dir", bob_data_arg, "ls", plain.to_str().context("UTF-8 path")?])?;
    ensure!(!ls_plain.status.success(), "ls on a plain dir should error");
    ensure!(
        String::from_utf8_lossy(&ls_plain.stderr).contains("not inside of a Syncweb folder"),
        "error should explain the folder rule: {}",
        String::from_utf8_lossy(&ls_plain.stderr)
    );

    let find = syncweb(&["--data-dir", bob_data_arg, "find", "*.txt", bob_folder_arg])?;
    ensure!(find.status.success());
    ensure!(
        String::from_utf8_lossy(&find.stdout).contains("hello.txt"),
        "find should match the metadata index: {}",
        String::from_utf8_lossy(&find.stdout)
    );

    let sort = syncweb(&["--data-dir", bob_data_arg, "sort", "--by", "size", bob_folder_arg])?;
    ensure!(sort.status.success());
    ensure!(
        String::from_utf8_lossy(&sort.stdout).contains("hello.txt"),
        "sort should include the entry: {}",
        String::from_utf8_lossy(&sort.stdout)
    );

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let shutdown_b = syncweb(&["--data-dir", bob_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown_b.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    for dir in [&alice_folder, &alice_data, &bob_folder, &bob_data, &plain] {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

#[test]
fn test_download_dry_run_previews_without_downloading() -> anyhow::Result<()> {
    // User story 4 (Ari): `download --dry-run`/`--preview` previews what a
    // folder fetch would grab without downloading anything. The preview
    // selection (which entries match the shared filter group, peer/count
    // limits) is exercised by the `FetchFilter::select` unit tests in core;
    // this test pins the CLI wiring: the envelope, the alias, and the
    // guarantee that nothing is materialized to disk.
    let alice_data = cli_test_dir("dry-run-alice")?;
    let alice_folder = cli_test_dir("dry-run-alice-folder")?;
    let bob_data = cli_test_dir("dry-run-bob")?;
    let bob_folder = cli_test_dir("dry-run-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;
    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon should start");
    wait_for_daemon_ready(alice_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        "--no-share",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let namespace = String::from_utf8(create.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();

    std::fs::write(alice_folder.join("movie.mp4"), vec![b'a'; 4096]).context("write movie")?;
    std::fs::write(alice_folder.join("notes.txt"), b"hello").context("write notes")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace])?;
    ensure!(share.status.success(), "share should succeed");
    let ticket = String::from_utf8(share.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();
    ensure!(ticket.starts_with("syncweb://"), "share should output a URL: {ticket}");

    let bob_start = daemon_start_bg(bob_data_arg)?;
    ensure!(bob_start.status.success(), "bob daemon should start");
    wait_for_daemon_ready(bob_data_arg)?;

    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "folders",
        "join",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        join.status.success(),
        "bob join should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );

    let bob_folder_arg = bob_folder.to_str().context("UTF-8 path")?;
    // Wait until bob's metadata shows alice's entries.
    let mut saw_entries = false;
    for _ in 0..60 {
        let ls = syncweb(&["--data-dir", bob_data_arg, "--json", "ls", bob_folder_arg])?;
        if ls.status.success() && String::from_utf8_lossy(&ls.stdout).contains("notes.txt") {
            saw_entries = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(saw_entries, "bob should list alice's entries before the dry-run");

    // Dry-run must never write to the disk.
    let dry = syncweb(&["--data-dir", bob_data_arg, "download", "--dry-run", bob_folder_arg])?;
    ensure!(
        dry.status.success(),
        "dry-run should succeed: {}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let dry_out = String::from_utf8_lossy(&dry.stdout);
    ensure!(
        dry_out.contains("would fetch"),
        "dry-run should print a preview line: {dry_out}"
    );
    ensure!(
        !dry_out.contains("downloaded"),
        "dry-run must not claim a download: {dry_out}"
    );
    ensure!(
        std::fs::read_dir(&bob_folder)?.next().is_none(),
        "dry-run must not materialize content onto the disk"
    );

    // --preview is a visible alias.
    let alias = syncweb(&["--data-dir", bob_data_arg, "download", "--preview", bob_folder_arg])?;
    ensure!(
        alias.status.success(),
        "--preview should work: {}",
        String::from_utf8_lossy(&alias.stderr)
    );
    ensure!(
        String::from_utf8_lossy(&alias.stdout).contains("would fetch"),
        "--preview should behave like --dry-run"
    );

    // Filter flags narrow the preview without erroring.
    for extra in [&["--ext", "txt"][..], &["--max-count", "1"][..], &["--remote-only"][..]] {
        let mut args = vec!["--data-dir", bob_data_arg, "download", "--dry-run"];
        args.extend_from_slice(extra);
        args.push(bob_folder_arg);
        let filtered = syncweb(&args)?;
        ensure!(
            filtered.status.success(),
            "dry-run {extra:?} should succeed: {}",
            String::from_utf8_lossy(&filtered.stderr)
        );
        ensure!(
            String::from_utf8_lossy(&filtered.stdout).contains("would fetch"),
            "dry-run {extra:?} should still print a preview: {}",
            String::from_utf8_lossy(&filtered.stdout)
        );
    }

    // --json emits the stable envelope.
    let json = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "--json",
        "download",
        "--dry-run",
        bob_folder_arg,
    ])?;
    ensure!(json.status.success(), "dry-run --json should succeed");
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json.stdout)).context("dry-run --json should be valid JSON")?;
    ensure!(value.get("folder").is_some(), "envelope should carry folder: {value}");
    ensure!(value.get("matched").is_some(), "envelope should carry matched: {value}");
    ensure!(value.get("bytes").is_some(), "envelope should carry bytes: {value}");
    let entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .context("envelope should carry an entries array")?;
    ensure!(
        entries
            .iter()
            .all(|e| e.get("path").is_some() && e.get("size").is_some() && e.get("peers").is_some()),
        "each entry should carry path/size/peers: {value}"
    );

    // Dry-run rejects unsupported sources with a clear message.
    let dest = cli_test_dir("dry-run-dest")?;
    let dest_arg = dest.to_str().context("UTF-8 path")?;
    let bad_dest = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "download",
        "--dry-run",
        bob_folder_arg,
        dest_arg,
    ])?;
    ensure!(
        !bad_dest.status.success() && String::from_utf8_lossy(&bad_dest.stderr).contains("not supported when copying"),
        "dry-run + destination should fail clearly: {}",
        String::from_utf8_lossy(&bad_dest.stderr)
    );
    let bad_ticket = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "download",
        "--dry-run",
        "--hash",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        bob_folder_arg,
    ])?;
    ensure!(
        !bad_ticket.status.success()
            && String::from_utf8_lossy(&bad_ticket.stderr).contains("only supported for folder sources"),
        "dry-run on a blob hash should fail clearly: {}",
        String::from_utf8_lossy(&bad_ticket.stderr)
    );

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let shutdown_b = syncweb(&["--data-dir", bob_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown_b.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    for dir in [&alice_folder, &alice_data, &bob_folder, &bob_data, &dest] {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

#[test]
fn test_download_dry_run_reports_remote_entries_with_metadata_only() -> anyhow::Result<()> {
    // User story 4 (Ari): `download --dry-run` must report the actual "would
    // fetch N" count. A `--metadata-only` join keeps content out of the store,
    // so the remote set is deterministic: entries are listed but their blobs
    // stay remote until an explicit `download`.
    let alice_data = cli_test_dir("dryrun-pos-alice")?;
    let alice_folder = cli_test_dir("dryrun-pos-alice-folder")?;
    let bob_data = cli_test_dir("dryrun-pos-bob")?;
    let bob_folder = cli_test_dir("dryrun-pos-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;
    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon should start");
    wait_for_daemon_ready(alice_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        "--no-share",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let namespace = String::from_utf8(create.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();

    std::fs::write(alice_folder.join("movie.mp4"), vec![b'a'; 4096]).context("write movie")?;
    std::fs::write(alice_folder.join("notes.txt"), b"hello").context("write notes")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace])?;
    ensure!(share.status.success(), "share should succeed");
    let ticket = String::from_utf8(share.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();
    ensure!(ticket.starts_with("syncweb://"), "share should output a URL: {ticket}");

    let bob_start = daemon_start_bg(bob_data_arg)?;
    ensure!(bob_start.status.success(), "bob daemon should start");
    wait_for_daemon_ready(bob_data_arg)?;

    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "folders",
        "join",
        "--metadata-only",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        join.status.success(),
        "bob join --metadata-only should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );

    let bob_folder_arg = bob_folder.to_str().context("UTF-8 path")?;
    // Entries sync (metadata), blobs never download — the remote set is stable.
    let mut saw_both = false;
    let mut bob_namespace = String::new();
    for _ in 0..60 {
        let ls = syncweb(&["--data-dir", bob_data_arg, "--json", "ls", bob_folder_arg])?;
        let out = String::from_utf8_lossy(&ls.stdout);
        if out.contains("movie.mp4") && out.contains("notes.txt") {
            saw_both = true;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&out) {
                bob_namespace = value
                    .get("folder")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
            }
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(saw_both, "bob should list both entries on a metadata-only join");
    ensure!(
        !bob_namespace.is_empty(),
        "ls envelope should carry the folder namespace"
    );

    // Positive count: the preview must report exactly the remote entries.
    let dry = syncweb(&["--data-dir", bob_data_arg, "download", "--dry-run", bob_folder_arg])?;
    ensure!(
        dry.status.success(),
        "dry-run should succeed: {}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let dry_out = String::from_utf8_lossy(&dry.stdout);
    ensure!(
        dry_out.contains("would fetch 2 entries"),
        "metadata-only join should make the dry-run report both remote entries: {dry_out}"
    );
    ensure!(
        dry_out.contains("movie.mp4") && dry_out.contains("notes.txt"),
        "preview should list both files: {dry_out}"
    );
    ensure!(
        std::fs::read_dir(&bob_folder)?.next().is_none(),
        "dry-run must not materialize anything onto the disk"
    );

    let json = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "--json",
        "download",
        "--dry-run",
        bob_folder_arg,
    ])?;
    ensure!(json.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json.stdout)).context("dry-run --json should be valid JSON")?;
    ensure!(
        value.get("matched") == Some(&serde_json::Value::from(2)),
        "matched should be 2: {value}"
    );
    ensure!(
        value.get("bytes") == Some(&serde_json::Value::from(4101)),
        "bytes should sum sizes: {value}"
    );

    let ext = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "download",
        "--dry-run",
        "--ext",
        "txt",
        bob_folder_arg,
    ])?;
    ensure!(ext.status.success());
    let ext_out = String::from_utf8_lossy(&ext.stdout);
    ensure!(
        ext_out.contains("would fetch 1 entry") && ext_out.contains("notes.txt") && !ext_out.contains("movie.mp4"),
        "dry-run --ext txt should narrow the preview: {ext_out}"
    );

    // A real download fetches the content directly from peers (the daemon path
    // runs a direct per-blob fetch for metadata-only folders); the dry-run then
    // reports 0.
    let dl = syncweb(&["--data-dir", bob_data_arg, "download", &bob_namespace])?;
    ensure!(
        dl.status.success(),
        "real download should succeed: {}",
        String::from_utf8_lossy(&dl.stderr)
    );
    ensure!(
        String::from_utf8_lossy(&dl.stdout).contains("downloaded"),
        "download should report progress: {}",
        String::from_utf8_lossy(&dl.stdout)
    );

    let mut now_local = false;
    for _ in 0..60 {
        let after = syncweb(&["--data-dir", bob_data_arg, "download", "--dry-run", bob_folder_arg])?;
        let out = String::from_utf8_lossy(&after.stdout);
        if out.contains("would fetch 0 entries") {
            now_local = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(
        now_local,
        "after a real download the dry-run should report nothing to fetch"
    );

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let shutdown_b = syncweb(&["--data-dir", bob_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown_b.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    for dir in [&alice_folder, &alice_data, &bob_folder, &bob_data] {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

#[test]
fn test_create_import_via_daemon_one_shot() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("create-import-dl")?;
    let folder = cli_test_dir("create-import-dl-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(data_dir_arg)?;

    std::fs::write(folder.join("a.txt"), b"aaa").context("write source file")?;

    // Daemon-mode create should ingest the non-empty directory in one shot.
    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        create.status.success(),
        "daemon create --import should succeed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();

    let report = syncweb(&["--json", "--data-dir", data_dir_arg, "stats", "files", &namespace])?;
    ensure!(
        report.status.success(),
        "stats files should succeed: {}",
        String::from_utf8_lossy(&report.stderr)
    );
    let report_value: serde_json::Value =
        serde_json::from_str(&String::from_utf8(report.stdout).context("UTF-8 output")?)
            .context("stats files should be JSON")?;
    ensure!(
        report_value.get("total_files") == Some(&serde_json::Value::from(1)),
        "daemon create should have imported a.txt, got: {report_value}"
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&folder);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_join_download_via_daemon_materializes_content() -> anyhow::Result<()> {
    let alice_data = cli_test_dir("join-dl-daemon-alice")?;
    let alice_folder = cli_test_dir("join-dl-daemon-alice-folder")?;
    let bob_data = cli_test_dir("join-dl-daemon-bob")?;
    let bob_folder = cli_test_dir("join-dl-daemon-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;
    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon start should succeed");
    wait_for_daemon_ready(alice_data_arg)?;
    let bob_start = daemon_start_bg(bob_data_arg)?;
    ensure!(bob_start.status.success(), "bob daemon start should succeed");
    wait_for_daemon_ready(bob_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();
    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace, "--write"])?;
    ensure!(share.status.success(), "share should succeed");
    let share_out = String::from_utf8(share.stdout).context("UTF-8 output")?;
    let ticket = ticket_from_share_output(&share_out);
    ensure!(ticket.starts_with("syncweb://"), "share should output a URL: {ticket}");

    std::fs::write(alice_folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    // Daemon-mode join with --download routes through bob's daemon via IPC.
    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "folders",
        "join",
        "--download-existing",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        join.status.success(),
        "daemon join --download-existing should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );
    let join_out = String::from_utf8(join.stdout).context("UTF-8 output")?;
    ensure!(
        join_out.contains("downloaded:"),
        "join should report a download count: {join_out}"
    );

    let content = std::fs::read_to_string(bob_folder.join("hello.txt")).context("read materialized file")?;
    ensure!(
        content == "hello world",
        "materialized content should match source, got: {content:?}"
    );

    let _ = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"]);
    let _ = syncweb(&["--data-dir", bob_data_arg, "stop", "--yes", "--force"]);
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&alice_folder);
    let _ = std::fs::remove_dir_all(&alice_data);
    let _ = std::fs::remove_dir_all(&bob_folder);
    let _ = std::fs::remove_dir_all(&bob_data);
    Ok(())
}

#[test]
fn test_network_peers_returns_envelope_via_daemon() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("network-peers")?;
    let folder = cli_test_dir("network-peers-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(data_dir_arg)?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "create",
        folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "folders create should succeed");
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .trim()
        .strip_prefix("syncweb://folder/")
        .and_then(|rest| rest.split('?').next())
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();

    std::fs::write(folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        data_dir_arg,
        "folders",
        "import",
        folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "folders import should succeed");

    // `network peers <ns> --json` must return the `{folder, peers, per_blob}`
    // envelope with a `peers` array and per-blob rows whose `% seeded` ≥ 0.
    let peers = syncweb(&["--json", "--data-dir", data_dir_arg, "network", "peers", &namespace])?;
    ensure!(
        peers.status.success(),
        "network peers should succeed: {}",
        String::from_utf8_lossy(&peers.stderr)
    );
    let value: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&peers.stdout))
        .context("network peers --json should be valid JSON")?;
    ensure!(value.get("folder").is_some(), "envelope should carry folder: {value}");
    let peer_list = value
        .get("peers")
        .and_then(serde_json::Value::as_array)
        .context("envelope should carry peers array")?;
    ensure!(
        peer_list.iter().all(|peer| peer.get("device_id").is_some()),
        "each peer should carry a device_id: {value}"
    );
    let per_blob = value
        .get("per_blob")
        .and_then(serde_json::Value::as_array)
        .context("envelope should carry per_blob array")?;
    ensure!(!per_blob.is_empty(), "per_blob should list the imported blobs: {value}");
    for blob in per_blob {
        ensure!(
            blob.get("path").is_some() && blob.get("hash").is_some(),
            "per_blob rows should carry path and hash: {value}"
        );
        let seeded = blob
            .get("pct_seeded")
            .and_then(serde_json::Value::as_f64)
            .context("per_blob row should carry pct_seeded")?;
        ensure!(seeded >= 0.0, "pct_seeded should be >= 0: {value}");
    }

    // `network peers` with no daemon must not guess: empty peers + per_blob.
    let no_daemon_dir = cli_test_dir("network-peers-no-daemon")?;
    let no_daemon_arg = no_daemon_dir.to_str().context("UTF-8 path")?;
    let empty = syncweb(&["--json", "--no-daemon", "--data-dir", no_daemon_arg, "network", "peers"])?;
    ensure!(empty.status.success(), "network peers without daemon should succeed");
    let empty_value: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&empty.stdout))
        .context("no-daemon network peers should be valid JSON")?;
    ensure!(
        empty_value.get("peers").and_then(serde_json::Value::as_array).is_some(),
        "no-daemon envelope should carry an empty peers array: {empty_value}"
    );
    ensure!(
        empty_value
            .get("per_blob")
            .and_then(serde_json::Value::as_array)
            .is_some(),
        "no-daemon envelope should carry an empty per_blob array: {empty_value}"
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&folder);
    let _ = std::fs::remove_dir_all(&data_dir);
    let _ = std::fs::remove_dir_all(&no_daemon_dir);
    Ok(())
}

// ---------------------------------------------------------------------------
// MANUAL_TESTING_PLAN.md regression coverage for the sections removed from the
// plan because they now pass.
// ---------------------------------------------------------------------------

fn stdout_str(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Plan 4.2/22: `find --local-only --json` wraps results under `paths`.
#[test]
fn test_find_local_only_json_uses_paths_key() -> anyhow::Result<()> {
    let dir = cli_test_dir("find-paths")?;
    std::fs::write(dir.join("a.mp3"), b"x")?;
    std::fs::create_dir_all(dir.join("sub"))?;
    std::fs::write(dir.join("sub/b.txt"), b"y")?;

    let out = syncweb(&["--json", "find", "--local-only", "*", dir.to_str().context("utf8")?])?;
    ensure!(out.status.success(), "find --local-only: {}", stdout_str(&out));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).context("parse json")?;
    let paths = value
        .get("paths")
        .and_then(serde_json::Value::as_array)
        .context("paths array")?;
    ensure!(paths.len() == 2, "expected 2 paths, got {value}");

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// Plan 14: `watch --show-filters` and `watch --dry-run --paths`.
#[test]
fn test_watch_show_filters_and_dry_run() -> anyhow::Result<()> {
    let data = cli_test_dir("watch-filters-data")?;
    let dir = cli_test_dir("watch-filters-dir")?;
    std::fs::write(dir.join("keep.txt"), b"keep")?;
    let data_arg = data.to_str().context("utf8")?;
    let dir_arg = dir.to_str().context("utf8")?;

    let show = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "watch",
        "--show-filters",
        dir_arg,
    ])?;
    ensure!(show.status.success(), "show-filters: {}", stdout_str(&show));
    ensure!(
        stdout_str(&show).contains("default_action"),
        "show-filters should print the loaded rules: {}",
        stdout_str(&show)
    );

    let paths_arg = dir.join("keep.txt").to_string_lossy().to_string();
    let dry = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "watch",
        "--dry-run",
        "--paths",
        &paths_arg,
        dir_arg,
    ])?;
    ensure!(dry.status.success(), "dry-run: {}", stdout_str(&dry));
    ensure!(
        stdout_str(&dry).contains("accept"),
        "dry-run should report the accept decision: {}",
        stdout_str(&dry)
    );

    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// Plan 21: `discovery.mdns` is the supported key; the old `discovery.local_mdns`
/// is rejected with a list of valid keys.
#[test]
fn test_config_discovery_mdns_toggle() -> anyhow::Result<()> {
    let data = cli_test_dir("discovery-config")?;
    let arg = data.to_str().context("utf8")?;

    let off = syncweb(&[
        "--data-dir",
        arg,
        "--no-daemon",
        "config",
        "set",
        "discovery.mdns",
        "false",
    ])?;
    ensure!(off.status.success(), "set false: {}", stdout_str(&off));
    let show = syncweb(&["--data-dir", arg, "--no-daemon", "config", "show", "discovery"])?;
    ensure!(show.status.success());
    ensure!(
        stdout_str(&show).contains("mdns = false"),
        "discovery show: {}",
        stdout_str(&show)
    );

    let on = syncweb(&[
        "--data-dir",
        arg,
        "--no-daemon",
        "config",
        "set",
        "discovery.mdns",
        "true",
    ])?;
    ensure!(on.status.success());

    let bad = syncweb(&[
        "--data-dir",
        arg,
        "--no-daemon",
        "config",
        "set",
        "discovery.local_mdns",
        "true",
    ])?;
    ensure!(!bad.status.success(), "discovery.local_mdns must be rejected");

    let _ = std::fs::remove_dir_all(&data);
    Ok(())
}

/// Plan 9.3: `package import --filter` drops filtered entries while importing.
#[test]
fn test_package_import_filter_drops_entries() -> anyhow::Result<()> {
    let data = cli_test_dir("pkg-filter-data")?;
    let imp = cli_test_dir("pkg-filter-imp")?;
    let src = cli_test_dir("pkg-filter-src")?;
    let out_dir = cli_test_dir("pkg-filter-out")?;
    std::fs::write(src.join("keep.txt"), b"keep")?;
    std::fs::write(src.join("drop.tmp"), b"drop")?;
    std::fs::write(src.join("also.txt"), b"also")?;
    let data_arg = data.to_str().context("utf8")?;
    let imp_arg = imp.to_str().context("utf8")?;
    let src_arg = src.to_str().context("utf8")?;
    let archive = out_dir.join("demo.car.zst");
    let archive_arg = archive.to_str().context("utf8")?;

    let add = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "package",
        "add",
        "--name",
        "demo",
        src_arg,
    ])?;
    ensure!(add.status.success(), "package add: {}", stdout_str(&add));
    let export = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "package",
        "export",
        src_arg,
        archive_arg,
    ])?;
    ensure!(export.status.success(), "package export: {}", stdout_str(&export));
    ensure!(archive.exists(), "export should write {archive_arg}");

    let import = syncweb(&[
        "--data-dir",
        imp_arg,
        "--no-daemon",
        "package",
        "import",
        "--filter",
        "name!=*.tmp",
        archive_arg,
    ])?;
    ensure!(import.status.success(), "package import: {}", stdout_str(&import));
    ensure!(
        stdout_str(&import).contains("(2 entries)"),
        "filter should drop *.tmp: {}",
        stdout_str(&import)
    );

    for dir in [&data, &imp, &src, &out_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

/// Plan 13.2: `stats network --json` emits the documented keys.
#[test]
fn test_stats_network_json_shape() -> anyhow::Result<()> {
    let data = cli_test_dir("stats-network")?;
    let arg = data.to_str().context("utf8")?;
    let out = syncweb(&["--data-dir", arg, "--no-daemon", "--json", "stats", "network"])?;
    ensure!(out.status.success(), "stats network: {}", stdout_str(&out));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).context("parse json")?;
    for key in [
        "total_upload",
        "total_download",
        "per_folder",
        "per_peer",
        "period_start",
    ] {
        ensure!(value.get(key).is_some(), "missing {key}: {value}");
    }
    let _ = std::fs::remove_dir_all(&data);
    Ok(())
}

/// Plan 19: the daemon's WebSocket bridge accepts a connection, answers a valid
/// `IpcRequest`, and reports invalid JSON as an error.
#[test]
fn test_websocket_bridge_round_trip() -> anyhow::Result<()> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let data = cli_test_dir("ws-bridge")?;
    let arg = data.to_str().context("utf8")?;
    let start = daemon_start_bg(arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(arg)?;

    let mut stream = TcpStream::connect("127.0.0.1:9192").context("connect ws bridge")?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;
    stream.write_all(
        b"GET /bridge HTTP/1.1\r\nHost: 127.0.0.1:9192\r\nUpgrade: websocket\r\n\
          Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
          Sec-WebSocket-Version: 13\r\n\r\n",
    )?;
    let mut buf = [0_u8; 1024];
    let n = stream.read(&mut buf)?;
    let handshake = String::from_utf8_lossy(buf.get(..n).unwrap_or_default()).to_string();
    ensure!(
        handshake.contains("101 Switching Protocols"),
        "handshake should upgrade: {handshake}"
    );

    let send_text = |sink: &mut TcpStream, text: &str| -> anyhow::Result<()> {
        let payload = text.as_bytes();
        ensure!(payload.len() < 126, "test payload must be a short frame");
        let mask = [1_u8, 2, 3, 4];
        let mut frame = vec![0x81, 0x80 | u8::try_from(payload.len())?];
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().zip(mask.iter().cycle()).map(|(b, m)| *b ^ *m));
        sink.write_all(&frame)?;
        Ok(())
    };
    let read_text = |sink: &mut TcpStream| -> anyhow::Result<String> {
        let mut header = [0_u8; 2];
        sink.read_exact(&mut header)?;
        let len = usize::from(header[1] & 0x7f);
        let mut body = vec![0_u8; len];
        sink.read_exact(&mut body)?;
        Ok(String::from_utf8_lossy(&body).to_string())
    };

    send_text(&mut stream, r#"{"command":{"command":"status"}}"#)?;
    let response = read_text(&mut stream)?;
    ensure!(response.contains("\"response\":\"status\""), "ws status: {response}");

    send_text(&mut stream, "not-json")?;
    let invalid = read_text(&mut stream)?;
    ensure!(invalid.contains("invalid JSON"), "ws invalid json: {invalid}");

    let shutdown = syncweb(&["--data-dir", arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let _ = std::fs::remove_dir_all(&data);
    Ok(())
}

/// Plan 18: the standalone media server serves blobs by hash, supports byte
/// ranges, and rejects malformed or unknown hashes without dropping the
/// connection.
#[test]
fn test_media_server_endpoints() -> anyhow::Result<()> {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    let data = cli_test_dir("media-data")?;
    let folder = cli_test_dir("media-folder")?;
    std::fs::write(folder.join("clip.txt"), b"hello media body")?;
    let data_arg = data.to_str().context("utf8")?;
    let folder_arg = folder.to_str().context("utf8")?;

    let create = syncweb(&["--data-dir", data_arg, "--no-daemon", "folders", "create", folder_arg])?;
    ensure!(create.status.success(), "create: {}", stdout_str(&create));
    let ls = syncweb(&["--data-dir", data_arg, "--no-daemon", "--json", "ls", folder_arg])?;
    ensure!(ls.status.success(), "ls: {}", stdout_str(&ls));
    let value: serde_json::Value = serde_json::from_slice(&ls.stdout).context("parse ls json")?;
    let hash = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("hash"))
        .and_then(serde_json::Value::as_str)
        .context("blob hash")?
        .to_owned();

    let port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_arg,
            "start",
            "--media-only",
            "--media-listen",
            &format!("127.0.0.1:{port}"),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawn media server")?;

    let mut up = false;
    for _ in 0..60 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            up = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    ensure!(up, "media server did not start listening");

    let status_line = |path: &str, range_header: Option<&str>| -> anyhow::Result<String> {
        use std::fmt::Write as _;
        let mut stream = TcpStream::connect(("127.0.0.1", port))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        let mut request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
        if let Some(byte_range) = range_header {
            let _ = write!(request, "Range: {byte_range}\r\n");
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes())?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        Ok(text.lines().next().unwrap_or_default().to_owned())
    };

    ensure!(status_line("/", None)?.contains("404"), "/ should return 404");
    ensure!(
        status_line("/media/deadbeef", None)?.contains("400"),
        "short hash should return 400"
    );
    let zeros = "0".repeat(64);
    ensure!(
        status_line(&format!("/media/{zeros}"), None)?.contains("404"),
        "unknown 64-hex hash should return 404"
    );
    ensure!(
        status_line(&format!("/media/{hash}"), None)?.contains("200"),
        "known blob should return 200"
    );
    ensure!(
        status_line(&format!("/media/{hash}"), Some("bytes=0-4"))?.contains("206"),
        "byte range should return 206"
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&folder);
    Ok(())
}

/// A folder joined from a read-only ticket cannot accept local writes, so the
/// daemon must not watch it: materialized files would otherwise be re-imported
/// in a failing retry loop (`Attempted to insert to read only replica`). Assert
/// that no watcher error is recorded for a read-only replica.
#[test]
fn test_read_only_folder_does_not_record_watcher_errors() -> anyhow::Result<()> {
    let alice_data = cli_test_dir("ro-watch-alice")?;
    let alice_folder = cli_test_dir("ro-watch-alice-folder")?;
    let bob_data = cli_test_dir("ro-watch-bob")?;
    let bob_folder = cli_test_dir("ro-watch-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;
    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;
    let alice_folder_arg = alice_folder.to_str().context("UTF-8 path")?;
    let bob_folder_arg = bob_folder.to_str().context("UTF-8 path")?;

    std::fs::write(alice_folder.join("alpha.txt"), b"alpha content")?;
    std::fs::write(alice_folder.join("beta.bin"), vec![b'x'; 8192])?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon should start");
    wait_for_daemon_ready(alice_data_arg)?;

    // `folders create` (no `--write`) publishes a read-only ticket.
    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        "--no-share",
        alice_folder_arg,
    ])?;
    ensure!(
        create.status.success(),
        "alice create: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let namespace = String::from_utf8(create.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();

    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace])?;
    ensure!(
        share.status.success(),
        "alice share: {}",
        String::from_utf8_lossy(&share.stderr)
    );
    let ticket = String::from_utf8(share.stdout)
        .context("UTF-8 output")?
        .trim()
        .to_owned();

    let start_b = daemon_start_bg(bob_data_arg)?;
    ensure!(start_b.status.success(), "bob daemon should start");
    wait_for_daemon_ready(bob_data_arg)?;

    let join = syncweb(&["--data-dir", bob_data_arg, "folders", "join", &ticket, bob_folder_arg])?;
    ensure!(
        join.status.success(),
        "bob join: {}",
        String::from_utf8_lossy(&join.stderr)
    );

    let mut saw_entries = false;
    for _ in 0..60 {
        let ls = syncweb(&["--data-dir", bob_data_arg, "--json", "ls", bob_folder_arg])?;
        if ls.status.success() && String::from_utf8_lossy(&ls.stdout).contains("alpha.txt") {
            saw_entries = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(saw_entries, "bob should see alice's entries");

    // Materialize the content to disk, then give the daemon watcher time to
    // (incorrectly) notice and retry the read-only import.
    let download = syncweb(&["--data-dir", bob_data_arg, "download", bob_folder_arg])?;
    ensure!(
        download.status.success(),
        "bob download: {}",
        String::from_utf8_lossy(&download.stderr)
    );
    std::thread::sleep(std::time::Duration::from_secs(4));

    let folders = syncweb(&["--data-dir", bob_data_arg, "--json", "folders"])?;
    ensure!(folders.status.success());
    let value: serde_json::Value = serde_json::from_slice(&folders.stdout).context("parse folders json")?;
    let entry = value
        .get("folders")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|folder| folder.get("namespace").and_then(serde_json::Value::as_str) == Some(namespace.as_str()))
        })
        .context("bob folder row")?;
    let errors = entry
        .get("errors")
        .and_then(serde_json::Value::as_array)
        .context("errors array")?;
    ensure!(
        errors.is_empty(),
        "read-only folder must not record watcher import errors: {errors:?}"
    );

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown.status.success());
    let shutdown_b = syncweb(&["--data-dir", bob_data_arg, "stop", "--yes", "--force"])?;
    ensure!(shutdown_b.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    for dir in [&alice_data, &alice_folder, &bob_data, &bob_folder] {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

/// Plan 14 (`NF-4`): `watch --dry-run --paths` must report a non-existent path
/// instead of aborting the whole run.
#[test]
fn test_watch_dry_run_missing_path_is_reported() -> anyhow::Result<()> {
    let data = cli_test_dir("watch-dryrun-missing-data")?;
    let dir = cli_test_dir("watch-dryrun-missing-dir")?;
    std::fs::write(dir.join("present.txt"), b"present")?;
    let data_arg = data.to_str().context("utf8")?;
    let present = dir.join("present.txt");
    let missing = dir.join("missing.tmp");

    let out = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "watch",
        "--dry-run",
        "--paths",
        present.to_str().context("utf8")?,
        missing.to_str().context("utf8")?,
        dir.to_str().context("utf8")?,
    ])?;
    ensure!(
        out.status.success(),
        "dry-run with a missing path must not abort: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = stdout_str(&out);
    ensure!(
        stdout.contains("present.txt"),
        "dry-run should evaluate the existing path: {stdout}"
    );
    ensure!(
        stdout.contains("missing.tmp"),
        "dry-run should report the missing path: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// Plan 13.2 (`NF-8`): a live subscription transfer must be reflected in
/// `stats network` while the daemon is still running.
#[test]
fn test_subscribe_transfer_records_bandwidth_stats() -> anyhow::Result<()> {
    let alice_data = cli_test_dir("bw-stats-alice-data")?;
    let alice_folder = cli_test_dir("bw-stats-alice-folder")?;
    let bob_data = cli_test_dir("bw-stats-bob-data")?;
    let bob_folder = cli_test_dir("bw-stats-bob-folder")?;
    let alice_data_arg = alice_data.to_str().context("UTF-8 path")?;
    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;
    let alice_folder_arg = alice_folder.to_str().context("UTF-8 path")?;
    let bob_folder_arg = bob_folder.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(alice_data_arg)?;
    ensure!(start.status.success(), "alice daemon should start");
    wait_for_daemon_ready(alice_data_arg)?;

    let create = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "folders",
        "create",
        "--no-share",
        alice_folder_arg,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let namespace = stdout_str(&create).trim().to_owned();
    ensure!(!namespace.is_empty(), "create should print a namespace");

    std::fs::write(alice_folder.join("hello.txt"), b"hello world")?;
    let import = syncweb(&["--data-dir", alice_data_arg, "folders", "import", alice_folder_arg])?;
    ensure!(import.status.success(), "alice import should succeed");

    let share = syncweb(&["--data-dir", alice_data_arg, "share", &namespace])?;
    ensure!(share.status.success(), "share should succeed");
    let ticket = stdout_str(&share).trim().to_owned();

    let bob_start = daemon_start_bg(bob_data_arg)?;
    ensure!(bob_start.status.success(), "bob daemon should start");
    wait_for_daemon_ready(bob_data_arg)?;

    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "folders",
        "join",
        "--subscribe",
        &ticket,
        bob_folder_arg,
    ])?;
    ensure!(
        join.status.success(),
        "bob join --subscribe should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );

    let mut saw_first = false;
    for _ in 0..60 {
        let ls = syncweb(&["--data-dir", bob_data_arg, "ls", bob_folder_arg])?;
        if ls.status.success() && stdout_str(&ls).contains("hello.txt") {
            saw_first = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(saw_first, "bob should see alice's first entry");

    // Add a new, larger blob after Bob's supervision is established; the
    // download must be accounted for in Bob's `stats network`.
    std::fs::write(alice_folder.join("big.bin"), vec![7_u8; 1_048_576])?;
    let import_big = syncweb(&["--data-dir", alice_data_arg, "folders", "import", alice_folder_arg])?;
    ensure!(import_big.status.success(), "alice import of big.bin should succeed");

    let mut download_bytes = 0_u64;
    for _ in 0..120 {
        let stats = syncweb(&["--data-dir", bob_data_arg, "--json", "stats", "network"])?;
        if stats.status.success() {
            let value: serde_json::Value = serde_json::from_slice(&stats.stdout).unwrap_or_default();
            download_bytes = value
                .get("total_download")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if download_bytes > 0 {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    ensure!(
        download_bytes > 0,
        "live subscription transfer should be reflected in stats network"
    );

    let _ = syncweb(&["--data-dir", alice_data_arg, "stop", "--yes", "--force"])?;
    let _ = syncweb(&["--data-dir", bob_data_arg, "stop", "--yes", "--force"])?;
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    for dir in [&alice_data, &alice_folder, &bob_data, &bob_folder] {
        let _ = std::fs::remove_dir_all(dir);
    }
    Ok(())
}

/// Daemon inventory (`NF-9`): a folder registered with a relative path must be
/// resolved to an absolute path so the daemon's watcher can find it.
#[test]
fn test_relative_folder_paths_are_absolute_in_daemon() -> anyhow::Result<()> {
    let data = cli_test_dir("relative-path-data")?;
    let work = cli_test_dir("relative-path-work")?;
    let relative = work.join("rel");
    std::fs::create_dir_all(&relative)?;
    let data_arg = data.to_str().context("utf8")?;

    let start = daemon_start_bg(data_arg)?;
    ensure!(start.status.success(), "daemon start should succeed");
    wait_for_daemon_ready(data_arg)?;

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .current_dir(&work)
        .args([
            "--data-dir",
            data_arg,
            "folders",
            "create",
            "--no-indexing",
            "--no-share",
            "rel",
        ])
        .output()
        .context("create with a relative path")?;
    ensure!(
        create.status.success(),
        "relative create should succeed: {}",
        String::from_utf8_lossy(&create.stderr)
    );

    let folders = syncweb(&["--data-dir", data_arg, "--json", "folders"])?;
    ensure!(folders.status.success());
    let value: serde_json::Value = serde_json::from_slice(&folders.stdout).context("parse folders json")?;
    let path = value
        .get("folders")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("path"))
        .and_then(serde_json::Value::as_str)
        .context("folder path")?;
    ensure!(
        std::path::Path::new(path).is_absolute(),
        "registered folder path should be absolute, got {path}"
    );
    ensure!(
        path.ends_with("rel"),
        "registered path should point at the relative folder: {path}"
    );

    let _ = syncweb(&["--data-dir", data_arg, "stop", "--yes", "--force"])?;
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::remove_dir_all(&data);
    Ok(())
}

/// Plan 15 (`NF-10`): `.syncignore` patterns must exclude matching files from a
/// `watch --once` import, and the ignore file itself must not be imported.
#[test]
fn test_watch_once_honors_syncignore() -> anyhow::Result<()> {
    let data = cli_test_dir("watch-syncignore-data")?;
    let dir = cli_test_dir("watch-syncignore-dir")?;
    std::fs::write(dir.join("keep.txt"), b"keep")?;
    std::fs::write(dir.join("drop.tmp"), b"drop")?;
    std::fs::write(dir.join(".syncignore"), b"# comment\n*.tmp\n")?;
    let data_arg = data.to_str().context("utf8")?;
    let dir_arg = dir.to_str().context("utf8")?;

    let create = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "folders",
        "create",
        "--no-indexing",
        "--no-share",
        dir_arg,
    ])?;
    ensure!(
        create.status.success(),
        "create should succeed: {}",
        String::from_utf8_lossy(&create.stderr)
    );

    let out = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "--json",
        "watch",
        "--once",
        dir_arg,
    ])?;
    ensure!(
        out.status.success(),
        "watch --once should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).context("parse watch json")?;
    let imported = value.get("imported").and_then(serde_json::Value::as_u64).unwrap_or(999);
    ensure!(
        imported == 1,
        "only keep.txt should be imported, got {imported}: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    let ls = syncweb(&["--data-dir", data_arg, "--no-daemon", "ls", dir_arg])?;
    ensure!(ls.status.success());
    let listing = stdout_str(&ls);
    ensure!(listing.contains("keep.txt"), "keep.txt should be present: {listing}");
    ensure!(!listing.contains("drop.tmp"), "drop.tmp should be ignored: {listing}");
    ensure!(
        !listing.contains(".syncignore"),
        ".syncignore should be ignored: {listing}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data);
    Ok(())
}

/// Plan 13.2 (`NF-12`): a folder outside its scheduled active window must not
/// report an active session after the daemon starts.
#[test]
fn test_inactive_schedule_window_pauses_folder() -> anyhow::Result<()> {
    let data = cli_test_dir("schedule-inactive-data")?;
    let dir = cli_test_dir("schedule-inactive-folder")?;
    let data_arg = data.to_str().context("utf8")?;
    let dir_arg = dir.to_str().context("utf8")?;

    let create = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "folders",
        "create",
        "--no-indexing",
        "--no-share",
        dir_arg,
    ])?;
    ensure!(
        create.status.success(),
        "create should succeed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let namespace = stdout_str(&create).trim().to_owned();
    ensure!(!namespace.is_empty(), "create should print a namespace");

    // Pick a one-hour window that starts two hours from the current UTC minute,
    // guaranteeing the current time is outside it regardless of when this runs.
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let minute = u16::try_from((seconds.rem_euclid(86_400)).div_euclid(60)).unwrap_or(0);
    let start = (minute + 120).rem_euclid(1_440);
    let end = (start + 60).rem_euclid(1_440);
    let clock = |value: u16| format!("{:02}:{:02}", value.div_euclid(60), value.rem_euclid(60));
    let window = format!("{}-{}", clock(start), clock(end));

    let set = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "config",
        "schedule",
        "set",
        "--active",
        &window,
    ])?;
    ensure!(
        set.status.success(),
        "schedule set should succeed: {}",
        String::from_utf8_lossy(&set.stderr)
    );

    let subscribe_key = format!("{namespace}.subscribe");
    let subscribe = syncweb(&[
        "--data-dir",
        data_arg,
        "--no-daemon",
        "config",
        "set",
        &subscribe_key,
        "on",
    ])?;
    ensure!(
        subscribe.status.success(),
        "subscribe config should succeed: {}",
        String::from_utf8_lossy(&subscribe.stderr)
    );

    let start_bg = daemon_start_bg(data_arg)?;
    ensure!(start_bg.status.success(), "daemon should start");
    wait_for_daemon_ready(data_arg)?;

    let folders = syncweb(&["--data-dir", data_arg, "--json", "folders"])?;
    ensure!(folders.status.success());
    let value: serde_json::Value = serde_json::from_slice(&folders.stdout).context("parse folders json")?;
    let session_active = value
        .get("folders")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("namespace").and_then(serde_json::Value::as_str) == Some(&namespace))
        })
        .and_then(|row| row.get("session_active"))
        .and_then(serde_json::Value::as_bool)
        .context("folder session_active")?;
    ensure!(
        !session_active,
        "folder outside its active window must not report an active session: {}",
        stdout_str(&folders)
    );

    let _ = syncweb(&["--data-dir", data_arg, "stop", "--yes", "--force"])?;
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data);
    Ok(())
}
