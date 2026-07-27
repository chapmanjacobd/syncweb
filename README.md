# syncweb

A delay-tolerant P2P web built on [Iroh](https://iroh.computer/).
Successor to [syncweb-py](https://github.com/chapmanjacobd/syncweb-py), rewritten in Rust.

Some highlights:

- Delta sync for large files -- Bao trees enable byte-range verification; only changed ranges re-sync for databases, VMs, video projects
- Syncthing relay piggyback -- when QUIC hole punching fails, tunnels through Syncthing's TCP relays for CGNAT traversal
- Public & private networks -- run without `--network` for fully open sharing, or create private named networks (`syncweb network create`) where every peer is authenticated via membership allowlist
- Per-network daemon isolation -- each named network gets its own process, data dir, and identity key
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
syncweb create ~/my-folder        # create a folder, get a sharing ticket
syncweb join <ticket>             # join a folder via ticket
syncweb folders                   # list local folders
syncweb devices                   # show device identity
```

## Configuration

TOML config at `~/.config/syncweb/config.toml`. See [docs/](docs/) for details.

## License

MIT
