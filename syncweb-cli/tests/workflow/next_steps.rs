//! The core setup flows (`folders create`, `folders join`, `share`, and
//! `package install`) must end with the same concise next-steps block — in
//! human output and as a `next_steps` array under `--json` — that states
//! whether content was fetched eagerly or stays lazy.

use anyhow::{Context, ensure};

use super::*;

#[test]
fn create_shows_shared_next_steps() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("next-steps-create");
    let human = alice.run_ok(&[
        "--no-daemon",
        "folders",
        "create",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    let stdout = human.stdout();
    ensure!(
        stdout.contains("next steps:"),
        "create should end with the next-steps block: {stdout}"
    );
    ensure!(
        stdout.contains("join <link>"),
        "shared create should point at the join link: {stdout}"
    );
    ensure!(
        stdout.contains("on demand"),
        "create should explain the fetch model: {stdout}"
    );

    let other_dir = world.root().join("next-steps-create-json");
    let json = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "folders",
        "create",
        other_dir.to_str().context("UTF-8 path")?,
    ])?;
    let value: serde_json::Value = serde_json::from_str(&json.stdout()).context("create JSON")?;
    let steps = value
        .get("next_steps")
        .and_then(serde_json::Value::as_array)
        .context("create JSON should carry next_steps")?;
    ensure!(!steps.is_empty(), "create next_steps should not be empty: {value}");
    ensure!(
        steps
            .iter()
            .any(|step| step.as_str().is_some_and(|text| text.contains("on demand"))),
        "create next_steps should explain the fetch model: {value}"
    );

    Ok(())
}

#[test]
fn join_next_steps_distinguish_lazy_and_eager() -> anyhow::Result<()> {
    let world = World::new(&["alice", "bob"])?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_dir = world.root().join("next-steps-join-src");
    let info = alice.create(&folder_dir)?;
    alice.write_file(&folder_dir.join("doc.txt"), b"content")?;
    alice.import(&folder_dir)?;

    // Default join stays lazy and reports how to fetch on demand.
    let lazy_dir = world.root().join("next-steps-join-lazy");
    let lazy = bob.join(&info.ticket, &lazy_dir)?;
    let stdout = lazy.stdout();
    ensure!(
        stdout.contains("next steps:"),
        "join should end with the next-steps block: {stdout}"
    );
    ensure!(
        stdout.contains("lazy") && stdout.contains("syncweb download"),
        "lazy join should explain on-demand fetch: {stdout}"
    );
    ensure!(
        stdout.contains("live sync is off"),
        "bare join should point at --subscribe: {stdout}"
    );

    // The JSON envelope carries the same list.
    let json_dir = world.root().join("next-steps-join-json");
    let json = bob.run_ok(&[
        "--json",
        "--no-daemon",
        "folders",
        "join",
        &info.ticket,
        json_dir.to_str().context("UTF-8 path")?,
    ])?;
    let value: serde_json::Value = serde_json::from_str(&json.stdout()).context("join JSON")?;
    let steps = value
        .get("next_steps")
        .and_then(serde_json::Value::as_array)
        .context("join JSON should carry next_steps")?;
    ensure!(
        steps
            .iter()
            .any(|step| step.as_str().is_some_and(|text| text.contains("lazy"))),
        "join next_steps should explain the lazy default: {value}"
    );

    // An eager join reports the fetch model as eager.
    let eager_dir = world.root().join("next-steps-join-eager");
    let eager = bob.join_with_options(&["--download-existing"], &info.ticket, &eager_dir)?;
    ensure!(
        eager.stdout().contains("eagerly"),
        "eager join should report eager fetch: {}",
        eager.stdout()
    );

    Ok(())
}

#[test]
fn share_next_steps_flag_write_and_pin_state() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("next-steps-share");
    alice.create(&folder_dir)?;

    let read = alice.run_ok(&["--no-daemon", "share", folder_dir.to_str().context("UTF-8 path")?])?;
    let stdout = read.stdout();
    ensure!(
        stdout.contains("next steps:"),
        "share should end with the next-steps block: {stdout}"
    );
    ensure!(stdout.contains("read-only"), "read share should say so: {stdout}");
    ensure!(
        stdout.contains("pinned") && stdout.contains("eager"),
        "share should explain the seeding model: {stdout}"
    );
    ensure!(
        stdout.contains("access --revoke"),
        "share should explain how to revoke: {stdout}"
    );

    let write = alice.run_ok(&[
        "--no-daemon",
        "share",
        "--write",
        folder_dir.to_str().context("UTF-8 path")?,
    ])?;
    ensure!(
        write.stdout().contains("WRITE link"),
        "write share should carry the write warning step: {}",
        write.stdout()
    );

    Ok(())
}

#[test]
fn package_install_next_steps_state_eager_materialization() -> anyhow::Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder_dir = world.root().join("next-steps-package-folder");
    let info = alice.create(&folder_dir)?;

    let package_dir = world.root().join("next-steps-package-src");
    alice.write_file(&package_dir.join("lib.txt"), b"lib content")?;
    let package_path = package_dir.to_str().context("UTF-8 package path")?;

    alice.run_ok(&["--no-daemon", "package", "add", "--name", "example", package_path])?;
    let publish = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "package",
        "publish",
        "--namespace",
        &info.namespace,
        "--sequence",
        "1",
        package_path,
    ])?;
    let published: serde_json::Value = serde_json::from_str(&publish.stdout()).context("publish JSON")?;
    let ticket = published
        .get("manifest_ticket")
        .and_then(serde_json::Value::as_str)
        .context("publish missing manifest_ticket")?
        .to_owned();

    let install = alice.run_ok(&["--json", "--no-daemon", "package", "install", &ticket])?;
    let value: serde_json::Value = serde_json::from_str(&install.stdout()).context("install JSON")?;
    let steps = value
        .get("next_steps")
        .and_then(serde_json::Value::as_array)
        .context("install JSON should carry next_steps")?;
    ensure!(
        steps
            .iter()
            .any(|step| step.as_str().is_some_and(|text| text.contains("eager"))),
        "install next_steps should state eager materialization: {value}"
    );
    ensure!(
        steps
            .iter()
            .any(|step| step.as_str().is_some_and(|text| text.contains("package verify"))),
        "install next_steps should point at verify: {value}"
    );

    // The human flow gets the same block; a fresh version keeps the install
    // itself idempotent (re-installing a version fails by design).
    alice.run_ok(&["--no-daemon", "package", "bump", "--version", "2.0.0", package_path])?;
    let publish2 = alice.run_ok(&[
        "--json",
        "--no-daemon",
        "package",
        "publish",
        "--namespace",
        &info.namespace,
        "--sequence",
        "2",
        package_path,
    ])?;
    let published2: serde_json::Value = serde_json::from_str(&publish2.stdout()).context("publish2 JSON")?;
    let ticket2 = published2
        .get("manifest_ticket")
        .and_then(serde_json::Value::as_str)
        .context("second publish missing manifest_ticket")?
        .to_owned();

    let human = alice.run_ok(&["--no-daemon", "package", "install", &ticket2])?;
    let stdout = human.stdout();
    ensure!(
        stdout.contains("next steps:"),
        "install should end with the next-steps block: {stdout}"
    );
    ensure!(
        stdout.contains("eager"),
        "install should state eager materialization: {stdout}"
    );

    Ok(())
}
