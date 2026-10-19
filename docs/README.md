# syncweb Design Documentation

Focused documentation for the syncweb design and implementation.

| File | Content |
|------|---------|
| [overview.md](overview.md) | Executive summary, architecture mapping, key decisions |
| [architecture.md](architecture.md) | Networks, iroh-willow architecture patterns |
| [data-models.md](data-models.md) | SyncwebFolder, SyncMode, Capabilities, Collections |
| [offline-conflict.md](offline-conflict.md) | Offline queue, conflict resolution, cache eviction |
| [relay.md](relay.md) | Syncthing relay piggyback, transport fallback |
| [filter-logging-schedule.md](filter-logging-schedule.md) | Filter engine, schedules, bandwidth accounting |
| [commands.md](commands.md) | find, stat, sort, create/config command designs |
| [packages.md](packages.md) | Living folders, collection/package manifests, lifecycle |
| [testing.md](testing.md) | Testing strategy, performance targets, risks |
| [status.md](status.md) | Current implemented-vs-planned product contract |
| [operations.md](operations.md) | Foreground daemon operation and service-manager supervision |
| [api-reference.md](api-reference.md) | Iroh 1.0.2 API reference + implementation patterns |
| [archive_format.md](archive_format.md) | Archive format design |
| [indexing.md](indexing.md) | Opt-in indexing service |

Design and user-story documents may describe future behavior, but executable
support is defined by [status.md](status.md).
