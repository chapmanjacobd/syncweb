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
    std::thread::sleep(std::time::Duration::from_secs(2));

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
    std::thread::sleep(std::time::Duration::from_secs(2));

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
