mod workflow;

use anyhow::{Context, ensure};

use workflow::World;

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
