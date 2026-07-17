# Proposal 3: Sharing UX and Transfer Policies

**Status:** Proposed
**Priority:** High
**Depends on:** PROPOSAL1, PROPOSAL4, PROPOSAL5

## Problem

The current plan exposes powerful primitives, but ordinary sharing still requires
understanding tickets, namespaces, and peers. Soulseek's durable advantage is a
simple loop: search, browse, queue, download, and watch for new content.

## Goals

- Make common file sharing discoverable and low-friction.
- Add queueing, fair scheduling, upload slots, and peer policies.
- Keep cryptographic capabilities underneath a friendly interface.
- Support both one-off downloads and continuous subscriptions.

## Design

Add a transfer scheduler with:

- per-peer and per-folder queues;
- configurable upload/download slots;
- pause, resume, cancel, retry, and priority;
- batch selection from search or collection results;
- per-peer quotas and rate limits;
- optional allowlists, denylists, and reciprocation policies.

Queue state is local and must not be treated as a global reputation system.
Content is fetched only after the manifest and provider information have been
verified.

## User-facing interface

```text
syncweb search <query>
syncweb queue add <result-or-path>
syncweb queue ls
syncweb queue pause <id>
syncweb browse <collection-or-publisher>
syncweb watch add <query>
syncweb transfers
```

Search results should support "download metadata," "preview," "queue file," and
"queue collection" actions.

## Scoped configuration

- **Network:** default fairness model, maximum concurrent peers, shared quotas.
- **Folder:** upload/download slots, priority, automatic-fetch policy.
- **File:** priority, preview policy, auto-fetch opt-in, and per-file deny.

Specific settings override inherited defaults. A file-level deny must win over an
automatic folder subscription.

## Implementation steps

1. Extract transfer scheduling from the sync engine.
2. Add persistent queue state and progress events.
3. Add slot, quota, and priority policies.
4. Wire search/browse results to queue actions.
5. Add preview-only and metadata-only operations.

## Acceptance criteria

- A user can search, queue multiple files, pause/resume transfers, and restart
  without losing queue state.
- Upload limits and priorities apply independently per peer and folder.
- A denied file is never fetched by an inherited automatic policy.
- All completed files pass manifest/blob verification before being materialized.

## Grounded UX example

The normal path should not expose tickets, namespaces, or provider selection:

```console
$ syncweb search 'live set flac'
ID       TITLE                         SIZE     SOURCES
r_18a    Live at Example Hall (FLAC)   1.4 GiB  3

$ syncweb queue add r_18a --include '**/*.flac'
Queued 12 files (1.4 GiB), priority normal

$ syncweb transfers
ID      STATE        PROGRESS          RATE       SOURCE
q_204   downloading  4/12, 388 MiB     18 MiB/s   2 of 3

$ syncweb queue pause q_204
Paused after current verified chunk

$ syncweb queue resume q_204 --priority high
Resumed; completed files will not be downloaded again
```

After restart, `transfers` should show the same queue and explain blocked states
such as `waiting for provider`, `denied by file policy`, or `quota resets in
18m`. A preview action should create a separate range-limited transfer rather
than silently promoting the entire file to the queue.

## Code patterns and library candidates

Represent scheduling as persisted intent plus derived runtime state:

```rust
enum TransferState {
    Queued,
    Resolving,
    Fetching { verified_bytes: u64 },
    Paused,
    Blocked(BlockReason),
    Complete,
    Failed { retry_at: Option<SystemTime>, error: String },
}

struct TransferRequest {
    id: TransferId,
    target: ContentRef,
    priority: Priority,
    policy_snapshot: PolicyDecision,
}
```

- Use a single scheduler actor fed by bounded `tokio::sync::mpsc` channels.
  `tokio::sync::Semaphore` can enforce global, peer, and folder slots.
- Persist state transitions transactionally in SQLite with `rusqlite`; recover
  nonterminal work as queued or paused rather than attempting to serialize live
  tasks.
- Use `tokio_util::sync::CancellationToken` for cooperative pause/cancel and
  `governor` or a token-bucket implementation for bandwidth and request quotas.
- Expose progress as a typed event stream. `indicatif` can render interactive
  progress, while `--json` emits stable newline-delimited events for scripts.
- Re-evaluate effective policy immediately before fetch and materialization.
  The stored policy snapshot explains why an item was queued but must not bypass
  a newer deny.

## Pros and cons

**Pros**

- Very high usefulness: persistent queues and clear progress turn lower-level
  sync primitives into an understandable daily workflow.
- Central scheduling makes fairness, quotas, retries, and observability
  consistent across manual downloads and subscriptions.
- Typed blocked reasons reduce support burden compared with generic failures.

**Cons**

- High runtime complexity: persistence, cancellation, retries, fairness, and
  provider failover create a substantial state machine.
- Multi-provider range scheduling can be deferred; implementing it early would
  add complexity before basic queue reliability is proven.
- Reciprocation policies can produce confusing incentives and should remain
  optional rather than part of correctness.
