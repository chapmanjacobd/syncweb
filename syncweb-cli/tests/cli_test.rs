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
fn help_output_lists_phase_one_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .arg("--help")
        .output()
        .expect("run syncweb help");

    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(help.contains("version"));
    assert!(help.contains("repl"));
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
