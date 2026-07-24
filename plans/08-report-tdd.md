# TDD Plan: `report` command

## Divergence
Name "report" implies submitting to an authority or sharing with peers. Actual: purely local annotation (hash + reason string) saved to a local JSON file — never leaves the machine. No signature, no identity, no network, no gossip.

## Current scope
- `handle_report` parses a hash, pushes a `ReportRecord { content, reason, created_at }` to `state.reports`
- Saved to `indexing-state.json` on disk
- Zero network communication
- Zero validation (no signature, no proof, no identity check)
- `ReportRecord` has no signature field, no sequence number, no topic identifier
- No other code ever reads `state.reports` — it's write-only dead data

## Decision
- Move `report` under `moderation` as `moderation report` — this clarifies its purpose
- Deprecate top-level `report` with a forwarding notice
- Add cryptographic signing of reports with the local node identity
- Add a gossip channel to broadcast signed reports to peers
- Add an `--import` flag to apply incoming reports from other peers as local moderation decisions

---

## Tests

### Phase 1 — Capture current (broken) "write-only" behavior

```rust
// syncweb-core/tests/moderation_test.rs  (new file)

#[test]
fn test_report_record_is_signed() {
    let report = ReportRecord::new(content_hash, "spam content")
        .sign_with(&identity_key)?;
    assert!(report.signature.is_some());
    assert!(report.verify(&identity_key.public()).is_ok());
}

#[test]
fn test_report_record_rejects_unsigned_verification() {
    let report = ReportRecord::new(content_hash, "spam content");
    assert!(report.verify(&some_key).is_err());
}
```

### Phase 2 — Report signature verification

```rust
#[test]
fn test_report_signature_mismatch_detected() {
    let key_a = SecretKey::generate();
    let key_b = SecretKey::generate();
    let report = ReportRecord::new(content_hash, "bad")
        .sign_with(&key_a)?;
    assert!(report.verify(&key_b.public()).is_err());
}

#[test]
fn test_report_tampering_detected() {
    let mut report = ReportRecord::new(content_hash, "bad")
        .sign_with(&key)?;
    report.reason = "tampered".to_owned();
    assert!(report.verify(&key.public()).is_err());
}
```

### Phase 3 — Report gossip propagation

```rust
// syncweb-core/tests/gossip_report_test.rs  (new file)

#[tokio::test]
async fn test_report_gossip_broadcasts_to_peers() -> anyhow::Result<()> {
    let (relay, relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let dir_a = TestDirectory::new("report-gossip-a")?;
    let dir_b = TestDirectory::new("report-gossip-b")?;

    let node_a = make_node(&dir_a, "alice", &relay, &relay_url).await?;
    let node_b = make_node(&dir_b, "bob", &relay, &relay_url).await?;

    let gossip_a = node_a.gossip_service();
    let gossip_b = node_b.gossip_service();

    // Both subscribe to the report topic (topic ID derived via blake3::hash per 01-shared-gossip-abstraction.md)
    let report_topic = TopicId::from_bytes(*blake3::hash(b"syncweb/reports").as_bytes());
    gossip_a.subscribe(report_topic).await?;
    gossip_b.subscribe(report_topic).await?;

    // Alice creates a signed report
    let report = ReportRecord::new(content_hash, "inappropriate content")
        .sign_with(node_a.endpoint().secret_key())?;

    // Alice broadcasts
    let bytes = serde_json::to_vec(&report)?;
    gossip_a.broadcast(report_topic, bytes.into()).await?;

    // Bob receives within timeout
    let received = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(msg) = gossip_b.try_recv(report_topic) {
                return msg;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await?;

    let received_report: ReportRecord = serde_json::from_slice(&received)?;
    assert_eq!(received_report.content, content_hash);

    // Bob verifies Alice's signature
    assert!(received_report.verify(&node_a.endpoint().public_key()).is_ok());

    node_a.stop().await?;
    node_b.stop().await?;
    Ok(())
}
```

### Phase 4 — `moderation report` CLI test

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_moderation_report_creates_signed_record() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("moderation-report");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir.to_str().unwrap(),
               "moderation", "report",
               "--record", &hash.to_string(),
               "--reason", "spam content"])
        .output()?;
    assert!(output.status.success());

    // Verify the report is persisted with a signature in SQLite (not JSON)
    // After 02-json-to-sqlite-migration, reports are in indexing.sqlite
    let db_path = data_dir.join("indexing.sqlite");
    let conn = rusqlite::Connection::open(&db_path)?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM reports", [], |r| r.get(0)
    )?;
    assert_eq!(count, 1);

    // Verify signature column is populated (not null)
    let has_sig: bool = conn.query_row(
        "SELECT signature IS NOT NULL FROM reports LIMIT 1", [], |r| r.get(0)
    )?;
    assert!(has_sig);

    fs::remove_dir_all(&data_dir)?;
    Ok(())
}

#[test]
fn test_deprecated_report_still_works_with_warning() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("report-deprecated");

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir.to_str().unwrap(),
               "report", &hash.to_string(), "--reason", "test"])
        .output()?;
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("deprecated"));

    fs::remove_dir_all(&data_dir)?;
    Ok(())
}
```

### Phase 5 — Auto-import of received reports as moderation

```rust
// syncweb-cli/tests/daemon_integration_test.rs

#[tokio::test]
async fn test_incoming_report_auto_moderates() -> anyhow::Result<()> {
    // 1. Bob's daemon is subscribed to the report gossip topic
    // 2. Alice sends a signed report about a hash
    // 3. Bob's daemon receives it, verifies the signature
    // 4. If Alice is trusted (or trust-on-first-report), auto-creates moderation record
    // 5. `syncweb moderation ls` should show the new record
    Ok(())
}
```

---

## Implementation

### Phase A — Add signing to `ReportRecord`

#### `syncweb-core/src/indexing/report.rs` (new file, or add to `moderation.rs`)

```rust
use iroh::PublicKey;
use ed25519_dalek::Signature;

/// Moderation report with optional cryptographic signing.
///
/// Persistence: stored in `indexing.sqlite` via the `reports` table
/// (Plan 02, migration v3). NOT stored in JSON — the JSON migration path
/// was superseded by the SQLite schema. The `reports` table has columns:
///   id, content_hash, reason, reporter, signature, created_at
///
/// Old reports from `indexing-state.json` are imported on first run via
/// the JSON import path in Plan 02's `import_indexing_state()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportRecord {
    pub content: Hash,
    pub reason: String,
    pub created_at: u64,
    pub reporter: Option<PublicKey>,    // NEW — hex pubkey of signer
    pub signature: Option<String>,      // NEW — hex-encoded Ed25519 signature
}

impl ReportRecord {
    pub fn new(content: Hash, reason: String) -> Self { ... }

    pub fn sign_with(mut self, secret_key: &SecretKey) -> Result<Self> {
        let payload = self.signing_payload();
        let sig = secret_key.sign(payload);
        self.reporter = Some(secret_key.public());
        self.signature = Some(hex::encode(sig.to_bytes()));
        Ok(self)
    }

    pub fn verify(&self, public_key: &PublicKey) -> Result<()> {
        let sig = self.signature.as_ref()
            .ok_or(SyncwebError::MissingSignature)?;
        let sig_bytes = hex::decode(sig)?;
        let signature = Signature::from_bytes(&sig_bytes);
        public_key.verify(self.signing_payload(), &signature)
            .map_err(|_| SyncwebError::InvalidSignature)
    }

    fn signing_payload(&self) -> Vec<u8> {
        // Canonical serialization: length-prefixed fields prevent ambiguity.
        // Using serde_json::to_vec on a canonical struct ensures the same
        // inputs always produce the same bytes, avoiding issues with
        // variable-length fields or platform-dependent representations.
        let canonical = (self.content.as_bytes(), &self.reason, self.created_at);
        serde_json::to_vec(&canonical).expect("serialization of canonical fields cannot fail")
    }
}
```

### Phase B — Gossip channel for reports

Uses the shared `TopicChannel<ReportRecord>` from `syncweb-core/src/gossip/` (defined in `01-shared-gossip-abstraction.md`). No separate `gossip_report.rs` module — call sites use `TopicChannel` directly (see Plan 01, which removes the old `indexing/gossip_report.rs`).

```rust
use syncweb_core::gossip::TopicChannel;

const REPORT_GOSSIP_TOPIC: &[u8] = b"syncweb/reports";
```

Usage in CLI handler:
```rust
let gossip_svc = node.gossip_service();
let topic = TopicChannel::<ReportRecord>::new(gossip_svc.gossip(), REPORT_GOSSIP_TOPIC);
topic.publish(&report).await?;
```

Usage in daemon listener:
```rust
let gossip_svc = self.node.gossip_service();
let topic = TopicChannel::<ReportRecord>::new(gossip_svc.gossip(), REPORT_GOSSIP_TOPIC);
let stream = gossip_svc.subscribe(topic.topic_id(), vec![]).await?;
let mut filtered = topic.receive_from(stream);
// ... process incoming reports ...
```

The `TopicChannel` handles serialization, deserialization, signature verification, and topic ID derivation automatically.

### Phase C — Wire `moderation report` CLI

#### `syncweb-cli/src/cli/commands.rs`

```rust
// Inside ModerationCommand enum:
#[command(about = "Sign and submit a moderation report (broadcast via gossip)")]
Report {
    #[arg(help = "Content hash to report")]
    record: String,
    #[arg(long, help = "Reason for the report")]
    reason: String,
    #[arg(long, help = "Also broadcast to peers via gossip")]
    broadcast: bool,
},

// Deprecate top-level Report
#[command(about = "Submit a moderation report (deprecated: use 'moderation report')")]
Report(ReportArgs),
```

#### `syncweb-cli/src/cli/indexing.rs` — `handle_moderation` dispatch

```rust
ModerationCommand::Report { record, reason, broadcast } => {
    let hash = parse_hash(&record)?;
    // Open node for signing + gossip
    let node = open_node(data_dir).await?;
    let report = ReportRecord::new(hash, reason)
        .sign_with(node.endpoint().secret_key())?;
    // Persist to indexing.sqlite (reports table, Plan 02 migration v3)
    indexing_db.insert_report(&report)?;
    // Broadcast if requested
    if broadcast {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let node = open_node(data_dir).await?;
            let gossip_svc = node.gossip_service();
            let topic = TopicChannel::<ReportRecord>::new(
                gossip_svc.gossip(), b"syncweb/reports"
            );
            topic.publish(&report).await?;
            node.stop().await?;
            Ok::<(), SyncwebError>(())
        })?;
    }
    // ...
}
```

### Phase D — Auto-import daemon listener

#### `syncweb-core/src/daemon/daemon.rs`

On startup, if indexing is enabled, spawn a task that:
1. Subscribes to `syncweb/reports` gossip topic (via `TopicChannel::<ReportRecord>::new(gossip, b"syncweb/reports")`)
2. Receives signed `ReportRecord` messages
3. Verifies the reporter's signature against the local trust policy (see `05-trust-delegate-tdd.md` for delegation infrastructure)
4. If the reporter is a trust root or has a valid delegation chain to the reported content, auto-creates a local `ModerationRecord` to hide the content
5. Logs the imported report

Note: The auto-import trust check depends on `05-trust-delegate-tdd.md` being implemented first. Without delegations, only self-signed reports (reporter == local node) or reports from configured trust roots are auto-imported. Unknown reporters are logged but not auto-moderated.

#### `ModerationRecord` type (auto-created from incoming reports)

```rust
/// A local moderation decision. Created when an incoming report passes trust verification,
/// or when the user manually moderates content.
///
/// Persistence: stored in a `moderation` table in `indexing.sqlite`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationRecord {
    pub content_hash: Hash,
    pub action: ModerationAction,
    pub reason: String,
    pub reporter: Option<PublicKey>,  // who reported it (None for local decisions)
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModerationAction {
    Hide,       // hide from local results
    Block,      // block content + provider
    Allow,      // explicit allow-override (whitelist)
}

impl ModerationRecord {
    pub fn from_report(report: &ReportRecord, action: ModerationAction) -> Self {
        Self {
            content_hash: report.content,
            action,
            reason: report.reason.clone(),
            reporter: report.reporter,
            created_at: report.created_at,
            expires_at: None,
        }
    }
}
```

#### `apply_report()` method on `IndexingService`

```rust
// In syncweb-core/src/indexing.rs or moderation.rs
impl IndexingService {
    /// Apply an incoming report as a local moderation decision.
    ///
    /// 1. Verifies the reporter is trusted via `TrustPolicy::is_trusted_for_at()`.
    /// 2. If trusted, auto-creates a `ModerationRecord` with `Block` action.
    /// 3. Unknown reporters are logged and ignored (no auto-moderation).
    pub fn apply_report(&self, report: &ReportRecord) -> Result<()> {
        let reporter = report.reporter
            .ok_or(SyncwebError::MissingSignature("report has no reporter".into()))?;
        if self.trust_policy.is_trusted_for_at(&reporter, &report.content, report.created_at).is_err() {
            tracing::info!("ignoring report from untrusted reporter: {}", reporter);
            return Ok(());
        }
        let moderation = ModerationRecord::from_report(report, ModerationAction::Block);
        self.database.insert_moderation(&moderation)?;
        Ok(())
    }
}
```

---

## Gossip/network integration note

- `report` currently has zero network integration — purely local
- The fix adds a dedicated gossip topic (`syncweb/reports`) for broadcasting signed reports
- Reports are self-authenticating (signed by the reporter's node key)
- Receiving peers can decide whether to trust the report based on their WoT configuration
- This aligns with the existing pattern: `trust stream publish` and gossiped `ProviderLease` messages already work this way

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-core/src/indexing.rs` | Add `pub mod report` (or add to `moderation.rs`) |
| `syncweb-core/src/indexing/report.rs` | New — `ReportRecord` with signing/verification, `SignedGossipMessage` impl |
| `syncweb-core/src/indexing/moderation.rs` | Add `apply_report()` to auto-create moderation records |
| `syncweb-core/src/daemon/daemon.rs` | Spawn report-gossip listener on startup |
| `syncweb-core/src/daemon/state.rs` | Track incoming report counts |
| `syncweb-cli/src/cli/commands.rs` | Move `report` under `moderation`, deprecate top-level |
| `syncweb-cli/src/cli/indexing.rs` | `handle_moderation` → add `Report` arm, add `handle_report` → deprecation wrapper |
| `syncweb-cli/src/main.rs` | Update dispatch |
| `syncweb-core/tests/moderation_test.rs` | New — signing, verification, tamper tests |
| `syncweb-core/tests/gossip_report_test.rs` | New — two-node gossip integration test |
| `syncweb-cli/tests/cli_test.rs` | `moderation report` CLI test, deprecated `report` warning test |
| `syncweb-cli/tests/daemon_integration_test.rs` | Auto-import daemon test |
| `syncweb-cli/tests/full_suite_test.rs` | Update help listing test |
