# Conflict Resolution Plan

Status: proposal. No conflict handling is implemented today. This document
surveys the design space, compares the options, and recommends a phased
approach.

## 1. Problem

iroh-docs is a multi-writer CRDT: records are keyed by
`(namespace, author, key)`, so two devices that edit the same path while
offline both keep a record for that key. There is no lost update at the CRDT
level — but syncweb never surfaces the divergence:

- Every read path collapses variants with
  `Query::single_latest_per_key()` (`syncweb-core/src/node/docs_engine.rs:282,299`).
  The winner is the entry with the greatest `timestamp()`; ties keep the first.
- `SyncEngine` observes `LiveEvent`s only to update counters/checkpoints
  (`syncweb-core/src/sync/engine.rs:487-545`); it never writes files.
- Materialization is explicit (`download`/`join --download`) and also reads the
  collapsed view (`syncweb-core/src/sync/partial_fetch.rs:66-93`,
  `syncweb-core/src/daemon/ipc.rs:2119-2161`).
- The daemon watcher re-imports on-disk files under the local author with a
  fresh timestamp (`syncweb-core/src/daemon/daemon.rs:1018-1030`), so a
  materialized remote winner can immediately be re-stamped as a newer local
  edit and "win" again.

Observed behavior (manual run 2026-10-05, `NF-6`): both nodes edited `a.txt`;
after re-import + sync each node still showed its own text on disk and no
`.diff`/`.conflict.<hash>` file. Distinct new files do converge at the doc
level (`NF-7`) but are not materialized.

The intended UX is already sketched in `docs/offline-conflict.md`.
`docs/data-models.md` no longer advertises an unimplemented encrypted store.

## 2. Requirements and constraints

- Multi-writer CRDT is the source of truth; we must not invent a second
  conflict format that the CRDT does not know about.
- No data loss: the losing version must remain recoverable.
- Operations must be bounded and streaming-friendly (large/binary files).
- The store may hold files that were never materialized to disk.
- `receiveonly` folders must never resolve by writing locally.
- Blob bytes are content-addressed and deduped; only the *entry* (key →
  content hash + timestamp + author) conflicts.

## 3. Options

### Option A — Keep all variants, no resolution

Expose every variant to the user (e.g. `ls --all-variants`) and write nothing
automatically.

- Pros: zero data loss, trivial to implement, no policy to get wrong.
- Cons: unusable for normal folders; the winner is undefined on disk; no
  convergence; every command must handle N variants.

### Option B — Last-writer-wins with conflict copies (materialize-time)

When materializing (or on demand), group entries by key, pick the winner as the
greatest `timestamp()` (ties broken deterministically by author), write it at
the canonical path, and write each loser as
`<stem>.conflict.<short-hash>.<ext>`.

- Pros: deterministic convergence; no data loss; matches Syncthing/`sync-conflict`
  conventions users already know; small, well-contained change.
- Cons: only triggers when content is materialized; conflict copies are whole
  files (wasteful for large blobs); no semantic merge.

### Option C — LWW + diff-or-copy (the documented design)

Same as B, but if both variants decode as UTF-8 text and the unified diff is
smaller than the losing file, write `<stem>.diff` instead of a full conflict
copy. Otherwise fall back to a full conflict copy.

- Pros: excellent for text (code, docs, markdown); low disk overhead; matches
  `docs/offline-conflict.md`.
- Cons: needs a diff implementation/limit; binary files still whole-file;
  still materialize-time only.

### Option D — Automatic continuous reconciliation (daemon-side)

On `LiveEvent::InsertRemote`, detect that a local variant for the same key
exists and immediately materialize the winner + loser (B or C) without waiting
for an explicit `download`.

- Pros: users see conflicts promptly; closes the `NF-7` gap where remote
  entries stay doc-only.
- Cons: needs the daemon to know each folder's mount root and write policy;
  must guard the watcher re-import loop; risk of writing into unexpected
  directories; more moving parts.

### Option E — Manual resolution queue

Record conflicts in `node.db` and expose `syncweb conflicts list|show|resolve`.
Materialization is deferred to the user.

- Pros: explicit control; no automatic writes; auditable.
- Cons: no convergence until the user acts; more UX surface; still needs a
  detector and a resolver.

### Option F — True three-way merge for text

Merge both variants against a common ancestor and write the merged result
(git-merge style), only falling back to conflict copies when there are
overlapping hunks.

- Pros: best possible text UX.
- Cons: needs ancestor tracking (an extra record per key/version), a merge
  library, and a policy for every failure mode; highest complexity and risk.

## 4. Comparison

| Option | Effort | Convergence | Data loss | Text UX | Binary UX | Auto on sync |
|--------|--------|-------------|-----------|---------|-----------|--------------|
| A all variants | Low | No | None | Poor | Poor | No |
| B LWW + copies | Low | Yes | None | Poor | OK | No |
| C LWW + diff/copy | Medium | Yes | None | Good | OK | No |
| D C + auto | High | Yes | None | Good | OK | Yes |
| E manual queue | Medium | User-driven | None | Good | OK | No |
| F 3-way merge | High | Yes | Minimal | Best | OK | Optional |

## 5. Recommendation

Ship Option C in two phases, deferring D and F:

1. Phase 1 — detector + materialize-time resolution (Option C).
   - Add `DocsEngine::all_variants(doc, key)` using
     `doc.get_many(Query::all().key_exact(key))` (the collapsing
     `single_latest_per_key()` view stays for normal reads).
   - Add a `Conflict`/`ConflictResolution` type (as sketched in
     `docs/offline-conflict.md`): winner = max `timestamp()`, tie broken by
     `author()`; loser written as `.conflict.<short-hash>` or, when both decode
     as text and the diff is smaller, `<stem>.diff`.
   - Apply this in `materialize_selected_content` and the daemon
     `materialize_folder` paths, then in `download`.
   - Add `syncweb conflicts` (list/show) so conflicts are visible without
     waiting for materialization.
   - Do not auto-write in `receiveonly` folders; surface conflicts in the
     folder status/errors instead.
2. Phase 2 — daemon reconciliation (Option D), guarded.
   - On `InsertRemote` for a key that already has a local variant, mark the
     folder as "conflict" and (if the folder is writable and has a mount root)
     materialize winner + loser.
   - Guard the watcher: tag a content hash as "recently materialized" so the
     re-import path does not stamp the materialized winner as a new local edit.
     This also fixes the `NF-7`/`NF-6` observation that remote entries stay
     doc-only.
3. Later — Option F for text once ancestor tracking exists.

Rationale: C directly satisfies the documented UX and the manual test
expectation (`.diff`/`.conflict.<hash>`), keeps the CRDT as the sole source of
truth, and is testable without daemon-timing flakiness. D removes the
"materialization only on demand" surprise but is safe only after the watcher
re-import guard exists.

## 6. Implementation notes

- New query helper: `syncweb-core/src/node/docs_engine.rs`
  (`all_variants`) beside `get_any`/`list_latest`.
- New module: `syncweb-core/src/conflict.rs` (or
  `syncweb-core/src/sync/conflict.rs`) with the resolution policy and naming.
- Materialization hooks:
  - `syncweb-core/src/sync/partial_fetch.rs:66` (`materialize_selected_content`)
  - `syncweb-core/src/daemon/ipc.rs:2119` (`materialize_folder`)
  - `syncweb-cli/src/main.rs` (`download_one`)
- Watcher guard: `syncweb-core/src/daemon/daemon.rs:1018-1030`, reusing the
  in-memory hash set already used to skip already-processed entries.
- CLI: `conflicts` subcommand + JSON envelope; folder status field
  `conflicts: N`.
- Naming rules follow `docs/offline-conflict.md`: `<stem>.diff` when the diff
  is smaller than the winner, otherwise
  `<stem>.conflict.<short-hash>.<ext>`; the winner always keeps the original
  path.

## 7. Test plan

- Unit: variant grouping / winner selection (timestamp, tie-break by author).
- Unit: diff-vs-copy decision for text and binary inputs.
- Integration: two nodes edit the same text file offline, re-import and sync,
  then materialize; assert winner at the original path and either `.diff` or
  `.conflict.<hash>` exists.
- Integration: binary divergence writes a full conflict copy.
- Integration (Phase 2): conflict is materialized automatically on
  `InsertRemote`, and the watcher does not re-stamp it as a new local edit
  (no error/retry loop, stable hashes after a settle period).
- `receiveonly` folder: conflict is reported but nothing is written.

## 8. Open questions

- Which default when timestamps are equal (author ordering is deterministic but
  arbitrary; consider a stable hash tie-break).
- Whether to keep conflict copies indefinitely or garbage-collect them once the
  user acknowledges (ties into `conflicts resolve`).
- Size limits for the text diff (avoid loading two 100 MB "text" files).
- Interaction with `--metadata-only` folders (no local bytes → keep conflicts
  doc-level only).
