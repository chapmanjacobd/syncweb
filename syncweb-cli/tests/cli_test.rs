use anyhow::{Context, ensure};
use iroh::{EndpointAddr, SecretKey};
use iroh_blobs::{BlobFormat, Hash, ticket::BlobTicket};
use std::process::Command;

fn workspace_version() -> anyhow::Result<String> {
    let cargo: toml::Value = toml::from_str(include_str!("../../Cargo.toml")).context("parse workspace Cargo.toml")?;
    let version = cargo
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .context("workspace.package.version")?
        .to_string();
    Ok(version)
}

fn ticket_for_test(hash: Hash) -> BlobTicket {
    let secret = SecretKey::from_bytes(&[1_u8; 32]);
    BlobTicket::new(EndpointAddr::new(secret.public()), hash, BlobFormat::Raw)
}

#[test]
fn version_command_outputs_version() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("version")
        .output()
        .context("run syncweb version")?;

    ensure!(output.status.success());
    let version = workspace_version()?;
    anyhow::ensure!(String::from_utf8(output.stdout).context("UTF-8 output")? == format!("syncweb {version}\n"));
    Ok(())
}

#[test]
fn help_output_lists_available_commands() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("--help")
        .output()
        .context("run syncweb help")?;

    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("version"));
    ensure!(help.contains("create"));
    ensure!(help.contains("join"));
    ensure!(help.contains("leave"));
    ensure!(help.contains("folders"));
    ensure!(help.contains("devices"));
    ensure!(help.contains("network"));
    ensure!(help.contains("config"));
    Ok(())
}

#[test]
fn config_command_persists_bep_settings() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-config-{}", uuid::Uuid::new_v4()));
    let set = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "config",
            "set",
            "bep.enabled",
            "true",
        ])
        .output()
        .context("run syncweb config set")?;
    ensure!(set.status.success());

    let show = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "config",
            "show",
            "bep",
        ])
        .output()
        .context("run syncweb config show")?;
    std::fs::remove_dir_all(directory).context("remove config directory")?;

    ensure!(show.status.success());
    let stdout = String::from_utf8(show.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("enabled = true"));
    Ok(())
}

#[test]
fn devices_command_displays_iroh_and_syncthing_ids() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", directory.to_str().context("UTF-8 path")?, "devices"])
        .output()
        .context("run syncweb devices")?;
    std::fs::remove_dir_all(directory).context("remove test directory")?;

    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("iroh: "));
    ensure!(stdout.contains("syncthing: "));
    Ok(())
}

#[test]
fn verbose_logging_is_structured() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--verbose", "version"])
        .output()
        .context("run verbose syncweb version")?;

    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.lines().next().is_some_and(|line| {
        line.contains("\"level\":\"DEBUG\"") && line.contains("\"message\":\"cli initialized\"")
    }));
    Ok(())
}

#[test]
fn rust_log_controls_log_level() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("version")
        .env("RUST_LOG", "syncweb=debug")
        .output()
        .context("run syncweb with RUST_LOG")?;

    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("\"level\":\"DEBUG\""));
    Ok(())
}

#[test]
fn test_create_command() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-create-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "create",
        ])
        .output()
        .context("run syncweb create")?;

    std::fs::remove_dir_all(&directory).context("remove test directory")?;

    ensure!(
        output.status.success(),
        "create should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("namespace: "), "should print namespace: {stdout}");
    ensure!(stdout.contains("ticket:"), "should print ticket: {stdout}");
    Ok(())
}

#[test]
fn test_folders_command_empty() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-folders-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "folders",
        ])
        .output()
        .context("run syncweb folders")?;

    std::fs::remove_dir_all(&directory).context("remove test directory")?;

    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        !stdout.contains("sendreceive") && !stdout.contains("sendonly") && !stdout.contains("receiveonly"),
        "no folders should be listed initially: {stdout}"
    );
    Ok(())
}

#[test]
fn test_folders_command_lists_created() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-folders2-{}", uuid::Uuid::new_v4()));
    let data_dir = directory.to_str().context("UTF-8 path")?.to_owned();

    let create_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "--no-daemon", "create"])
        .output()
        .context("run syncweb create")?;
    ensure!(create_output.status.success());

    let folders_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "--no-daemon", "folders"])
        .output()
        .context("run syncweb folders")?;

    std::fs::remove_dir_all(&directory).context("remove test directory")?;

    ensure!(folders_output.status.success());
    let stdout = String::from_utf8(folders_output.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("sendreceive"),
        "folder should show sendreceive mode: {stdout}"
    );
    Ok(())
}

#[test]
fn test_join_command() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-join-{}", uuid::Uuid::new_v4()));
    let data_dir = directory.to_str().context("UTF-8 path")?.to_owned();

    let create_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "--no-daemon", "create"])
        .output()
        .context("run syncweb create")?;
    ensure!(create_output.status.success());
    let create_stdout = String::from_utf8(create_output.stdout).context("UTF-8 output")?;
    let ticket = create_stdout
        .lines()
        .find(|line| line.starts_with("ticket: "))
        .context("should have ticket line")?
        .trim_start_matches("ticket: ")
        .trim()
        .to_owned();

    let join_dir = directory.join("join_target");
    std::fs::create_dir(&join_dir).context("create join dir")?;

    let join_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            &data_dir,
            "--no-daemon",
            "join",
            "--once",
            &ticket,
            join_dir.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run syncweb join")?;

    std::fs::remove_dir_all(&directory).context("remove test directory")?;

    ensure!(
        join_output.status.success(),
        "join should succeed: {:?}",
        String::from_utf8_lossy(&join_output.stderr)
    );
    let stdout = String::from_utf8(join_output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("joined: "), "should print joined: {stdout}");
    Ok(())
}

#[test]
fn commands_and_json_version_are_available() -> anyhow::Result<()> {
    let help = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("--help")
        .output()
        .context("run syncweb help")?;
    ensure!(help.status.success());
    let help_text = String::from_utf8(help.stdout).context("UTF-8 help")?;
    ensure!(help_text.contains("watch"));
    ensure!(help_text.contains("stats"));
    ensure!(help_text.contains("verify"));
    ensure!(help_text.contains("schedule"));

    let version = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--json", "version"])
        .output()
        .context("run syncweb --json version")?;
    ensure!(version.status.success());
    let value: serde_json::Value = serde_json::from_slice(&version.stdout)?;
    let ws_version = workspace_version()?;
    ensure!(value.get("version") == Some(&serde_json::Value::from(ws_version)));
    Ok(())
}

#[test]
fn schedule_and_stats_commands_persist_state() -> anyhow::Result<()> {
    let directory = cli_test_dir("schedule-state");
    let data_dir = directory.to_str().context("UTF-8 path")?.to_owned();
    let schedule = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "schedule", "set", "--active", "22:00-06:00"])
        .output()
        .context("run schedule set")?;
    ensure!(
        schedule.status.success(),
        "schedule set failed: {}",
        String::from_utf8_lossy(&schedule.stderr)
    );

    let stats = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "--json", "stats"])
        .output()
        .context("run stats")?;
    ensure!(stats.status.success());
    let value: serde_json::Value = serde_json::from_slice(&stats.stdout)?;
    ensure!(value.get("total_download") == Some(&serde_json::Value::from(0)));
    std::fs::remove_dir_all(directory)?;
    Ok(())
}

fn cli_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("syncweb-cli-{name}-{}", uuid::Uuid::new_v4()))
}

#[test]
fn test_ls_streaming() -> anyhow::Result<()> {
    let source = cli_test_dir("ls-streaming");
    std::fs::create_dir_all(source.join("sub")).context("create dirs")?;
    std::fs::write(source.join("a.txt"), b"a").context("write a")?;
    std::fs::write(source.join("sub/b.txt"), b"b").context("write b")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["ls", source.to_str().context("UTF-8 path")?])
        .output()
        .context("run syncweb ls")?;

    std::fs::remove_dir_all(&source).context("cleanup")?;

    ensure!(output.status.success());
    let stdout = String::from_utf8(output.stdout)
        .context("UTF-8 output")?
        .replace('\\', "/");
    let lines: Vec<&str> = stdout.lines().collect();
    anyhow::ensure!(lines.len() == 2, "should list 2 files: {stdout}");
    ensure!(lines.contains(&"a.txt"));
    ensure!(lines.contains(&"sub/b.txt"));
    Ok(())
}

#[test]
fn test_ls_sort() -> anyhow::Result<()> {
    let source = cli_test_dir("ls-sort");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("large.txt"), [0_u8; 1000]).context("write large")?;
    std::fs::write(source.join("small.txt"), b"s").context("write small")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["ls", source.to_str().context("UTF-8 path")?, "--sort", "peers"])
        .output()
        .context("run syncweb ls --sort")?;

    std::fs::remove_dir_all(&source).context("cleanup")?;

    ensure!(
        output.status.success(),
        "ls --sort should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    anyhow::ensure!(stdout.lines().count() == 2, "should list 2 files when sorted: {stdout}");
    Ok(())
}

#[test]
fn test_find_regex_glob_exact() -> anyhow::Result<()> {
    let source = cli_test_dir("find-modes");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("report-01.pdf"), b"r").context("write report")?;
    std::fs::write(source.join("data.txt"), b"d").context("write data")?;

    let output_regex = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "find",
            r"report-\d+\.pdf",
            source.to_str().context("UTF-8 path")?,
            "--kind",
            "regex",
        ])
        .output()
        .context("run syncweb find regex")?;
    ensure!(output_regex.status.success());
    let stdout = String::from_utf8(output_regex.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("report-01.pdf"), "regex should find report: {stdout}");

    let output_glob = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "find",
            "*.txt",
            source.to_str().context("UTF-8 path")?,
            "--kind",
            "glob",
        ])
        .output()
        .context("run syncweb find glob")?;
    ensure!(output_glob.status.success());
    let stdout_glob = String::from_utf8(output_glob.stdout).context("UTF-8 output")?;
    ensure!(
        stdout_glob.contains("data.txt"),
        "glob should find data.txt: {stdout_glob}"
    );

    let output_exact = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "find",
            "data",
            source.to_str().context("UTF-8 path")?,
            "--kind",
            "exact",
        ])
        .output()
        .context("run syncweb find exact")?;
    ensure!(output_exact.status.success());
    let stdout_exact = String::from_utf8(output_exact.stdout).context("UTF-8 output")?;
    ensure!(
        stdout_exact.contains("data.txt"),
        "exact should find data.txt: {stdout_exact}"
    );

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn test_sort_algorithms() -> anyhow::Result<()> {
    let source = cli_test_dir("sort-algo");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("a.txt"), b"a").context("write")?;
    std::fs::write(source.join("b.txt"), b"b").context("write")?;

    for algorithm in ["niche", "frecency", "peers", "random", "folder"] {
        let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
            .args(["sort", source.to_str().context("UTF-8 path")?, "--by", algorithm])
            .output()
            .context("run syncweb sort")?;
        ensure!(
            output.status.success(),
            "sort --by {algorithm} should succeed: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
        anyhow::ensure!(
            stdout.lines().count() == 2,
            "sort {algorithm} should list 2 files: {stdout}"
        );
    }

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn test_stat_detailed() -> anyhow::Result<()> {
    let source = cli_test_dir("stat-detail");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("file.txt"), b"hello world").context("write")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["stat", source.join("file.txt").to_str().context("UTF-8 path")?])
        .output()
        .context("run syncweb stat")?;

    std::fs::remove_dir_all(&source).context("cleanup")?;

    ensure!(
        output.status.success(),
        "stat should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("Path:"), "should show Path: {stdout}");
    ensure!(stdout.contains("Size: 11"), "should show Size: 11 {stdout}");
    ensure!(stdout.contains("Hash:"), "should show Hash: {stdout}");
    ensure!(
        stdout.contains("Available: true"),
        "should show Available: true {stdout}"
    );

    let output_terse = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "stat",
            source.join("file.txt").to_str().context("UTF-8 path")?,
            "--terse",
        ])
        .output()
        .context("run syncweb stat --terse")?;

    if output_terse.status.success() {
        let stdout_terse = String::from_utf8(output_terse.stdout).context("UTF-8 output")?;
        ensure!(stdout_terse.contains("11"), "terse should contain size: {stdout_terse}");
    }
    Ok(())
}

#[test]
fn test_download_selective() -> anyhow::Result<()> {
    let source = cli_test_dir("download-src");
    let dest = cli_test_dir("download-dest");
    std::fs::create_dir_all(source.join("sub")).context("create dirs")?;
    std::fs::write(source.join("keep.txt"), b"keep").context("write")?;
    std::fs::write(source.join("sub/nested.txt"), b"nested").context("write")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "download",
            source.join("keep.txt").to_str().context("UTF-8 path")?,
            dest.join("copied.txt").to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run syncweb download single")?;

    ensure!(
        output.status.success(),
        "download single file should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    ensure!(dest.join("copied.txt").exists());
    anyhow::ensure!(std::fs::read(dest.join("copied.txt")).context("read")? == b"keep");

    let dir_dest = cli_test_dir("download-dir-dest");
    let output_dir = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "download",
            source.to_str().context("UTF-8 path")?,
            dir_dest.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run syncweb download directory")?;

    ensure!(
        output_dir.status.success(),
        "download directory should succeed: {:?}",
        String::from_utf8_lossy(&output_dir.stderr)
    );
    ensure!(dir_dest.join("keep.txt").exists());
    ensure!(dir_dest.join("sub/nested.txt").exists());

    std::fs::remove_dir_all(&source).context("cleanup source")?;
    std::fs::remove_dir_all(&dest).context("cleanup dest")?;
    let _ = std::fs::remove_dir_all(&dir_dest);
    Ok(())
}

#[test]
fn download_auto_starts_daemon_when_not_running() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("download-auto-daemon");
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    let download = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg, "download", "not-a-namespace"])
        .output()
        .context("run daemon-routed download")?;
    let status = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg, "status"])
        .output()
        .context("query auto-started daemon")?;
    let shutdown = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg, "shutdown", "--force"])
        .output()
        .context("stop auto-started daemon")?;
    std::fs::remove_dir_all(&data_dir).context("cleanup auto-started daemon")?;

    ensure!(!download.status.success());
    ensure!(
        String::from_utf8(download.stderr)
            .context("download UTF-8 error")?
            .contains("invalid download namespace")
    );
    ensure!(status.status.success());
    ensure!(
        String::from_utf8(status.stdout)
            .context("status UTF-8 error")?
            .contains("daemon: running")
    );
    ensure!(shutdown.status.success());
    Ok(())
}

#[test]
fn test_init_outputs_url() -> anyhow::Result<()> {
    let directory = cli_test_dir("init-test");
    let data_dir = cli_test_dir("init-data");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "init",
            directory.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run syncweb init")?;

    std::fs::remove_dir_all(&directory).context("cleanup folder")?;
    std::fs::remove_dir_all(&data_dir).context("cleanup data")?;

    ensure!(
        output.status.success(),
        "init should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("path:"), "should print path: {stdout}");
    ensure!(stdout.contains("namespace:"), "should print namespace: {stdout}");
    ensure!(stdout.contains("ticket:"), "should print ticket: {stdout}");
    ensure!(stdout.contains("share_url:"), "should print share_url: {stdout}");
    Ok(())
}

#[test]
fn network_commands_are_available() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("--help")
        .output()
        .context("run syncweb help")?;
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("automatic"));
    ensure!(help.contains("subscribe"));

    let network = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["network", "--help"])
        .output()
        .context("run network help")?;
    let network_help = String::from_utf8(network.stdout).context("UTF-8 output")?;
    for command in ["create", "ls", "join", "leave", "invite", "kick"] {
        ensure!(network_help.contains(command));
    }
    Ok(())
}

#[test]
fn network_create_and_list_persist() -> anyhow::Result<()> {
    let directory = cli_test_dir("network");
    let data_dir = directory.to_str().context("UTF-8 path")?;
    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "work"])
        .output()
        .context("create network")?;
    ensure!(
        create.status.success(),
        "network create failed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let list = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "ls"])
        .output()
        .context("list networks")?;
    std::fs::remove_dir_all(directory).context("cleanup")?;
    ensure!(list.status.success());
    ensure!(String::from_utf8(list.stdout).context("UTF-8 output")?.contains("work"));
    Ok(())
}

#[test]
fn automatic_dry_run_uses_filter_engine() -> anyhow::Result<()> {
    let directory = cli_test_dir("automatic");
    std::fs::create_dir_all(&directory).context("create directory")?;
    std::fs::write(directory.join("file.txt"), b"data").context("write file")?;
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "automatic",
            "--dry-run",
            "--paths",
            directory.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run automatic dry-run")?;
    std::fs::remove_dir_all(directory).context("cleanup")?;
    ensure!(output.status.success());
    ensure!(
        String::from_utf8(output.stdout)
            .context("UTF-8 output")?
            .contains("accept")
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CLI command coverage
// ---------------------------------------------------------------------------

#[test]
fn subscribe_help_lists_options() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["subscribe", "--help"])
        .output()
        .context("run subscribe --help")?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("ingest-only"), "should list ingest-only: {help}");
    ensure!(help.contains("ignore-self"), "should list ignore-self: {help}");
    ensure!(help.contains("prefix"), "should list prefix: {help}");
    ensure!(help.contains("glob"), "should list glob: {help}");
    ensure!(help.contains("max-count"), "should list max-count: {help}");
    ensure!(help.contains("max-size"), "should list max-size: {help}");
    Ok(())
}

#[test]
fn automatic_help_lists_filters_and_dry_run() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["automatic", "--help"])
        .output()
        .context("run automatic --help")?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("show-filters"), "should list show-filters: {help}");
    ensure!(help.contains("dry-run"), "should list dry-run: {help}");
    ensure!(help.contains("paths"), "should list paths: {help}");
    ensure!(help.contains("filters"), "should list filters path: {help}");
    Ok(())
}

#[test]
fn network_create_with_label_and_invite_only() -> anyhow::Result<()> {
    let directory = cli_test_dir("net-create-opts");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir,
            "network",
            "create",
            "secure-net",
            "--label",
            "Secure",
            "--invite-only",
        ])
        .output()
        .context("create network with options")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(
        output.status.success(),
        "network create --label --invite-only should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("created:"), "should print created: {stdout}");
    ensure!(stdout.contains("secure-net"), "should contain network name: {stdout}");
    Ok(())
}

#[test]
fn network_list_inspects_single_network() -> anyhow::Result<()> {
    let directory = cli_test_dir("net-list-inspect");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "inspect-me"])
        .output()
        .context("create network")?;
    ensure!(create.status.success());

    let list = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "ls", "inspect-me"])
        .output()
        .context("inspect network")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(list.status.success());
    let stdout = String::from_utf8(list.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("inspect-me"), "should show network name: {stdout}");
    ensure!(
        stdout.contains("Members") || stdout.contains("members"),
        "should show members count: {stdout}"
    );
    ensure!(
        stdout.contains("Folders") || stdout.contains("folders"),
        "should show folders count: {stdout}"
    );
    Ok(())
}

#[test]
fn network_invite_outputs_ticket() -> anyhow::Result<()> {
    let directory = cli_test_dir("net-invite");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "invite-net"])
        .output()
        .context("create network")?;
    ensure!(create.status.success());

    let invite = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "invite", "invite-net"])
        .output()
        .context("invite to network")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(
        invite.status.success(),
        "network invite should succeed: {}",
        String::from_utf8_lossy(&invite.stderr)
    );
    let stdout = String::from_utf8(invite.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("syncweb://network/"),
        "should output a ticket URL: {stdout}"
    );
    Ok(())
}

#[test]
fn network_kick_nonexistent_device_fails() -> anyhow::Result<()> {
    let directory = cli_test_dir("net-kick");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "kick-net"])
        .output()
        .context("create network")?;
    ensure!(create.status.success());

    let kick = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir,
            "network",
            "kick",
            "kick-net",
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        ])
        .output()
        .context("kick from network")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(!kick.status.success(), "kicking a non-member should fail");
    Ok(())
}

#[test]
fn network_leave_removes_from_list() -> anyhow::Result<()> {
    let directory = cli_test_dir("net-leave");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "leave-net"])
        .output()
        .context("create network")?;
    ensure!(create.status.success());

    let leave = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "leave", "leave-net"])
        .output()
        .context("leave network")?;
    ensure!(
        leave.status.success(),
        "network leave should succeed: {}",
        String::from_utf8_lossy(&leave.stderr)
    );

    let list = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "ls"])
        .output()
        .context("list after leave")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(list.status.success());
    let stdout = String::from_utf8(list.stdout).context("UTF-8 output")?;
    ensure!(
        !stdout.contains("leave-net"),
        "network should be gone after leave: {stdout}"
    );
    Ok(())
}

#[test]
fn network_join_invalid_ticket_fails() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["network", "join", "not-a-valid-ticket"])
        .output()
        .context("join with invalid ticket")?;

    ensure!(!output.status.success(), "joining with invalid ticket should fail");
    Ok(())
}

#[test]
fn create_with_network_flag_adds_folder_to_network() -> anyhow::Result<()> {
    let directory = cli_test_dir("create-network");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let net = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "team-net"])
        .output()
        .context("create network")?;
    ensure!(net.status.success());

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "--no-daemon", "create", "--network", "team-net"])
        .output()
        .context("create with --network")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(
        create.status.success(),
        "create --network should succeed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("namespace:"), "should print namespace: {stdout}");
    Ok(())
}

#[test]
fn join_with_network_flag_adds_folder_to_network() -> anyhow::Result<()> {
    let directory = cli_test_dir("join-network");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let net = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "join-net"])
        .output()
        .context("create network")?;
    ensure!(net.status.success());

    let create = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "--no-daemon", "create"])
        .output()
        .context("create folder for ticket")?;
    ensure!(create.status.success());
    let stdout = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let ticket = stdout
        .lines()
        .find(|l| l.starts_with("ticket: "))
        .context("should have ticket line")?
        .trim_start_matches("ticket: ")
        .trim()
        .to_owned();

    let join_dir = directory.join("join_target");
    std::fs::create_dir(&join_dir).context("create join dir")?;

    let join = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir,
            "--no-daemon",
            "join",
            "--once",
            &ticket,
            join_dir.to_str().context("UTF-8 path")?,
            "--network",
            "join-net",
        ])
        .output()
        .context("join with --network")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(
        join.status.success(),
        "join --network should succeed: {}",
        String::from_utf8_lossy(&join.stderr)
    );
    let join_stdout = String::from_utf8(join.stdout).context("UTF-8 output")?;
    ensure!(join_stdout.contains("joined:"), "should print joined: {join_stdout}");
    Ok(())
}

#[test]
fn network_duplicate_name_rejected() -> anyhow::Result<()> {
    let directory = cli_test_dir("net-dup");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let first = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "dup-net"])
        .output()
        .context("first create")?;
    ensure!(first.status.success());

    let second = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir, "network", "create", "dup-net"])
        .output()
        .context("second create")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(!second.status.success(), "duplicate network name should be rejected");
    Ok(())
}

#[test]
fn automatic_show_filters_empty_config() -> anyhow::Result<()> {
    let directory = cli_test_dir("auto-show");
    std::fs::create_dir_all(&directory).context("create directory")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "automatic",
            "--show-filters",
            "--filters",
            directory.join("nonexistent.toml").to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run automatic --show-filters")?;
    std::fs::remove_dir_all(&directory).context("cleanup")?;

    ensure!(
        output.status.success(),
        "show-filters should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("rules"), "should show rules table: {stdout}");
    Ok(())
}

#[test]
fn completions_generates_valid_bash_output() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["completions", "bash"])
        .output()
        .context("run syncweb completions bash")?;
    ensure!(
        output.status.success(),
        "completions bash should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("syncweb"),
        "bash completions should reference syncweb: {stdout}"
    );
    ensure!(
        stdout.contains("complete"),
        "bash completions should contain complete keyword: {stdout}"
    );
    Ok(())
}

#[test]
fn completions_generates_valid_zsh_output() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["completions", "zsh"])
        .output()
        .context("run syncweb completions zsh")?;
    ensure!(
        output.status.success(),
        "completions zsh should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("syncweb"),
        "zsh completions should reference syncweb: {stdout}"
    );
    Ok(())
}

#[test]
fn completions_generates_valid_fish_output() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["completions", "fish"])
        .output()
        .context("run syncweb completions fish")?;
    ensure!(
        output.status.success(),
        "completions fish should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("syncweb"),
        "fish completions should reference syncweb: {stdout}"
    );
    Ok(())
}

#[test]
fn completions_generates_valid_powershell_output() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["completions", "powershell"])
        .output()
        .context("run syncweb completions powershell")?;
    ensure!(
        output.status.success(),
        "completions powershell should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("syncweb"),
        "powershell completions should reference syncweb: {stdout}"
    );
    Ok(())
}

#[test]
fn trust_provider_list_outputs_empty_table() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-trust-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "trust",
            "provider",
            "list",
        ])
        .output()
        .context("run syncweb trust provider list")?;
    let _ = std::fs::remove_dir_all(&directory);
    ensure!(
        output.status.success(),
        "trust provider list should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn trust_provider_ban_and_unban() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-trust-ban-{}", uuid::Uuid::new_v4()));
    let fake_key = "aabbccdd".repeat(8);
    let ban = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "trust",
            "provider",
            "ban",
            &fake_key,
            "--reason",
            "test ban",
        ])
        .output()
        .context("run syncweb trust provider ban")?;
    ensure!(
        ban.status.success(),
        "trust provider ban should succeed: {:?}",
        String::from_utf8_lossy(&ban.stderr)
    );
    let unban = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "trust",
            "provider",
            "unban",
            &fake_key,
        ])
        .output()
        .context("run syncweb trust provider unban")?;
    let _ = std::fs::remove_dir_all(&directory);
    ensure!(
        unban.status.success(),
        "trust provider unban should succeed: {:?}",
        String::from_utf8_lossy(&unban.stderr)
    );
    Ok(())
}

#[test]
fn trust_provider_show_displays_output() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-trust-show-{}", uuid::Uuid::new_v4()));
    let fake_key = "11223344".repeat(8);
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "trust",
            "provider",
            "show",
            &fake_key,
        ])
        .output()
        .context("run syncweb trust provider show")?;
    let _ = std::fs::remove_dir_all(&directory);
    ensure!(
        output.status.success(),
        "trust provider show should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn trust_provider_vouch_and_distrust() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-trust-vouch-{}", uuid::Uuid::new_v4()));
    let key = iroh::SecretKey::generate();
    let fake_key = hex::encode(key.public().as_bytes());
    let vouch = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "trust",
            "provider",
            "vouch",
            &fake_key,
            "--reason",
            "good provider",
        ])
        .output()
        .context("run syncweb trust provider vouch")?;
    ensure!(
        vouch.status.success(),
        "trust provider vouch should succeed: {:?}",
        String::from_utf8_lossy(&vouch.stderr)
    );
    let distrust = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().context("UTF-8 path")?,
            "trust",
            "provider",
            "distrust",
            &fake_key,
            "--reason",
            "bad provider",
        ])
        .output()
        .context("run syncweb trust provider distrust")?;
    let _ = std::fs::remove_dir_all(&directory);
    ensure!(
        distrust.status.success(),
        "trust provider distrust should succeed: {:?}",
        String::from_utf8_lossy(&distrust.stderr)
    );
    Ok(())
}

#[test]
fn trust_stream_publish_help() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["trust", "stream", "publish", "--help"])
        .output()
        .context("run syncweb trust stream publish --help")?;
    ensure!(output.status.success(), "trust stream publish --help should succeed");
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.contains("--provider"), "help should list --provider flag");
    Ok(())
}

#[test]
fn test_provider_add_with_valid_ticket() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("provider-add");
    let hash = Hash::from_bytes([2_u8; 32]);
    let ticket = ticket_for_test(hash);
    let ticket_str = ticket.to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "provider",
            "add",
            &hash.to_string(),
            &ticket_str,
        ])
        .output()
        .context("run syncweb provider add")?;
    ensure!(
        output.status.success(),
        "provider add should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        stdout.contains("provider added"),
        "output should indicate provider added"
    );

    let state_path = data_dir.join("indexing-state.json");
    ensure!(state_path.exists(), "indexing-state.json should exist");
    let state: serde_json::Value = serde_json::from_slice(&std::fs::read(&state_path).context("read indexing state")?)?;
    let mirrors = state
        .get("links")
        .and_then(|l| l.get("mirrors"))
        .and_then(|m| m.as_array())
        .context("mirrors should be an array")?;
    ensure!(!mirrors.is_empty(), "should have at least one mirror");

    std::fs::remove_dir_all(&data_dir)?;
    Ok(())
}
