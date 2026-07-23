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

fn wait_for_daemon_ready(data_dir_arg: &str) -> anyhow::Result<()> {
    for _ in 0..150 {
        let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon: running") {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    anyhow::bail!("timed out waiting for daemon to become ready");
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
    ensure!(stdout.contains("ticket:"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_dir);
    Ok(())
}

#[test]
fn test_daemon_start_and_stop() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-lifecycle")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
fn test_daemon_shutdown_alias() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-shutdown-alias")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let shutdown = syncweb(&["--data-dir", data_dir_arg, "daemon-shutdown"])?;
    ensure!(shutdown.status.success(), "daemon-shutdown should succeed");

    let mut stopped = false;
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_secs_f64(0.25));
        let status = syncweb(&["--data-dir", data_dir_arg, "status"])?;
        if status.status.success() && stdout_contains(&status, "daemon not running") {
            stopped = true;
            break;
        }
    }
    ensure!(stopped, "daemon should be stopped after daemon-shutdown");
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let reload = syncweb(&["--data-dir", data_dir_arg, "daemon-reload"])?;
    ensure!(reload.status.success(), "daemon-reload should succeed");

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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
fn test_daemon_init_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-init")?;
    let dir = cli_test_dir("daemon-init-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
    ensure!(start.status.success());
    wait_for_daemon_ready(data_dir_arg)?;

    let init = syncweb(&["--data-dir", data_dir_arg, "init", dir.to_str().context("UTF-8 path")?])?;
    ensure!(init.status.success(), "init should succeed via daemon");
    let stdout = String::from_utf8(init.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("namespace:"));
    ensure!(stdout.contains("ticket:"));

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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
        let health = syncweb(&["--data-dir", data_dir_arg, "health", ns])?;
        ensure!(health.status.success(), "health should succeed via daemon");
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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

    let reload = syncweb(&["--data-dir", data_dir_arg, "daemon-reload"])?;
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
        let subscribe = syncweb(&["--data-dir", data_dir_arg, "subscribe", ns, "--ingest-only"])?;
        ensure!(subscribe.status.success(), "subscribe should succeed via daemon");
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
        let publish = syncweb(&["--data-dir", data_dir_arg, "publish", ns])?;
        ensure!(publish.status.success(), "publish should succeed via daemon");
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
fn test_daemon_unsubscribe_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-unsubscribe")?;
    let dir = cli_test_dir("daemon-unsubscribe-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
        let unsubscribe = syncweb(&["--data-dir", data_dir_arg, "unsubscribe", ns])?;
        ensure!(
            unsubscribe.status.success() || String::from_utf8_lossy(&unsubscribe.stderr).contains("no active session")
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
fn test_daemon_verify_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-verify")?;
    let dir = cli_test_dir("daemon-verify-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
fn test_init_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["init", "--help"])?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--no-daemon") || help.contains("daemon"));
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
fn test_health_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["health", "--help"])?;
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
    let output = syncweb(&["subscribe", "--help"])?;
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
fn test_unsubscribe_help_mentions_daemon_routing() -> anyhow::Result<()> {
    let output = syncweb(&["unsubscribe", "--help"])?;
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
fn test_daemon_unwatch_via_ipc() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("daemon-unwatch")?;
    let dir = cli_test_dir("daemon-unwatch-folder")?;
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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

    let unwatch = syncweb(&["--data-dir", data_dir_arg, "unwatch", ns])?;
    ensure!(
        unwatch.status.success() || String::from_utf8_lossy(&unwatch.stderr).contains("no active session"),
        "unwatch via namespace ID should succeed or report no active session"
    );

    let folders = syncweb(&["--data-dir", data_dir_arg, "folders"])?;
    ensure!(folders.status.success());
    let folder_stdout = String::from_utf8(folders.stdout).context("UTF-8 output")?;
    ensure!(
        !folder_stdout.contains(ns) || String::from_utf8_lossy(&unwatch.stderr).contains("no active session"),
        "unwatched namespace should not appear in folder list unless it had no active session"
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

    let start = syncweb(&["--data-dir", data_dir_arg, "start", "--bg"])?;
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
    ensure!(stdout.contains("ticket:"));

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
