# syncweb-py to iroh-syncthing Conversion Plan

This document has been split into focused topic files under [`docs/`](docs/).

## Quick Links

| Topic | File |
|-------|------|
| Executive summary + key decisions | [docs/overview.md](docs/overview.md) |
| Networks + iroh-willow patterns | [docs/architecture.md](docs/architecture.md) |
| Data models | [docs/data-models.md](docs/data-models.md) |
| Offline queue + conflict resolution | [docs/offline-conflict.md](docs/offline-conflict.md) |
| Syncthing relay piggyback | [docs/relay.md](docs/relay.md) |
| Filter engine, logging, schedules | [docs/filter-logging-schedule.md](docs/filter-logging-schedule.md) |
| Command designs (find/sort/stat/init) | [docs/commands.md](docs/commands.md) |
| Module structure + parallel scanning | [docs/module-structure.md](docs/module-structure.md) |
| Packages, policies, living folders | [docs/packages.md](docs/packages.md) |
| Implementation phases (1-7) | [docs/phases.md](docs/phases.md) |
| Testing strategy + risks | [docs/testing.md](docs/testing.md) |
| API reference + impl patterns | [docs/api-reference.md](docs/api-reference.md) |

See [docs/README.md](docs/README.md) for the full index.

---

*Document version: 3.2*
*Amended: 2026-07-17*
*Target: iroh 1.0.2, iroh-blobs 0.103.0, iroh-docs 0.101.0, iroh-gossip 0.101.0, distributed-topic-tracker 0.3.5*
