use anyhow::{Context, ensure};

use super::*;

fn run(device: &Device, args: &[&str]) -> anyhow::Result<CmdOutput> {
    let mut all = vec!["--no-daemon"];
    all.extend_from_slice(args);
    device.run_ok(&all)
}

fn json(output: &CmdOutput) -> anyhow::Result<serde_json::Value> {
    serde_json::from_str(&output.stdout()).context("parse JSON output")
}

fn first<'a>(value: &'a serde_json::Value, label: &str) -> anyhow::Result<&'a serde_json::Value> {
    value
        .as_array()
        .and_then(|array| array.first())
        .with_context(|| format!("{label} should be a non-empty array: {value}"))
}

fn job_state(device: &Device, label: &str) -> anyhow::Result<serde_json::Value> {
    let output = run(device, &["--json", "transfer", "info"])?;
    let state = first(&json(&output)?, label)?
        .get("state")
        .context("transfer job state missing")?
        .clone();
    Ok(state)
}

#[test]
fn transfer_root_and_enqueue() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let root_dir = alice.data_dir().join("storage");
    let namespace = iroh_docs::NamespaceId::from(&[1_u8; 32]).to_string();
    let hash = iroh_blobs::Hash::from_bytes([2_u8; 32]).to_string();

    let root = run(
        alice,
        &[
            "transfer",
            "root",
            "root-a",
            root_dir.to_str().context("UTF-8 path")?,
            "--min-free",
            "0",
        ],
    )?;
    ensure!(root.stdout().contains("saved storage root"));

    let remaining = run(alice, &["--json", "transfer", "remaining"])?;
    let remaining_json = json(&remaining)?;
    let root_record = first(&remaining_json, "transfer remaining")?;
    ensure!(root_record.get("id") == Some(&serde_json::Value::from("root-a")));
    ensure!(root_record.get("enabled") == Some(&serde_json::Value::from(true)));

    let enqueue = run(
        alice,
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
    let job_id = enqueue
        .stdout()
        .lines()
        .find_map(|line| line.strip_prefix("queued transfer job "))
        .context("enqueue output missing job id")?
        .trim()
        .to_owned();

    let info = run(alice, &["--json", "transfer", "info"])?;
    let info_json = json(&info)?;
    let job = first(&info_json, "transfer info")?;
    ensure!(job.get("id") == Some(&serde_json::Value::from(job_id.as_str())));
    ensure!(job.get("namespace") == Some(&serde_json::Value::from(namespace.as_str())));
    ensure!(job.get("state") == Some(&serde_json::Value::from("queued")));
    ensure!(job.get("size") == Some(&serde_json::Value::from(1_048_576_u64)));

    let dry = run(alice, &["--json", "transfer", "allocate", "--dry-run"])?;
    let dry_json = json(&dry)?;
    ensure!(dry_json.get("dry_run") == Some(&serde_json::Value::from(true)));
    ensure!(
        dry_json
            .get("allocated")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|allocated| allocated.len() == 1),
        "dry run should allocate one job: {dry_json}"
    );

    let allocate = run(alice, &["transfer", "allocate"])?;
    ensure!(allocate.stdout().contains("allocated"), "allocate output");

    let assigned = run(alice, &["--json", "transfer", "info"])?;
    let assigned_json = json(&assigned)?;
    let assigned_job = first(&assigned_json, "transfer info after allocate")?;
    ensure!(
        assigned_job.get("root") == Some(&serde_json::Value::from("root-a")),
        "job should be assigned to root: {assigned_job}"
    );

    let _pause = run(alice, &["transfer", "pause", &job_id])?;
    ensure!(job_state(alice, "transfer info after pause")? == serde_json::Value::from("paused"));

    let _resume = run(alice, &["transfer", "resume", &job_id])?;
    ensure!(job_state(alice, "transfer info after resume")? == serde_json::Value::from("queued"));

    let _cancel = run(alice, &["transfer", "cancel", &job_id])?;
    ensure!(job_state(alice, "transfer info after cancel")? == serde_json::Value::from("cancelled"));

    let _retry = run(alice, &["transfer", "retry", &job_id])?;
    ensure!(job_state(alice, "transfer info after retry")? == serde_json::Value::from("queued"));

    Ok(())
}
