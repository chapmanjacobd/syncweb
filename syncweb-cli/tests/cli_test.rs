use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn version_command_outputs_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("version")
        .output()
        .expect("run syncweb version");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 output"),
        "syncweb 0.1.0\n"
    );
}

#[test]
fn help_output_lists_available_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("--help")
        .output()
        .expect("run syncweb help");

    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(help.contains("version"));
    assert!(help.contains("repl"));
    assert!(help.contains("create"));
    assert!(help.contains("join"));
    assert!(help.contains("accept"));
    assert!(help.contains("drop"));
    assert!(help.contains("folders"));
    assert!(help.contains("devices"));
    assert!(help.contains("network"));
    assert!(help.contains("config"));
}

#[test]
fn config_command_persists_bep_settings() {
    let directory = std::env::temp_dir().join(format!("syncweb-config-{}", uuid::Uuid::new_v4()));
    let set = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().expect("UTF-8 path"),
            "config",
            "set",
            "bep.enabled",
            "true",
        ])
        .output()
        .expect("run syncweb config set");
    assert!(set.status.success());

    let show = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            directory.to_str().expect("UTF-8 path"),
            "config",
            "show",
            "bep",
        ])
        .output()
        .expect("run syncweb config show");
    std::fs::remove_dir_all(directory).expect("remove config directory");

    assert!(show.status.success());
    let stdout = String::from_utf8(show.stdout).expect("UTF-8 output");
    assert!(stdout.contains("enabled = true"));
}

#[test]
fn devices_command_displays_iroh_and_syncthing_ids() {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", directory.to_str().expect("UTF-8 path"), "devices"])
        .output()
        .expect("run syncweb devices");
    std::fs::remove_dir_all(directory).expect("remove test directory");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("iroh: "));
    assert!(stdout.contains("syncthing: "));
}

#[test]
fn repl_command_starts_and_exits() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start syncweb repl");
    child
        .stdin
        .take()
        .expect("repl stdin")
        .write_all(b"help\nexit\n")
        .expect("write repl input");
    let output = child.wait_with_output().expect("wait for repl");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("syncweb repl"));
    assert!(stdout.contains("Commands: help, exit, quit"));
}

#[test]
fn verbose_logging_is_structured() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--verbose", "version"])
        .output()
        .expect("run verbose syncweb version");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.lines().next().is_some_and(|line| {
        line.contains("\"level\":\"DEBUG\"") && line.contains("\"message\":\"cli initialized\"")
    }));
}

#[test]
fn rust_log_controls_log_level() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("version")
        .env("RUST_LOG", "syncweb=debug")
        .output()
        .expect("run syncweb with RUST_LOG");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("\"level\":\"DEBUG\""));
}

#[test]
fn test_create_command() {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-create-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", directory.to_str().expect("UTF-8 path"), "create"])
        .output()
        .expect("run syncweb create");

    std::fs::remove_dir_all(&directory).expect("remove test directory");

    assert!(
        output.status.success(),
        "create should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("namespace: "), "should print namespace: {stdout}");
    assert!(stdout.contains("ticket:"), "should print ticket: {stdout}");
}

#[test]
fn test_folders_command_empty() {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-folders-{}", uuid::Uuid::new_v4()));
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", directory.to_str().expect("UTF-8 path"), "folders"])
        .output()
        .expect("run syncweb folders");

    std::fs::remove_dir_all(&directory).expect("remove test directory");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.trim().is_empty(), "no folders should be listed initially");
}

#[test]
fn test_folders_command_lists_created() {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-folders2-{}", uuid::Uuid::new_v4()));
    let data_dir = directory.to_str().expect("UTF-8 path").to_owned();

    let create_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "create"])
        .output()
        .expect("run syncweb create");
    assert!(create_output.status.success());

    let folders_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "folders"])
        .output()
        .expect("run syncweb folders");

    std::fs::remove_dir_all(&directory).expect("remove test directory");

    assert!(folders_output.status.success());
    let stdout = String::from_utf8(folders_output.stdout).expect("UTF-8 output");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "should list exactly one folder");
    let first_line = lines.first().expect("should have at least one line");
    assert!(
        first_line.contains("sendreceive"),
        "folder should show sendreceive mode: {first_line}"
    );
}

#[test]
fn test_join_command() {
    let directory = std::env::temp_dir().join(format!("syncweb-cli-join-{}", uuid::Uuid::new_v4()));
    let data_dir = directory.to_str().expect("UTF-8 path").to_owned();

    let create_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "create"])
        .output()
        .expect("run syncweb create");
    assert!(create_output.status.success());
    let create_stdout = String::from_utf8(create_output.stdout).expect("UTF-8 output");
    let ticket = create_stdout
        .lines()
        .find(|line| line.starts_with("ticket: "))
        .expect("should have ticket line")
        .trim_start_matches("ticket: ")
        .trim()
        .to_owned();

    let join_dir = directory.join("join_target");
    std::fs::create_dir(&join_dir).expect("create join dir");

    let join_output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            &data_dir,
            "join",
            &ticket,
            join_dir.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run syncweb join");

    std::fs::remove_dir_all(&directory).expect("remove test directory");

    assert!(
        join_output.status.success(),
        "join should succeed: {:?}",
        String::from_utf8_lossy(&join_output.stderr)
    );
    let stdout = String::from_utf8(join_output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("joined: "), "should print joined: {stdout}");
}

fn cli_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("syncweb-cli-phase3-{name}-{}", uuid::Uuid::new_v4()))
}

#[test]
fn test_ls_streaming() {
    let source = cli_test_dir("ls-streaming");
    std::fs::create_dir_all(source.join("sub")).expect("create dirs");
    std::fs::write(source.join("a.txt"), b"a").expect("write a");
    std::fs::write(source.join("sub/b.txt"), b"b").expect("write b");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["ls", source.to_str().expect("UTF-8 path")])
        .output()
        .expect("run syncweb ls");

    std::fs::remove_dir_all(&source).expect("cleanup");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "should list 2 files: {stdout}");
    assert!(lines.contains(&"a.txt"));
    assert!(lines.contains(&"sub/b.txt"));
}

#[test]
fn test_ls_sort() {
    let source = cli_test_dir("ls-sort");
    std::fs::create_dir_all(&source).expect("create dir");
    std::fs::write(source.join("large.txt"), [0_u8; 1000]).expect("write large");
    std::fs::write(source.join("small.txt"), b"s").expect("write small");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["ls", source.to_str().expect("UTF-8 path"), "--sort", "peers"])
        .output()
        .expect("run syncweb ls --sort");

    std::fs::remove_dir_all(&source).expect("cleanup");

    assert!(
        output.status.success(),
        "ls --sort should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert_eq!(stdout.lines().count(), 2, "should list 2 files when sorted: {stdout}");
}

#[test]
fn test_find_regex_glob_exact() {
    let source = cli_test_dir("find-modes");
    std::fs::create_dir_all(&source).expect("create dir");
    std::fs::write(source.join("report-01.pdf"), b"r").expect("write report");
    std::fs::write(source.join("data.txt"), b"d").expect("write data");

    let output_regex = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "find",
            r"report-\d+\.pdf",
            source.to_str().expect("UTF-8 path"),
            "--kind",
            "regex",
        ])
        .output()
        .expect("run syncweb find regex");
    assert!(output_regex.status.success());
    let stdout = String::from_utf8(output_regex.stdout).expect("UTF-8 output");
    assert!(stdout.contains("report-01.pdf"), "regex should find report: {stdout}");

    let output_glob = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["find", "*.txt", source.to_str().expect("UTF-8 path"), "--kind", "glob"])
        .output()
        .expect("run syncweb find glob");
    assert!(output_glob.status.success());
    let stdout_glob = String::from_utf8(output_glob.stdout).expect("UTF-8 output");
    assert!(
        stdout_glob.contains("data.txt"),
        "glob should find data.txt: {stdout_glob}"
    );

    let output_exact = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["find", "data", source.to_str().expect("UTF-8 path"), "--kind", "exact"])
        .output()
        .expect("run syncweb find exact");
    assert!(output_exact.status.success());
    let stdout_exact = String::from_utf8(output_exact.stdout).expect("UTF-8 output");
    assert!(
        stdout_exact.contains("data.txt"),
        "exact should find data.txt: {stdout_exact}"
    );

    std::fs::remove_dir_all(&source).expect("cleanup");
}

#[test]
fn test_sort_algorithms() {
    let source = cli_test_dir("sort-algo");
    std::fs::create_dir_all(&source).expect("create dir");
    std::fs::write(source.join("a.txt"), b"a").expect("write");
    std::fs::write(source.join("b.txt"), b"b").expect("write");

    for algorithm in ["niche", "frecency", "peers", "random", "folder"] {
        let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
            .args(["sort", source.to_str().expect("UTF-8 path"), "--by", algorithm])
            .output()
            .expect("run syncweb sort");
        assert!(
            output.status.success(),
            "sort --by {algorithm} should succeed: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert_eq!(
            stdout.lines().count(),
            2,
            "sort {algorithm} should list 2 files: {stdout}"
        );
    }

    std::fs::remove_dir_all(&source).expect("cleanup");
}

#[test]
fn test_stat_detailed() {
    let source = cli_test_dir("stat-detail");
    std::fs::create_dir_all(&source).expect("create dir");
    std::fs::write(source.join("file.txt"), b"hello world").expect("write");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["stat", source.join("file.txt").to_str().expect("UTF-8 path")])
        .output()
        .expect("run syncweb stat");

    std::fs::remove_dir_all(&source).expect("cleanup");

    assert!(
        output.status.success(),
        "stat should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("Path:"), "should show Path: {stdout}");
    assert!(stdout.contains("Size: 11"), "should show Size: 11 {stdout}");
    assert!(stdout.contains("Hash:"), "should show Hash: {stdout}");
    assert!(
        stdout.contains("Available: true"),
        "should show Available: true {stdout}"
    );

    let output_terse = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["stat", source.join("file.txt").to_str().expect("UTF-8 path"), "--terse"])
        .output()
        .expect("run syncweb stat --terse");

    if output_terse.status.success() {
        let stdout_terse = String::from_utf8(output_terse.stdout).expect("UTF-8 output");
        assert!(stdout_terse.contains("11"), "terse should contain size: {stdout_terse}");
    }
}

#[test]
fn test_download_selective() {
    let source = cli_test_dir("download-src");
    let dest = cli_test_dir("download-dest");
    std::fs::create_dir_all(source.join("sub")).expect("create dirs");
    std::fs::write(source.join("keep.txt"), b"keep").expect("write");
    std::fs::write(source.join("sub/nested.txt"), b"nested").expect("write");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "download",
            source.join("keep.txt").to_str().expect("UTF-8 path"),
            dest.join("copied.txt").to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run syncweb download single");

    assert!(
        output.status.success(),
        "download single file should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dest.join("copied.txt").exists());
    assert_eq!(std::fs::read(dest.join("copied.txt")).expect("read"), b"keep");

    let dir_dest = cli_test_dir("download-dir-dest");
    let output_dir = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "download",
            source.to_str().expect("UTF-8 path"),
            dir_dest.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run syncweb download directory");

    assert!(
        output_dir.status.success(),
        "download directory should succeed: {:?}",
        String::from_utf8_lossy(&output_dir.stderr)
    );
    assert!(dir_dest.join("keep.txt").exists());
    assert!(dir_dest.join("sub/nested.txt").exists());

    std::fs::remove_dir_all(&source).expect("cleanup source");
    std::fs::remove_dir_all(&dest).expect("cleanup dest");
    let _ = std::fs::remove_dir_all(&dir_dest);
}

#[test]
fn test_init_outputs_url() {
    let directory = cli_test_dir("init-test");
    let data_dir = cli_test_dir("init-data");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args([
            "--data-dir",
            data_dir.to_str().expect("UTF-8 path"),
            "init",
            directory.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run syncweb init");

    std::fs::remove_dir_all(&directory).expect("cleanup folder");
    std::fs::remove_dir_all(&data_dir).expect("cleanup data");

    assert!(
        output.status.success(),
        "init should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("path:"), "should print path: {stdout}");
    assert!(stdout.contains("namespace:"), "should print namespace: {stdout}");
    assert!(stdout.contains("ticket:"), "should print ticket: {stdout}");
    assert!(stdout.contains("share_url:"), "should print share_url: {stdout}");
}
