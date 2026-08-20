use std::fs;
use std::process::{Command, Output};

use anyhow::{Context, Result, ensure};

fn data_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("syncweb-transfer-{name}-{}", uuid::Uuid::new_v4()))
}

fn run(data_dir: &std::path::Path, args: &[&str]) -> Result<Output> {
    let data_dir_arg = data_dir.to_str().context("UTF-8 path")?;
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg, "--no-daemon"])
        .args(args)
        .output()
        .with_context(|| format!("run syncweb {args:?}"))
}

fn assert_success(output: &Output, label: &str) -> Result<()> {
    ensure!(
        output.status.success(),
        "{label} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn json(output: &Output) -> Result<serde_json::Value> {
    serde_json::from_slice(&output.stdout).context("parse JSON output")
}

fn first<'a>(value: &'a serde_json::Value, label: &str) -> Result<&'a serde_json::Value> {
    value
        .as_array()
        .and_then(|array| array.first())
        .with_context(|| format!("{label} should be a non-empty array: {value}"))
}

fn job_state(data_dir: &std::path::Path, label: &str) -> Result<serde_json::Value> {
    let output = run(data_dir, &["--json", "transfer", "info"])?;
    assert_success(&output, label)?;
    let state = first(&json(&output)?, "transfer info")?
        .get("state")
        .context("transfer job state missing")?
        .clone();
    Ok(state)
}

#[test]
fn transfer_root_and_enqueue() -> Result<()> {
    let data_dir = data_dir("root-enqueue");
    let root_dir = data_dir.join("storage");
    let namespace = iroh_docs::NamespaceId::from(&[1_u8; 32]).to_string();
    let hash = iroh_blobs::Hash::from_bytes([2_u8; 32]).to_string();

    let root = run(
        &data_dir,
        &[
            "transfer",
            "root",
            "root-a",
            root_dir.to_str().context("UTF-8 path")?,
            "--min-free",
            "0",
        ],
    )?;
    assert_success(&root, "transfer root")?;
    ensure!(String::from_utf8_lossy(&root.stdout).contains("saved storage root"));

    let remaining = run(&data_dir, &["--json", "transfer", "remaining"])?;
    assert_success(&remaining, "transfer remaining")?;
    let remaining_json = json(&remaining)?;
    let root_record = first(&remaining_json, "transfer remaining")?;
    ensure!(root_record.get("id") == Some(&serde_json::Value::from("root-a")));
    ensure!(root_record.get("enabled") == Some(&serde_json::Value::from(true)));

    let enqueue = run(
        &data_dir,
        &[
            "transfer",
            "enqueue",
            "--namespace",
            &namespace,
            "--path",
            "media/clip.mp4",
            "--hash",
            &hash,
            "1048576",
        ],
    )?;
    assert_success(&enqueue, "transfer enqueue")?;
    let job_id = String::from_utf8(enqueue.stdout)?
        .lines()
        .find_map(|line| line.strip_prefix("queued transfer job "))
        .context("enqueue output missing job id")?
        .trim()
        .to_owned();

    let info = run(&data_dir, &["--json", "transfer", "info"])?;
    assert_success(&info, "transfer info")?;
    let info_json = json(&info)?;
    let job = first(&info_json, "transfer info")?;
    ensure!(job.get("id") == Some(&serde_json::Value::from(job_id.as_str())));
    ensure!(job.get("namespace") == Some(&serde_json::Value::from(namespace.as_str())));
    ensure!(job.get("state") == Some(&serde_json::Value::from("queued")));
    ensure!(job.get("size") == Some(&serde_json::Value::from(1_048_576_u64)));

    let dry = run(&data_dir, &["--json", "transfer", "allocate", "--dry-run"])?;
    assert_success(&dry, "transfer allocate --dry-run")?;
    let dry_json = json(&dry)?;
    ensure!(dry_json.get("dry_run") == Some(&serde_json::Value::from(true)));
    ensure!(
        dry_json
            .get("allocated")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|allocated| allocated.len() == 1),
        "dry run should allocate one job: {dry_json}"
    );

    let allocate = run(&data_dir, &["transfer", "allocate"])?;
    assert_success(&allocate, "transfer allocate")?;
    let allocate_out = String::from_utf8_lossy(&allocate.stdout);
    ensure!(allocate_out.contains("allocated"), "allocate output: {allocate_out}");

    let assigned = run(&data_dir, &["--json", "transfer", "info"])?;
    assert_success(&assigned, "transfer info after allocate")?;
    let assigned_json = json(&assigned)?;
    let assigned_job = first(&assigned_json, "transfer info after allocate")?;
    ensure!(
        assigned_job.get("root") == Some(&serde_json::Value::from("root-a")),
        "job should be assigned to root: {assigned_job}"
    );

    let pause = run(&data_dir, &["transfer", "pause", &job_id])?;
    assert_success(&pause, "transfer pause")?;
    ensure!(job_state(&data_dir, "transfer info after pause")? == serde_json::Value::from("paused"));

    let resume = run(&data_dir, &["transfer", "resume", &job_id])?;
    assert_success(&resume, "transfer resume")?;
    ensure!(job_state(&data_dir, "transfer info after resume")? == serde_json::Value::from("queued"));

    let cancel = run(&data_dir, &["transfer", "cancel", &job_id])?;
    assert_success(&cancel, "transfer cancel")?;
    ensure!(job_state(&data_dir, "transfer info after cancel")? == serde_json::Value::from("cancelled"));

    let retry = run(&data_dir, &["transfer", "retry", &job_id])?;
    assert_success(&retry, "transfer retry")?;
    ensure!(job_state(&data_dir, "transfer info after retry")? == serde_json::Value::from("queued"));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}