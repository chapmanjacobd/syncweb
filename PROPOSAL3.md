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
