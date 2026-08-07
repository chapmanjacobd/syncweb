# Plan 008: Unify `publish` commands

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Structure | `publish <kind>` with kinds `folder|blob|collection|catalog` | yes | |
| D2 | Keep `collection publish` / `indexing publish` as aliases | keep / drop | keep (visible aliases) | |
| D3 | `unpublish` stays a sibling top-level command | yes / `publish rm` | yes (keep sibling) | |

## Context for executing agent

- Repo: `syncweb-cli`. "Publish" appears three times in the command tree with different shapes.
- Current state:
  - Top-level `Command::Publish(PublishArgs)` at `cli/commands.rs:81-82` and
    `Command::Unpublish(UnpublishArgs)` at `cli/commands.rs:83-84`.
    - `PublishArgs` at `cli/commands.rs:756-762`: `namespace`, `--blob <hash>` ("Publish this
      content hash as an unauthenticated blob ticket"). `handle_publish` at `main.rs:2657`.
    - `UnpublishArgs` at `cli/commands.rs:764-770`: `namespace`, `--blob <hash>`.
      `handle_unpublish` at `main.rs:2694`.
  - `collection publish` at `cli/commands.rs:797-808` ("Store a collection manifest and
    mutable head in a folder"). Handled in `handle_collection` (main.rs, `Snapshot`/`Collection`
    dispatch around `main.rs:229`).
  - `indexing publish` at `cli/commands.rs:920-927` ("Publish folder metadata to a catalog").
    Handled by `cli::indexing::handle_indexing` (`indexing.rs:120`).
- Grouped help: `publish`/`unpublish` under `Sharing & Publishing` (`cli/args.rs:80-87`),
  `collection` under `Content` (`cli/args.rs:88-93`), `indexing` under `Indexing` (`cli/args.rs:110-112`).

## Goal

```sh
syncweb publish folder PATH [--namespace N]   # was top-level publish (folder/blob)
syncweb publish blob NAMESPACE HASH           # was publish --blob
syncweb publish collection PATH --namespace N # was collection publish
syncweb publish catalog PATH --catalog C      # was indexing publish
syncweb unpublish NAMESPACE --blob HASH       # unchanged sibling
```

## Structural change (do first)

1. In `cli/commands.rs`: convert `Command::Publish` into a group
   `Publish { #[command(subcommand)] command: PublishCommand }` with variants
   `Folder`, `Blob`, `Collection`, `Catalog` (arg structs mirror the current three sources).
2. Keep `Command::Unpublish` as-is (D3).
3. If D2=keep: add clap aliases so `collection publish ...` and `indexing publish ...` still
   resolve (clap aliases only remap the immediate subcommand, so this needs alias handling in
   the group dispatch — verify feasibility; if not feasible, keep thin forwarding variants).

## Execution steps

1. Move the payloads: `handle_publish` (`main.rs:2657`), the `collection publish` arm of
   `handle_collection`, and the `indexing publish` arm of `handle_indexing` (`indexing.rs:120`)
   all dispatch through one `handle_publish(ctx, PublishCommand)`.
2. Update dispatch at `main.rs:227-229` for `Command::Publish { command }` and the changed
   `Collection`/`Indexing` variants (only if D2=drop or forwarding needed).
3. Update `help_categories!`: `Sharing & Publishing` keeps `publish`/`unpublish`;
   `Content` keeps `collection`; `Indexing` keeps `indexing`. No category change if the
   variants stay, but the `category_of` match must still compile (exhaustive).
4. Docs: `docs/commands.md` mapping rows for `publish`, `collection publish`, `indexing publish`;
   examples at `docs/commands.md:496-497` and any `collection`/`indexing` examples.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
./target/debug/syncweb publish --help     # confirm folder|blob|collection|catalog
./target/debug/syncweb collection publish --help   # works if D2=keep
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-cli/src/cli/indexing.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

Run after 007. Interacts with 002 (provider folding may target `publish` if that D1 is chosen)
and 011 (if `indexing`/`collection` groups are flattened).
