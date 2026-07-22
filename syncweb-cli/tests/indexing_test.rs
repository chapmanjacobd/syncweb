use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use anyhow::{Context, Result, ensure};
use serde_json::Value;

const CONTENT_HASH: &str = "26209f835986cd30d5925b3bdbd30358d6d7ae1ea0f863ab69b9c40c2b91b18a";

fn data_dir(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("syncweb-indexing-{test_name}-{}", uuid::Uuid::new_v4()))
}

fn run(data_dir: &Path, args: &[&str]) -> Result<Output> {
    let data_dir_arg = data_dir.to_str().context("data directory is not UTF-8")?;
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg])
        .args(args)
        .output()
        .with_context(|| format!("run syncweb {args:?}"))
}

fn assert_success(output: &Output, command: &str) -> Result<()> {
    ensure!(
        output.status.success(),
        "{command} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn json_output(output: &Output) -> Result<Value> {
    serde_json::from_slice(&output.stdout).context("parse JSON output")
}

#[test]
fn indexing_enable_disable_uses_persistent_folder_namespace() -> Result<()> {
    let data_dir = data_dir("enable-disable");
    let folder = data_dir.join("folder");
    fs::create_dir_all(&folder)?;

    let folder_path = folder.to_str().context("folder path is not UTF-8")?;
    let created = run(&data_dir, &["--json", "create", folder_path])?;
    assert_success(&created, "create")?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("create namespace is not a string")?
        .to_owned();

    let enabled = run(&data_dir, &["indexing", "enable", &namespace])?;
    assert_success(&enabled, "indexing enable")?;
    ensure!(String::from_utf8_lossy(&enabled.stdout).contains("enabled:"));

    let disabled = run(&data_dir, &["indexing", "disable", &namespace])?;
    assert_success(&disabled, "indexing disable")?;
    ensure!(String::from_utf8_lossy(&disabled.stdout).contains("disabled:"));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn mutable_links_advance_sequences_across_processes() -> Result<()> {
    let data_dir = data_dir("mutable-links");
    let first = run(&data_dir, &["link", "create", CONTENT_HASH, "--name", "latest"])?;
    assert_success(&first, "first mutable link")?;
    let link = String::from_utf8(first.stdout)?
        .lines()
        .find_map(|line| line.strip_prefix("link: "))
        .context("mutable link output missing link")?
        .to_owned();

    let second = run(&data_dir, &["link", "create", CONTENT_HASH, "--name", "latest"])?;
    assert_success(&second, "second mutable link")?;

    let resolved = run(&data_dir, &["--json", "link", "resolve", &link])?;
    assert_success(&resolved, "mutable link resolve")?;
    ensure!(json_output(&resolved)?.get("sequence") == Some(&Value::from(2)));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn attest_report_and_moderation_state_persist() -> Result<()> {
    let data_dir = data_dir("moderation");

    let attested = run(&data_dir, &["attest", CONTENT_HASH, "--license", "MIT"])?;
    assert_success(&attested, "attest")?;

    let reported = run(&data_dir, &["report", CONTENT_HASH, "--reason", "test report"])?;
    assert_success(&reported, "report")?;

    let hidden = run(&data_dir, &["moderation", "hide", CONTENT_HASH])?;
    assert_success(&hidden, "moderation hide")?;

    let listed = run(&data_dir, &["--json", "moderation", "ls"])?;
    assert_success(&listed, "moderation ls")?;
    ensure!(json_output(&listed)?.as_array().is_some_and(|items| items.len() == 1));

    let trust_output = run(&data_dir, &["--json", "trust", "show", CONTENT_HASH])?;
    assert_success(&trust_output, "trust show")?;
    let trust = json_output(&trust_output)?;
    ensure!(trust.get("moderation") == Some(&Value::from("hide")));
    ensure!(
        trust
            .get("attestations")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1)
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn indexing_publish_and_search_round_trip() -> Result<()> {
    let data_dir = data_dir("publish-search");
    let folder = data_dir.join("content");
    fs::create_dir_all(&folder)?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(&data_dir, &["--json", "create", folder_path])?;
    assert_success(&created, "create")?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let enabled = run(&data_dir, &["indexing", "enable", &namespace])?;
    assert_success(&enabled, "indexing enable")?;

    let published = run(
        &data_dir,
        &["indexing", "publish", &namespace, "--catalog", "test-catalog"],
    )?;
    assert_success(&published, "indexing publish")?;
    let stdout = String::from_utf8_lossy(&published.stdout).to_string();
    ensure!(
        stdout.contains("published:"),
        "publish output should confirm publication"
    );

    let searched = run(&data_dir, &["indexing", "search", "test"])?;
    assert_success(&searched, "indexing search")?;

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn indexing_health_checks_verified_providers() -> Result<()> {
    let data_dir = data_dir("health");
    let output = run(&data_dir, &["--json", "indexing", "health", CONTENT_HASH])?;
    assert_success(&output, "indexing health")?;
    let health = json_output(&output)?;
    ensure!(health.get("hash").is_some(), "health should report hash");
    ensure!(
        health.get("verified").and_then(Value::as_i64) == Some(0),
        "new hash should have zero verified providers"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn indexing_filter_add_persists_rule() -> Result<()> {
    let data_dir = data_dir("filter-add");
    let added = run(&data_dir, &["indexing", "filter", "add", "hash", CONTENT_HASH])?;
    assert_success(&added, "indexing filter add")?;
    let stdout = String::from_utf8_lossy(&added.stdout).to_string();
    ensure!(stdout.contains("added:"), "filter add should confirm addition");

    let added_file = run(&data_dir, &["indexing", "filter", "add", "file", "*.mp4"])?;
    assert_success(&added_file, "indexing filter add file")?;

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn link_create_private_and_revoke() -> Result<()> {
    let data_dir = data_dir("link-revoke");
    let created = run(&data_dir, &["--json", "link", "create", CONTENT_HASH, "--private"])?;
    assert_success(&created, "link create --private")?;
    let link = json_output(&created)?
        .get("link")
        .context("link output missing link")?
        .as_str()
        .context("link is not a string")?
        .to_owned();
    ensure!(
        link.starts_with("syncweb://private/"),
        "private link should use capability URI"
    );

    let revoked = run(&data_dir, &["link", "revoke", &link])?;
    assert_success(&revoked, "link revoke")?;
    let stdout = String::from_utf8_lossy(&revoked.stdout).to_string();
    ensure!(stdout.contains("revoked:"), "revoke output should confirm revocation");

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_delegate_and_show() -> Result<()> {
    let data_dir = data_dir("trust-delegate");
    let new_key = iroh::SecretKey::generate().public();
    let new_key_hex = new_key.to_string();

    let delegated = run(&data_dir, &["trust", "delegate", &new_key.to_string()])?;
    assert_success(&delegated, "trust delegate")?;
    let stdout = String::from_utf8_lossy(&delegated.stdout).to_string();
    ensure!(
        stdout.contains("delegated:"),
        "delegate output should confirm delegation"
    );

    let shown = run(&data_dir, &["--json", "trust", "show", &new_key_hex])?;
    assert_success(&shown, "trust show")?;
    let trust = json_output(&shown)?;
    let trust_value = trust.get("trust").and_then(Value::as_str);
    ensure!(
        trust_value == Some("trusted-delegation") || trust_value == Some("trusted-root"),
        "delegated publisher should be trusted, got {trust_value:?}"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn indexing_meta_add_persists_metadata() -> Result<()> {
    let data_dir = data_dir("meta-add");
    let added = run(
        &data_dir,
        &["indexing", "meta", "add", CONTENT_HASH, "title", "test content"],
    )?;
    assert_success(&added, "indexing meta add")?;
    let stdout = String::from_utf8_lossy(&added.stdout).to_string();
    ensure!(stdout.contains("metadata:"), "meta add should confirm metadata");

    let added_second = run(
        &data_dir,
        &["indexing", "meta", "add", CONTENT_HASH, "author", "tester"],
    )?;
    assert_success(&added_second, "indexing meta add second")?;

    let shown = run(&data_dir, &["--json", "trust", "show", CONTENT_HASH])?;
    assert_success(&shown, "trust show after meta add")?;
    let trust = json_output(&shown)?;
    ensure!(
        trust
            .get("metadata")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 2),
        "trust show should list two metadata entries"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn provider_trust_and_ban_commands_persist_across_processes() -> Result<()> {
    let data_dir = data_dir("provider-trust");
    let provider = iroh::SecretKey::generate().public().to_string();

    let vouched = run(&data_dir, &["trust", "provider", "vouch", &provider])?;
    assert_success(&vouched, "trust provider vouch")?;
    let shown = run(&data_dir, &["--json", "trust", "provider", "show", &provider])?;
    assert_success(&shown, "trust provider show")?;
    ensure!(json_output(&shown)?.get("trust") == Some(&Value::from("trusted")));

    let banned = run(
        &data_dir,
        &["trust", "provider", "ban", &provider, "--reason", "test ban"],
    )?;
    assert_success(&banned, "trust provider ban")?;
    let listed = run(&data_dir, &["--json", "trust", "provider", "list"])?;
    assert_success(&listed, "trust provider list")?;
    ensure!(
        json_output(&listed)?
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item.get("bans"))
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1)
    );

    let distrusted = run(&data_dir, &["trust", "provider", "distrust", &provider])?;
    assert_success(&distrusted, "trust provider distrust")?;
    let unbanned = run(&data_dir, &["trust", "provider", "unban", &provider])?;
    assert_success(&unbanned, "trust provider unban")?;
    let final_state = run(&data_dir, &["--json", "trust", "provider", "show", &provider])?;
    assert_success(&final_state, "trust provider final show")?;
    let final_state_json = json_output(&final_state)?;
    ensure!(final_state_json.get("trust") == Some(&Value::from("distrusted")));
    ensure!(
        final_state_json
            .get("bans")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_stream_publish_and_subscribe_aggregates_signed_signal() -> Result<()> {
    let publisher_dir = data_dir("trust-stream-publisher");
    let subscriber_dir = data_dir("trust-stream-subscriber");
    let provider = iroh::SecretKey::generate().public().to_string();
    let devices = run(&publisher_dir, &["devices"])?;
    assert_success(&devices, "publisher devices")?;
    let reporter = String::from_utf8(devices.stdout)?
        .lines()
        .find_map(|line| line.strip_prefix("iroh: "))
        .context("publisher identity missing")?
        .to_owned();

    let published = run(
        &publisher_dir,
        &[
            "--json",
            "trust",
            "stream",
            "publish",
            "--provider",
            &provider,
            "--signal",
            "failure",
        ],
    )?;
    assert_success(&published, "trust stream publish")?;
    let ticket = json_output(&published)?
        .get("ticket")
        .and_then(Value::as_str)
        .context("trust stream ticket missing")?
        .to_owned();

    let delegated = run(&subscriber_dir, &["trust", "delegate", &reporter])?;
    assert_success(&delegated, "delegate trust stream reporter")?;
    let subscribed = run(&subscriber_dir, &["--json", "trust", "stream", "subscribe", &ticket])?;
    assert_success(&subscribed, "trust stream subscribe")?;
    ensure!(json_output(&subscribed)?.get("accepted") == Some(&Value::from(1)));

    fs::remove_dir_all(publisher_dir)?;
    fs::remove_dir_all(subscriber_dir)?;
    Ok(())
}
