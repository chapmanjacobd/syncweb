# Plan 011: Flatten deep groups

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Which groups to flatten | `trust provider/*` + `trust stream/*`; `indexing meta/*` + `indexing filter/*` | flatten these two groups | |
| D2 | Flattened verb naming | `trust ban-provider`, `trust list-providers`, `trust stream-publish`, `indexing add-meta`, `indexing add-filter` | hyphenated verb + subject | |
| D3 | Keep two-level forms as aliases | keep / drop | keep via aliases | |

## Context for executing agent

- Repo: `syncweb-cli`. Two groups are nested two levels deep, making them harder to discover
  and type; the rest of the CLI is flat or one level deep. This is a structural UI change
  (no handlers change, only routing/names).
- Current state:
  - `TrustCommand` at `cli/commands.rs:1010-1042`: `Show`, `Delegate`, `RevokeDelegation`,
    `Provider { command: ProviderTrustCommand }`, `Stream { command: TrustStreamCommand }`.
    - `ProviderTrustCommand` at `cli/commands.rs:1044-1089`: `Show`, `List`, `Ban`, `Unban`,
      `Vouch`, `Distrust`.
    - `TrustStreamCommand` at `cli/commands.rs:1091-1106`: `Subscribe`, `Publish`.
    - Dispatched via `cli::indexing::handle_trust` (`indexing.rs:786`) →
      `handle_provider_trust` (`indexing.rs:820`) and the stream handler.
  - `IndexingCommand` at `cli/commands.rs:914-946`: `Enable`, `Disable`, `Publish`, `Search`,
    `Health`, `Meta { command: MetaCommand }`, `Filter { command: FilterCommand }`.
    - `MetaCommand` at `cli/commands.rs:948-958`: `Add`.
    - `FilterCommand` at `cli/commands.rs:960-970`: `Add`, `Subscribe`.
    - Dispatched via `cli::indexing::handle_indexing` (`indexing.rs:120`).
- Grouped help: `trust`/`attest`/`moderation` under `Trust & Moderation` (`cli/args.rs:113-117`),
  `indexing` under `Indexing` (`cli/args.rs:110-112`).

## Goal

One-level subcommands:

```sh
syncweb trust ban-provider PROVIDER [--hash H] [--reason R]
syncweb trust list-providers
syncweb trust vouch PROVIDER [--scope S] [--broadcast]
syncweb trust stream-publish --provider P --signal S [--hash H]
syncweb indexing add-meta HASH KEY VALUE [--sequence N]
syncweb indexing add-filter device VALUE
```

## Structural change (do first)

1. In `cli/commands.rs`: replace `TrustCommand::Provider { command }` and
   `TrustCommand::Stream { command }` with flat variants named per D2 (e.g.
   `ListProviders`, `BanProvider`, `UnbanProvider`, `Vouch`, `Distrust`, `StreamSubscribe`,
   `StreamPublish`). Same for `IndexingCommand::Meta`/`Filter` → `AddMeta`, `AddFilter`,
   `SubscribeFilter`.
2. If D3=keep: add clap aliases matching the old nested paths where clap allows (nested-path
   aliases are not directly expressible; if so, keep the nested group as a thin forwarding
   variant instead — verify which is feasible).
3. Delete the now-unused nested enums (`ProviderTrustCommand`, `TrustStreamCommand`,
   `MetaCommand`, `FilterCommand`) or keep them only if forwarding requires them.

## Execution steps

1. Rewire `handle_trust` (`indexing.rs:786`) and `handle_indexing` (`indexing.rs:120`) to the
   flat variants (same handler bodies, new match arms).
2. Update dispatch in `main.rs:232-241` only if variants/names changed at the `Command` level
   (they should not — `Command::Trust`/`Command::Indexing` stay group roots).
3. Update `help_categories!` only if the root names changed (they do not).
4. Docs: `docs/commands.md` and `docs/indexing.md` for the affected subcommand examples.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
./target/debug/syncweb trust --help        # confirm flat one-level list
./target/debug/syncweb indexing --help     # confirm flat one-level list
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/indexing.rs`
- `syncweb-cli/src/main.rs` (only if forwarding variants are needed)
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`, `docs/indexing.md`

## Dependencies

Run after 010. Do not start until 002 has decided where `provider add` lives (it lands in
`trust provider add` by recommendation, which this plan flattens).
