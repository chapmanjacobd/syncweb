use anyhow::{Context, ensure};
use std::fs;

use super::*;

#[test]
fn create_and_list_folder() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("shared-docs");
    let info = alice.create(&folder_dir)?;

    ensure!(!info.ticket.is_empty(), "ticket should not be empty");
    ensure!(!info.namespace.is_empty(), "namespace should not be empty");

    let folders = alice.folders()?;
    ensure!(
        folders.iter().any(|f| f.contains(&info.namespace)),
        "folders should list the created namespace: {folders:?}"
    );

    Ok(())
}

#[test]
fn write_import_find_stat() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("my-folder");
    alice.create(&folder_dir)?;

    let file_path = folder_dir.join("hello.txt");
    alice.write_file(&file_path, b"hello world")?;
    alice.import(&folder_dir)?;

    let ls = alice.ls(&folder_dir)?;
    ensure!(
        ls.iter().any(|f| f.contains("hello.txt")),
        "ls should find hello.txt: {ls:?}"
    );

    let found = alice.find("hello*", &folder_dir)?;
    ensure!(
        found.iter().any(|f| f.contains("hello.txt")),
        "find should match hello.txt: {found:?}"
    );

    let content = alice.file_content(&file_path)?;
    ensure!(content == b"hello world");

    Ok(())
}

#[test]
fn multiple_files_workflow() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("project");
    alice.create(&folder_dir)?;

    alice.write_file(&folder_dir.join("src/main.rs"), b"fn main() {}")?;
    alice.write_file(&folder_dir.join("README.md"), b"# Project")?;
    alice.write_file(&folder_dir.join("data/config.toml"), b"[section]")?;
    alice.import(&folder_dir)?;

    let ls = alice.ls(&folder_dir)?;
    ensure!(ls.iter().any(|f| f.contains("main.rs")), "ls: {ls:?}");
    ensure!(ls.iter().any(|f| f.contains("README.md")), "ls: {ls:?}");

    let found_rs = alice.find("*.rs", &folder_dir)?;
    ensure!(found_rs.iter().any(|f| f.contains("main.rs")), "find .rs: {found_rs:?}");

    let found_md = alice.find("*.md", &folder_dir)?;
    ensure!(
        found_md.iter().any(|f| f.contains("README.md")),
        "find .md: {found_md:?}"
    );

    Ok(())
}

#[test]
fn create_with_mode() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let output = alice.run_ok(&[
        "--no-daemon",
        "create",
        "--mode",
        "sendonly",
        world.root().join("sendonly-folder").to_str().context("UTF-8 path")?,
    ])?;
    ensure!(output.stdout().contains("namespace:"));

    Ok(())
}

#[test]
fn config_set_and_show_via_helpers() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let _ = alice.config_set("bep.enabled", "true")?;

    let show = alice.config_show()?;
    ensure!(show.stdout().contains("bep"), "should show bep: {}", show.stdout());

    Ok(())
}

#[test]
fn data_dir_helper_exposes_isolated_dir() -> anyhow::Result<()> {
    let world = World::new(&["alice", "bob"])?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    ensure!(
        alice.data_dir() == world.root().join("data-alice"),
        "alice data dir should be <root>/data-alice: {}",
        alice.data_dir().display()
    );
    ensure!(
        alice.data_dir().is_dir(),
        "alice data dir should exist: {}",
        alice.data_dir().display()
    );

    ensure!(
        bob.data_dir() == world.root().join("data-bob"),
        "bob data dir should be <root>/data-bob: {}",
        bob.data_dir().display()
    );
    ensure!(
        alice.data_dir() != bob.data_dir(),
        "devices should have distinct data dirs"
    );

    Ok(())
}

#[test]
fn multi_device_sync_workflow() -> anyhow::Result<()> {
    let world = World::new(&["alice", "bob"])?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    ensure!(
        world.devices().iter().any(|d| d.name() == "alice") && world.devices().iter().any(|d| d.name() == "bob"),
        "world.devices() should expose both devices"
    );
    ensure!(world.devices().len() == 2, "world.devices() should report both devices");

    let folder_dir_a = world.root().join("alice-sync");
    let info = alice.create(&folder_dir_a)?;
    alice.write_file(&folder_dir_a.join("note.txt"), b"synced note")?;
    alice.import(&folder_dir_a)?;

    let folder_dir_b = world.root().join("bob-sync");
    bob.join(&info.ticket, &folder_dir_b)?;

    let folders = bob.folders()?;
    ensure!(
        folders.iter().any(|f| f.contains(&info.namespace)),
        "bob should see the joined folder: {folders:?}"
    );

    Ok(())
}

#[test]
fn version_and_devices() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let version = alice.run_ok(&["version"])?;
    ensure!(version.stdout().contains("syncweb"));

    let devices = alice.run_ok(&["devices"])?;
    ensure!(!devices.stdout().is_empty(), "devices should produce output");

    Ok(())
}

#[test]
fn network_create_invite_leave() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let create = alice.network_create("team")?;
    ensure!(
        create.stdout().contains("created:"),
        "should print created: {}",
        create.stdout()
    );

    let list = alice.network_list()?;
    ensure!(list.iter().any(|l| l.contains("team")), "should list team: {list:?}");

    let invite = alice.network_invite("team")?;
    ensure!(
        invite.stdout().contains("syncweb://network/"),
        "should output ticket: {}",
        invite.stdout()
    );

    alice.network_leave("team")?;

    let list_after = alice.network_list()?;
    ensure!(
        list_after.iter().all(|l| !l.contains("team")),
        "team should be gone: {list_after:?}"
    );

    Ok(())
}

#[test]
fn snapshot_create_and_list() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("snap-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("doc.txt"), b"content")?;
    alice.import(&folder_dir)?;

    let snap = alice.snapshot_create(&folder_dir)?;
    ensure!(
        snap.stdout().contains("snapshot:"),
        "should print snapshot: {}",
        snap.stdout()
    );
    ensure!(
        snap.stdout().contains("files:"),
        "should print files: {}",
        snap.stdout()
    );

    let list = alice.snapshot_list()?;
    ensure!(!list.stdout().is_empty(), "snapshot list should have output");

    Ok(())
}

#[test]
fn snapshot_restore_diff_delete_round_trip() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("snap-roundtrip");
    alice.create(&folder_dir)?;
    let doc = folder_dir.join("doc.txt");
    alice.write_file(&doc, b"v1 content")?;
    let v1 = alice.snapshot_create_id(&folder_dir)?;

    alice.write_file(&doc, b"v2 content")?;
    let v2 = alice.snapshot_create_id(&folder_dir)?;

    ensure!(v1 != v2, "snapshots should differ");

    let diff = alice.snapshot_diff(&folder_dir, &v1, &v2)?;
    ensure!(
        diff.stdout().contains("modified"),
        "diff should report modified: {}",
        diff.stdout()
    );

    let restore_dir = world.root().join("snap-restored");
    let restore = alice.snapshot_restore(&restore_dir, &v1)?;
    ensure!(
        restore.stdout().contains("restored"),
        "restore output: {}",
        restore.stdout()
    );
    ensure!(
        fs::read(restore_dir.join("doc.txt"))? == b"v1 content",
        "restored content should match v1"
    );

    let delete = alice.snapshot_delete(&folder_dir, &v1)?;
    ensure!(
        delete.stdout().contains("deleted"),
        "delete output: {}",
        delete.stdout()
    );

    let list = alice.snapshot_list_json()?;
    let ids = list.as_array().context("snapshot list should be an array")?;
    ensure!(ids.len() == 1, "only v2 should remain after delete: {list}");
    let remaining = ids
        .first()
        .context("snapshot list should not be empty")?
        .get("id")
        .and_then(serde_json::Value::as_str)
        .context("snapshot id")?;
    ensure!(remaining == v2, "remaining snapshot should be v2, got {remaining}");

    Ok(())
}

#[test]
fn snapshot_create_with_description() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("snap-described");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("note.txt"), b"note")?;

    let snap = alice.snapshot_create_described(&folder_dir, "release-tag", "1")?;
    ensure!(
        snap.stdout().contains("snapshot:"),
        "should print snapshot: {}",
        snap.stdout()
    );

    let list = alice.snapshot_list_json()?;
    let array = list.as_array().context("snapshot list should be an array")?;
    ensure!(array.len() == 1, "should list one snapshot: {list}");
    let entry = array.first().context("snapshot list is empty")?;
    ensure!(
        entry.get("description") == Some(&serde_json::Value::from("release-tag")),
        "description should be recorded: {list}"
    );

    Ok(())
}

#[test]
fn stats_and_db_check() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let stats = alice.stats_network()?;
    ensure!(
        stats.stdout().contains("total_download"),
        "should show stats: {}",
        stats.stdout()
    );

    let check = alice.db_check()?;
    ensure!(
        check.stdout().contains("0 errors") || check.stdout().contains("healthy"),
        "db check should pass: {}",
        check.stdout()
    );

    Ok(())
}

#[test]
fn stat_shows_file_metadata() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("stat-folder");
    alice.create(&folder_dir)?;
    let file = folder_dir.join("data.bin");
    alice.write_file(&file, b"some data here")?;
    alice.import(&folder_dir)?;

    let stat = alice.stat(&file)?;
    ensure!(stat.stdout().contains("Path:"), "should show Path: {}", stat.stdout());
    ensure!(stat.stdout().contains("Size:"), "should show Size: {}", stat.stdout());

    Ok(())
}

#[test]
fn verify_folder_integrity() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("verify-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("test.txt"), b"verify me")?;
    alice.import(&folder_dir)?;

    let verify = alice.verify(&folder_dir)?;
    ensure!(
        verify.stdout().contains("ok")
            || verify.stdout().contains("passed")
            || verify.stdout().contains("healthy")
            || verify.success(),
        "verify should pass: {}",
        verify.stdout()
    );

    Ok(())
}

#[test]
fn two_devices_join_same_folder() -> anyhow::Result<()> {
    let world = World::new(&["alice", "bob"])?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_dir_a = world.root().join("alice-docs");
    let info = alice.create(&folder_dir_a)?;

    alice.write_file(&folder_dir_a.join("shared.txt"), b"from alice")?;
    alice.import(&folder_dir_a)?;

    let folder_dir_b = world.root().join("bob-docs");
    bob.join(&info.ticket, &folder_dir_b)?;

    let folders = bob.folders()?;
    ensure!(
        folders.iter().any(|f| f.contains(&info.namespace)),
        "bob should see the joined folder: {folders:?}"
    );

    Ok(())
}

#[test]
fn create_multiple_folders() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let dir1 = world.root().join("folder-a");
    let dir2 = world.root().join("folder-b");
    let info1 = alice.create(&dir1)?;
    let info2 = alice.create(&dir2)?;

    ensure!(
        info1.namespace != info2.namespace,
        "folders should have different namespaces"
    );

    let folders = alice.folders()?;
    ensure!(folders.len() >= 2, "should list at least 2 folders: {folders:?}");

    Ok(())
}

#[test]
fn join_with_mode_receiveonly() -> anyhow::Result<()> {
    let world = World::new(&["alice", "bob"])?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_dir = world.root().join("alice-ro");
    let info = alice.create(&folder_dir)?;

    let bob_dir = world.root().join("bob-ro");
    bob.join_with_options(&["--mode", "receiveonly"], &info.ticket, &bob_dir)?;

    let folders = bob.folders()?;
    ensure!(
        folders.iter().any(|f| f.contains(&info.namespace)),
        "bob should see the joined folder: {folders:?}"
    );

    Ok(())
}

#[test]
fn create_with_relay_fallback() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("relay-folder");
    let info = alice.run_ok(&[
        "--no-daemon",
        "create",
        "--relay-fallback",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(info.stdout().contains("namespace:"));
    ensure!(info.stdout().contains("ticket:"));

    Ok(())
}

#[test]
fn join_with_relay_fallback() -> anyhow::Result<()> {
    let world = World::new(&["alice", "bob"])?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_dir = world.root().join("relay-docs");
    let info = alice.create(&folder_dir)?;

    let bob_dir = world.root().join("bob-relay-docs");
    bob.join_with_options(&["--relay-fallback"], &info.ticket, &bob_dir)?;

    let folders = bob.folders()?;
    ensure!(
        folders.iter().any(|f| f.contains(&info.namespace)),
        "bob should see the joined folder: {folders:?}"
    );

    Ok(())
}

#[test]
fn join_subscribe_help_lists_new_options() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let help = alice.run_ok(&["join", "--help"])?;
    let text = help.stdout();
    ensure!(
        text.contains("--ignore-self"),
        "help should mention --ignore-self: {text}"
    );
    ensure!(text.contains("--prefix"), "help should mention --prefix: {text}");
    ensure!(text.contains("--max-count"), "help should mention --max-count: {text}");
    ensure!(text.contains("--max-size"), "help should mention --max-size: {text}");

    Ok(())
}

#[test]
fn leave_default_keeps_files() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("leave-keeps");
    let info = alice.create(&folder_dir)?;
    std::fs::write(folder_dir.join("file.txt"), b"content")?;

    alice.leave(&info.namespace)?;

    ensure!(folder_dir.exists(), "folder directory should remain after leave");

    Ok(())
}

#[test]
fn network_events_and_health() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let create = alice.run_ok(&["--json", "network", "create", "team"])?;
    let created: serde_json::Value = serde_json::from_str(&create.stdout()).context("parse network create JSON")?;
    let id = created
        .get("id")
        .and_then(serde_json::Value::as_str)
        .context("network id missing from create output")?;

    alice.run_ok(&["network", "invite", "team"])?;

    let events = alice.run_ok(&["network", "events", id, "--limit", "5"])?;
    ensure!(
        events.stdout().contains("Events for network"),
        "should print events header: {}",
        events.stdout()
    );
    ensure!(
        events.stdout().contains("member_added"),
        "should list member_added event: {}",
        events.stdout()
    );
    ensure!(
        events.stdout().contains("ticket_created"),
        "should list ticket_created event: {}",
        events.stdout()
    );

    let health = alice.run_ok(&["network", "health", "--network", id])?;
    ensure!(
        health.stdout().contains("events:"),
        "should report event count: {}",
        health.stdout()
    );
    ensure!(
        health.stdout().contains("sessions:"),
        "should report session count: {}",
        health.stdout()
    );

    Ok(())
}

#[test]
fn stats_files_by_and_top_largest() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("stats-files");
    let info = alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("small.txt"), b"s")?;
    alice.write_file(&folder_dir.join("medium.txt"), &[0_u8; 2048])?;
    alice.write_file(&folder_dir.join("large.txt"), &[0_u8; 4096])?;
    alice.import(&folder_dir)?;

    let report = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "stats",
        "files",
        "--folder",
        &info.namespace,
        "--by",
        "size",
        "--top-largest",
        "2",
    ])?;
    let value: serde_json::Value = serde_json::from_str(&report.stdout()).context("parse stats files JSON")?;
    ensure!(
        value.get("total_files") == Some(&serde_json::Value::from(3)),
        "should count 3 files: {value}"
    );
    let largest = value
        .get("largest_files")
        .and_then(serde_json::Value::as_array)
        .context("largest_files should be an array")?;
    ensure!(largest.len() == 2, "should list 2 largest files: {value}");
    let first = largest.first().context("largest_files is empty")?;
    ensure!(
        first.get(0).and_then(serde_json::Value::as_str) == Some("large.txt"),
        "largest file should be large.txt: {value}"
    );
    ensure!(
        first.get(1).and_then(serde_json::Value::as_u64) == Some(4096),
        "largest file size should be 4096: {value}"
    );

    Ok(())
}

#[test]
fn stats_seeding_with_content_filter() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("stats-seeding");
    let info = alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("doc.txt"), b"seeding content")?;
    alice.import(&folder_dir)?;

    let base = alice.run_ok(&["--json", "--no-daemon", "stats", "seeding", "--folder", &info.namespace])?;
    let base_json: serde_json::Value = serde_json::from_str(&base.stdout()).context("parse stats seeding JSON")?;
    ensure!(
        base_json.get("total") == Some(&serde_json::Value::from(1)),
        "should report 1 blob: {base_json}"
    );
    let hash = base_json
        .get("least_seeded")
        .and_then(serde_json::Value::as_array)
        .and_then(|a| a.first())
        .and_then(|b| b.get("hash"))
        .and_then(serde_json::Value::as_str)
        .context("seeding report missing hash")?;

    let filtered = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "stats",
        "seeding",
        "--folder",
        &info.namespace,
        "--hash",
        hash,
    ])?;
    let filtered_json: serde_json::Value =
        serde_json::from_str(&filtered.stdout()).context("parse filtered stats seeding JSON")?;
    ensure!(
        filtered_json.get("total") == Some(&serde_json::Value::from(1)),
        "matching hash should keep the blob: {filtered_json}"
    );

    let none = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "stats",
        "seeding",
        "--folder",
        &info.namespace,
        "--glob",
        "*.md",
    ])?;
    let none_json: serde_json::Value = serde_json::from_str(&none.stdout()).context("parse glob stats seeding JSON")?;
    ensure!(
        none_json.get("total") == Some(&serde_json::Value::from(0)),
        "non-matching glob should filter everything out: {none_json}"
    );

    Ok(())
}
