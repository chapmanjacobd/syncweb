# TDD Plan: `trust delegate` / `trust revoke-delegation`

## Note

This plan was referenced by `08-report-tdd.md` (auto-import of reports based on WoT trust) and `07-attest-tdd.md` (verification based on trust delegations). Both depend on delegation infrastructure existing.

## Current State

No delegation concept exists. The Web of Trust (`WotState`) in `indexing-state.json` stores:
- `trust_roots`: self-signed roots
- `delegations`: empty (field exists, never populated)

`policy.is_trusted_for_at(issuer, content)` always returns `false` for non-root issuers because there are no delegations.

## Decision

Add two commands:
1. `trust delegate <from-pubkey> <to-pubkey> [--scope <hash>] [--max-depth <N>]` — create a delegation
2. `trust revoke-delegation <from-pubkey> <to-pubkey> [--scope <hash>]` — revoke an active delegation

Delegations are local bookkeeping (stored in `indexing.sqlite` via Plan 1). No gossip broadcast for v1 — delegations are local policy decisions, not network announcements.

---

## Tests

### Phase 1 — Delegation creation and storage

```rust
// syncweb-core/tests/trust_delegate_test.rs

#[test]
fn test_delegate_creates_new_delegation() {
    let mut state = IndexingState::default();
    let mut policy = TrustPolicy::new();
    let from = PublicKey::from_bytes(&[1u8; 32]);
    let to = PublicKey::from_bytes(&[2u8; 32]);
    let hash = Hash::from_bytes([4u8; 32]);
    let now = 1000;

    // Add `from` as a trust root so delegation chain is valid
    policy.roots.insert(from);

    state.delegations.push(Delegation {
        from: from.to_string(),
        to: to.to_string(),
        scope: None,
        max_depth: 1,
        created_at: now,
        revoked_at: None,
    });

    assert_eq!(state.delegations.len(), 1);
    assert!(policy.is_trusted_for_at(&to, &hash, 2000).is_ok());
}

#[test]
fn test_delegate_respects_max_depth() {
    // depth=1: to can attest, but to's delegates cannot
    let mut state = IndexingState::default();
    let mut policy = TrustPolicy::new();
    let from = PublicKey::from_bytes(&[1u8; 32]);
    let to = PublicKey::from_bytes(&[2u8; 32]);
    let third = PublicKey::from_bytes(&[3u8; 32]);
    let hash = Hash::from_bytes([4u8; 32]);

    policy.roots.insert(from);

    state.delegations.push(Delegation { from: from.to_string(), to: to.to_string(), max_depth: 1, scope: None, created_at: 1000, revoked_at: None });
    state.delegations.push(Delegation { from: to.to_string(), to: third.to_string(), max_depth: 1, scope: None, created_at: 1000, revoked_at: None });

    // `third` is trusted by `to`, but `from` only delegated to `to` with depth=1
    // `third` should NOT be trusted for `from`'s scope
    assert!(policy.is_trusted_for_at(&third, &hash, 2000).is_err());
}

#[test]
fn test_delegate_scoped_to_hash() {
    let mut state = IndexingState::default();
    let mut policy = TrustPolicy::new();
    let from = PublicKey::from_bytes(&[1u8; 32]);
    let hash_a = Hash::from_bytes([1u8; 32]);
    let hash_b = Hash::from_bytes([2u8; 32]);
    let to = PublicKey::from_bytes(&[2u8; 32]);

    policy.roots.insert(from);

    state.delegations.push(Delegation {
        from: from.to_string(), to: to.to_string(), scope: Some(hash_a.to_string()), max_depth: 1, created_at: 1000, revoked_at: None,
    });

    assert!(policy.is_trusted_for_at(&to, &hash_a, 2000).is_ok());
    assert!(policy.is_trusted_for_at(&to, &hash_b, 2000).is_err());
}
```

### Phase 2 — Delegation revocation

```rust
#[test]
fn test_delegate_revoke_removes_delegation() {
    let mut state = IndexingState::default();
    let mut policy = TrustPolicy::new();
    let from = PublicKey::from_bytes(&[1u8; 32]);
    let to = PublicKey::from_bytes(&[2u8; 32]);
    let hash = Hash::from_bytes([4u8; 32]);

    policy.roots.insert(from);

    // Create then revoke
    state.delegations.push(Delegation { from: from.to_string(), to: to.to_string(), max_depth: 1, scope: None, created_at: 1000, revoked_at: None });
    state.delegations[0].revoked_at = Some(1500);

    assert!(policy.is_trusted_for_at(&to, &hash, 2000).is_err());
}

#[test]
fn test_delegate_revoke_only_affects_target() {
    // Revoking delegation to `to` should not affect delegations to `other`
    let mut state = IndexingState::default();
    let mut policy = TrustPolicy::new();
    let from = PublicKey::from_bytes(&[1u8; 32]);
    let to = PublicKey::from_bytes(&[2u8; 32]);
    let other = PublicKey::from_bytes(&[3u8; 32]);
    let hash = Hash::from_bytes([4u8; 32]);

    policy.roots.insert(from);

    state.delegations.push(Delegation { from: from.to_string(), to: to.to_string(), max_depth: 1, scope: None, created_at: 1000, revoked_at: None });
    state.delegations.push(Delegation { from: from.to_string(), to: other.to_string(), max_depth: 1, scope: None, created_at: 1000, revoked_at: None });
    state.delegations[0].revoked_at = Some(1500);

    assert!(policy.is_trusted_for_at(&to, &hash, 2000).is_err());    // revoked
    assert!(policy.is_trusted_for_at(&other, &hash, 2000).is_ok());  // still valid
}
```

### Phase 3 — CLI integration

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_trust_delegate_subcommand() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("trust-delegate");
    let from = "f1a2b3c4...".to_string(); // existing root
    let to = "d5e6f7a8...".to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir.to_str().unwrap(),
               "trust", "delegate", &from, &to, "--max-depth", "2"])
        .output()?;
    assert!(output.status.success());

    // After 02-json-to-sqlite-migration, delegations live in indexing.sqlite
    let db_path = data_dir.join("indexing.sqlite");
    let conn = rusqlite::Connection::open(&db_path)?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM trust_delegations", [], |r| r.get(0)
    )?;
    assert_eq!(count, 1);

    fs::remove_dir_all(&data_dir)?;
    Ok(())
}

#[test]
fn test_trust_revoke_delegation_subcommand() -> anyhow::Result<()> {
    // Similar to above but with revoke
    Ok(())
}
```

## Gossip/network integration note

Delegations are purely local for v1. They represent your policy about who you trust to speak about what content. Broadcasting them over gossip is a future concern (it would let peers understand your trust topology). The existing `syncweb/provider-trust-stream/v1` topic could carry signed `TrustDelegation` messages in the future.

## Implementation

### `syncweb-core/src/indexing/wot.rs`

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delegation {
    pub from: String,          // hex pubkey of the delegator
    pub to: String,            // hex pubkey of the delegate
    pub scope: Option<String>, // optional content hash scope
    pub max_depth: u32,
    pub created_at: u64,
    pub revoked_at: Option<u64>,
}

impl TrustPolicy {
    /// Returns true if `issuer` is trusted to make claims about `content`
    /// at the given timestamp, considering delegation chain depth.
    pub fn is_trusted_for_at(&self, issuer: &PublicKey, content: &Hash, at: u64) -> Result<()> {
        // 1. Direct trust root?
        if self.roots.contains(issuer) {
            return Ok(());
        }
        // 2. Check delegations (BFS up to max_depth)
        self.check_delegation_chain(issuer, content, at)
    }

    fn check_delegation_chain(&self, issuer: &PublicKey, content: &Hash, at: u64) -> Result<()> {
        // BFS from issuer back to a root via delegation records.
        // For each delegation in the chain:
        //   1. Check scope: if delegation.scope is Some, verify it matches the content hash
        //   2. Check revocation: skip delegations where revoked_at <= at
        //   3. Check depth: track depth from root; fail if chain exceeds any delegation's max_depth
        //   4. If chain reaches a trust root (self.roots.contains(issuer)), return Ok(())
        //   5. If no valid chain found, return Err(SyncwebError::UntrustedIssuer)
        //
        // NOTE: This is a stub. The full BFS implementation is described above but
        // must be completed during implementation. Tests in Phase 1 (test_delegate_respects_max_depth)
        // and Phase 1 (test_delegate_scoped_to_hash) validate the expected behavior.
        // Until implemented, all non-root issuers will fail the trust check (return Err),
        // which is the correct safe default — no implicit trust without delegation evidence.
        Err(SyncwebError::UntrustedIssuer) // Stub: implement BFS per above spec
    }
}
```

### `syncweb-cli/src/cli/commands.rs`

```rust
pub struct TrustDelegateArgs {
    pub from: String,
    pub to: String,
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub max_depth: u32,
}

pub struct TrustRevokeDelegationArgs {
    pub from: String,
    pub to: String,
    #[arg(long)]
    pub scope: Option<String>,
}
```

## Dependency

This plan depends on `02-json-to-sqlite-migration.md` for the `trust_delegations` table in `indexing.sqlite`. Without that table, delegations are stored in `indexing-state.json` with no schema or query support.

## Integration with other plans

| Plan | Dependency on trust-delegate |
|------|------------------------------|
| `08-report-tdd.md` — auto-import | Verifies reporter's signature against local trust delegations |
| `07-attest-tdd.md` — verify | Checks if attestation issuer is trusted via delegation chain |
| `06-trust-vouch-tdd.md` | Shares trust signals via gossip; delegations determine signal weight |

## Files to modify

| File | Changes |
|------|---------|
| `syncweb-core/src/indexing/wot.rs` | Add `Delegation` struct, `check_delegation_chain()`, update `is_trusted_for_at()` |
| `syncweb-core/src/indexing.rs` | Re-export new types |
| `syncweb-core/tests/trust_delegate_test.rs` | NEW — delegation creation, revocation, scope tests |
| `syncweb-cli/src/cli/commands.rs` | Add `Delegate` and `RevokeDelegation` subcommands |
| `syncweb-cli/src/cli/indexing.rs` | Wire `handle_delegate` and `handle_revoke_delegation` |
| `syncweb-cli/tests/cli_test.rs` | CLI integration tests |