use std::fs;

use anyhow::{Context, ensure};

use super::*;

#[test]
fn ls_folder_lists_metadata_entries() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("meta-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("report.md"), b"# report")?;
    alice.write_file(&folder_dir.join("data.txt"), b"data")?;
    alice.import(&folder_dir)?;

    let listed = alice.ls(&folder_dir)?;
    ensure!(
        listed.iter().any(|line| line.contains("report.md")),
        "ls should list report.md from the metadata index: {listed:?}"
    );
    ensure!(
        listed.iter().any(|line| line.contains("data.txt")),
        "ls should list data.txt from the metadata index: {listed:?}"
    );
    Ok(())
}

#[test]
fn ls_plain_dir_errors_outside_folder() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let plain = world.root().join("plain-local-dir");
    fs::create_dir_all(&plain)?;
    fs::write(plain.join("local.txt"), b"x")?;

    let output = alice.run(&["--no-daemon", "ls", plain.to_str().context("UTF-8 path")?])?;
    ensure!(!output.success(), "ls on a plain directory should error");
    ensure!(
        output.stderr().contains("not inside of a Syncweb folder"),
        "error should explain the folder rule: {}",
        output.stderr()
    );
    Ok(())
}

#[test]
fn ls_local_only_lists_disk_anywhere() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let plain = world.root().join("disk-dir");
    fs::create_dir_all(plain.join("sub"))?;
    fs::write(plain.join("a.txt"), b"a")?;
    fs::write(plain.join("sub/b.txt"), b"b")?;

    let output = alice.run_ok(&[
        "--no-daemon",
        "ls",
        "--local-only",
        plain.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        output.stdout().contains("a.txt"),
        "--local-only lists disk files: {}",
        output.stdout()
    );
    ensure!(
        output.stdout().contains("sub/b.txt"),
        "--local-only lists nested disk files: {}",
        output.stdout()
    );
    Ok(())
}

#[test]
fn find_and_sort_over_metadata_table() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("find-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("note.md"), b"# note")?;
    alice.write_file(&folder_dir.join("big.bin"), &[0_u8; 500])?;
    alice.write_file(&folder_dir.join("small.txt"), b"s")?;
    alice.import(&folder_dir)?;

    let found = alice.find("*.md", &folder_dir)?;
    ensure!(
        found.iter().any(|line| line.contains("note.md")),
        "find should match the metadata index: {found:?}"
    );

    let sort = alice.run_ok(&[
        "--no-daemon",
        "sort",
        "--by",
        "size",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        sort.stdout().contains("big.bin"),
        "sort --by size should include the largest file: {}",
        sort.stdout()
    );

    let sort_threads = alice.run_ok(&[
        "--no-daemon",
        "sort",
        "--by",
        "size",
        "--threads",
        "8",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        sort_threads.stdout().contains("big.bin"),
        "--threads is accepted outside --local-only: {}",
        sort_threads.stdout()
    );
    Ok(())
}

#[test]
fn ls_path_prefix_filters_entries() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("prefix-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("docs/guide.md"), b"guide")?;
    alice.write_file(&folder_dir.join("other/log.txt"), b"log")?;
    alice.import(&folder_dir)?;

    let output = alice.run_ok(&[
        "--no-daemon",
        "ls",
        "--path-prefix",
        "docs",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        output.stdout().contains("docs/guide.md"),
        "--path-prefix docs should include docs/guide.md: {}",
        output.stdout()
    );
    ensure!(
        !output.stdout().contains("other/log.txt"),
        "--path-prefix docs should exclude other/log.txt: {}",
        output.stdout()
    );
    Ok(())
}

#[test]
fn remote_only_excludes_local_rows() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("remote-only-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("local.txt"), b"local")?;
    alice.import(&folder_dir)?;

    // Every imported row is local, so --remote-only narrows to nothing.
    let output = alice.run_ok(&[
        "--no-daemon",
        "ls",
        "--remote-only",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        !output.stdout().contains("local.txt"),
        "--remote-only should exclude local rows: {}",
        output.stdout()
    );
    Ok(())
}

#[test]
fn ls_json_emits_stable_envelope() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("json-folder");
    alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("hello.txt"), b"hello")?;
    alice.import(&folder_dir)?;

    let output = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "ls",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    let value: serde_json::Value = serde_json::from_str(&output.stdout()).context("ls --json should be valid JSON")?;
    ensure!(
        value.get("folder").is_some(),
        "envelope should carry folder: {}",
        output.stdout()
    );
    ensure!(
        value.get("path").is_some(),
        "envelope should carry path: {}",
        output.stdout()
    );
    let entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .context("envelope should carry entries array")?;
    ensure!(
        entries.iter().any(|entry| entry["path"] == "hello.txt"),
        "entries should include hello.txt: {entries:?}"
    );
    ensure!(
        entries.iter().any(|entry| entry["local"] == true),
        "imported rows should be local: {entries:?}"
    );
    Ok(())
}
