# UX and operations follow-up

Status: Proposed
Sources: `docs/status.md`, `docs/user-stories.md`, and the operational
themes in the design documentation

This plan collects actionable work that is independent of the communal
transport protocol. The user stories remain the source for personas and
friction; this file turns their improvement notes into implementable tasks.

## First-run and joining

- [ ] Add a first-run wizard that detects a new `data_dir`, explains daemon
  ownership, and offers setup, join, or restore.

## Command-surface and safety

- [ ] Keep global filters, progress, JSON envelopes, and error conventions
  consistent as communal-swarm commands are added.

## Completion criteria

- A new user can set up or join without knowing whether the daemon, folder,
  package, or network abstraction is involved.
- A long-lived node can be supervised, restarted, backed up, and restored
  without losing queued work or silently changing access policy.
- Destructive and resource-consuming actions are explicit, explainable, and
  scriptable.
- Documentation distinguishes current behavior from planned behavior and
  remains linked to executable command/test surfaces.
