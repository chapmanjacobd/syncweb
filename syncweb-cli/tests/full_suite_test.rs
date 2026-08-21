use anyhow::{Context, ensure};
use std::fs;
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
        "create",
        "join",
        "leave",
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
        "watch",
        "stats",
        "verify",
        "publish",
        "unpublish",
        "package",
        "network",
        "indexing",
        "link",
        "provider",
        "trust",
        "attest",
        "moderation",
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
    let version = workspace_version()?;
    ensure!(value.get("version") == Some(&serde_json::Value::from(version)));
    Ok(())
}

#[test]
fn json_stats_output() -> anyhow::Result<()> {
    let data_dir = test_dir("json-stats");
    let output = run_with_data(&data_dir, &["--json", "stats", "network"])?;
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
// 7.5 – Manpages and help subcommand
// ---------------------------------------------------------------------------

#[test]
fn manpages_generate_dot_one_files() -> anyhow::Result<()> {
    let out_dir = test_dir("manpages");
    let output = run(&["manpages", out_dir.to_str().context("UTF-8 path")?])?;
    assert_success(&output, "manpages")?;
    let stdout = stdout_string(&output)?;
    ensure!(
        stdout.contains("manpages generated"),
        "should report generation: {stdout}"
    );

    let mut names = Vec::new();
    for entry in fs::read_dir(&out_dir).context("read manpage dir")? {
        names.push(
            entry
                .context("manpage entry")?
                .file_name()
                .to_string_lossy()
                .into_owned(),
        );
    }
    fs::remove_dir_all(&out_dir)?;

    ensure!(
        names.contains(&"syncweb.1".to_string()),
        "should contain syncweb.1: {names:?}"
    );
    ensure!(
        names.contains(&"syncweb-create.1".to_string()),
        "should contain per-command manpages: {names:?}"
    );
    ensure!(
        !names.contains(&"syncweb-help.1".to_string()),
        "help subcommand should be skipped: {names:?}"
    );
    Ok(())
}

#[test]
fn manpages_default_directory_writes_man() -> anyhow::Result<()> {
    let cwd = test_dir("manpages-default");
    fs::create_dir_all(&cwd).context("create cwd")?;
    let output = cli()
        .arg("manpages")
        .current_dir(&cwd)
        .output()
        .context("run syncweb manpages in default dir")?;
    assert_success(&output, "manpages default dir")?;
    let stdout = stdout_string(&output)?;
    ensure!(
        stdout.contains("manpages generated in man"),
        "should report the default man dir: {stdout}"
    );
    ensure!(
        cwd.join("man").join("syncweb.1").exists(),
        "should write syncweb.1 into ./man"
    );
    fs::remove_dir_all(&cwd)?;
    Ok(())
}

#[test]
fn help_subcommand_reports_specific_and_grouped_help() -> anyhow::Result<()> {
    let version = run(&["help", "version"])?;
    assert_success(&version, "help version")?;
    let version_out = stdout_string(&version)?;
    ensure!(
        version_out.contains("Usage: syncweb version"),
        "help version should print version usage: {version_out}"
    );

    let direct = run(&["version", "--help"])?;
    assert_success(&direct, "version --help")?;
    ensure!(
        stdout_string(&direct)? == version_out,
        "help version should match version --help"
    );

    let bare = run(&["help"])?;
    assert_success(&bare, "bare help")?;
    let bare_out = stdout_string(&bare)?;
    ensure!(
        bare_out.contains("Daemon:"),
        "bare help should print grouped help: {bare_out}"
    );

    let top = run(&["--help"])?;
    assert_success(&top, "--help")?;
    ensure!(
        stdout_string(&top)? == bare_out,
        "bare help subcommand should match --help"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Full workflow: create → folders → ls → find → sort → stat → config → schedule → stats → verify
// ---------------------------------------------------------------------------

#[test]
fn create_folders_list_works() -> anyhow::Result<()> {
    let data_dir = test_dir("create-folders");
    let output = run_with_data(&data_dir, &["--no-daemon", "create", "--no-import"])?;
    assert_success(&output, "create")?;
    let stdout = stdout_string(&output)?;
    ensure!(stdout.contains("namespace:"), "should print namespace: {stdout}");

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
            "package",
            "init",
            package_dir.to_str().context("UTF-8 package path")?,
            "--name",
            "example",
        ],
    )?;
    assert_success(&init, "package init")?;
    let add = run_with_data(
        &data_dir,
        &["package", "add", package_dir.to_str().context("UTF-8 package path")?],
    )?;
    assert_success(&add, "package add")?;

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
fn package_bump() -> anyhow::Result<()> {
    let data_dir = test_dir("collection-versions");
    let package_dir = test_dir("collection-versions-pkg");
    fs::create_dir_all(&package_dir)?;
    fs::write(package_dir.join("a.txt"), b"a")?;
    let package_path = package_dir.to_str().context("UTF-8 package path")?;
    let root = package_dir
        .parent()
        .context("package dir should have a parent")?
        .to_str()
        .context("UTF-8 root path")?;

    let init = run_with_data(&data_dir, &["package", "init", package_path, "--name", "example"])?;
    assert_success(&init, "package init")?;

    let add = run_with_data(&data_dir, &["package", "add", package_path])?;
    assert_success(&add, "package add")?;

    let bump = run_with_data(
        &data_dir,
        &[
            "package",
            "bump",
            package_path,
            "--version",
            "2.0.0",
            "--changelog",
            "second release",
        ],
    )?;
    assert_success(&bump, "package bump")?;
    let bump_out = stdout_string(&bump)?;
    ensure!(bump_out.contains("version: 2.0.0"), "bump output: {bump_out}");

    let db = syncweb_core::storage::node_db::NodeDatabase::open(data_dir.join("default").join("node.db"))?;
    let manifest_bytes = db
        .load_workspace_manifest(root)?
        .context("workspace manifest should exist")?;
    let manifest = syncweb_core::folder::CollectionManifest::from_bytes(manifest_bytes)?;
    ensure!(manifest.version == "2.0.0", "manifest version: {}", manifest.version);
    ensure!(
        manifest.changelog.as_deref() == Some("second release"),
        "changelog should be recorded: {:?}",
        manifest.changelog
    );
    let parent = manifest.parent.context("bumped manifest should record a parent")?;
    ensure!(!parent.to_string().is_empty(), "parent hash should be present");

    let bump_json = run_with_data(
        &data_dir,
        &["--json", "package", "bump", package_path, "--version", "3.0.0"],
    )?;
    assert_success(&bump_json, "package bump json")?;
    let value: serde_json::Value = serde_json::from_slice(&bump_json.stdout)?;
    ensure!(
        value.get("version") == Some(&serde_json::Value::from("3.0.0")),
        "{value}"
    );

    fs::remove_dir_all(data_dir)?;
    fs::remove_dir_all(package_dir)?;
    Ok(())
}

fn logical_paths_of(data_dir: &std::path::Path, root: &str) -> anyhow::Result<Vec<String>> {
    let db = syncweb_core::storage::node_db::NodeDatabase::open(data_dir.join("default").join("node.db"))?;
    let manifest_bytes = db
        .load_workspace_manifest(root)?
        .context("workspace manifest should exist")?;
    let manifest = syncweb_core::folder::CollectionManifest::from_bytes(manifest_bytes)?;
    Ok(manifest
        .entries
        .iter()
        .map(|entry| entry.logical_path.to_string_lossy().into_owned())
        .collect())
}

#[test]
fn package_multipath_common_root_rebasing() -> anyhow::Result<()> {
    let data_dir = test_dir("package-multipath");

    // Example 1: /library/thingdata/ + /library/thing.txt -> root /library
    let library = test_dir("library");
    let thingdata = library.join("thingdata");
    fs::create_dir_all(&thingdata)?;
    fs::write(thingdata.join("a.txt"), b"a")?;
    fs::write(library.join("thing.txt"), b"thing")?;
    fs::write(library.join("sibling.txt"), b"excluded")?;

    let lib_str = library.to_str().context("UTF-8 library path")?;
    let td_str = thingdata.to_str().context("UTF-8 thingdata path")?;
    let thing_txt_path = library.join("thing.txt");
    let thing_txt = thing_txt_path.to_str().context("UTF-8 thing.txt path")?;

    let init = run_with_data(&data_dir, &["package", "init", td_str, thing_txt, "--name", "example"])?;
    assert_success(&init, "package init multi-path")?;
    let paths = logical_paths_of(&data_dir, lib_str)?;
    ensure!(
        paths.iter().any(|p| p == "thingdata/a.txt"),
        "expected thingdata/a.txt in {paths:?}"
    );
    ensure!(
        paths.iter().any(|p| p == "thing.txt"),
        "expected thing.txt in {paths:?}"
    );
    ensure!(
        !paths.iter().any(|p| p == "sibling.txt"),
        "sibling at the root must be excluded: {paths:?}"
    );

    // Example 2: /library/dir/thingdata/ + /library/dir/thing.txt -> root /library/dir
    let library2 = test_dir("library2");
    let dir = library2.join("dir");
    let thingdata2 = dir.join("thingdata");
    fs::create_dir_all(&thingdata2)?;
    fs::write(thingdata2.join("b.txt"), b"b")?;
    fs::write(dir.join("thing.txt"), b"thing2")?;
    let dir_str = dir.to_str().context("UTF-8 dir path")?;
    let td_second_str = thingdata2.to_str().context("UTF-8 thingdata2 path")?;
    let thing_second_path = dir.join("thing.txt");
    let thing_second = thing_second_path.to_str().context("UTF-8 thing2.txt path")?;

    let init2 = run_with_data(
        &data_dir,
        &["package", "init", td_second_str, thing_second, "--name", "example2"],
    )?;
    assert_success(&init2, "package init example 2")?;
    let paths2 = logical_paths_of(&data_dir, dir_str)?;
    ensure!(
        paths2.iter().any(|p| p == "thingdata/b.txt"),
        "expected thingdata/b.txt in {paths2:?}"
    );
    ensure!(
        paths2.iter().any(|p| p == "thing.txt"),
        "expected thing.txt in {paths2:?}"
    );

    fs::remove_dir_all(data_dir)?;
    fs::remove_dir_all(library)?;
    fs::remove_dir_all(library2)?;
    Ok(())
}

fn publish_manifest_ticket(
    data_dir: &std::path::Path,
    namespace: &str,
    package_path: &str,
    sequence: &str,
) -> anyhow::Result<String> {
    let publish = run_with_data(
        data_dir,
        &[
            "--json",
            "--no-daemon",
            "package",
            "publish",
            package_path,
            "--namespace",
            namespace,
            "--sequence",
            sequence,
        ],
    )?;
    assert_success(&publish, "package publish")?;
    let value: serde_json::Value = serde_json::from_slice(&publish.stdout)?;
    Ok(value
        .get("manifest_ticket")
        .and_then(serde_json::Value::as_str)
        .context("publish missing manifest_ticket")?
        .to_owned())
}

fn install_package(data_dir: &std::path::Path, ticket: &str) -> anyhow::Result<String> {
    let install = run_with_data(data_dir, &["--json", "package", "install", ticket])?;
    assert_success(&install, "package install")?;
    let value: serde_json::Value = serde_json::from_slice(&install.stdout)?;
    ensure!(
        value.get("status") == Some(&serde_json::Value::from("installed")),
        "{value}"
    );
    ensure!(
        value.get("version") == Some(&serde_json::Value::from("1.0.0")),
        "{value}"
    );
    Ok(value
        .get("collection")
        .and_then(serde_json::Value::as_str)
        .context("install output missing collection")?
        .to_owned())
}

fn upgrade_package(data_dir: &std::path::Path, ticket: &str) -> anyhow::Result<()> {
    let upgrade = run_with_data(data_dir, &["--json", "package", "upgrade", ticket])?;
    assert_success(&upgrade, "package upgrade")?;
    let value: serde_json::Value = serde_json::from_slice(&upgrade.stdout)?;
    ensure!(
        value.get("status") == Some(&serde_json::Value::from("installed")),
        "{value}"
    );
    ensure!(
        value.get("version") == Some(&serde_json::Value::from("2.0.0")),
        "{value}"
    );
    Ok(())
}

#[test]
fn package_import_search_install_upgrade_remove() -> anyhow::Result<()> {
    let data_dir = test_dir("package-lifecycle");
    let folder_dir = test_dir("package-folder");
    let package_dir = test_dir("package-src");
    fs::create_dir_all(&package_dir)?;
    fs::write(package_dir.join("lib.txt"), b"lib content")?;
    fs::write(package_dir.join("readme.md"), b"readme")?;
    let package_path = package_dir.to_str().context("UTF-8 package path")?;

    let created = run_with_data(
        &data_dir,
        &[
            "--json",
            "--no-daemon",
            "create",
            folder_dir.to_str().context("UTF-8 folder path")?,
        ],
    )?;
    assert_success(&created, "create")?;
    let created_json: serde_json::Value = serde_json::from_slice(&created.stdout)?;
    let namespace = created_json
        .get("namespace")
        .and_then(serde_json::Value::as_str)
        .context("create output missing namespace")?;

    let init = run_with_data(&data_dir, &["package", "init", package_path, "--name", "example"])?;
    assert_success(&init, "package init")?;
    let add = run_with_data(&data_dir, &["package", "add", package_path])?;
    assert_success(&add, "package add")?;

    let ticket1 = publish_manifest_ticket(&data_dir, namespace, package_path, "1")?;
    let collection = install_package(&data_dir, &ticket1)?;

    let bump = run_with_data(
        &data_dir,
        &[
            "package",
            "bump",
            package_path,
            "--version",
            "2.0.0",
            "--changelog",
            "second release",
        ],
    )?;
    assert_success(&bump, "package bump")?;

    let ticket2 = publish_manifest_ticket(&data_dir, namespace, package_path, "2")?;
    upgrade_package(&data_dir, &ticket2)?;

    let list = run_with_data(&data_dir, &["--json", "package", "list"])?;
    assert_success(&list, "package list")?;
    let list_json: serde_json::Value = serde_json::from_slice(&list.stdout)?;
    let installed = list_json.as_array().context("package list should be an array")?;
    ensure!(installed.len() == 1, "one collection installed: {list_json}");
    let installed_collection = installed.first().context("package list is empty")?;
    ensure!(installed_collection.get("collection") == Some(&serde_json::Value::from(collection.as_str())));
    ensure!(installed_collection.get("current") == Some(&serde_json::Value::from("2.0.0")));

    let search = run_with_data(&data_dir, &["--json", "package", "search", &collection])?;
    assert_success(&search, "package search")?;
    let search_json: serde_json::Value = serde_json::from_slice(&search.stdout)?;
    let results = search_json.as_array().context("package search should be an array")?;
    ensure!(
        results
            .iter()
            .any(|item| item.get("collection") == Some(&serde_json::Value::from(collection.as_str()))),
        "search should find the collection: {search_json}"
    );

    let versions = run_with_data(&data_dir, &["--json", "package", "versions", &collection])?;
    assert_success(&versions, "package versions")?;
    let versions_json: serde_json::Value = serde_json::from_slice(&versions.stdout)?;
    let version_list = versions_json
        .as_array()
        .context("package versions should be an array")?;
    ensure!(
        version_list.iter().any(|v| v == &serde_json::Value::from("1.0.0")),
        "versions should include 1.0.0: {versions_json}"
    );
    ensure!(
        version_list.iter().any(|v| v == &serde_json::Value::from("2.0.0")),
        "versions should include 2.0.0: {versions_json}"
    );

    let switch = run_with_data(&data_dir, &["--json", "package", "switch", &collection, "1.0.0"])?;
    assert_success(&switch, "package switch")?;
    let switch_json: serde_json::Value = serde_json::from_slice(&switch.stdout)?;
    ensure!(
        switch_json.get("version") == Some(&serde_json::Value::from("1.0.0")),
        "{switch_json}"
    );

    let remove = run_with_data(&data_dir, &["--json", "package", "remove", &collection, "2.0.0"])?;
    assert_success(&remove, "package remove")?;
    let remove_json: serde_json::Value = serde_json::from_slice(&remove.stdout)?;
    ensure!(
        remove_json.get("status") == Some(&serde_json::Value::from("removed")),
        "{remove_json}"
    );

    let verify = run_with_data(&data_dir, &["--json", "package", "verify", &collection])?;
    assert_success(&verify, "package verify")?;
    let verify_json: serde_json::Value = serde_json::from_slice(&verify.stdout)?;
    ensure!(
        verify_json.get("status") == Some(&serde_json::Value::from("verified")),
        "{verify_json}"
    );

    let export_v3 = test_dir("package-v3.car.zst");
    let export = run_with_data(
        &data_dir,
        &[
            "--json",
            "package",
            "export",
            "--version",
            "3.0.0",
            package_path,
            export_v3.to_str().context("UTF-8 output path")?,
        ],
    )?;
    assert_success(&export, "package export --version")?;
    let export_json: serde_json::Value = serde_json::from_slice(&export.stdout)?;
    let export_entry = export_json
        .as_array()
        .and_then(|array| array.first())
        .context("package export should be a non-empty array")?;
    ensure!(
        export_entry.get("version") == Some(&serde_json::Value::from("3.0.0")),
        "export should use the requested version: {export_json}"
    );

    let import = run_with_data(
        &data_dir,
        &[
            "--json",
            "package",
            "import",
            export_v3.to_str().context("UTF-8 archive path")?,
        ],
    )?;
    assert_success(&import, "package import")?;
    let import_json: serde_json::Value = serde_json::from_slice(&import.stdout)?;
    ensure!(
        import_json.get("status") == Some(&serde_json::Value::from("imported")),
        "{import_json}"
    );
    ensure!(
        import_json.get("version") == Some(&serde_json::Value::from("3.0.0")),
        "{import_json}"
    );

    fs::remove_dir_all(data_dir)?;
    fs::remove_dir_all(folder_dir)?;
    fs::remove_dir_all(package_dir)?;
    Ok(())
}

#[test]
fn package_info_from_ticket_and_hash() -> anyhow::Result<()> {
    let data_dir = test_dir("package-info");
    let folder_dir = test_dir("package-info-folder");
    let package_dir = test_dir("package-info-src");
    fs::create_dir_all(&package_dir)?;
    fs::write(package_dir.join("lib.txt"), b"lib content")?;
    let package_path = package_dir.to_str().context("UTF-8 package path")?;

    let created = run_with_data(
        &data_dir,
        &[
            "--json",
            "--no-daemon",
            "create",
            folder_dir.to_str().context("UTF-8 folder path")?,
        ],
    )?;
    assert_success(&created, "create")?;
    let created_json: serde_json::Value = serde_json::from_slice(&created.stdout)?;
    let namespace = created_json
        .get("namespace")
        .and_then(serde_json::Value::as_str)
        .context("create output missing namespace")?;

    let init = run_with_data(&data_dir, &["package", "init", package_path, "--name", "example"])?;
    assert_success(&init, "package init")?;
    let add = run_with_data(&data_dir, &["package", "add", package_path])?;
    assert_success(&add, "package add")?;

    let publish = run_with_data(
        &data_dir,
        &[
            "--json",
            "--no-daemon",
            "package",
            "publish",
            package_path,
            "--namespace",
            namespace,
            "--sequence",
            "1",
        ],
    )?;
    assert_success(&publish, "package publish")?;
    let publish_json: serde_json::Value = serde_json::from_slice(&publish.stdout)?;
    let ticket = publish_json
        .get("manifest_ticket")
        .and_then(serde_json::Value::as_str)
        .context("publish missing manifest_ticket")?;
    let manifest_hash = publish_json
        .get("manifest")
        .and_then(serde_json::Value::as_str)
        .context("publish missing manifest")?;

    let info = run_with_data(&data_dir, &["--json", "package", "info", ticket])?;
    assert_success(&info, "package info ticket")?;
    let info_json: serde_json::Value = serde_json::from_slice(&info.stdout)?;
    ensure!(
        info_json.get("collection_id").is_some(),
        "info should include collection_id: {info_json}"
    );
    ensure!(
        info_json.get("version") == Some(&serde_json::Value::from("1.0.0")),
        "{info_json}"
    );

    let devices = run_with_data(&data_dir, &["devices"])?;
    assert_success(&devices, "devices")?;
    let node_id = String::from_utf8(devices.stdout)?
        .lines()
        .find_map(|line| line.strip_prefix("iroh: "))
        .map(str::trim)
        .map(String::from)
        .context("devices output missing iroh id")?;

    let info_hash = run_with_data(
        &data_dir,
        &[
            "--json",
            "package",
            "info",
            "--hash",
            manifest_hash,
            "--node-id",
            &node_id,
        ],
    )?;
    assert_success(&info_hash, "package info --hash --node-id")?;
    let info_hash_json: serde_json::Value = serde_json::from_slice(&info_hash.stdout)?;
    ensure!(
        info_hash_json.get("collection_id") == info_json.get("collection_id"),
        "hash and ticket paths should describe the same collection"
    );

    fs::remove_dir_all(data_dir)?;
    fs::remove_dir_all(folder_dir)?;
    fs::remove_dir_all(package_dir)?;
    Ok(())
}

#[test]
fn package_search_channel_and_bootstrap() -> anyhow::Result<()> {
    let data_dir = test_dir("package-search-opts");

    // An unconfigured channel falls back to a local gossip search that
    // completes quickly with no results.
    let search = run_with_data(
        &data_dir,
        &[
            "--json",
            "package",
            "search",
            "example",
            "--channel",
            "unconfigured",
            "--timeout-ms",
            "100",
        ],
    )?;
    assert_success(&search, "package search --channel --timeout-ms")?;
    let search_json: serde_json::Value = serde_json::from_slice(&search.stdout)?;
    ensure!(search_json.is_array(), "search should produce an array: {search_json}");

    // --bootstrap is accepted; a malformed node id fails fast with a parse
    // error instead of hanging on a connection attempt.
    let invalid = run_with_data(
        &data_dir,
        &[
            "--json",
            "package",
            "search",
            "example",
            "--bootstrap",
            "not-a-valid-node",
        ],
    )?;
    ensure!(
        !invalid.status.success(),
        "--bootstrap with an invalid node id should fail"
    );
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    ensure!(
        stderr.contains("invalid") || stderr.contains("public key"),
        "expected a key parse error, got: {stderr}"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn schedule_and_stats_persist() -> anyhow::Result<()> {
    let data_dir = test_dir("sched-stats");
    let sched = run_with_data(&data_dir, &["config", "schedule", "set", "--active", "22:00-06:00"])?;
    assert_success(&sched, "config schedule set")?;

    let stats = run_with_data(&data_dir, &["--json", "stats", "network"])?;
    assert_success(&stats, "stats")?;
    let value: serde_json::Value = serde_json::from_slice(&stats.stdout)?;
    ensure!(value.get("total_download") == Some(&serde_json::Value::from(0)));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn create_outputs_all_fields() -> anyhow::Result<()> {
    let folder_dir = test_dir("create-folder");
    let data_dir = test_dir("create-data");
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir.to_str().context("UTF-8 path")?,
            "--no-daemon",
            "create",
            folder_dir.to_str().context("UTF-8 path")?,
        ])
        .output()
        .with_context(|| "run syncweb create --no-daemon")?;
    let _ = fs::remove_dir_all(&folder_dir);
    let _ = fs::remove_dir_all(&data_dir);
    assert_success(&output, "create")?;
    let stdout = stdout_string(&output)?;
    ensure!(stdout.contains("path:"), "should print path: {stdout}");
    ensure!(stdout.contains("namespace:"), "should print namespace: {stdout}");
    ensure!(
        !stdout.contains("ticket"),
        "create should not print a ticket (use share): {stdout}"
    );
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
fn verbose_and_rust_log_control_logging() -> anyhow::Result<()> {
    let verbose = run(&["--verbose", "version"])?;
    assert_success(&verbose, "verbose version")?;
    let verbose_out = String::from_utf8(verbose.stderr).context("UTF-8 stderr")?;
    ensure!(
        verbose_out.contains("\"level\":\"DEBUG\""),
        "verbose should produce debug output on stderr: {verbose_out}"
    );

    let rust_log = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("version")
        .env("RUST_LOG", "syncweb=debug")
        .output()
        .context("run with RUST_LOG")?;
    assert_success(&rust_log, "RUST_LOG version")?;
    let rust_log_out = String::from_utf8(rust_log.stderr).context("UTF-8 stderr")?;
    ensure!(rust_log_out.contains("\"level\":\"DEBUG\""));
    Ok(())
}

#[test]
fn syncweb_core_dependency_version_matches_workspace() -> anyhow::Result<()> {
    let workspace_version = workspace_version()?;
    let cli_toml: toml::Value = toml::from_str(include_str!("../Cargo.toml")).context("parse cli Cargo.toml")?;
    let core_version = cli_toml
        .get("dependencies")
        .and_then(|d| d.get("syncweb-core"))
        .and_then(|c| c.get("version"))
        .and_then(|v| v.as_str())
        .context("syncweb-core version in cli Cargo.toml")?;
    ensure!(
        core_version == workspace_version,
        "syncweb-core dependency version '{core_version}' should match workspace version '{workspace_version}'"
    );
    Ok(())
}
