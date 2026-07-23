use anyhow::{Context, ensure};
use std::fs;
use std::io::Write as _;
use std::process::{Command, Stdio};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
}

fn test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("syncweb-suite-{name}-{}", uuid::Uuid::new_v4()))
}

fn run(args: &[&str]) -> anyhow::Result<std::process::Output> {
    cli()
        .args(args)
        .output()
        .with_context(|| format!("run syncweb {args:?}"))
}

fn run_with_data(data_dir: &std::path::Path, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let mut all_args = vec!["--data-dir", data_dir.to_str().context("UTF-8 path")?];
    all_args.extend_from_slice(args);
    run(&all_args)
}

fn assert_success(output: &std::process::Output, label: &str) -> anyhow::Result<()> {
    ensure!(
        output.status.success(),
        "{label} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn stdout_string(output: &std::process::Output) -> anyhow::Result<String> {
    String::from_utf8(output.stdout.clone()).context("UTF-8 output")
}

// ---------------------------------------------------------------------------
// 7.1 – All commands are discoverable via --help
// ---------------------------------------------------------------------------

#[test]
fn full_help_lists_all_commands() -> anyhow::Result<()> {
    let output = run(&["--help"])?;
    assert_success(&output, "help")?;
    let help = stdout_string(&output)?;
    for cmd in [
        "version",
        "repl",
        "create",
        "join",
        "leave",
        "unsubscribe",
        "folders",
        "devices",
        "config",
        "ls",
        "find",
        "sort",
        "stat",
        "download",
        "import",
        "snapshot",
        "health",
        "init",
        "automatic",
        "watch",
        "stats",
        "verify",
        "schedule",
        "subscribe",
        "publish",
        "unpublish",
        "collection",
        "package",
        "network",
        "completions",
        "manpages",
    ] {
        ensure!(help.contains(cmd), "help should list '{cmd}'");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 7.2 – JSON output
// ---------------------------------------------------------------------------

#[test]
fn json_version_output() -> anyhow::Result<()> {
    let output = run(&["--json", "version"])?;
    assert_success(&output, "json version")?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    ensure!(value.get("version") == Some(&serde_json::Value::from("0.1.0")));
    Ok(())
}

#[test]
fn json_stats_output() -> anyhow::Result<()> {
    let data_dir = test_dir("json-stats");
    let output = run_with_data(&data_dir, &["--json", "stats"])?;
    let _ = fs::remove_dir_all(&data_dir);
    assert_success(&output, "json stats")?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    ensure!(
        value.get("total_download").is_some(),
        "stats JSON should have total_download"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 7.3 – Config round-trip
// ---------------------------------------------------------------------------

#[test]
fn config_round_trip_via_cli() -> anyhow::Result<()> {
    let data_dir = test_dir("config-rt");
    let _ = run_with_data(&data_dir, &["config", "set", "bep.enabled", "true"]);
    let _ = run_with_data(&data_dir, &["config", "set", "schedule.active_hours", "08:00-22:00"]);

    let show = run_with_data(&data_dir, &["config", "show"])?;
    let _ = fs::remove_dir_all(&data_dir);
    assert_success(&show, "config show")?;
    let stdout = stdout_string(&show)?;
    ensure!(stdout.contains("bep"), "should show bep section: {stdout}");
    ensure!(stdout.contains("schedule"), "should show schedule section: {stdout}");
    Ok(())
}

// ---------------------------------------------------------------------------
// 7.4 – Shell completions
// ---------------------------------------------------------------------------

#[test]
fn all_shell_completions_produce_output() -> anyhow::Result<()> {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let output = run(&["completions", shell])?;
        assert_success(&output, &format!("completions {shell}"))?;
        let stdout = stdout_string(&output)?;
        ensure!(
            stdout.contains("syncweb"),
            "{shell} completions should reference syncweb"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Full workflow: create → folders → ls → find → sort → stat → config → schedule → stats → verify
// ---------------------------------------------------------------------------

#[test]
fn create_folders_list_works() -> anyhow::Result<()> {
    let data_dir = test_dir("create-folders");
    let output = run_with_data(&data_dir, &["--no-daemon", "create"])?;
    assert_success(&output, "create")?;
    let stdout = stdout_string(&output)?;
    ensure!(stdout.contains("namespace:"), "should print namespace: {stdout}");
    ensure!(stdout.contains("ticket:"), "should print ticket: {stdout}");

    let folders = run_with_data(&data_dir, &["--no-daemon", "folders"])?;
    assert_success(&folders, "folders")?;
    let folders_stdout = stdout_string(&folders)?;
    ensure!(
        folders_stdout.lines().count() >= 1,
        "should list at least one folder: {folders_stdout}"
    );
    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn ls_find_sort_stat_workflow() -> anyhow::Result<()> {
    let source = test_dir("ls-find-sort-stat");
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("report-01.pdf"), b"report content")?;
    fs::write(source.join("data.txt"), b"data content")?;
    fs::write(source.join("sub/image.png"), b"png content")?;

    let ls = run(&["ls", source.to_str().context("UTF-8 path")?])?;
    assert_success(&ls, "ls")?;
    let ls_out = stdout_string(&ls)?;
    ensure!(ls_out.contains("report-01.pdf"), "ls should find report: {ls_out}");
    ensure!(ls_out.contains("data.txt"), "ls should find data: {ls_out}");

    let find = run(&[
        "find",
        r"report-\d+\.pdf",
        source.to_str().context("UTF-8 path")?,
        "--kind",
        "regex",
    ])?;
    assert_success(&find, "find regex")?;
    let find_out = stdout_string(&find)?;
    ensure!(find_out.contains("report-01.pdf"), "find should match: {find_out}");

    let sort = run(&["sort", source.to_str().context("UTF-8 path")?, "--by", "peers"])?;
    assert_success(&sort, "sort")?;
    let sort_out = stdout_string(&sort)?;
    ensure!(sort_out.lines().count() == 3, "sort should list 3 files: {sort_out}");

    let stat = run(&["stat", source.join("data.txt").to_str().context("UTF-8 path")?])?;
    assert_success(&stat, "stat")?;
    let stat_out = stdout_string(&stat)?;
    ensure!(stat_out.contains("Path:"), "stat should show Path: {stat_out}");
    ensure!(stat_out.contains("Size:"), "stat should show Size: {stat_out}");

    fs::remove_dir_all(source)?;
    Ok(())
}

#[test]
fn download_single_file_and_directory() -> anyhow::Result<()> {
    let source = test_dir("dl-source");
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("file.txt"), b"hello")?;
    fs::write(source.join("sub/nested.txt"), b"nested")?;

    let dest = test_dir("dl-dest");
    let dl = run(&[
        "download",
        source.join("file.txt").to_str().context("UTF-8 path")?,
        dest.join("out.txt").to_str().context("UTF-8 path")?,
    ])?;
    assert_success(&dl, "download single")?;
    ensure!(dest.join("out.txt").exists(), "downloaded file should exist");
    ensure!(fs::read(dest.join("out.txt"))? == b"hello");

    let dir_dest = test_dir("dl-dir-dest");
    let dl_dir = run(&[
        "download",
        source.to_str().context("UTF-8 path")?,
        dir_dest.to_str().context("UTF-8 path")?,
    ])?;
    assert_success(&dl_dir, "download directory")?;
    ensure!(dir_dest.join("file.txt").exists());
    ensure!(dir_dest.join("sub/nested.txt").exists());

    fs::remove_dir_all(source)?;
    fs::remove_dir_all(dest)?;
    fs::remove_dir_all(dir_dest)?;
    Ok(())
}

#[test]
fn package_archive_export_cli() -> anyhow::Result<()> {
    let data_dir = test_dir("drop-export-data");
    let package_dir = test_dir("drop-export-package");
    fs::create_dir_all(&package_dir)?;
    fs::write(package_dir.join("readme.txt"), b"readme")?;
    fs::write(package_dir.join("movie.mp4"), b"movie")?;

    let init = run_with_data(
        &data_dir,
        &[
            "collection",
            "init",
            package_dir.to_str().context("UTF-8 package path")?,
            "--name",
            "example",
        ],
    )?;
    assert_success(&init, "collection init")?;
    let add = run_with_data(
        &data_dir,
        &["collection", "add", package_dir.to_str().context("UTF-8 package path")?],
    )?;
    assert_success(&add, "collection add")?;

    let output = package_dir.join("example.car.zst");
    let export = run_with_data(
        &data_dir,
        &[
            "package",
            "export",
            "--filter",
            "ext!=mp4",
            package_dir.to_str().context("UTF-8 package path")?,
            output.to_str().context("UTF-8 output path")?,
        ],
    )?;
    assert_success(&export, "package archive export")?;
    ensure!(output.is_file(), "drop archive should be created");
    ensure!(fs::metadata(output)?.len() > 0, "drop archive should not be empty");

    fs::remove_dir_all(data_dir)?;
    fs::remove_dir_all(package_dir)?;
    Ok(())
}

#[test]
fn schedule_and_stats_persist() -> anyhow::Result<()> {
    let data_dir = test_dir("sched-stats");
    let sched = run_with_data(&data_dir, &["schedule", "set", "--active", "22:00-06:00"])?;
    assert_success(&sched, "schedule set")?;

    let stats = run_with_data(&data_dir, &["--json", "stats"])?;
    assert_success(&stats, "stats")?;
    let value: serde_json::Value = serde_json::from_slice(&stats.stdout)?;
    ensure!(value.get("total_download") == Some(&serde_json::Value::from(0)));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn init_outputs_all_fields() -> anyhow::Result<()> {
    let folder_dir = test_dir("init-folder");
    let data_dir = test_dir("init-data");
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "init",
            folder_dir.to_str().context("UTF-8 path")?,
        ])
        .output()
        .with_context(|| "run syncweb init --no-daemon")?;
    let _ = fs::remove_dir_all(&folder_dir);
    let _ = fs::remove_dir_all(&data_dir);
    assert_success(&output, "init")?;
    let stdout = stdout_string(&output)?;
    ensure!(stdout.contains("path:"), "should print path: {stdout}");
    ensure!(stdout.contains("namespace:"), "should print namespace: {stdout}");
    ensure!(stdout.contains("ticket:"), "should print ticket: {stdout}");
    ensure!(stdout.contains("share_url:"), "should print share_url: {stdout}");
    Ok(())
}

#[test]
fn network_create_list_invite_leave() -> anyhow::Result<()> {
    let data_dir = test_dir("network-workflow");

    let create = run_with_data(&data_dir, &["network", "create", "team"])?;
    assert_success(&create, "network create")?;
    let create_out = stdout_string(&create)?;
    ensure!(create_out.contains("created:"), "should print created: {create_out}");

    let list = run_with_data(&data_dir, &["network", "ls"])?;
    assert_success(&list, "network ls")?;
    let list_out = stdout_string(&list)?;
    ensure!(list_out.contains("team"), "should list team: {list_out}");

    let invite = run_with_data(&data_dir, &["network", "invite", "team"])?;
    assert_success(&invite, "network invite")?;
    let invite_out = stdout_string(&invite)?;
    ensure!(
        invite_out.contains("syncweb://network/"),
        "should output ticket: {invite_out}"
    );

    let leave = run_with_data(&data_dir, &["network", "leave", "team"])?;
    assert_success(&leave, "network leave")?;

    let list_after = run_with_data(&data_dir, &["network", "ls"])?;
    assert_success(&list_after, "network ls after leave")?;
    let list_after_out = stdout_string(&list_after)?;
    ensure!(
        !list_after_out.contains("team"),
        "team should be gone: {list_after_out}"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn repl_starts_and_responds() -> anyhow::Result<()> {
    let mut child = cli()
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("start repl")?;
    child
        .stdin
        .take()
        .context("repl stdin")?
        .write_all(b"help\nexit\n")
        .context("write repl input")?;
    let output = child.wait_with_output().context("wait for repl")?;
    assert_success(&output, "repl")?;
    let stdout = stdout_string(&output)?;
    ensure!(stdout.contains("syncweb repl"));
    Ok(())
}

#[test]
fn verbose_and_rust_log_control_logging() -> anyhow::Result<()> {
    let verbose = run(&["--verbose", "version"])?;
    assert_success(&verbose, "verbose version")?;
    let verbose_out = stdout_string(&verbose)?;
    ensure!(
        verbose_out.contains("\"level\":\"DEBUG\""),
        "verbose should produce debug output: {verbose_out}"
    );

    let rust_log = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("version")
        .env("RUST_LOG", "syncweb=debug")
        .output()
        .context("run with RUST_LOG")?;
    assert_success(&rust_log, "RUST_LOG version")?;
    let rust_log_out = stdout_string(&rust_log)?;
    ensure!(rust_log_out.contains("\"level\":\"DEBUG\""));
    Ok(())
}
