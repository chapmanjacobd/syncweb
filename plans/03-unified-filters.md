# Plan 03 — One content-filter vocabulary: `find`, `sort`, `download`, `verify`, lazy `ls` agree

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 01 (lazy `ls` adds `--remote-only`) · Replaces story: #3 (Ari)

## Goal

Ari's job — "rare niche file, >500MB, `.mkv`/`.mp4`, that I don't have yet" —
should be one logical expression with **one flag vocabulary** on `find`, `sort`,
`download`, and `verify`. Today each command keeps its own dialect, and the
content-fetch commands (`download`/`verify`) have almost no filters at all.

## Evidence (diagnosed from code, re-verified where sited)

- `FindArgs` (syncweb-cli/src/cli/commands.rs:390-485) is the richest surface:
  positional `pattern` + `--kind exact|glob|regex` (default `glob`); `-i/--ignore-case`,
  `-s/--case-sensitive`, `-F/--fixed-strings`, `-p/--full-path`, `-H/--hidden`,
  `-L/--follow-links`, `-a/--absolute-path`, `-d/--download`;
  `--depth` (aliases `levels`), `--min-depth/--max-depth`;
  `--sizes` (aliases `size`, `S`); `--modified-within` (alias `changed-within`),
  `--modified-before` (alias `changed-before`), `--time-modified`;
  `-e/--extension` (aliases `ext`, `exts`, `extensions`); `--type f|d|l`.
- `SortArgs` (commands.rs:488-536): no extension/size/type/time; has
  `--depth` (alias `levels`) + `--min-depth/--max-depth`, plus sort-specific
  `--by`, `--min-seeders/--max-seeders`, `--niche`, `--frecency-weight`,
  `--limit-size`, `--enrich`.
- `DownloadArgs` + `VerifyArgs` (commands.rs:562-583, 737-746) share the
  flattened `ContentFilter` (syncweb-cli/src/cli/filter.rs:9): **only**
  `--hash`, `--path-prefix`, `--glob` — plus `ProviderSelector` (`--from`,
  `--min-providers`, `--no-sharing`) and `--min/max-peers`, `--min/max-count`.
  **No** extension/size/type/depth/time filter on download/verify today.
- `ContentFilter` already converts to `VerifyFilter` via `TryFrom`
  (filter.rs:26-44); the `find`/`sort` path builds a `FindQuery`
  (syncweb-core/src/search.rs:29) from `FindArgs` by hand in `handle_find`
  (main.rs:4338-4410). The two filter worlds are not connected.

## Scope guard

- CLI flags + unified help/completions only. No daemon/core protocol changes.
- Aliases for old spellings, so scripts don't break; grouped help regeneration
  stays deterministic (see plan 07).

## Steps

### Phase A — one shared filter group

1. Introduce a clap `Args` group `ContentFilterArgs` in syncweb-cli/src/cli/filter.rs:
   - `-e/--ext <ext>` (repeatable; hidden aliases `--extension`, `exts`,
     `extensions`)
   - `--size <+N|-N|N..M>` (repeatable; hidden alias `--sizes`; `+1GB`-style
     sizes stay)
   - `--depth <+N|-N>` (repeatable with `--min-depth/--max-depth` explicit
     aliases)
   - `--type <f|d|l>`
   - `--modified-within/--modified-before` (repeatable)
2. Flatten the group into `FindArgs`/`SortArgs` (`#[command(flatten)]`) and keep
   the old flags as hidden aliases (`--extension`, `--levels`, `--sizes`).
   `SortArgs` gains `--ext/--size/--type/--modified-*`; `FindArgs` keeps its
   match semantics (`--kind exact|glob|regex` stays find-only).
3. Start a verified `ContentFilterArgs → FindQuery` converter in
   syncweb-cli/src/cli/filter.rs so every command builds the same `FindQuery`
   from one group (mirrors the existing `ContentFilter → VerifyFilter`
   `TryFrom` already in that file).

### Phase B — close the filter gap on `download`/`verify` and the `glob` trap

4. `download`/`verify` gain `--ext/--size/--type/--modified-within` via the
   group. They keep `--hash`/`--path-prefix`/`--glob`; `download` keeps
   `ContentFilter` + `ProviderSelector`, `verify` keeps `VerifyFilter` +
   `--fix`. Add `ContentFilterArgs → ContentFilter` so the new fields map onto
   the download/verify filter without breaking their existing hash/path filters.
5. **Resolve the `glob` naming trap.** `ContentFilter.glob` (filter.rs:17,
   help: "Only entries whose path matches this glob pattern") matches an entry
   **path**; `find` has no `--glob` flag — it matches a positional `pattern`
   against the **filename** (or full path with `--full-path`) using
   `--kind glob|regex|exact`. Same word, two meanings across commands. Rename
   the content-side flag to `--path-glob` (keep `--glob` as a hidden alias on
   download/verify), and document `find`'s positional pattern as filename
   matching.

### Phase C — wire the group into lazy `ls` once plan 01 lands

6. `ls --remote-only` (plan 01) plus `find/sort/download/verify` all consume the
   same `ContentFilterArgs`. Also accept `--remote-only` on `find`/`download`
   once plan 01 proves lazy browsing (same "not yet on disk" predicate), as a
   shared flag in the group rather than per-command.

## Tests

- `syncweb-cli/src/cli/filter.rs` unit: `ContentFilterArgs → FindQuery` and
  `→ ContentFilter`/`VerifyFilter` conversion maps every field of the group;
  empty group == no filter.
- `syncweb-cli/tests/workflow_test.rs`: two separate commands on a seeded
  fixture return the same relative-path set — `sort --ext mp4 --size +500MB`
  and `find --ext mp4 --size +500MB` agree.
- `--json` on the filtered outputs stays shape-stable (plan 08 assertion).
- Help: `syncweb find --help` and `syncweb sort --help` both list the shared
  group, and `--extension`/`--levels`/`--sizes` don't appear as primary (hidden
  aliases).

## Risks / rollback

- `--glob` rename on download/verify: keep hidden alias so existing scripts
  keep working; rollback = keep `--glob` as the visible spelling.
- Behavior-neutral for all current scripts; no core changes.

## Handoff notes

- `FindQuery` lives in `syncweb-core/src/search.rs`, `ContentFilter` in
  `syncweb-cli/src/cli/filter.rs`; the new `ContentFilterArgs` + converters
  belong in `syncweb-cli/src/cli/filter.rs` (do not add to
  `syncweb-core/src/filter.rs`, which is an unrelated `FilterEngine`).
