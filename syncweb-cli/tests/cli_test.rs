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
fn publish_aliases_are_not_available() -> anyhow::Result<()> {
    for args in [["collection", "publish", "--help"], ["indexing", "publish", "--help"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
            .args(args)
            .output()
            .with_context(|| format!("run syncweb {args:?}"))?;
        ensure!(!output.status.success(), "removed alias should fail: {args:?}");
    }
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
    let stderr = String::from_utf8(output.stderr).context("UTF-8 output")?;
    ensure!(stderr.lines().next().is_some_and(|line| {
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
    let stderr = String::from_utf8(output.stderr).context("UTF-8 output")?;
    ensure!(stderr.contains("\"level\":\"DEBUG\""));
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
    ensure!(help_text.contains("config"));

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
        .args([
            "--data-dir",
            &data_dir,
            "config",
            "schedule",
            "set",
            "--active",
            "22:00-06:00",
        ])
        .output()
        .context("run config schedule set")?;
    ensure!(
        schedule.status.success(),
        "config schedule set failed: {}",
        String::from_utf8_lossy(&schedule.stderr)
    );

    let stats = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "--json", "stats", "network"])
        .output()
        .context("run stats network")?;
    ensure!(stats.status.success());
    let value: serde_json::Value = serde_json::from_slice(&stats.stdout)?;
    ensure!(value.get("total_download") == Some(&serde_json::Value::from(0)));
    std::fs::remove_dir_all(directory)?;
    Ok(())
}

fn cli_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("syncweb-cli-{name}-{}", uuid::Uuid::new_v4()))
}

fn syncweb(args: &[&str]) -> anyhow::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(args)
        .output()
        .with_context(|| format!("run syncweb {args:?}"))
}

#[test]
fn stats_network_filters_and_reset() -> anyhow::Result<()> {
    let directory = cli_test_dir("stats-network-filter");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let stats_db = syncweb_core::storage::stats_db::StatsDatabase::open(directory.join("default").join("stats.db"))?;
    stats_db.record_download(1024, 1, Some("folderA"), Some("peer1"), None)?;
    stats_db.record_download(2048, 2, Some("folderA"), Some("peer1"), None)?;
    stats_db.record_upload(512, 1, Some("folderA"), Some("peer1"), None)?;
    drop(stats_db);

    let before = syncweb(&["--data-dir", data_dir, "--json", "stats", "network"])?;
    ensure!(
        before.status.success(),
        "stats network before reset: {}",
        String::from_utf8_lossy(&before.stderr)
    );
    let before_json: serde_json::Value = serde_json::from_slice(&before.stdout)?;
    ensure!(
        before_json.get("total_download") == Some(&serde_json::Value::from(3072)),
        "total_download before reset: {before_json}"
    );

    let filtered = syncweb(&[
        "--data-dir",
        data_dir,
        "stats",
        "network",
        "--folder",
        "folderA",
        "--peer",
        "peer1",
        "--period",
        "1d",
    ])?;
    ensure!(
        filtered.status.success(),
        "stats network with filters: {}",
        String::from_utf8_lossy(&filtered.stderr)
    );
    let filtered_out = String::from_utf8(filtered.stdout)?;
    ensure!(
        filtered_out.contains("total_download"),
        "filtered stats output: {filtered_out}"
    );

    let reset = syncweb(&["--data-dir", data_dir, "--json", "stats", "network", "--reset"])?;
    ensure!(
        reset.status.success(),
        "stats network --reset: {}",
        String::from_utf8_lossy(&reset.stderr)
    );
    let after: serde_json::Value = serde_json::from_slice(&reset.stdout)?;
    ensure!(
        after.get("total_download") == Some(&serde_json::Value::from(0)),
        "total_download after reset: {after}"
    );

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn config_schedule_bandwidth_and_period() -> anyhow::Result<()> {
    let directory = cli_test_dir("config-schedule-bandwidth");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let set = syncweb(&[
        "--data-dir",
        data_dir,
        "config",
        "schedule",
        "set",
        "--active",
        "08:00-18:00",
        "--bandwidth",
        "2M",
        "--period",
        "08:00-18:00",
    ])?;
    ensure!(
        set.status.success(),
        "config schedule set bandwidth: {}",
        String::from_utf8_lossy(&set.stderr)
    );

    let show = syncweb(&["--data-dir", data_dir, "config", "show", "schedule"])?;
    ensure!(show.status.success());
    let stdout = String::from_utf8(show.stdout)?;
    ensure!(
        stdout.contains("08:00-18:00"),
        "schedule active hours persisted: {stdout}"
    );
    ensure!(stdout.contains("2M"), "schedule bandwidth rate persisted: {stdout}");
    ensure!(stdout.contains("[[bandwidth]]"), "bandwidth window persisted: {stdout}");

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn config_schedule_folder_override() -> anyhow::Result<()> {
    let directory = cli_test_dir("config-schedule-folder");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let set = syncweb(&[
        "--data-dir",
        data_dir,
        "config",
        "schedule",
        "folder",
        "project",
        "--active",
        "09:00-17:00",
        "--max-upload",
        "500K",
        "--max-download",
        "1M",
    ])?;
    ensure!(
        set.status.success(),
        "config schedule folder: {}",
        String::from_utf8_lossy(&set.stderr)
    );

    let show = syncweb(&["--data-dir", data_dir, "config", "show", "schedule"])?;
    ensure!(show.status.success());
    let stdout = String::from_utf8(show.stdout)?;
    ensure!(
        stdout.contains("[folders.project]"),
        "folder override persisted: {stdout}"
    );
    ensure!(stdout.contains("500K"), "max_upload persisted: {stdout}");
    ensure!(stdout.contains("1M"), "max_download persisted: {stdout}");

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn config_no_subcommand_prints_full_toml() -> anyhow::Result<()> {
    let directory = cli_test_dir("config-full-toml");
    let data_dir = directory.to_str().context("UTF-8 path")?;

    let output = syncweb(&["--data-dir", data_dir, "config"])?;
    ensure!(
        output.status.success(),
        "bare config: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    ensure!(stdout.contains("auto_fallback"), "should print bep settings: {stdout}");
    ensure!(stdout.contains("[schedule]"), "should print schedule section: {stdout}");
    ensure!(
        stdout.contains("[discovery]"),
        "should print discovery section: {stdout}"
    );
    ensure!(
        stdout.contains("[subscribe.folders]"),
        "should print subscribe section: {stdout}"
    );

    std::fs::remove_dir_all(directory)?;
    Ok(())
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
fn test_sort_with_enrich_flag() -> anyhow::Result<()> {
    let source = cli_test_dir("sort-enrich");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("a.txt"), b"content_a").context("write")?;
    std::fs::write(source.join("b.txt"), b"content_b").context("write")?;

    // --enrich with no daemon should warn and fall back to local-only sorting
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "sort",
            source.to_str().context("UTF-8 path")?,
            "--by",
            "peers",
            "--enrich",
        ])
        .output()
        .context("run syncweb sort --by peers --enrich")?;
    ensure!(
        output.status.success(),
        "sort --enrich should still succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

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
#[cfg(unix)]
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
    for _ in 0..50 {
        let poll = Command::new(env!("CARGO_BIN_EXE_syncweb"))
            .args(["--data-dir", data_dir_arg, "status"])
            .output()?;
        if poll.status.success() && String::from_utf8_lossy(&poll.stdout).contains("daemon not running") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    std::fs::remove_dir_all(&data_dir).context("cleanup auto-started daemon")?;

    ensure!(!download.status.success());
    let download_stderr = String::from_utf8(download.stderr).context("download UTF-8 error")?;
    ensure!(
        download_stderr.contains("invalid download namespace"),
        "expected 'invalid download namespace' in stderr, got: {download_stderr:?}"
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
fn test_create_outputs_url() -> anyhow::Result<()> {
    let directory = cli_test_dir("create-test");
    let data_dir = cli_test_dir("create-data");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "create",
            directory.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run syncweb create")?;

    std::fs::remove_dir_all(&directory).context("cleanup folder")?;
    std::fs::remove_dir_all(&data_dir).context("cleanup data")?;

    ensure!(
        output.status.success(),
        "create should succeed: {:?}",
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
    ensure!(help.contains("watch"));
    ensure!(help.contains("join"));
    ensure!(help.contains("leave"));

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
fn watch_dry_run_uses_filter_engine() -> anyhow::Result<()> {
    let directory = cli_test_dir("watch");
    std::fs::create_dir_all(&directory).context("create directory")?;
    std::fs::write(directory.join("file.txt"), b"data").context("write file")?;
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "watch",
            "--dry-run",
            "--paths",
            directory.to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run watch dry-run")?;
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
        .args(["join", "--help"])
        .output()
        .context("run join --help")?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--subscribe"), "should list --subscribe: {help}");
    ensure!(help.contains("ingest-only"), "should list ingest-only: {help}");
    ensure!(help.contains("ignore-self"), "should list ignore-self: {help}");
    ensure!(help.contains("sync-prefix"), "should list sync-prefix: {help}");
    ensure!(help.contains("glob"), "should list glob: {help}");
    ensure!(help.contains("max-count"), "should list max-count: {help}");
    ensure!(help.contains("max-size"), "should list max-size: {help}");
    Ok(())
}

#[test]
fn watch_help_lists_filters_and_dry_run() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["watch", "--help"])
        .output()
        .context("run watch --help")?;
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
fn watch_show_filters_empty_config() -> anyhow::Result<()> {
    let directory = cli_test_dir("auto-show");
    std::fs::create_dir_all(&directory).context("create directory")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "watch",
            "--show-filters",
            "--filters",
            directory.join("nonexistent.toml").to_str().context("UTF-8 path")?,
        ])
        .output()
        .context("run watch --show-filters")?;
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

    let result = {
        let db = syncweb_core::indexing::IndexingDatabase::open(data_dir.join("default").join("indexing.sqlite"))?;
        let (_pointers, mirrors, _revoked) = db.load_links()?;
        ensure!(!mirrors.is_empty(), "should have at least one mirror");
        Ok(())
    };
    std::fs::remove_dir_all(&data_dir)?;
    result
}

#[test]
fn trust_provider_vouch_help_shows_broadcast_flag() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["trust", "provider", "vouch", "--help"])
        .output()
        .context("run syncweb trust provider vouch --help")?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--broadcast"), "vouch help should show --broadcast flag");
    ensure!(
        help.contains("Broadcast vouch via gossip trust stream"),
        "vouch help should describe --broadcast"
    );
    Ok(())
}

#[test]
fn trust_provider_distrust_help_shows_broadcast_flag() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["trust", "provider", "distrust", "--help"])
        .output()
        .context("run syncweb trust provider distrust --help")?;
    ensure!(output.status.success());
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(
        help.contains("--broadcast"),
        "distrust help should show --broadcast flag"
    );
    Ok(())
}

#[test]
fn trust_provider_vouch_with_broadcast() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-vouch-broadcast-{}", uuid::Uuid::new_v4()));
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
            "--broadcast",
        ])
        .output()
        .context("run syncweb trust provider vouch --broadcast")?;
    let _ = std::fs::remove_dir_all(&directory);
    ensure!(
        vouch.status.success(),
        "trust provider vouch --broadcast should succeed: {:?}",
        String::from_utf8_lossy(&vouch.stderr)
    );
    Ok(())
}

#[test]
fn trust_provider_distrust_with_broadcast() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-distrust-broadcast-{}", uuid::Uuid::new_v4()));
    let key = iroh::SecretKey::generate();
    let fake_key = hex::encode(key.public().as_bytes());
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
            "--broadcast",
        ])
        .output()
        .context("run syncweb trust provider distrust --broadcast")?;
    let _ = std::fs::remove_dir_all(&directory);
    ensure!(
        distrust.status.success(),
        "trust provider distrust --broadcast should succeed: {:?}",
        String::from_utf8_lossy(&distrust.stderr)
    );
    Ok(())
}

#[test]
fn trust_provider_vouch_without_broadcast_still_local() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-vouch-local-{}", uuid::Uuid::new_v4()));
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
        .context("run syncweb trust provider vouch (no broadcast)")?;
    ensure!(
        vouch.status.success(),
        "trust provider vouch without --broadcast should succeed: {:?}",
        String::from_utf8_lossy(&vouch.stderr)
    );
    let _ = std::fs::remove_dir_all(&directory);
    Ok(())
}

#[test]
fn mirror_help_output_lists_mirror_command() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["mirror", "--help"])
        .output()
        .context("run syncweb mirror --help")?;
    ensure!(output.status.success(), "mirror --help should succeed");
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("provider"), "help should mention provider");
    ensure!(help.contains("--network"), "help should mention --network");
    ensure!(help.contains("--dry-run"), "help should mention --dry-run");
    ensure!(help.contains("--no-sharing"), "help should mention --no-sharing");
    Ok(())
}

#[test]
fn mirror_without_args_fails_gracefully() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["mirror"])
        .output()
        .context("run syncweb mirror without args")?;
    ensure!(!output.status.success(), "mirror without args should fail");
    Ok(())
}

#[test]
fn link_create_help_lists_untested_options() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["link", "create", "--help"])
        .output()
        .context("run syncweb link create --help")?;
    ensure!(output.status.success(), "link create --help should succeed");
    let help = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(help.contains("--version"), "help should mention --version");
    ensure!(help.contains("--sequence"), "help should mention --sequence");
    ensure!(help.contains("--expires"), "help should mention --expires");
    ensure!(help.contains("--publish"), "help should mention --publish");
    Ok(())
}

// ---------------------------------------------------------------------------
// Plan 003 — Files coverage (ls / find / sort / stat / download / import / verify)
// ---------------------------------------------------------------------------

#[test]
fn find_case_sensitivity_and_fixed_strings() -> anyhow::Result<()> {
    let source = cli_test_dir("find-case");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("Report.TXT"), b"r").context("write Report")?;
    std::fs::write(source.join("data.txt"), b"d").context("write data")?;
    std::fs::write(source.join("fix*.txt"), b"f").context("write literal asterisk name")?;
    let src = source.to_str().context("UTF-8 path")?;

    let ci = syncweb(&["find", "REPORT", src, "--kind", "exact", "-i"])?;
    ensure!(
        ci.status.success(),
        "find -i should succeed: {}",
        String::from_utf8_lossy(&ci.stderr)
    );
    let out_insensitive = String::from_utf8(ci.stdout).context("UTF-8 output")?;
    ensure!(
        out_insensitive.contains("Report.TXT"),
        "-i should match case-insensitively: {out_insensitive}"
    );

    let cs = syncweb(&["find", "report", src, "--kind", "exact", "-s"])?;
    ensure!(cs.status.success());
    let out_sensitive = String::from_utf8(cs.stdout).context("UTF-8 output")?;
    ensure!(
        !out_sensitive.contains("Report.TXT"),
        "-s should only match exact case: {out_sensitive}"
    );

    let fixed = syncweb(&["find", "*.txt", src, "-F", "--kind", "exact"])?;
    ensure!(fixed.status.success());
    let out_fixed = String::from_utf8(fixed.stdout).context("UTF-8 output")?;
    ensure!(
        out_fixed.contains("fix*.txt"),
        "-F should match the literal name: {out_fixed}"
    );
    ensure!(!out_fixed.contains("data.txt"), "-F should not glob-match: {out_fixed}");

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn find_path_hidden_and_absolute() -> anyhow::Result<()> {
    let source = cli_test_dir("find-path");
    std::fs::create_dir_all(source.join("project-2024/sub")).context("create dirs")?;
    std::fs::write(source.join("project-2024/sub/file.md"), b"m").context("write")?;
    std::fs::write(source.join(".hidden.txt"), b"h").context("write hidden")?;
    let src = source.to_str().context("UTF-8 path")?;

    let full = syncweb(&["find", "2024", src, "-p", "--kind", "exact"])?;
    ensure!(full.status.success());
    let out_full = String::from_utf8(full.stdout).context("UTF-8 output")?;
    ensure!(
        out_full.contains("file.md"),
        "-p should match the full relative path: {out_full}"
    );

    let hidden = syncweb(&["find", "*", src, "-H"])?;
    ensure!(hidden.status.success());
    let out_hidden = String::from_utf8(hidden.stdout).context("UTF-8 output")?;
    ensure!(
        out_hidden.contains(".hidden.txt"),
        "-H should include hidden files: {out_hidden}"
    );

    let default = syncweb(&["find", "*", src])?;
    ensure!(default.status.success());
    let out_default = String::from_utf8(default.stdout).context("UTF-8 output")?;
    ensure!(
        !out_default.contains(".hidden.txt"),
        "default find should exclude hidden files: {out_default}"
    );

    let abs = syncweb(&["find", "file.md", src, "-a"])?;
    ensure!(abs.status.success());
    let out_abs = String::from_utf8(abs.stdout).context("UTF-8 output")?;
    let absolute = source.join("project-2024/sub/file.md");
    ensure!(
        out_abs.contains(&absolute.to_string_lossy().into_owned()),
        "-a should print absolute paths: {out_abs}"
    );

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn find_depth_and_size_constraints() -> anyhow::Result<()> {
    let source = cli_test_dir("find-depth");
    std::fs::create_dir_all(source.join("sub/deep")).context("create dirs")?;
    std::fs::write(source.join("a.txt"), b"a").context("write a")?;
    std::fs::write(source.join("sub/b.txt"), [0_u8; 100]).context("write b")?;
    std::fs::write(source.join("sub/deep/c.txt"), [0_u8; 1000]).context("write c")?;
    let src = source.to_str().context("UTF-8 path")?;

    let depth = syncweb(&["find", "*", src, "--depth=-1"])?;
    ensure!(depth.status.success());
    let out_depth = String::from_utf8(depth.stdout).context("UTF-8 output")?;
    ensure!(
        out_depth.contains("a.txt"),
        "--depth=-1 should include a.txt: {out_depth}"
    );
    ensure!(
        !out_depth.contains("b.txt"),
        "--depth=-1 should exclude sub/b.txt: {out_depth}"
    );

    let min = syncweb(&["find", "*", src, "--min-depth", "2"])?;
    ensure!(min.status.success());
    let out_min = String::from_utf8(min.stdout).context("UTF-8 output")?;
    ensure!(
        out_min.contains("b.txt"),
        "--min-depth 2 should include b.txt: {out_min}"
    );
    ensure!(
        !out_min.contains("a.txt"),
        "--min-depth 2 should exclude a.txt: {out_min}"
    );

    let min_size = syncweb(&["find", "*", src, "--sizes", "+50"])?;
    ensure!(min_size.status.success());
    let out_min_size = String::from_utf8(min_size.stdout).context("UTF-8 output")?;
    ensure!(
        out_min_size.contains("b.txt") && out_min_size.contains("c.txt"),
        "--sizes +50 should include b and c: {out_min_size}"
    );
    ensure!(
        !out_min_size.contains("a.txt"),
        "--sizes +50 should exclude a.txt: {out_min_size}"
    );

    let max_size = syncweb(&["find", "*", src, "--sizes=-500"])?;
    ensure!(max_size.status.success());
    let out_max_size = String::from_utf8(max_size.stdout).context("UTF-8 output")?;
    ensure!(
        out_max_size.contains("a.txt") && out_max_size.contains("b.txt"),
        "--sizes=-500 should include a and b: {out_max_size}"
    );
    ensure!(
        !out_max_size.contains("c.txt"),
        "--sizes=-500 should exclude c.txt: {out_max_size}"
    );

    let pct = syncweb(&["find", "*", src, "--sizes", "100%10"])?;
    ensure!(pct.status.success());
    let out_pct = String::from_utf8(pct.stdout).context("UTF-8 output")?;
    ensure!(
        out_pct.contains("b.txt"),
        "percentage size should match b.txt: {out_pct}"
    );
    ensure!(
        !out_pct.contains("a.txt"),
        "percentage size should exclude a.txt: {out_pct}"
    );

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn find_extension_and_type() -> anyhow::Result<()> {
    let source = cli_test_dir("find-ext");
    std::fs::create_dir_all(source.join("sub")).context("create dir")?;
    std::fs::write(source.join("a.txt"), b"a").context("write a")?;
    std::fs::write(source.join("b.md"), b"b").context("write b")?;
    let src = source.to_str().context("UTF-8 path")?;

    let txt = syncweb(&["find", "*", src, "-e", "txt"])?;
    ensure!(txt.status.success());
    let out_txt = String::from_utf8(txt.stdout).context("UTF-8 output")?;
    ensure!(out_txt.contains("a.txt"), "-e txt should include a.txt: {out_txt}");
    ensure!(!out_txt.contains("b.md"), "-e txt should exclude b.md: {out_txt}");

    let both = syncweb(&["find", "*", src, "-e", "txt", "-e", "md"])?;
    ensure!(both.status.success());
    let out_both = String::from_utf8(both.stdout).context("UTF-8 output")?;
    ensure!(
        out_both.contains("a.txt") && out_both.contains("b.md"),
        "-e txt -e md should include both: {out_both}"
    );

    let dirs = syncweb(&["find", "*", src, "--type", "d"])?;
    ensure!(dirs.status.success());
    let out_dirs = String::from_utf8(dirs.stdout).context("UTF-8 output")?;
    ensure!(out_dirs.contains("sub"), "--type d should list dirs: {out_dirs}");
    ensure!(
        !out_dirs.contains("a.txt"),
        "--type d should not list files: {out_dirs}"
    );

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
#[cfg(unix)]
fn find_follow_links_and_downloadable() -> anyhow::Result<()> {
    use std::os::unix::fs::symlink;
    let source = cli_test_dir("find-links");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("target.txt"), b"t").context("write target")?;
    symlink(source.join("target.txt"), source.join("link.txt")).context("create symlink")?;
    let src = source.to_str().context("UTF-8 path")?;

    let syms = syncweb(&["find", "*", src, "--type", "l"])?;
    ensure!(syms.status.success());
    let out_syms = String::from_utf8(syms.stdout).context("UTF-8 output")?;
    ensure!(
        out_syms.contains("link.txt"),
        "--type l should list symlinks: {out_syms}"
    );

    for extra in [&["-L"][..], &["-d"][..], &["-L", "-d", "--threads", "2"][..]] {
        let mut args = vec!["find", "*", src];
        args.extend_from_slice(extra);
        let output = syncweb(&args)?;
        ensure!(
            output.status.success(),
            "find {extra:?} should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn sort_additional_algorithms() -> anyhow::Result<()> {
    let source = cli_test_dir("sort-more");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("a.txt"), b"a").context("write")?;
    std::fs::write(source.join("b.txt"), b"b").context("write")?;
    let src = source.to_str().context("UTF-8 path")?;

    for algorithm in [
        "time",
        "date",
        "week",
        "month",
        "year",
        "size",
        "folder-size",
        "folder-avg-size",
        "folder-date",
        "folder-time",
        "count",
    ] {
        let output = syncweb(&["sort", src, "--by", algorithm])?;
        ensure!(
            output.status.success(),
            "sort --by {algorithm} should succeed: {}",
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
fn sort_filters_and_scoring_tuning() -> anyhow::Result<()> {
    let source = cli_test_dir("sort-tuning");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("a.txt"), b"a").context("write")?;
    std::fs::write(source.join("b.txt"), b"b").context("write")?;
    let src = source.to_str().context("UTF-8 path")?;

    let output = syncweb(&[
        "sort",
        src,
        "--by",
        "size",
        "--min-seeders",
        "0",
        "--max-seeders",
        "10",
        "--niche",
        "5",
        "--frecency-weight",
        "7",
        "--limit-size",
        "1GB",
        "--depth=-2",
        "--threads",
        "1",
    ])?;
    ensure!(
        output.status.success(),
        "sort with filters should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    anyhow::ensure!(stdout.lines().count() == 2, "should list 2 files: {stdout}");

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn stat_format_template() -> anyhow::Result<()> {
    let source = cli_test_dir("stat-format");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("file.txt"), b"hello world").context("write")?;
    let file_path = source.join("file.txt");
    let path_str = file_path.to_str().context("UTF-8 path")?;

    let output = syncweb(&["stat", path_str, "--format", "%s"])?;
    ensure!(
        output.status.success(),
        "stat --format should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).context("UTF-8 output")?;
    ensure!(stdout.trim() == "11", "format %s should print the size: {stdout}");

    let conflict = syncweb(&["stat", path_str, "--format", "%s", "--terse"])?;
    ensure!(!conflict.status.success(), "--format and --terse should conflict");

    std::fs::remove_dir_all(&source).context("cleanup")?;
    Ok(())
}

#[test]
fn download_filter_and_provider_options() -> anyhow::Result<()> {
    let source = cli_test_dir("download-src2");
    let dest = cli_test_dir("download-dest2");
    std::fs::create_dir_all(&source).context("create dir")?;
    std::fs::write(source.join("keep.txt"), b"keep").context("write")?;
    let src_file = source.join("keep.txt");
    let src_str = src_file.to_str().context("UTF-8 path")?;
    let dst_file = dest.join("copied.txt");
    let dst_str = dst_file.to_str().context("UTF-8 path")?;

    let output = syncweb(&[
        "download",
        src_str,
        dst_str,
        "--threads",
        "1",
        "--min-providers",
        "3",
        "--no-sharing",
    ])?;
    ensure!(
        output.status.success(),
        "download with provider options should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    anyhow::ensure!(std::fs::read(dest.join("copied.txt")).context("read")? == b"keep");

    let hash = syncweb(&[
        "download",
        src_str,
        "--hash",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ])?;
    ensure!(!hash.status.success(), "--hash without --from should fail");

    let filters = syncweb(&["download", src_str, dst_str, "--max-count", "1"])?;
    ensure!(
        !filters.status.success(),
        "fetch filters with a destination should fail"
    );

    std::fs::remove_dir_all(&source).context("cleanup source")?;
    std::fs::remove_dir_all(&dest).context("cleanup dest")?;
    Ok(())
}

#[test]
fn import_folder_threads_enrich() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("import-data");
    let managed = cli_test_dir("import-managed");
    std::fs::create_dir_all(&managed).context("create managed dir")?;
    let src_dir = cli_test_dir("import-src");
    std::fs::create_dir_all(&src_dir).context("create src dir")?;
    std::fs::write(src_dir.join("new.txt"), b"content").context("write")?;
    let data_dir_s = data_dir.to_str().context("UTF-8 path")?;

    let create = syncweb(&[
        "--data-dir",
        data_dir_s,
        "--no-daemon",
        "create",
        managed.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        create.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .context("namespace line")?
        .trim_start_matches("namespace:")
        .trim()
        .to_owned();

    let import = syncweb(&[
        "--data-dir",
        data_dir_s,
        "--no-daemon",
        "import",
        src_dir.join("new.txt").to_str().context("UTF-8 path")?,
        "--folder",
        &namespace,
        "--threads",
        "1",
        "--enrich",
    ])?;
    ensure!(
        import.status.success(),
        "import with options should succeed: {}",
        String::from_utf8_lossy(&import.stderr)
    );
    let import_out = String::from_utf8(import.stdout).context("UTF-8 output")?;
    ensure!(
        import_out.contains("new.txt"),
        "import should list the file: {import_out}"
    );

    std::fs::remove_dir_all(&data_dir).context("cleanup data")?;
    std::fs::remove_dir_all(&managed).context("cleanup managed")?;
    std::fs::remove_dir_all(&src_dir).context("cleanup src")?;
    Ok(())
}

#[test]
fn verify_fix_and_filters() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("verify-data");
    let managed = cli_test_dir("verify-managed");
    std::fs::create_dir_all(&managed).context("create managed dir")?;
    let src_dir = cli_test_dir("verify-src");
    std::fs::create_dir_all(&src_dir).context("create src dir")?;
    std::fs::write(src_dir.join("file.txt"), b"content").context("write")?;
    let data_dir_s = data_dir.to_str().context("UTF-8 path")?;
    let managed_s = managed.to_str().context("UTF-8 path")?;

    let create = syncweb(&["--data-dir", data_dir_s, "--no-daemon", "create", managed_s])?;
    ensure!(
        create.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let create_out = String::from_utf8(create.stdout).context("UTF-8 output")?;
    let namespace = create_out
        .lines()
        .find(|line| line.starts_with("namespace:"))
        .context("namespace line")?
        .trim_start_matches("namespace:")
        .trim()
        .to_owned();

    let import = syncweb(&[
        "--data-dir",
        data_dir_s,
        "--no-daemon",
        "import",
        src_dir.join("file.txt").to_str().context("UTF-8 path")?,
        "--folder",
        &namespace,
        "--threads",
        "1",
    ])?;
    ensure!(
        import.status.success(),
        "import failed: {}",
        String::from_utf8_lossy(&import.stderr)
    );

    let verify = syncweb(&["--data-dir", data_dir_s, "--no-daemon", "verify", managed_s])?;
    ensure!(
        verify.status.success(),
        "verify failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let out = String::from_utf8(verify.stdout).context("UTF-8 output")?;
    ensure!(out.contains("verified: 1"), "should verify the file: {out}");

    let fix = syncweb(&["--data-dir", data_dir_s, "--no-daemon", "verify", managed_s, "--fix"])?;
    ensure!(
        fix.status.success(),
        "verify --fix failed: {}",
        String::from_utf8_lossy(&fix.stderr)
    );
    let out_fix = String::from_utf8(fix.stdout).context("UTF-8 output")?;
    ensure!(out_fix.contains("repair:"), "should print repair section: {out_fix}");

    let filtered = syncweb(&[
        "--data-dir",
        data_dir_s,
        "--no-daemon",
        "verify",
        managed_s,
        "--path-prefix",
        "file.txt",
        "--glob",
        "*.txt",
        "--from",
        "fake",
        "--min-providers",
        "2",
        "--no-sharing",
    ])?;
    ensure!(
        filtered.status.success(),
        "verify with filters should succeed: {}",
        String::from_utf8_lossy(&filtered.stderr)
    );

    std::fs::remove_dir_all(&data_dir).context("cleanup data")?;
    std::fs::remove_dir_all(&managed).context("cleanup managed")?;
    std::fs::remove_dir_all(&src_dir).context("cleanup src")?;
    Ok(())
}
