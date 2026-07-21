# syncweb-core ![License: MIT](https://img.shields.io/badge/license-MIT-blue) [![syncweb-core on crates.io](https://img.shields.io/crates/v/syncweb-core)](https://crates.io/crates/syncweb-core) [![syncweb-core on docs.rs](https://docs.rs/syncweb-core/badge.svg)](https://docs.rs/syncweb-core) [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)](https://github.com/chapmanjacobd/syncweb)

## syncweb-core

Core library for `syncweb`, enabling delay-tolerant web surfing and decentralized synchronization.

This crate provides the foundational building blocks for the syncweb application, including:

* Decentralized folder synchronization and package management.
* Network and node management using the Iroh stack.
* File system scanning, filtering, and statistical analysis.
* Delay-tolerant networking capabilities.

### Modules

* `error`: Common error types and `Result` aliases.
* `filter`: Tools for filtering files during synchronization and scanning.
* `folder`: Management of synchronized folders, collections, and packages.
* `fs`: File system utilities, including parallel scanning.
* `indexing`: Opt-in SQLite/FTS5 indexing for synchronized folders.
* `net`: Network management and routing configurations.
* `node`: Iroh node integration and identity management.
* `search`: Find engine for querying synchronized assets.
* `sync`: The core synchronization engine and session management.
