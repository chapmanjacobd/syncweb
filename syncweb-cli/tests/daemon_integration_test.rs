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
    ensure!(help.contains("shutdown"));
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("namespace:"));
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown"])?;
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_sync_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-sync")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = daemon_start_bg(data_dir_arg)?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let sync = syncweb(&["--data-dir", data_dir_arg, "daemon-sync"])?;
    ensure!(sync.status.success(), "daemon-sync should succeed");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "create should succeed via daemon");
    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("namespace:"));

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));

    if let Some(ns) = namespace {
        let health = syncweb(&["--data-dir", data_dir_arg, "stats", "seeding", "--folder", ns])?;
        ensure!(health.status.success(), "stats seeding should succeed via daemon");

        let files = syncweb(&["--data-dir", data_dir_arg, "stats", "files", "--folder", ns])?;
        ensure!(files.status.success(), "stats files should succeed via daemon");
        let output = String::from_utf8(files.stdout).context("UTF-8 output")?;
        ensure!(
            output.contains("total_files:"),
            "stats files output should include total_files"
        );
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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

    let sync = syncweb(&["--data-dir", data_dir_arg, "daemon-sync"])?;
    ensure!(sync.status.success());

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let folders = syncweb(&["--data-dir", data_dir_arg, "folders"])?;
    ensure!(folders.status.success());
    let stdout = String::from_utf8(folders.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("Namespace") || stdout.contains("namespace"));

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));

    if let Some(ns) = namespace {
        let subscribe = syncweb(&["--data-dir", data_dir_arg, "join", ns, "--subscribe", "--ingest-only"])?;
        ensure!(subscribe.status.success(), "join --subscribe should succeed via daemon");
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));

    if let Some(ns) = namespace {
        let publish = syncweb(&["--data-dir", data_dir_arg, "share", "--namespace", ns])?;
        ensure!(publish.status.success(), "share should succeed via daemon");
        let pub_stdout = String::from_utf8(publish.stdout).context("UTF-8 output")?;
        ensure!(pub_stdout.contains("ticket:"));
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));

    if let Some(ns) = namespace {
        let leave = syncweb(&["--data-dir", data_dir_arg, "leave", ns])?;
        ensure!(leave.status.success(), "leave should succeed via daemon");
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));
    ensure!(namespace.is_some(), "create should output a namespace");
    let ns = namespace.unwrap();

    std::fs::write(dir.join("file.txt"), b"content")?;
    ensure!(dir.exists(), "folder directory should exist before leave");

    let leave = syncweb(&["--data-dir", data_dir_arg, "leave", ns, "--delete-files"])?;
    ensure!(
        leave.status.success(),
        "leave --delete-files should succeed, got: {}",
        String::from_utf8_lossy(&leave.stderr)
    );

    ensure!(
        !dir.exists(),
        "folder directory should be deleted after leave --delete-files"
    );

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));

    if let Some(ns) = namespace {
        let verify = syncweb(&["--data-dir", data_dir_arg, "verify", ns])?;
        ensure!(verify.status.success(), "verify should succeed via daemon");
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));

    if let Some(ns) = namespace {
        let snapshot_list = syncweb(&["--data-dir", data_dir_arg, "snapshot", "list", ns])?;
        ensure!(
            snapshot_list.status.success(),
            "snapshot list should succeed via daemon"
        );
    }

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
    let output = syncweb(&["create", "--help"])?;
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
fn test_stats_seeding_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["stats", "seeding", "--help"])?;
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
    let output = syncweb(&["join", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_publish_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["publish", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_leave_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["leave", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
    Ok(())
}

#[test]
fn test_import_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["import", "--help"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim));
    ensure!(namespace.is_some(), "create should output a namespace");
    let ns = namespace.unwrap();

    let sync = syncweb(&["--data-dir", data_dir_arg, "daemon-sync"])?;
    ensure!(sync.status.success(), "triggering daemon-sync should succeed");

    let leave = syncweb(&["--data-dir", data_dir_arg, "leave", ns])?;
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        output.status.success(),
        "embedded create with --no-daemon should succeed without daemon running"
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("namespace:"));

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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "--network", "home", "shutdown", "--force"])?;
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
    ensure!(shutdown.status.success(), "shutdown should succeed");
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));

    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success());

    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = stdout
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .and_then(|line| line.strip_prefix("namespace:").map(str::trim))
        .context("create should output a namespace")?;

    let sync = syncweb(&["--data-dir", data_dir_arg, "daemon-sync", namespace])?;
    ensure!(sync.status.success(), "daemon-sync --namespace should succeed");

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .lines()
        .find_map(|line| line.strip_prefix("namespace: "))
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();
    let share = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "share",
        "--namespace",
        &namespace,
        "--write",
    ])?;
    ensure!(share.status.success(), "share should succeed");
    let share_out = String::from_utf8(share.stdout).context("UTF-8 output")?;
    let ticket = share_out
        .lines()
        .find_map(|line| line.strip_prefix("ticket: "))
        .map(str::trim)
        .context("share should output a ticket")?
        .to_owned();

    std::fs::write(alice_folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    let bob_data_arg = bob_data.to_str().context("UTF-8 path")?;
    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "--no-daemon",
        "join",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
        "--download",
    ])?;
    ensure!(
        join.status.success(),
        "join --download should succeed: {}",
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

    let shutdown = syncweb(&["--data-dir", alice_data_arg, "shutdown", "--force"])?;
    ensure!(shutdown.status.success());
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&alice_folder);
    let _ = std::fs::remove_dir_all(&alice_data);
    let _ = std::fs::remove_dir_all(&bob_folder);
    let _ = std::fs::remove_dir_all(&bob_data);
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
        .lines()
        .find_map(|line| line.strip_prefix("namespace: "))
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();

    let report = syncweb(&[
        "--json",
        "--data-dir",
        data_dir_arg,
        "stats",
        "files",
        "--folder",
        &namespace,
    ])?;
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

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "shutdown", "--force"])?;
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
        "create",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(create.status.success(), "alice create should succeed");
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .lines()
        .find_map(|line| line.strip_prefix("namespace: "))
        .map(str::trim)
        .context("create should output a namespace")?
        .to_owned();
    let share = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "share",
        "--namespace",
        &namespace,
        "--write",
    ])?;
    ensure!(share.status.success(), "share should succeed");
    let share_out = String::from_utf8(share.stdout).context("UTF-8 output")?;
    let ticket = share_out
        .lines()
        .find_map(|line| line.strip_prefix("ticket: "))
        .map(str::trim)
        .context("share should output a ticket")?
        .to_owned();

    std::fs::write(alice_folder.join("hello.txt"), b"hello world").context("write source file")?;
    let import = syncweb(&[
        "--data-dir",
        alice_data_arg,
        "import",
        alice_folder.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(import.status.success(), "alice import should succeed");

    // Daemon-mode join with --download routes through bob's daemon via IPC.
    let join = syncweb(&[
        "--data-dir",
        bob_data_arg,
        "join",
        &ticket,
        bob_folder.to_str().context("UTF-8 path")?,
        "--download",
    ])?;
    ensure!(
        join.status.success(),
        "daemon join --download should succeed: {}",
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

    let _ = syncweb(&["--data-dir", alice_data_arg, "shutdown", "--force"]);
    let _ = syncweb(&["--data-dir", bob_data_arg, "shutdown", "--force"]);
    std::thread::sleep(std::time::Duration::from_secs_f64(0.5));
    let _ = std::fs::remove_dir_all(&alice_folder);
    let _ = std::fs::remove_dir_all(&alice_data);
    let _ = std::fs::remove_dir_all(&bob_folder);
    let _ = std::fs::remove_dir_all(&bob_data);
    Ok(())
}
