# Plan 004: Merge `media` into `start`

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Media-only mode surface | `start --media-only` / `start server media` / keep `media` | `start --media-only` | |
| D2 | Does `--media-only` also start the sync daemon | yes / no | no (standalone, matches current `media`) | |
| D3 | Keep top-level `media` as alias | keep / drop | drop | |

## Context for executing agent

- Repo: `syncweb-cli`. `media` is a standalone HTTP media server that duplicates the daemon's
  media-serving surface (`start --media-listen`).
- Current state:
  - `Command::Media(MediaArgs)` at `cli/commands.rs:147-148`; `MediaArgs` at `cli/commands.rs:156-162`
    (`--listen` default `127.0.0.1:9193`, `--data-dir`).
  - `handle_media` at `main.rs:69-89`: opens a node with `syncweb_core::init::open_node`,
    runs `syncweb_core::media::MediaServer::new(args.listen, node.blob_store().clone())`,
    blocks on `server.run` until Ctrl-C. Does not touch the daemon.
  - `StartArgs` at `cli/commands.rs:629-653` already has `--media-listen` (`commands.rs:651-652`);
    `handle_start` forwards it into `daemon_config.media_listen` (`main.rs:444`) and the daemon
    runs the same `MediaServer` type when it is set (`syncweb-core/src/daemon/daemon.rs:467-468`).
  - Dispatch at `main.rs:235`: `Command::Media(command) => handle_media(command).await?`.
- Grouped help: `media` under `Network` (`cli/args.rs:94-98`).

## Goal

One command surface for running media serving: `start --media-only [--listen ADDR]`.
Remove the standalone top-level `media` group (keep alias if D3=keep).

## Structural change (do first)

1. In `cli/commands.rs`: remove `Command::Media` (or keep as hidden forwarding alias if D3=keep).
2. Add to `StartArgs` (`cli/commands.rs:629-653`): `#[arg(long, help = "Run only the media HTTP server and exit")] pub media_only: bool`.
3. In `handle_start` (find it in `main.rs`; `handle_automatic` at `main.rs:2542` calls it with
   `bg: true`), add an early branch: if `media_only`, run the `handle_media` logic and return.

## Execution steps

1. Move the body of `handle_media` (`main.rs:69-89`) into a helper `run_media_server(listen, data_dir)`
   callable from both the old `handle_media` (if alias kept) and `handle_start --media-only`.
   Note `handle_media` currently receives `MediaArgs`; the helper should take `SocketAddr` + data dir.
2. Update dispatch at `main.rs:235` (remove or re-route `Command::Media`).
3. Update `help_categories!`: remove `Command::Media(_) => "media",` from `Network`
   (`cli/args.rs:96`). If D1=`start server media`, create that group instead.
4. Docs: `docs/commands.md` — `media` table row and any `media` examples; `README.md` if referenced.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-media.1                # unless alias keeps a man page
./target/debug/syncweb start --help      # confirm --media-only appears
./target/debug/syncweb media --help      # works if D3=keep
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

None. The `MediaServer` type is already shared by the standalone path and the daemon
(`syncweb-core/src/media/server.rs`, `daemon.rs:467-468`), so `--media-only` reuses it directly.
