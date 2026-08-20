use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use anyhow::{Context, Result, ensure};
use ed25519_dalek::SigningKey;
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde_json::Value;
use syncweb_core::indexing::denylist::{DenylistRule, FilterList};

const CONTENT_HASH: &str = "26209f835986cd30d5925b3bdbd30358d6d7ae1ea0f863ab69b9c40c2b91b18a";

fn data_dir(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("syncweb-indexing-{test_name}-{}", uuid::Uuid::new_v4()))
}

fn run(data_dir: &Path, args: &[&str]) -> Result<Output> {
    let data_dir_arg = data_dir.to_str().context("data directory is not UTF-8")?;
    Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir_arg, "--no-daemon"])
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

    let attested = run(&data_dir, &["attest", "create", CONTENT_HASH, "--license", "MIT"])?;
    assert_success(&attested, "attest")?;

    let reported = run(
        &data_dir,
        &["moderation", "report", CONTENT_HASH, "--reason", "test report"],
    )?;
    assert_success(&reported, "moderation report")?;

    let hidden = run(&data_dir, &["moderation", "hide", CONTENT_HASH])?;
    assert_success(&hidden, "moderation hide")?;

    let listed = run(&data_dir, &["--json", "moderation", "ls"])?;
    assert_success(&listed, "moderation ls")?;
    ensure!(json_output(&listed)?.as_array().is_some_and(|items| items.len() == 1));

    let trust_output = run(&data_dir, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
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
        &["publish", "catalog", &namespace, "--catalog", "test-catalog"],
    )?;
    assert_success(&published, "publish catalog")?;
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
        trust_value == Some("trusted-delegation"),
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

    let shown = run(&data_dir, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
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

// ---------------------------------------------------------------------------
// Plan 004 — Sharing & publishing coverage
// ---------------------------------------------------------------------------

#[test]
fn publish_blob_and_unpublish_round_trip() -> Result<()> {
    let data_dir = data_dir("publish-blob");
    let folder = data_dir.join("content");
    fs::create_dir_all(&folder)?;
    let file = folder.join("hello.txt");
    fs::write(&file, b"hello publish blob")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(&data_dir, &["--json", "create", folder_path])?;
    assert_success(&created, "create")?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let imported = run(
        &data_dir,
        &[
            "import",
            file.to_str().context("file path is not UTF-8")?,
            "--folder",
            &namespace,
        ],
    )?;
    assert_success(&imported, "import")?;

    let hash = Hash::from_bytes(*blake3::hash(b"hello publish blob").as_bytes());
    let hash_str = hash.to_string();

    let published = run(&data_dir, &["--json", "publish", "blob", &namespace, &hash_str])?;
    assert_success(&published, "publish blob")?;
    let published_json = json_output(&published)?;
    ensure!(
        published_json.get("blob_ticket").is_some(),
        "publish blob should emit a blob ticket"
    );

    let unpublished = run(&data_dir, &["--json", "unpublish", &namespace, "--blob", &hash_str])?;
    assert_success(&unpublished, "unpublish blob")?;
    ensure!(
        json_output(&unpublished)?.get("status") == Some(&Value::from("unpublished")),
        "unpublish should confirm the pin was removed"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn publish_collection_with_sequence_and_bootstrap() -> Result<()> {
    let data_dir = data_dir("publish-collection");
    let folder = data_dir.join("content");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("readme.txt"), b"readme")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(&data_dir, &["--json", "create", folder_path])?;
    assert_success(&created, "create")?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let pkg = data_dir.join("pkg");
    fs::create_dir_all(&pkg)?;
    fs::write(pkg.join("lib.txt"), b"lib content")?;
    let pkg_path = pkg.to_str().context("pkg path is not UTF-8")?;

    let init = run(&data_dir, &["collection", "init", pkg_path, "--name", "sample"])?;
    assert_success(&init, "collection init")?;
    let add = run(&data_dir, &["collection", "add", pkg_path])?;
    assert_success(&add, "collection add")?;

    let published = run(
        &data_dir,
        &[
            "--json",
            "publish",
            "collection",
            pkg_path,
            "--namespace",
            &namespace,
            "--sequence",
            "3",
        ],
    )?;
    assert_success(&published, "publish collection")?;
    let published_json = json_output(&published)?;
    ensure!(
        published_json.get("sequence") == Some(&Value::from(3)),
        "publish collection should use the requested sequence"
    );
    ensure!(
        published_json.get("manifest").is_some(),
        "publish collection should emit a manifest hash"
    );
    ensure!(
        published_json.get("manifest_ticket").is_some(),
        "publish collection should emit a manifest ticket"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn publish_catalog_with_tags() -> Result<()> {
    let data_dir = data_dir("publish-catalog-tags");
    let folder = data_dir.join("content");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("clip.mp4"), b"video")?;
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
        &[
            "publish",
            "catalog",
            &namespace,
            "--catalog",
            "tagged",
            "--tag",
            "sci-fi",
        ],
    )?;
    assert_success(&published, "publish catalog with tag")?;
    let stdout = String::from_utf8_lossy(&published.stdout).to_string();
    ensure!(
        stdout.contains("published:"),
        "publish output should confirm publication"
    );
    ensure!(
        stdout.contains("catalog: tagged"),
        "publish output should reference the catalog"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn mirror_from_provider_and_network() -> Result<()> {
    let data_dir = data_dir("mirror");
    let content = data_dir.join("content");
    fs::create_dir_all(&content)?;
    fs::write(content.join("hello.txt"), b"hello mirror")?;
    let content_path = content.to_str().context("content path is not UTF-8")?;

    let net = run(&data_dir, &["network", "create", "mirror-net"])?;
    assert_success(&net, "network create")?;

    let created = run(
        &data_dir,
        &["--json", "create", "--network", "mirror-net", content_path],
    )?;
    assert_success(&created, "create")?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let imported = run(
        &data_dir,
        &[
            "import",
            content.join("hello.txt").to_str().context("file is not UTF-8")?,
            "--folder",
            &namespace,
        ],
    )?;
    assert_success(&imported, "import")?;

    let dry = run(&data_dir, &["--json", "mirror", "--network", "mirror-net", "--dry-run"])?;
    assert_success(&dry, "mirror --network --dry-run")?;
    let dry_json = json_output(&dry)?;
    ensure!(
        dry_json.get("dry_run") == Some(&Value::from(true)),
        "dry-run should report without fetching"
    );
    let total = dry_json
        .get("total_blobs")
        .and_then(Value::as_u64)
        .context("mirror result missing total_blobs")?;
    ensure!(total >= 1, "network mirror should discover at least one blob");

    let real = run(
        &data_dir,
        &[
            "--json",
            "mirror",
            "--network",
            "mirror-net",
            "--min-providers",
            "2",
            "--no-sharing",
        ],
    )?;
    assert_success(&real, "mirror --network")?;
    let real_json = json_output(&real)?;
    ensure!(
        real_json.get("total_blobs") == Some(&Value::from(total)),
        "real mirror should discover the same blobs"
    );
    ensure!(
        real_json.get("skipped") == Some(&Value::from(total)),
        "already-local blobs should be skipped"
    );
    ensure!(
        real_json.get("failed") == Some(&Value::from(0)),
        "no remote fetch should fail for local blobs"
    );

    let provider = iroh::SecretKey::generate().public().to_string();
    let provider_mirror = run(&data_dir, &["--json", "mirror", &provider])?;
    assert_success(&provider_mirror, "mirror provider")?;
    ensure!(
        json_output(&provider_mirror)?.get("total_blobs") == Some(&Value::from(0)),
        "unknown provider should expose no blobs to mirror"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn link_create_version_sequence_expires_publish() -> Result<()> {
    let data_dir = data_dir("link-create-options");
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

    let created_link = run(
        &data_dir,
        &[
            "--json",
            "link",
            "create",
            CONTENT_HASH,
            "--name",
            "latest",
            "--version",
            "2",
            "--sequence",
            "5",
            "--publish",
            &namespace,
        ],
    )?;
    assert_success(&created_link, "link create with options")?;
    let link = json_output(&created_link)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();
    ensure!(
        link.starts_with("syncweb://name/"),
        "mutable link should use a name URI"
    );

    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .saturating_add(3600);
    let created_private = run(
        &data_dir,
        &[
            "--json",
            "link",
            "create",
            CONTENT_HASH,
            "--private",
            "--expires",
            &expires.to_string(),
        ],
    )?;
    assert_success(&created_private, "link create --expires")?;
    let private_link = json_output(&created_private)?
        .get("link")
        .and_then(Value::as_str)
        .context("private link output missing link")?
        .to_owned();
    ensure!(
        private_link.starts_with("syncweb://private/"),
        "expiring link should be a private capability"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn link_resolve_with_version() -> Result<()> {
    let data_dir = data_dir("link-resolve-version");
    let created = run(
        &data_dir,
        &[
            "--json",
            "link",
            "create",
            CONTENT_HASH,
            "--name",
            "release",
            "--version",
            "2",
        ],
    )?;
    assert_success(&created, "link create")?;
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let resolved = run(&data_dir, &["--json", "link", "resolve", &link, "--version", "2"])?;
    assert_success(&resolved, "link resolve --version")?;
    let resolved_json = json_output(&resolved)?;
    ensure!(
        resolved_json.get("version") == Some(&Value::from("2")),
        "resolution should report the requested version"
    );
    ensure!(
        resolved_json.get("manifest").is_some(),
        "resolution should include a manifest"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn link_revoke_with_broadcast() -> Result<()> {
    let data_dir = data_dir("link-revoke-broadcast");
    let created = run(&data_dir, &["--json", "link", "create", CONTENT_HASH, "--private"])?;
    assert_success(&created, "link create private")?;
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let revoked = run(&data_dir, &["link", "revoke", &link, "--broadcast"])?;
    assert_success(&revoked, "link revoke --broadcast")?;
    let stdout = String::from_utf8_lossy(&revoked.stdout).to_string();
    ensure!(stdout.contains("revoked:"), "revoke output should confirm revocation");

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Plan 007 — Indexing, trust & attest, moderation coverage
// ---------------------------------------------------------------------------

#[test]
fn indexing_search_with_limit() -> Result<()> {
    let data_dir = data_dir("search-limit");
    let folder = data_dir.join("content");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("test-file.txt"), b"searchable content")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(&data_dir, &["--json", "create", folder_path])?;
    assert_success(&created, "create")?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let imported = run(
        &data_dir,
        &[
            "import",
            folder.join("test-file.txt").to_str().context("file is not UTF-8")?,
            "--folder",
            &namespace,
        ],
    )?;
    assert_success(&imported, "import")?;

    let enabled = run(&data_dir, &["indexing", "enable", &namespace])?;
    assert_success(&enabled, "indexing enable")?;

    let published = run(&data_dir, &["publish", "catalog", &namespace, "--catalog", "library"])?;
    assert_success(&published, "publish catalog")?;

    let searched = run(&data_dir, &["--json", "indexing", "search", "test", "--limit", "5"])?;
    assert_success(&searched, "indexing search --limit")?;
    let search_json = json_output(&searched)?;
    let results = search_json.as_array().context("search should emit a JSON array")?;
    ensure!(!results.is_empty(), "search should find the imported record");
    ensure!(results.len() <= 5, "search limit should cap results to 5");

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn indexing_meta_add_with_sequence() -> Result<()> {
    let data_dir = data_dir("meta-add-sequence");
    let added = run(
        &data_dir,
        &[
            "--json",
            "indexing",
            "meta",
            "add",
            CONTENT_HASH,
            "category",
            "test",
            "--sequence",
            "7",
        ],
    )?;
    assert_success(&added, "indexing meta add --sequence")?;
    let meta = json_output(&added)?;
    ensure!(meta.get("status") == Some(&Value::from("added")));
    ensure!(
        meta.get("sequence") == Some(&Value::from(7)),
        "meta add should record the requested sequence"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn indexing_filter_add_device_and_subscribe() -> Result<()> {
    let data_dir = data_dir("filter-device-subscribe");

    let added = run(&data_dir, &["indexing", "filter", "add", "device", "device-123"])?;
    assert_success(&added, "indexing filter add device")?;
    ensure!(
        String::from_utf8_lossy(&added.stdout).contains("added:"),
        "filter add should confirm the device rule"
    );

    let signing = SigningKey::from_bytes(&[9; 32]);
    let list = FilterList::new(
        NamespaceId::from([7; 32]),
        1,
        vec![DenylistRule::file(b"blocked.txt")],
        &signing,
    )?;
    let filter_path = data_dir.join("filter-list.json");
    fs::write(&filter_path, list.to_bytes()?)?;

    let subscribed = run(
        &data_dir,
        &[
            "--json",
            "indexing",
            "filter",
            "subscribe",
            filter_path.to_str().context("filter path is not UTF-8")?,
        ],
    )?;
    assert_success(&subscribed, "indexing filter subscribe")?;
    let status = json_output(&subscribed)?;
    ensure!(status.get("status") == Some(&Value::from("subscribed")));
    ensure!(status.get("sequence") == Some(&Value::from(1)));
    ensure!(status.get("entries") == Some(&Value::from(1)));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_delegate_with_scope_expiry_depth() -> Result<()> {
    let data_dir = data_dir("trust-delegate-options");
    let publisher = iroh::SecretKey::generate().public().to_string();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .saturating_add(3600);

    let delegated = run(
        &data_dir,
        &[
            "--json",
            "trust",
            "delegate",
            &publisher,
            "--scope",
            CONTENT_HASH,
            "--expires",
            &expires.to_string(),
            "--sequence",
            "2",
            "--max-depth",
            "2",
        ],
    )?;
    assert_success(&delegated, "trust delegate with options")?;
    let delegation = json_output(&delegated)?;
    ensure!(delegation.get("status") == Some(&Value::from("delegated")));
    ensure!(
        delegation.get("expires_at") == Some(&Value::from(expires)),
        "delegate should honor the requested expiration"
    );
    ensure!(delegation.get("scope") == Some(&Value::from(CONTENT_HASH)));
    ensure!(delegation.get("max_depth") == Some(&Value::from(2)));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_revoke_delegation() -> Result<()> {
    let data_dir = data_dir("trust-revoke-delegation");
    let publisher = iroh::SecretKey::generate().public().to_string();

    let delegated = run(&data_dir, &["trust", "delegate", &publisher])?;
    assert_success(&delegated, "trust delegate")?;
    let shown = run(&data_dir, &["--json", "trust", "show", &publisher])?;
    assert_success(&shown, "trust show before revoke")?;
    ensure!(
        json_output(&shown)?.get("trust") == Some(&Value::from("trusted-delegation")),
        "delegate should be trusted before revocation"
    );

    let revoked = run(&data_dir, &["--json", "trust", "revoke-delegation", &publisher])?;
    assert_success(&revoked, "trust revoke-delegation")?;
    ensure!(json_output(&revoked)?.get("status") == Some(&Value::from("revoked")));

    let shown_after = run(&data_dir, &["--json", "trust", "show", &publisher])?;
    assert_success(&shown_after, "trust show after revoke")?;
    ensure!(
        json_output(&shown_after)?.get("trust") == Some(&Value::from("untrusted")),
        "revoked delegation should no longer be trusted"
    );

    let scoped_publisher = iroh::SecretKey::generate().public().to_string();
    let scoped_delegated = run(
        &data_dir,
        &["trust", "delegate", &scoped_publisher, "--scope", CONTENT_HASH],
    )?;
    assert_success(&scoped_delegated, "trust delegate scoped")?;
    let scoped_revoked = run(
        &data_dir,
        &[
            "--json",
            "trust",
            "revoke-delegation",
            &scoped_publisher,
            "--scope",
            CONTENT_HASH,
        ],
    )?;
    assert_success(&scoped_revoked, "trust revoke-delegation --scope")?;
    let scoped = json_output(&scoped_revoked)?;
    ensure!(scoped.get("status") == Some(&Value::from("revoked")));
    ensure!(scoped.get("scope") == Some(&Value::from(CONTENT_HASH)));

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_provider_ban_scoped_and_durable() -> Result<()> {
    let data_dir = data_dir("provider-ban-scoped");
    let provider = iroh::SecretKey::generate().public().to_string();
    let duration = 120_u64;
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let banned = run(
        &data_dir,
        &[
            "--json",
            "trust",
            "provider",
            "ban",
            &provider,
            "--hash",
            CONTENT_HASH,
            "--duration",
            &duration.to_string(),
            "--reason",
            "scoped abuse",
        ],
    )?;
    assert_success(&banned, "trust provider ban")?;
    let result = json_output(&banned)?;
    ensure!(result.get("status") == Some(&Value::from("banned")));
    let ban = result.get("ban").context("ban output missing ban")?;
    ensure!(ban.get("hash") == Some(&Value::from(CONTENT_HASH)));
    ensure!(ban.get("reason") == Some(&Value::from("scoped abuse")));
    let expires_at = ban
        .get("expires_at")
        .and_then(Value::as_u64)
        .context("ban should expire")?;
    ensure!(
        expires_at.saturating_sub(before) >= duration,
        "ban duration should be honored"
    );

    let shown = run(
        &data_dir,
        &["--json", "trust", "provider", "show", &provider, "--hash", CONTENT_HASH],
    )?;
    assert_success(&shown, "trust provider show")?;
    let report = json_output(&shown)?;
    ensure!(
        report
            .get("bans")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1),
        "scoped ban should be visible for the content hash"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_provider_vouch_and_distrust_with_scope() -> Result<()> {
    let data_dir = data_dir("provider-vouch-scope");
    let provider = iroh::SecretKey::generate().public().to_string();

    let vouched = run(
        &data_dir,
        &[
            "--json",
            "trust",
            "provider",
            "vouch",
            &provider,
            "--scope",
            CONTENT_HASH,
        ],
    )?;
    assert_success(&vouched, "trust provider vouch --scope")?;
    let vouch = json_output(&vouched)?;
    ensure!(vouch.get("status") == Some(&Value::from("updated")));
    ensure!(vouch.get("action") == Some(&Value::from("vouch")));
    ensure!(vouch.get("scope") == Some(&Value::from(CONTENT_HASH)));
    ensure!(vouch.get("sequence") == Some(&Value::from(1)));

    let distrusted = run(
        &data_dir,
        &[
            "--json",
            "trust",
            "provider",
            "distrust",
            &provider,
            "--scope",
            CONTENT_HASH,
        ],
    )?;
    assert_success(&distrusted, "trust provider distrust --scope")?;
    let distrust = json_output(&distrusted)?;
    ensure!(distrust.get("action") == Some(&Value::from("distrust")));
    ensure!(distrust.get("sequence") == Some(&Value::from(2)));

    let shown = run(
        &data_dir,
        &["--json", "trust", "provider", "show", &provider, "--hash", CONTENT_HASH],
    )?;
    assert_success(&shown, "trust provider show")?;
    let report = json_output(&shown)?;
    ensure!(
        report.get("trust") == Some(&Value::from("distrusted")),
        "the most recent scoped record should win"
    );
    ensure!(
        report
            .get("records")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 2)
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn trust_stream_publish_with_hash_and_sequence() -> Result<()> {
    let data_dir = data_dir("trust-stream-hash");
    let provider = iroh::SecretKey::generate().public().to_string();

    let published = run(
        &data_dir,
        &[
            "--json",
            "trust",
            "stream",
            "publish",
            "--provider",
            &provider,
            "--signal",
            "failure",
            "--hash",
            CONTENT_HASH,
            "--sequence",
            "5",
        ],
    )?;
    assert_success(&published, "trust stream publish")?;
    let result = json_output(&published)?;
    ensure!(result.get("status") == Some(&Value::from("published")));
    ensure!(result.get("provider") == Some(&Value::from(provider)));
    ensure!(result.get("sequence") == Some(&Value::from(5)));
    let ticket = result
        .get("ticket")
        .and_then(Value::as_str)
        .context("trust stream ticket missing")?;
    ensure!(
        Path::new(ticket.strip_prefix("file://").unwrap_or(ticket)).exists(),
        "trust stream ticket file should be written"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn attest_provenance_derivative_and_broadcast() -> Result<()> {
    let data_dir = data_dir("attest-provenance");

    let provenance = run(
        &data_dir,
        &[
            "--json",
            "attest",
            "create",
            CONTENT_HASH,
            "--provenance",
            "archive",
            "--sequence",
            "3",
        ],
    )?;
    assert_success(&provenance, "attest create provenance")?;
    let provenance_json = json_output(&provenance)?;
    ensure!(provenance_json.get("status") == Some(&Value::from("attested")));
    ensure!(provenance_json.get("value") == Some(&Value::from("archive")));

    let derivative = run(
        &data_dir,
        &["--json", "attest", "create", CONTENT_HASH, "--derivative", "remix"],
    )?;
    assert_success(&derivative, "attest create derivative")?;
    ensure!(json_output(&derivative)?.get("status") == Some(&Value::from("attested")));

    let license = run(
        &data_dir,
        &[
            "--json",
            "attest",
            "create",
            CONTENT_HASH,
            "--license",
            "MIT",
            "--broadcast",
        ],
    )?;
    assert_success(&license, "attest create license broadcast")?;
    ensure!(json_output(&license)?.get("status") == Some(&Value::from("attested")));

    let shown = run(&data_dir, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
    assert_success(&shown, "trust show after attestations")?;
    let trust = json_output(&shown)?;
    let attestations = trust
        .get("attestations")
        .and_then(Value::as_array)
        .context("attestations missing")?;
    ensure!(attestations.len() == 3, "all three attestations should persist");
    ensure!(
        attestations.iter().any(|att| {
            att.get("value") == Some(&Value::from("archive")) && att.get("sequence") == Some(&Value::from(3))
        }),
        "provenance attestation should record its sequence"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn attest_verify_with_timeout() -> Result<()> {
    let data_dir = data_dir("attest-verify");
    let created = run(&data_dir, &["attest", "create", CONTENT_HASH, "--license", "MIT"])?;
    assert_success(&created, "attest create")?;

    let verified = run(
        &data_dir,
        &["--json", "attest", "verify", CONTENT_HASH, "--timeout", "1"],
    )?;
    assert_success(&verified, "attest verify --timeout")?;
    let results = json_output(&verified)?;
    ensure!(results.is_array(), "attest verify should emit a JSON array");
    ensure!(
        results.as_array().is_some_and(Vec::is_empty),
        "no peers should broadcast attestations for the hash"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}

#[test]
fn moderation_hide_with_reason_and_report_broadcast() -> Result<()> {
    let data_dir = data_dir("moderation-options");

    let hidden = run(
        &data_dir,
        &["--json", "moderation", "hide", CONTENT_HASH, "--reason", "private data"],
    )?;
    assert_success(&hidden, "moderation hide --reason")?;
    let hide = json_output(&hidden)?;
    ensure!(hide.get("status") == Some(&Value::from("hidden")));
    ensure!(hide.get("sequence") == Some(&Value::from(1)));

    let reported = run(
        &data_dir,
        &[
            "--json",
            "moderation",
            "report",
            CONTENT_HASH,
            "--reason",
            "abuse",
            "--broadcast",
        ],
    )?;
    assert_success(&reported, "moderation report --broadcast")?;
    let report = json_output(&reported)?;
    ensure!(report.get("status") == Some(&Value::from("reported")));
    ensure!(report.get("reason") == Some(&Value::from("abuse")));

    let trust_output = run(&data_dir, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
    assert_success(&trust_output, "trust show after moderation")?;
    ensure!(
        json_output(&trust_output)?.get("moderation") == Some(&Value::from("hide")),
        "hidden record should stay hidden after a report"
    );

    fs::remove_dir_all(data_dir)?;
    Ok(())
}
