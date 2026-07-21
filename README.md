# syncweb

A delay-tolerant P2P web built on [Iroh](https://iroh.computer/). 

A successor to [syncweb-py](https://github.com/chapmanjacobd/syncweb-py), rewritten in Rust.

## Features

- Content-addressed storage -- BLAKE3 + Bao trees for verified streaming, range requests, and deduplication
- Lazy & selective sync -- blobs fetched on-demand; `ls`/`find` show metadata without downloading
- CRDT conflict resolution -- last-writer-wins with best-effort text diffs for concurrent edits
- Decentralized peer discovery -- via BitTorrent DHT (`distributed-topic-tracker`); no central bootstrap server
- NAT traversal -- QUIC transport with relay fallback
- No separate daemon -- library-first, embeddable `IrohNode`
- Named networks -- multi-folder, multi-device groups under gossip topics
- Parallel file operations -- Rayon-based parallel scanning/import/export
- Bandwidth scheduling -- time-of-day and per-folder bandwidth limits
- Deleted file tracking -- undelete support and audit trails

## Installation

```sh
cargo install --locked syncweb
```

### From source

```sh
git clone https://github.com/chapmanjacobd/syncweb.git
cd syncweb
cargo install .
```

Or using the Makefile:

```sh
make install                # builds release and installs the binary
make completions            # generates shell completions (bash, zsh, fish, elvish, powershell)
make man                    # generates man pages
```

## Quick Start

```sh
# Create a folder and get a sharing ticket
syncweb create ~/my-folder

# Join a folder via ticket
syncweb join <ticket>

# List local folders
syncweb folders

# Show device identity
syncweb devices

# Generate shell completions
syncweb completions bash > ~/.local/share/bash-completion/completions/syncweb
```

## Commands

| Command | Description |
|---------|-------------|
| `create` | Create a folder + namespace, output a sharing ticket |
| `join` | Join a folder via ticket |
| `accept` | Accept/grant capability for a namespace |
| `drop` | Remove/revoke a namespace |
| `folders` | List local folders with sync modes |
| `devices` | Show device identity (iroh `NodeId` + Syncthing `DeviceId`) |
| `config` | Show/set configuration |
| `network test-relay` | Test Syncthing relay connectivity |
| `repl` | Interactive REPL |
| `completions` | Generate shell completions |
| `manpages` | Generate man pages |

### Dependencies

| Crate | Version |
|-------|---------|
| iroh | 1.0.2 |
| iroh-blobs | 0.103.0 |
| iroh-docs | 0.101.0 |
| iroh-gossip | 0.101.0 |
| distributed-topic-tracker | 0.3.5 |

## Configuration

TOML-based config at `~/.config/syncweb/config.toml`. Sections include `[node]`, `[relay]`, `[discovery]`, `[folders]`, `[bandwidth]`, `[schedule]`, `[filter]`, and more.

See the [docs/](docs/) directory for detailed design documentation.

## License

[MIT](https://opensource.org/licenses/MIT)
