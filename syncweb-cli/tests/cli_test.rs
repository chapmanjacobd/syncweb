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
