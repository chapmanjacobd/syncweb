use anyhow::{Context, ensure};

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
fn config_round_trip() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let _ = alice.run_ok(&["config", "set", "bep.enabled", "true"])?;

    let show = alice.run_ok(&["config", "show"])?;
    ensure!(show.stdout().contains("bep"), "should show bep: {}", show.stdout());

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
