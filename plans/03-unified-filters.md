# Plan 03 — One content-filter vocabulary: `find`, `sort`, `download`, `verify`, lazy `ls` agree

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 01 (lazy `ls` adds `--remote-only`) · Replaces story: #3 (Ari)

## Goal

Ari's job — "rare niche file, >500MB, `.mkv`/`.mp4`, that I don't have yet" —
should be one logical expression with **one flag vocabulary** on `find`, `sort`,
`download`, and `verify`. Today each command keeps its own dialect: extension
flags are `-e/--extension` on `find`, `--ext` on `download`'s `ContentFilter`;
depth is `+N/-N` on `find` but `--min-depth/--max-depth` on `sort`; size/time/
type exist on `find` only, and the `ContentFilter` `glob` means a different
thing than `find --glob`.

## Evidence (diagnosed from code, re-verified where sited)

- `FindArgs` (syncweb-cli/src/cli/commands.rs:390-536): `-e/--extension`
  (aliases `ext`, `exts`), `--size[s]` (`+1GB/-1GB`), `--depth` (+N/-N, alias
  `levels`) **plus** separate `--min-depth/--max-depth`,
  `--type <f|d|l>`, `--modified-within/-before/-time`.
- `SortArgs` (commands.rs:487-536): no extension/size/type; `--min-depth/
  --max-depth`; adds `--min-seeders/--max-seeders/--niche/--frecency-weight`,
  rejects `--ext`.
- `DownloadArgs` + `ContentFilter` (commands.rs:561-583; cli/filter.rs): a
  **different** surface — `--hash/--path-prefix/--glob` + `--min/max-peers`,
  `--min/max-count`, `--from`/`--min-providers`; no size/type/time.
- `ContentFilter` already exists in core (syncweb-core/src/filter.rs) and is
  shared by `download` + `verify`; it is **not** connected to `find`/`sort`.

## Scope guard

- CLI flags + unified help/completions only. No daemon/core protocol changes.
- Aliases for old spellings, so scripts don't break; grouped help regeneration
  stays deterministic (see plan 07).

## Steps

### Phase A — one shared filter group

1. Introduce a clap `Args` group `ContentFilterArgs` in syncweb-cli/src/cli/filter.rs:
   - `-e/--ext <ext>` (repeatable; hidden aliases `--extension`, `exts`)
   - `--size <+N|-N|N..M>` (repeatable; hidden alias `--sizes`; `+1GB`-style
     sizes stay)
   - `--depth <+N|-N>` (repeatable with `--min-depth/--max-depth` explicit
     aliases)
   - `--type <f|d|l>`
   - `--modified-within/--modified-before` (repeatable)
2. Flatten the group into `FindArgs`/`SortArgs` (`#[command(flatten)]`) and keep
   the old flags as hidden aliases (`--extension`, `--levels`, `--types`,
   `--sizes`). `SortArgs` gains `--ext/--size/--type/--modified-*`; `FindArgs`
   keeps its match semantics (`--kind exact|glob|regex` stays find-only).
3. Start a verified `ContentFilterArgs → FindQuery` and
   `ContentFilterArgs(+hash) → VerifyFilter` converter in filter.rs so every
   command builds the same `FindQuery`/`ContentFilter` from one group.

### Phase B — delete the filename-vs-path `glob` trap

4. `ContentFilter.glob` (used by download/verify) matches a file **path**;
   `FindArgs` `--glob` matches a **filename/pattern** (FindQuery::Glob). Same
   word, two meanings. Rename the content-side flag to `--path-glob` (keep
   `--glob` as a hidden alias on download/verify), and document `--glob` on
   find/sort as pattern-matching. 
5. `download`/`verify` gain `--ext/--size/--type/--modified-within` via the
   group, and their hash/path-prefix stay. `verify` keeps `VerifyFilter`
   semantics; `download` keeps `ContentFilter`.

### Phase C — wire the group into lazy `ls` once plan 01 lands

6. `ls --remote-only` (plan 01) plus `find/sort/download/verify` all consume the
   same `ContentFilterArgs`. Also accept `--remote-only` on `find`/`download`
   once plan 01 proves lazy browsing (same "not yet on disk" predicate), as a
   shared flag in the group rather than per-command.

## Tests

- `syncweb-cli/src/cli/filter.rs` unit: `ContentFilterArgs→FindQuery` and
  `→VerifyFilter` conversion maps every field of the group; empty group == no
  filter two separate commands on a seeded fixture return the same relative-path
  set; `sort --ext mp4 --size +500MB` and `find --ext mp4 --size +500MB`
  agree (workflow_test.rs).
- `--json` on the filtered outputs stays shape-stable (plan 08 assertion).
- Help: `syncweb find --help` and `syncweb sort --help` both list the shared
  group, and `--extension`/`--levels` don't appear as primary (hidden aliases).

## Risks / rollback

- `--glob` rename on download/verify: keep hidden alias so existing scripts
  keep working; rollback = keep `--glob` as the visible spelling.
- Behavior-neutral for all current scripts; no core changes.
