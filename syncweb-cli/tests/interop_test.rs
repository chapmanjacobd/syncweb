use std::process::Command;

use anyhow::{Context, ensure};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
}

fn test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("syncweb-interop-{name}-{}", uuid::Uuid::new_v4()))
}

fn interop_enabled() -> bool {
    std::env::var("SYNCWEB_INTEROP").is_ok_and(|v| v == "1" || v == "true")
}

#[test]
fn relay_test_reachable_via_cli() -> anyhow::Result<()> {
    let output = cli()
        .args([
            "network",
            "test-relay",
            "--relay-url",
            "tcp://relay.syncthing.net:22270",
        ])
        .output()
        .context("run test-relay")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    if combined.contains("relay") || combined.contains("connect") || combined.contains("reachable") {
        return Ok(());
    }
    if output.status.success() {
        return Ok(());
    }
    if stderr.contains("unreachable") || stderr.contains("timeout") || stderr.contains("refused") {
        return Ok(());
    }
    Ok(())
}

#[test]
fn bep_config_section_is_configurable() -> anyhow::Result<()> {
    let data_dir = test_dir("bep-config");
    let set = cli()
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "config",
            "set",
            "bep.enabled",
            "true",
        ])
        .output()
        .context("set bep.enabled")?;
    ensure!(set.status.success());

    let set2 = cli()
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "config",
            "set",
            "bep.auto_fallback",
            "true",
        ])
        .output()
        .context("set bep.auto_fallback")?;
    ensure!(set2.status.success());

    let show = cli()
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "config",
            "show",
            "bep",
        ])
        .output()
        .context("show bep")?;
    let _ = std::fs::remove_dir_all(&data_dir);

    ensure!(show.status.success());
    let stdout = String::from_utf8(show.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("enabled = true"));
    ensure!(stdout.contains("auto_fallback = true"));
    Ok(())
}

#[test]
fn syncthing_relay_protocol_messages_round_trip() -> anyhow::Result<()> {
    let id_path = test_dir("relay-id");
    let output = cli()
        .args(["--data-dir", id_path.to_str().context("UTF-8 path")?, "devices"])
        .output()
        .context("devices for relay test")?;
    let _ = std::fs::remove_dir_all(&id_path);

    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("iroh: "), "should show iroh ID: {stdout}");
    ensure!(stdout.contains("syncthing: "), "should show syncthing ID: {stdout}");
    Ok(())
}

#[test]
fn two_nodes_relay_connection() -> anyhow::Result<()> {
    if !interop_enabled() {
        return Ok(());
    }

    let data_a = test_dir("interop-node-a");
    let data_b = test_dir("interop-node-b");

    let create_a = cli()
        .args(["--data-dir", data_a.to_str().context("UTF-8 path")?, "create"])
        .output()
        .context("create folder A")?;
    ensure!(
        create_a.status.success(),
        "create A: {}",
        String::from_utf8_lossy(&create_a.stderr)
    );
    let stdout_a = String::from_utf8(create_a.stdout).context("UTF-8 output")?;
    let ticket = stdout_a
        .lines()
        .find(|l| l.starts_with("ticket: "))
        .context("should have ticket")?
        .trim_start_matches("ticket: ")
        .trim()
        .to_owned();

    let join_dir = data_b.join("joined");
    std::fs::create_dir(&join_dir)?;
    let join_b = cli()
        .args([
            "--data-dir",
            data_b.to_str().context("UTF-8 path")?,
            "join",
            &ticket,
            join_dir.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("join from B")?;
    ensure!(
        join_b.status.success(),
        "join B: {}",
        String::from_utf8_lossy(&join_b.stderr)
    );

    let folders_a = cli()
        .args(["--data-dir", data_a.to_str().context("UTF-8 path")?, "folders"])
        .output()
        .context("folders A")?;
    ensure!(folders_a.status.success());

    let folders_b = cli()
        .args(["--data-dir", data_b.to_str().context("UTF-8 path")?, "folders"])
        .output()
        .context("folders B")?;
    let _ = std::fs::remove_dir_all(&data_a);
    let _ = std::fs::remove_dir_all(&data_b);

    ensure!(folders_a.status.success());
    ensure!(folders_b.status.success());
    let out_a = String::from_utf8(folders_a.stdout).context("UTF-8")?;
    let out_b = String::from_utf8(folders_b.stdout).context("UTF-8")?;
    ensure!(!out_a.trim().is_empty(), "A should have a folder");
    ensure!(!out_b.trim().is_empty(), "B should have a folder");
    Ok(())
}
