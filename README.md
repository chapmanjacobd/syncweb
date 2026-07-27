# syncweb

A delay-tolerant P2P web built on [Iroh](https://iroh.computer/).
Successor to [syncweb-py](https://github.com/chapmanjacobd/syncweb-py), rewritten in Rust.

Some highlights:

- Delta sync for large files -- Bao trees enable byte-range verification; only changed ranges re-sync for databases, VMs, video projects
- Snapshots are instant -- content-addressed storage means snapshots are just references to existing blobs, zero data copying
- Syncthing relay piggyback -- when QUIC hole punching fails, tunnels through Syncthing's TCP relays for CGNAT traversal
- Per-network daemon isolation -- each named network gets its own process, data dir, and identity key; blobs shared across networks are deduped via filesystem hardlinks
- Peer discovery via BitTorrent DHT -- no central bootstrap server; uses distributed-topic-tracker on the mainline DHT
- Partial folder fetch -- fetch the least-seeded blobs first to improve network health

## Install

```sh
cargo install --locked syncweb
```

Or from source:
```sh
git clone https://github.com/chapmanjacobd/syncweb.git
cd syncweb && cargo install .
```

## Quick Start

```sh
syncweb create ~/my-folder       # create a folder, get a sharing ticket
syncweb join <ticket>             # join a folder via ticket
syncweb folders                   # list local folders
syncweb devices                   # show device identity
```

## Commands

| Command | Description |
|---------|-------------|
| `create` | Create a folder + namespace, output a sharing ticket |
| `join` | Join a folder via ticket |
| `accept` | Accept/grant capability for a namespace |
| `drop` | Remove/revoke a namespace |
| `folders` | List local folders |
| `devices` | Show device identity |
| `config` | Show/set configuration |
| `network test-relay` | Test relay connectivity |
| `repl` | Interactive REPL |
| `completions` | Generate shell completions |
| `manpages` | Generate man pages |

## Configuration

TOML config at `~/.config/syncweb/config.toml`. See [docs/](docs/) for details.

## License

MIT
