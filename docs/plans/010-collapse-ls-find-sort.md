# Plan 010: Collapse `ls`/`find`/`sort`

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Scope | Full merge (one `ls` with filters + sort) vs partial (`sort` folds into `ls --sort`, `find` stays separate) | Full merge | |
| D2 | Invocation shape | `ls [PATH] [PATTERN]` (pattern optional) | yes | |
| D3 | Sort output mode | streaming by default; `--sort-by` forces collection (as today) | yes | |
| D4 | Keep `find` / `sort` / `stat` top-level aliases | keep / drop | keep `find`, `sort`; `stat` stays a separate command | |

## Context for executing agent

- Repo: `syncweb-cli`. The largest change. `ls`, `find`, and `sort` are all "list files"
  variants that share scanner plumbing and even the same flags (`depth`/`min-depth`/
  `max-depth`/`threads` appear in both `FindArgs` and `SortArgs`). `ls` already has a
  `--sort` flag.
- Current state:
  - `LocalPathArgs` at `cli/commands.rs:316-328`: `path`, `--sort <String>` ("Collect and sort
    output instead of streaming it"), `--threads`. `handle_ls` at `main.rs:4095`.
  - `FindArgs` at `cli/commands.rs:330-426`: `pattern`, `path`, `--kind exact|glob|regex`,
    `-i/--ignore-case`, `-s/--case-sensitive`, `-F/--fixed-strings`, `-p/--full-path`,
    `-H/--hidden`, `-L/--follow-links`, `-a/--absolute-path`, `-d/--download`,
    `--depth`/`--min-depth`/`--max-depth`, `--sizes`, `--modified-within`, `--modified-before`,
    `--time-modified`, `-e/--extension`, `--type f|d|l`, `--threads`.
    `handle_find` at `main.rs:4133`.
  - `SortArgs` at `cli/commands.rs:428-477`: `path`, `--by` (niche/frecency/peers/random/
    folder/time/date/week/month/year/size/folder-*), `--min-seeders`, `--max-seeders`,
    `--niche`, `--frecency-weight`, `--limit-size`, `--depth`, `--min-depth`, `--max-depth`,
    `--threads`, `--enrich`. `handle_sort` at `main.rs:4246`.
  - `StatArgs` at `cli/commands.rs:479-492`; `handle_stat` at `main.rs:4442`. Keep as its own
    command (single-file metadata is a distinct operation).
  - The `sort`/`find` output pipelines differ: `find` streams, `sort` collects and uses
    `PeerTracker`/daemon `--enrich` data. Inspect the internals (grep `FindEngine`, `Sorter`,
    `ParallelScanner`) before merging.
- Grouped help: `ls`/`find`/`sort`/`stat`/`download`/`import`/`verify`/`health` under `Files`
  (`cli/args.rs:66-75`). Note 009 moves `health` out of this category.

## Goal

```sh
syncweb ls [PATH] [PATTERN]                 # stream entries (was ls / find default)
syncweb ls PATH --kind regex --size +1GB --type f --hidden   # find filters
syncweb ls PATH --sort-by niche --enrich --limit-size 10GB   # was sort
```

## Structural change (do first)

1. In `cli/commands.rs`: merge `LocalPathArgs` + `FindArgs` + `SortArgs` into one
   `LsArgs` struct (or one `LsArgs` plus flattened `FindFilters` / `SortOptions` structs so
   `--sort-by` flags only apply when sorting).
2. Remove `Command::Find` and `Command::Sort`; keep `Command::Ls` with aliases `find`/`sort`
   (D4). `Command::Stat` stays.
3. Update dispatch at `main.rs:211-213` (Ls, Find, Sort arms).

## Execution steps

1. Write `handle_ls(ctx, LsArgs)` that dispatches internally to the three existing bodies:
   - no pattern + no sort → existing `handle_ls` streaming path.
   - pattern/filter flags present → existing `handle_find` path.
   - `--sort-by`/`--enrich` present → existing `handle_sort` path (collects).
   Choose precedence explicitly (e.g. `--sort-by` wins; a pattern without filters also routes
   to the find path).
2. Merge flag sets; keep flag names/aliases stable so existing scripts keep working.
   `--sort` on `ls` today ("collect and sort") is superseded by `--sort-by`; decide whether to
   keep `--sort` as an alias (recommend yes).
3. Update `help_categories!`: `Files` category keeps `ls`; remove `find`, `sort`
   (`cli/args.rs:68-69`).
4. Docs: `docs/commands.md` — find/sort design sections (lines 3-133, 225-318), mapping rows,
   and examples at `docs/commands.md:503-515`, `docs/commands.md:550-563`. This is the biggest
   doc update of the whole effort.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-find.1 man/syncweb-sort.1    # if D4=drop
./target/debug/syncweb ls --help       # confirm merged flags
./target/debug/syncweb find PAT --size +1GB --type f   # alias works (D4)
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

Run last among the structural merges (after 009, which removes `health` from the same
`Files` category). Highest risk; consider a follow-up hardening plan for the sort/find
pipelines if output regressions are found.
