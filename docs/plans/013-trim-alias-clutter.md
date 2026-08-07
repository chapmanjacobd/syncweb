# Plan 013: Trim alias clutter

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | `--embedded` (alias for `--no-daemon`, `cli/args.rs:263-269`) | keep / drop | drop | |
| D2 | `-d/--download` + `--dl`/`--downloadable` on `find` (`cli/commands.rs:356-363`) | keep one long + one short | keep `-d`, `--download`; drop `--dl`, `--downloadable` | |
| D3 | `--depth`/`--levels`, `--size`/`-S`, `--changed-within`/`--changed-before`, `-e`+`--ext`/`--exts`/`--extensions` on `find`/`sort` | keep primary + short only | keep primary long + short; drop wordy aliases | |
| D4 | `--by`/`--sort`/`-u` on `sort` (`cli/commands.rs:432-443`) | keep one long + short | keep `--sort-by`, `-u`; drop `--by`, `--sort` | |
| D5 | `--TS`/`--LS` on `sort --limit-size` (`cli/commands.rs:452`) | keep / drop | drop | |

## Context for executing agent

- Repo: `syncweb-cli`. Every visible alias renders as a bracketed `[alias: ...]` line in the
  custom grouped help (`cli/args.rs:143-151` in `spec_string`), so each alias adds visual noise
  to `syncweb help`. Trim to one primary long flag + at most one short.
- Do this only after 010 (the `ls`/`find`/`sort` merge) because that plan restructures the
  same arg structs; adjusting aliases before it would be wasted work.
- Current aliases to review (all in `cli/commands.rs` unless noted):
  - `cli/args.rs:263-269` — `--no-daemon` `visible_alias = "embedded"`.
  - `find` `--download` aliases `dl`, `downloadable` (`cli/commands.rs:356-363`).
  - `find` `--depth` aliases `depth`, `levels` (`cli/commands.rs:364-371`); `--sizes` alias `S`
    (`376-383`); `--modified-within` alias `changed-within` (`384-390`); `--modified-before`
    alias `changed-before` (`391-397`); `--time-modified` (`398-403`); `--extension` aliases
    `ext`, `exts`, `extensions` (`404-413`).
  - `sort` `--by` aliases `sort`, `u` (`cli/commands.rs:432-443`); `--limit-size` aliases `TS`,
    `LS` (`452`); `--depth` aliases (`454-460`).
  - Global `--data-dir` `visible_alias = "embedded"`? (no — `embedded` is on `--no-daemon`).
    Verify by reading `cli/args.rs`.

## Goal

`syncweb help` shows one canonical flag name per option. Backward compatibility for renamed
flags is handled by aliases that are not visible (`#[alias(...)]` non-visible) when the
primary name is kept.

## Structural change (do first)

1. In `cli/commands.rs` and `cli/args.rs`: delete or make non-visible the aliases chosen in
   D1-D5.
2. For dropped visible aliases, convert to non-visible `#[alias(...)]` where the alias name
   differs from the primary (so scripts keep working without cluttering help).

## Execution steps

1. Apply the alias edits per the decision table.
2. Confirm `print_grouped_help` (`cli/args.rs:173-211`) output loses the `[alias: ...]` suffixes
   for trimmed aliases.
3. Docs: `docs/commands.md` examples that use dropped aliases (e.g. `syncweb find --glob`
   at `docs/commands.md:103`) must use the canonical flag.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
./target/debug/syncweb help          # inspect Options section for leftover aliases
./target/debug/syncweb ls --help     # after 010, check merged flags
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

Run after 010 (touches `find`/`sort` arg structs that 010 merges).
