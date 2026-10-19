# Syncweb plans

This directory contains actionable implementation plans. It is intentionally
separate from `docs/`, which describes the product contract, architecture, and
user-facing behavior.

Plans should state their current status, prerequisites, affected code, tests,
rollout risks, and measurable completion criteria. When an implementation
changes the supported product behavior, update `docs/status.md` in the same
change.

## Active plans

| Plan | Scope | Status |
|------|-------|--------|
| [communal-swarming.md](communal-swarming.md) | Provider inventories, multi-source transfer, replication, LAN scaling, communal admission, and offline enrollment | Proposed |
| [conflict-resolution.md](conflict-resolution.md) | Remaining conflict materialization and conflict-resolution CLI work | Partially implemented |
| [ux-and-operations.md](ux-and-operations.md) | First-run, daemon supervision, backup/restore, command UX, and documentation drift | Proposed |
| [roadmap.md](roadmap.md) | The historical feature-area roadmap migrated from `docs/phases.md` | Complete; details archived |

## Source-document migration

`conflict-resolution.md` and `roadmap.md` were moved from `docs/` because they
are implementation plans rather than stable reference documentation. The
user stories, status contract, and design references remain in `docs/`; their
actionable follow-up work links back to the plans above.
