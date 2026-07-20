# syncweb Design Documentation

Focused documentation for the syncweb design and implementation.

## Files

| File | Content |
|------|---------|
| [overview.md](overview.md) | Executive summary, architecture mapping, core architecture diagram, key technical decisions |
| [architecture.md](architecture.md) | Networks (multi-folder groups), iroh-willow architecture patterns (Engine, SessionMode, IntentHandle, etc.) |
| [data-models.md](data-models.md) | SyncwebFolder, SyncMode, Capabilities, Collections, Backup/Snapshot, URL format, DeviceIdentity, PartialFetch |
| [offline-conflict.md](offline-conflict.md) | Offline queue, conflict resolution UX, peer availability tracking, cache eviction |
| [relay.md](relay.md) | Syncthing relay piggyback, DeviceId compatibility, transport fallback |
| [filter-logging-schedule.md](filter-logging-schedule.md) | Filter engine, logging/observability, sync schedules, platform settings, integrity verification, bandwidth accounting, watch mode |
| [commands.md](commands.md) | find, stat, sort, init/config command designs + CLI command mapping + new CLI options |
| [module-structure.md](module-structure.md) | Module tree + parallel scanning/import/export |
| [packages.md](packages.md) | Living folders, scoped policies, collection/package manifests, publish/discover/install/upgrade lifecycle |
| [phases.md](phases.md) | Implementation phases (1-7) |
| [testing.md](testing.md) | Testing strategy, performance targets, risks, success criteria |
| [api-reference.md](api-reference.md) | Appendix: Iroh 1.0.2 API reference + grounded implementation patterns |

## Additional docs

| File | Content |
|------|---------|
| [drop_format.md](drop_format.md) | Drop format design |
| [indexing.md](indexing.md) | Opt-in indexing service |

---

*Document version: 3.2*
*Amended: 2026-07-17*
*Target: iroh 1.0.2, iroh-blobs 0.103.0, iroh-docs 0.101.0, iroh-gossip 0.101.0, distributed-topic-tracker 0.3.5*
