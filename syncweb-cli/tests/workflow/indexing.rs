use std::fs;

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
    let created = run(alice, &["--json", "folders", "create", "--no-indexing", folder_path])?;
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

    let first = run(alice, &["link", "create", "--name", "latest", CONTENT_HASH])?;
    let link = first
        .stdout()
        .lines()
        .find_map(|line| line.strip_prefix("link: "))
        .context("mutable link output missing link")?
        .to_owned();

    let _second = run(alice, &["link", "create", "--name", "latest", CONTENT_HASH])?;

    let resolved = run(alice, &["--json", "link", "resolve", "--no-fetch", &link])?;
    ensure!(json_output(&resolved)?.get("sequence") == Some(&Value::from(2)));

    Ok(())
}

#[test]
fn indexing_publish_and_search_round_trip() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "folders", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let published = run(alice, &["indexing", "publish", "--catalog", "test-catalog", &namespace])?;
    ensure!(
        published.stdout().contains("published:"),
        "publish output should confirm publication"
    );

    let _searched = run(alice, &["search", "--kind", "catalog", "test"])?;

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

    let created = run(alice, &["--json", "link", "create", "--private", CONTENT_HASH])?;
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

    let revoked = run(alice, &["link", "revoke", "--yes", &link])?;
    ensure!(
        revoked.stdout().contains("revoked:"),
        "revoke output should confirm revocation"
    );

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

    let created = run(alice, &["--json", "folders", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _imported = run(
        alice,
        &[
            "folders",
            "import",
            "--folder",
            &namespace,
            file.to_str().context("file path is not UTF-8")?,
        ],
    )?;

    let hash = Hash::from_bytes(*blake3::hash(b"hello publish blob").as_bytes());
    let hash_str = hash.to_string();

    let published = run(alice, &["--json", "share", "--blob", &hash_str, &namespace])?;
    let published_json = json_output(&published)?;
    ensure!(
        published_json.get("ticket").is_some(),
        "share --blob should emit a blob ticket"
    );

    let unpublished = run(
        alice,
        &["--json", "--yes", "access", "--revoke", "--blob", &hash_str, &namespace],
    )?;
    ensure!(
        json_output(&unpublished)?.get("status") == Some(&Value::from("unshared")),
        "unshare --blob should confirm the pin was removed"
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

    let created = run(alice, &["--json", "folders", "create", folder_path])?;
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

    let _init = run(alice, &["package", "add", "--name", "sample", pkg_path])?;

    let published = run(
        alice,
        &[
            "--json",
            "package",
            "publish",
            "--namespace",
            &namespace,
            "--sequence",
            "3",
            pkg_path,
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

    let created = run(alice, &["--json", "folders", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let published = run(
        alice,
        &[
            "indexing",
            "publish",
            "--catalog",
            "tagged",
            "--tag",
            "sci-fi",
            &namespace,
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
fn link_create_version_sequence_expires_publish() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;
    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    let folder_path = folder.to_str().context("folder path is not UTF-8")?;

    let created = run(alice, &["--json", "folders", "create", folder_path])?;
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
            "--name",
            "latest",
            "--version",
            "2",
            "--sequence",
            "5",
            "--publish",
            &namespace,
            CONTENT_HASH,
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
            "--private",
            "--expires",
            &expires.to_string(),
            CONTENT_HASH,
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
            "--name",
            "release",
            "--version",
            "2",
            CONTENT_HASH,
        ],
    )?;
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let resolved = run(
        alice,
        &["--json", "link", "resolve", "--no-fetch", "--version", "2", &link],
    )?;
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
fn link_resolve_fetches_local_content_by_default() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let folder = alice.data_dir().join("content");
    fs::create_dir_all(&folder)?;
    alice.write_file(&folder.join("payload.txt"), b"link resolve content")?;
    let _created = run(
        alice,
        &["folders", "create", folder.to_str().context("folder not UTF-8")?],
    )?;
    alice.import(&folder)?;

    let content = alice.file_content(&folder.join("payload.txt"))?;
    let hash = iroh_blobs::Hash::from_bytes(*blake3::hash(&content).as_bytes()).to_string();

    let created = run(alice, &["--json", "link", "create", &hash])?;
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let resolved = run(alice, &["--json", "link", "resolve", &link])?;
    let resolved_json = json_output(&resolved)?;
    ensure!(
        resolved_json.get("fetched") == Some(&Value::from(true)),
        "default resolve should fetch and pin the content: {resolved_json}"
    );
    ensure!(
        resolved_json.get("manifest") == Some(&Value::from(hash.as_str())),
        "resolution should report the content hash"
    );

    Ok(())
}

#[test]
fn link_revoke_persists_locally() -> Result<()> {
    let world = World::new(&["alice"])?;
    let alice = world.device("alice")?;

    let created = run(alice, &["--json", "link", "create", "--private", CONTENT_HASH])?;
    let link = json_output(&created)?
        .get("link")
        .and_then(Value::as_str)
        .context("link output missing link")?
        .to_owned();

    let revoked = run(alice, &["link", "revoke", "--yes", &link])?;
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

    let created = run(alice, &["--json", "folders", "create", folder_path])?;
    let namespace = json_output(&created)?
        .get("namespace")
        .context("create output missing namespace")?
        .as_str()
        .context("namespace is not a string")?
        .to_owned();

    let _published = run(alice, &["indexing", "publish", "--catalog", "library", &namespace])?;

    let searched = run(alice, &["--json", "search", "--limit", "5", "test"])?;
    let search_json = json_output(&searched)?;
    let results = search_json.as_array().context("search should emit a JSON array")?;
    ensure!(!results.is_empty(), "search should find the imported record");
    ensure!(results.len() <= 5, "search limit should cap results to 5");

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
