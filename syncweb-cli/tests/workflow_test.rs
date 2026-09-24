mod workflow;

use anyhow::{Context, ensure};

use workflow::World;

#[test]
fn status_reports_transfer_queue() -> anyhow::Result<()> {
    // User story 6 (Oli): `status` must surface the durable transfer queue
    // (pending/progress) in both the table and the --json envelope.
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let empty = alice.run_ok(&["--json", "status"])?;
    let empty_value: serde_json::Value = serde_json::from_str(&empty.stdout()).context("status JSON")?;
    let empty_transfers = empty_value.get("transfers").context("status should carry transfers")?;
    ensure!(
        empty_transfers.get("queued") == Some(&serde_json::Value::from(0)),
        "no jobs should be queued initially: {empty_transfers}"
    );
    ensure!(
        empty_transfers.get("pending_bytes") == Some(&serde_json::Value::from(0)),
        "no pending bytes initially: {empty_transfers}"
    );

    let namespace = iroh_docs::NamespaceId::from(&[1_u8; 32]).to_string();
    let hash = iroh_blobs::Hash::from_bytes([2_u8; 32]).to_string();
    let enqueue = alice.run_ok(&[
        "--no-daemon",
        "transfer",
        "enqueue",
        "--namespace",
        &namespace,
        "--path",
        "media/clip.mp4",
        "--hash",
        &hash,
        "--size",
        "1048576",
    ])?;
    ensure!(
        enqueue.stdout().contains("queued transfer job"),
        "enqueue output: {}",
        enqueue.stdout()
    );

    let status = alice.run_ok(&["--json", "status"])?;
    let value: serde_json::Value = serde_json::from_str(&status.stdout()).context("status JSON after enqueue")?;
    let transfers = value.get("transfers").context("status should carry transfers")?;
    ensure!(
        transfers.get("queued") == Some(&serde_json::Value::from(1)),
        "one queued job should be visible: {transfers}"
    );
    ensure!(
        transfers.get("pending_bytes") == Some(&serde_json::Value::from(1_048_576_u64)),
        "pending bytes should reflect the queued job: {transfers}"
    );
    ensure!(
        transfers.get("completed") == Some(&serde_json::Value::from(0)),
        "no completed jobs yet: {transfers}"
    );

    let human = alice.run_ok(&["status"])?;
    ensure!(
        human.stdout().contains("transfers: 1 queued"),
        "human status should show the transfer queue: {}",
        human.stdout()
    );

    Ok(())
}

#[test]
fn access_dashboard_lists_and_revokes_write() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("access-docs");
    let info = alice.create(&folder_dir)?;

    let human = alice.run_ok(&["--no-daemon", "access"])?;
    let text = human.stdout();
    ensure!(
        text.contains("Write?") && text.contains("Mode"),
        "access table should carry the Write?/Mode markers: {text}"
    );
    ensure!(text.contains("yes"), "a --write share should show Write? yes: {text}");

    let json = alice.run_ok(&["--json", "--no-daemon", "access"])?;
    let value: serde_json::Value = serde_json::from_str(&json.stdout()).context("access should emit JSON")?;
    let row_array = value.as_array().context("access JSON should be an array")?;
    ensure!(row_array.len() == 1, "the folder should be listed once: {value}");
    let row = row_array.first().context("access JSON is empty")?;
    ensure!(
        row.get("folder").and_then(serde_json::Value::as_str) == Some(info.namespace.as_str()),
        "row should target the created folder: {value}"
    );
    ensure!(
        row.get("write") == Some(&serde_json::Value::from(true)),
        "write share should be visible: {value}"
    );
    ensure!(
        row.get("mode").and_then(serde_json::Value::as_str) == Some("sendreceive"),
        "mode should be sendreceive: {value}"
    );
    let shares = row
        .get("shares")
        .and_then(serde_json::Value::as_array)
        .context("shares array")?;
    ensure!(
        shares
            .iter()
            .any(|s| s.get("access").and_then(serde_json::Value::as_str) == Some("write")),
        "a write share row should be present: {value}"
    );

    let revoked = alice.run_ok(&["--no-daemon", "access", "--revoke", &info.namespace, "--write", "--yes"])?;
    ensure!(
        revoked.stdout().contains("unshared"),
        "access --revoke --write should unshare: {}",
        revoked.stdout()
    );

    let after = alice.run_ok(&["--json", "--no-daemon", "access"])?;
    let after_value: serde_json::Value = serde_json::from_str(&after.stdout()).context("access JSON after revoke")?;
    let after_row = after_value
        .as_array()
        .and_then(|rows| rows.first())
        .context("one row should remain after revoke")?;
    ensure!(
        after_row.get("write") == Some(&serde_json::Value::from(false)),
        "revoking the write share should flip Write? to no: {after_value}"
    );
    let after_shares = after_row
        .get("shares")
        .and_then(serde_json::Value::as_array)
        .context("shares array after revoke")?;
    ensure!(
        after_shares
            .iter()
            .all(|s| s.get("access").and_then(serde_json::Value::as_str) != Some("write")),
        "no write share should remain: {after_value}"
    );

    Ok(())
}

#[test]
fn stats_network_period_json_shape() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let stats_db =
        syncweb_core::storage::stats_db::StatsDatabase::open(alice.data_dir().join("default").join("stats.db"))?;
    stats_db.record_download(1024, 1, Some("folderA"), Some("peer1"), None)?;
    stats_db.record_upload(512, 1, Some("folderA"), Some("peer1"), None)?;
    drop(stats_db);

    let json = alice.run_ok(&["--json", "stats", "network", "--period", "24h"])?;
    let value: serde_json::Value = serde_json::from_str(&json.stdout()).context("stats network should emit JSON")?;
    ensure!(
        value.get("total_upload").is_some(),
        "JSON should carry total_upload: {value}"
    );
    ensure!(
        value.get("total_download").is_some(),
        "JSON should carry total_download: {value}"
    );
    ensure!(
        value.get("per_folder").and_then(serde_json::Value::as_object).is_some(),
        "JSON should carry per_folder object: {value}"
    );
    ensure!(
        value.get("per_peer").and_then(serde_json::Value::as_object).is_some(),
        "JSON should carry per_peer object: {value}"
    );
    ensure!(
        value.get("period_start").is_some(),
        "JSON should carry period_start: {value}"
    );
    ensure!(
        value.get("total_download") == Some(&serde_json::Value::from(1024)),
        "download counter should be visible: {value}"
    );

    Ok(())
}

#[test]
fn stats_network_since_filters_window() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let stats_db =
        syncweb_core::storage::stats_db::StatsDatabase::open(alice.data_dir().join("default").join("stats.db"))?;
    stats_db.record_download(1024, 1, Some("recent"), Some("peer1"), None)?;
    drop(stats_db);

    // A future window boundary excludes every recorded transfer.
    let future = syncweb_core::daemon::current_timestamp().saturating_add(3600);
    let json = alice.run_ok(&["--json", "stats", "network", "--since", &future.to_string()])?;
    let value: serde_json::Value = serde_json::from_str(&json.stdout()).context("stats network --since JSON")?;
    ensure!(
        value.get("total_download") == Some(&serde_json::Value::from(0)),
        "a future --since should exclude all transfers: {value}"
    );
    let folder = value
        .get("per_folder")
        .and_then(serde_json::Value::as_object)
        .context("per_folder should be an object")?;
    ensure!(
        !folder.contains_key("recent"),
        "the recent transfer should be filtered out by a future --since: {value}"
    );

    // Without a window the transfer is visible again.
    let all = alice.run_ok(&["--json", "stats", "network"])?;
    let all_value: serde_json::Value =
        serde_json::from_str(&all.stdout()).context("stats network JSON without window")?;
    ensure!(
        all_value.get("total_download") == Some(&serde_json::Value::from(1024)),
        "without --since the transfer should be visible: {all_value}"
    );

    Ok(())
}

#[test]
fn stats_network_follow_once_prints_snapshot() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let stats_db =
        syncweb_core::storage::stats_db::StatsDatabase::open(alice.data_dir().join("default").join("stats.db"))?;
    stats_db.record_download(2048, 2, Some("folderA"), Some("peer1"), None)?;
    drop(stats_db);

    let once = alice.run_ok(&["--json", "stats", "network", "--follow", "--once"])?;
    let value: serde_json::Value =
        serde_json::from_str(&once.stdout()).context("--follow --once should emit a snapshot")?;
    ensure!(
        value.get("total_download") == Some(&serde_json::Value::from(2048)),
        "--follow --once should print the current snapshot: {value}"
    );
    ensure!(
        value.get("per_folder").is_some() && value.get("per_peer").is_some(),
        "follow snapshot should keep the stable envelope: {value}"
    );

    Ok(())
}

#[test]
fn stats_network_follow_streams_new_sync_events() -> anyhow::Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let data_dir = alice.data_dir().to_str().context("UTF-8 path")?.to_owned();

    let mut child = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", &data_dir, "--json", "stats", "network", "--follow"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn stats network --follow")?;
    let stdout = child.stdout.take().context("follow stdout")?;
    let mut lines = BufReader::new(stdout).lines();

    // The stream opens with the current snapshot.
    let first = lines
        .next()
        .context("follow should emit its opening snapshot")?
        .context("read first follow line")?;
    let opening: serde_json::Value = serde_json::from_str(&first).context("opening follow line should be JSON")?;
    ensure!(
        opening.get("total_upload").is_some() && opening.get("total_download").is_some(),
        "opening follow line should be a bandwidth snapshot: {opening}"
    );

    // Recording a sync session while the stream is live must surface as a line.
    let stats_db =
        syncweb_core::storage::stats_db::StatsDatabase::open(alice.data_dir().join("default").join("stats.db"))?;
    let session = stats_db.record_sync_session_start("net1", "folderA")?;
    stats_db.record_sync_session_finish(session, 3, 2048, 0, "completed")?;
    drop(stats_db);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut observed = false;
    while std::time::Instant::now() < deadline {
        match lines.next() {
            Some(Ok(line)) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line)
                    && value.get("event") == Some(&serde_json::Value::from("sync_session"))
                {
                    observed = true;
                    break;
                }
            }
            Some(Err(error)) => anyhow::bail!("follow stream errored: {error}"),
            None => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    ensure!(
        observed,
        "a sync session recorded while following should appear in the stream"
    );

    Ok(())
}
