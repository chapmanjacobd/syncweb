use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use ed25519_dalek::SigningKey;
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde_json::Value;
use syncweb_core::indexing::denylist::{DenylistRule, FilterList};

use super::*;

const CONTENT_HASH: &str = "26209f835986cd30d5925b3bdbd30358d6d7ae1ea0f863ab69b9c40c2b91b18a";

fn run(device: &Device, args: &[&str]) -> Result<CmdOutput> {
    let mut all = vec!["--no-daemon"];
    all.extend_from_slice(args);
    device.run_ok(&all)
}

fn json_output(output: &CmdOutput) -> Result<Value> {
    serde_json::from_str(&output.stdout()).context("parse JSON output")
}

#[test]
fn indexing_enable_disable_uses_persistent_folder_namespace() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("folder");
    fs::create_dir_all(&folder)?;

    let folder_path = folder.to_str().context("folder path is not UTF-8")?;
    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let enabled = run(alice, &["indexing", "enable", &namespace])?;
    ensure!(enabled.stdout().contains("enabled:"));

    let disabled = run(alice, &["indexing", "disable", &namespace])?;
    ensure!(disabled.stdout().contains("disabled:"));

    Ok(())
}

#[test]
fn mutable_links_advance_sequences_across_processes() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let first = run(alice, &["link", "create", CONTENT_HASH, "--name", "latest"])?;
    let link = first
        .stdout()
        .lines()
        .find_map(|line| line.strip_prefix("link: "))
        .context("mutable link output missing link")?
        .to_owned();

    let _second = run(alice, &["link", "create", CONTENT_HASH, "--name", "latest"])?;

    let resolved = run(alice, &["--json", "link", "resolve", &link])?;
    ensure!(json_output(&resolved)?.get("sequence") == Some(&Value::from(2)));

    Ok(())
}

#[test]
fn attest_report_and_moderation_state_persist() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let _attested = run(alice, &["attest", "create", CONTENT_HASH, "--license", "MIT"])?;

    let _reported = run(
        alice,
        &["moderation", "report", CONTENT_HASH, "--reason", "test report"],
    )?;

    let _hidden = run(alice, &["moderation", "hide", CONTENT_HASH])?;

    let listed = run(alice, &["--json", "moderation", "ls"])?;
    ensure!(json_output(&listed)?.as_array().is_some_and(|items| items.len() == 1));

    let trust_output = run(alice, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
    let trust = json_output(&trust_output)?;
    ensure!(trust.get("moderation") == Some(&Value::from("hide")));
    ensure!(
        trust
            .get("attestations")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1)
    );

    Ok(())
}

#[test]
fn indexing_publish_and_search_round_trip() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _enabled = run(alice, &["indexing", "enable", &namespace])?;

    let published = run(alice, &["publish", "catalog", &namespace, "--catalog", "test-catalog"])?;
    ensure!(
        published.stdout().contains("published:"),
        "publish output should confirm publication"
    );

    let _searched = run(alice, &["indexing", "search", "test"])?;

    Ok(())
}

#[test]
fn indexing_health_checks_verified_providers() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let output = run(alice, &["--json", "indexing", "health", CONTENT_HASH])?;
    let health = json_output(&output)?;
    ensure!(health.get("hash").is_some(), "health should report hash");
    ensure!(
        health.get("verified").and_then(Value::as_i64) == Some(0),
        "new hash should have zero verified providers"
    );

    Ok(())
}

#[test]
fn indexing_filter_add_persists_rule() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let added = run(alice, &["indexing", "filter", "add", "hash", CONTENT_HASH])?;
    ensure!(added.stdout().contains("added:"), "filter add should confirm addition");

    let _added_file = run(alice, &["indexing", "filter", "add", "file", "*.mp4"])?;

    Ok(())
}

#[test]
fn link_create_private_and_revoke() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let created = run(alice, &["--json", "link", "create", CONTENT_HASH, "--private"])?;
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

    let revoked = run(alice, &["link", "revoke", &link])?;
    ensure!(
        revoked.stdout().contains("revoked:"),
        "revoke output should confirm revocation"
    );

    Ok(())
}

#[test]
fn trust_delegate_and_show() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let new_key = iroh::SecretKey::generate().public();
    let new_key_hex = new_key.to_string();

    let delegated = run(alice, &["trust", "delegate", &new_key.to_string()])?;
    ensure!(
        delegated.stdout().contains("delegated:"),
        "delegate output should confirm delegation"
    );

    let shown = run(alice, &["--json", "trust", "show", &new_key_hex])?;
    let trust = json_output(&shown)?;
    let trust_value = trust.get("trust").and_then(Value::as_str);
    ensure!(
        trust_value == Some("trusted-delegation"),
        "delegated publisher should be trusted, got {trust_value:?}"
    );

    Ok(())
}

#[test]
fn indexing_meta_add_persists_metadata() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let added = run(
        alice,
        &["indexing", "meta", "add", CONTENT_HASH, "title", "test content"],
    )?;
    ensure!(added.stdout().contains("metadata:"), "meta add should confirm metadata");

    let _added_second = run(alice, &["indexing", "meta", "add", CONTENT_HASH, "author", "tester"])?;

    let shown = run(alice, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
    let trust = json_output(&shown)?;
    ensure!(
        trust
            .get("metadata")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 2),
        "trust show should list two metadata entries"
    );

    Ok(())
}

#[test]
fn provider_trust_and_ban_commands_persist_across_processes() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let provider = iroh::SecretKey::generate().public().to_string();

    let _vouched = run(alice, &["trust", "provider", "vouch", &provider])?;
    let shown = run(alice, &["--json", "trust", "provider", "show", &provider])?;
    ensure!(json_output(&shown)?.get("trust") == Some(&Value::from("trusted")));

    let _banned = run(alice, &["trust", "provider", "ban", &provider, "--reason", "test ban"])?;
    let listed = run(alice, &["--json", "trust", "provider", "list"])?;
    ensure!(
        json_output(&listed)?
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item.get("bans"))
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1)
    );

    let _distrusted = run(alice, &["trust", "provider", "distrust", &provider])?;
    let _unbanned = run(alice, &["trust", "provider", "unban", &provider])?;
    let final_state = run(alice, &["--json", "trust", "provider", "show", &provider])?;
    let final_state_json = json_output(&final_state)?;
    ensure!(final_state_json.get("trust") == Some(&Value::from("distrusted")));
    ensure!(
        final_state_json
            .get("bans")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    );

    Ok(())
}

#[test]
fn trust_stream_publish_and_subscribe_aggregates_signed_signal() -> Result<()> {
    let world = World::new(&["publisher", "subscriber"])?;
    let publisher_node = world.device("publisher")?;
    let subscriber_node = world.device("subscriber")?;
    let provider = iroh::SecretKey::generate().public().to_string();

    let devices = run(publisher_node, &["devices"])?;
    let reporter = devices
        .stdout()
        .lines()
        .find_map(|line| line.strip_prefix("iroh: "))
        .context("publisher identity missing")?
        .to_owned();

    let published = run(
        publisher_node,
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
    let ticket = json_output(&published)?
        .get("ticket")
        .and_then(Value::as_str)
        .context("trust stream ticket missing")?
        .to_owned();

    let _delegated = run(subscriber_node, &["trust", "delegate", &reporter])?;
    let subscribed = run(subscriber_node, &["--json", "trust", "stream", "subscribe", &ticket])?;
    ensure!(json_output(&subscribed)?.get("accepted") == Some(&Value::from(1)));

    Ok(())
}

// ---------------------------------------------------------------------------
// Plan 004 — Sharing & publishing coverage
// ---------------------------------------------------------------------------

#[test]
fn publish_blob_and_unpublish_round_trip() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    let file = folder.join("hello.txt");
    fs::write(&file, b"hello publish blob")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _imported = run(
        alice,
        &[
            "import",
            file.to_str().context("file path is not UTF-8")?,
            "--folder",
            &namespace,
        ],
    )?;

    let hash = Hash::from_bytes(*blake3::hash(b"hello publish blob").as_bytes());
    let hash_str = hash.to_string();

    let published = run(alice, &["--json", "publish", "blob", &namespace, &hash_str])?;
    let published_json = json_output(&published)?;
    ensure!(
        published_json.get("blob_ticket").is_some(),
        "publish blob should emit a blob ticket"
    );

    let unpublished = run(alice, &["--json", "unpublish", &namespace, "--blob", &hash_str])?;
    ensure!(
        json_output(&unpublished)?.get("status") == Some(&Value::from("unpublished")),
        "unpublish should confirm the pin was removed"
    );

    Ok(())
}

#[test]
fn publish_collection_with_sequence_and_bootstrap() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("readme.txt"), b"readme")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let pkg = alice.data_dir().join("pkg");
    fs::create_dir_all(&pkg)?;
    fs::write(pkg.join("lib.txt"), b"lib content")?;
    let pkg_path = pkg.to_str().context("pkg path is not UTF-8")?;

    let _init = run(alice, &["package", "init", pkg_path, "--name", "sample"])?;
    let _add = run(alice, &["package", "add", pkg_path])?;

    let published = run(
        alice,
        &[
            "--json",
            "package",
            "publish",
            pkg_path,
            "--namespace",
            &namespace,
            "--sequence",
            "3",
        ],
    )?;
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

    Ok(())
}

#[test]
fn publish_catalog_with_tags() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("clip.mp4"), b"video")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _enabled = run(alice, &["indexing", "enable", &namespace])?;

    let published = run(
        alice,
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
    ensure!(
        published.stdout().contains("published:"),
        "publish output should confirm publication"
    );
    ensure!(
        published.stdout().contains("catalog: tagged"),
        "publish output should reference the catalog"
    );

    Ok(())
}

#[test]
fn mirror_from_provider_and_network() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let content = alice.data_dir().join("content");
    fs::create_dir_all(&content)?;
    fs::write(content.join("hello.txt"), b"hello mirror")?;
    let content_path = content.to_str().context("content path is not UTF-8")?;

    let _net = run(alice, &["network", "create", "mirror-net"])?;

    let created = run(alice, &["--json", "create", "--network", "mirror-net", content_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _imported = run(
        alice,
        &[
            "import",
            content.join("hello.txt").to_str().context("file is not UTF-8")?,
            "--folder",
            &namespace,
        ],
    )?;

    let dry = run(alice, &["--json", "mirror", "--network", "mirror-net", "--dry-run"])?;
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
        alice,
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
    let provider_mirror = run(alice, &["--json", "mirror", &provider])?;
    ensure!(
        json_output(&provider_mirror)?.get("total_blobs") == Some(&Value::from(0)),
        "unknown provider should expose no blobs to mirror"
    );

    Ok(())
}

#[test]
fn link_create_version_sequence_expires_publish() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let created_link = run(
        alice,
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
        alice,
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
    let private_link = json_output(&created_private)?
        .get("link")
        .and_then(Value::as_str)
        .context("private link output missing link")?
        .to_owned();
    ensure!(
        private_link.starts_with("syncweb://private/"),
        "expiring link should be a private capability"
    );

    Ok(())
}

#[test]
fn link_resolve_with_version() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let created = run(
        alice,
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
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let resolved = run(alice, &["--json", "link", "resolve", &link, "--version", "2"])?;
    let resolved_json = json_output(&resolved)?;
    ensure!(
        resolved_json.get("version") == Some(&Value::from("2")),
        "resolution should report the requested version"
    );
    ensure!(
        resolved_json.get("manifest").is_some(),
        "resolution should include a manifest"
    );

    Ok(())
}

#[test]
fn link_revoke_with_broadcast() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let created = run(alice, &["--json", "link", "create", CONTENT_HASH, "--private"])?;
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let revoked = run(alice, &["link", "revoke", &link, "--broadcast"])?;
    ensure!(
        revoked.stdout().contains("revoked:"),
        "revoke output should confirm revocation"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Plan 007 — Indexing, trust & attest, moderation coverage
// ---------------------------------------------------------------------------

#[test]
fn indexing_search_with_limit() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("test-file.txt"), b"searchable content")?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _imported = run(
        alice,
        &[
            "import",
            folder.join("test-file.txt").to_str().context("file is not UTF-8")?,
            "--folder",
            &namespace,
        ],
    )?;

    let _enabled = run(alice, &["indexing", "enable", &namespace])?;

    let _published = run(alice, &["publish", "catalog", &namespace, "--catalog", "library"])?;

    let searched = run(alice, &["--json", "indexing", "search", "test", "--limit", "5"])?;
    let search_json = json_output(&searched)?;
    let results = search_json.as_array().context("search should emit a JSON array")?;
    ensure!(!results.is_empty(), "search should find the imported record");
    ensure!(results.len() <= 5, "search limit should cap results to 5");

    Ok(())
}

#[test]
fn indexing_meta_add_with_sequence() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let added = run(
        alice,
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
    let meta = json_output(&added)?;
    ensure!(meta.get("status") == Some(&Value::from("added")));
    ensure!(
        meta.get("sequence") == Some(&Value::from(7)),
        "meta add should record the requested sequence"
    );

    Ok(())
}

#[test]
fn indexing_filter_add_device_and_subscribe() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let added = run(alice, &["indexing", "filter", "add", "device", "device-123"])?;
    ensure!(
        added.stdout().contains("added:"),
        "filter add should confirm the device rule"
    );

    let signing = SigningKey::from_bytes(&[9; 32]);
    let list = FilterList::new(
        NamespaceId::from([7; 32]),
        1,
        vec![DenylistRule::file(b"blocked.txt")],
        &signing,
    )?;
    let filter_path = alice.data_dir().join("filter-list.json");
    fs::write(&filter_path, list.to_bytes()?)?;

    let subscribed = run(
        alice,
        &[
            "--json",
            "indexing",
            "filter",
            "subscribe",
            filter_path.to_str().context("filter path is not UTF-8")?,
        ],
    )?;
    let status = json_output(&subscribed)?;
    ensure!(status.get("status") == Some(&Value::from("subscribed")));
    ensure!(status.get("sequence") == Some(&Value::from(1)));
    ensure!(status.get("entries") == Some(&Value::from(1)));

    Ok(())
}

#[test]
fn trust_delegate_with_scope_expiry_depth() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let publisher = iroh::SecretKey::generate().public().to_string();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .saturating_add(3600);

    let delegated = run(
        alice,
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
    let delegation = json_output(&delegated)?;
    ensure!(delegation.get("status") == Some(&Value::from("delegated")));
    ensure!(
        delegation.get("expires_at") == Some(&Value::from(expires)),
        "delegate should honor the requested expiration"
    );
    ensure!(delegation.get("scope") == Some(&Value::from(CONTENT_HASH)));
    ensure!(delegation.get("max_depth") == Some(&Value::from(2)));

    Ok(())
}

#[test]
fn trust_revoke_delegation() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let publisher = iroh::SecretKey::generate().public().to_string();

    let _delegated = run(alice, &["trust", "delegate", &publisher])?;
    let shown = run(alice, &["--json", "trust", "show", &publisher])?;
    ensure!(
        json_output(&shown)?.get("trust") == Some(&Value::from("trusted-delegation")),
        "delegate should be trusted before revocation"
    );

    let revoked = run(alice, &["--json", "trust", "revoke-delegation", &publisher])?;
    ensure!(json_output(&revoked)?.get("status") == Some(&Value::from("revoked")));

    let shown_after = run(alice, &["--json", "trust", "show", &publisher])?;
    ensure!(
        json_output(&shown_after)?.get("trust") == Some(&Value::from("untrusted")),
        "revoked delegation should no longer be trusted"
    );

    let scoped_publisher = iroh::SecretKey::generate().public().to_string();
    let _scoped_delegated = run(
        alice,
        &["trust", "delegate", &scoped_publisher, "--scope", CONTENT_HASH],
    )?;
    let scoped_revoked = run(
        alice,
        &[
            "--json",
            "trust",
            "revoke-delegation",
            &scoped_publisher,
            "--scope",
            CONTENT_HASH,
        ],
    )?;
    let scoped = json_output(&scoped_revoked)?;
    ensure!(scoped.get("status") == Some(&Value::from("revoked")));
    ensure!(scoped.get("scope") == Some(&Value::from(CONTENT_HASH)));

    Ok(())
}

#[test]
fn trust_provider_ban_scoped_and_durable() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let provider = iroh::SecretKey::generate().public().to_string();
    let duration = 120_u64;
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let banned = run(
        alice,
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
        alice,
        &["--json", "trust", "provider", "show", &provider, "--hash", CONTENT_HASH],
    )?;
    let report = json_output(&shown)?;
    ensure!(
        report
            .get("bans")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1),
        "scoped ban should be visible for the content hash"
    );

    Ok(())
}

#[test]
fn trust_provider_vouch_and_distrust_with_scope() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let provider = iroh::SecretKey::generate().public().to_string();

    let vouched = run(
        alice,
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
    let vouch = json_output(&vouched)?;
    ensure!(vouch.get("status") == Some(&Value::from("updated")));
    ensure!(vouch.get("action") == Some(&Value::from("vouch")));
    ensure!(vouch.get("scope") == Some(&Value::from(CONTENT_HASH)));
    ensure!(vouch.get("sequence") == Some(&Value::from(1)));

    let distrusted = run(
        alice,
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
    let distrust = json_output(&distrusted)?;
    ensure!(distrust.get("action") == Some(&Value::from("distrust")));
    ensure!(distrust.get("sequence") == Some(&Value::from(2)));

    let shown = run(
        alice,
        &["--json", "trust", "provider", "show", &provider, "--hash", CONTENT_HASH],
    )?;
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

    Ok(())
}

#[test]
fn trust_stream_publish_with_hash_and_sequence() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let provider = iroh::SecretKey::generate().public().to_string();

    let published = run(
        alice,
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

    Ok(())
}

#[test]
fn attest_provenance_derivative_and_broadcast() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let provenance = run(
        alice,
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
    let provenance_json = json_output(&provenance)?;
    ensure!(provenance_json.get("status") == Some(&Value::from("attested")));
    ensure!(provenance_json.get("value") == Some(&Value::from("archive")));

    let derivative = run(
        alice,
        &["--json", "attest", "create", CONTENT_HASH, "--derivative", "remix"],
    )?;
    ensure!(json_output(&derivative)?.get("status") == Some(&Value::from("attested")));

    let license = run(
        alice,
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
    ensure!(json_output(&license)?.get("status") == Some(&Value::from("attested")));

    let shown = run(alice, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
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

    Ok(())
}

#[test]
fn attest_verify_with_timeout() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let _created = run(alice, &["attest", "create", CONTENT_HASH, "--license", "MIT"])?;

    let verified = run(alice, &["--json", "attest", "verify", CONTENT_HASH, "--timeout", "1"])?;
    let results = json_output(&verified)?;
    ensure!(results.is_array(), "attest verify should emit a JSON array");
    ensure!(
        results.as_array().is_some_and(Vec::is_empty),
        "no peers should broadcast attestations for the hash"
    );

    Ok(())
}

#[test]
fn moderation_hide_with_reason_and_report_broadcast() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let hidden = run(
        alice,
        &["--json", "moderation", "hide", CONTENT_HASH, "--reason", "private data"],
    )?;
    let hide = json_output(&hidden)?;
    ensure!(hide.get("status") == Some(&Value::from("hidden")));
    ensure!(hide.get("sequence") == Some(&Value::from(1)));

    let reported = run(
        alice,
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
    let report = json_output(&reported)?;
    ensure!(report.get("status") == Some(&Value::from("reported")));
    ensure!(report.get("reason") == Some(&Value::from("abuse")));

    let trust_output = run(alice, &["--json", "trust", "show", CONTENT_HASH, "--content"])?;
    ensure!(
        json_output(&trust_output)?.get("moderation") == Some(&Value::from("hide")),
        "hidden record should stay hidden after a report"
    );

    Ok(())
}
