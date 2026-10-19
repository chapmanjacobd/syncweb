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
- [ ] Add clear network invitation provisioning and explain the distinction
  between an invitation, a folder, a package, and a community.

## Daemon supervision and recovery

- [ ] Define supported service-manager integrations and document foreground
  versus supervised operation.
- [ ] Add an opt-in CLI path to start or connect to the daemon when commands
  require it, without silently starting a daemon for read-only commands.
- [ ] Show stale PID, crashed daemon, IPC version mismatch, and recovery
  guidance as actionable errors.
- [ ] Add restart and shutdown recovery tests for watched folders and
  long-lived seeding.

## Command-surface and safety

- [ ] Reconcile the many specialized commands with the documented core flows
  (`setup`, `create`, `join`, `ls`, `find`, `download`, `share`, `publish`,
  `install`, `status`, `config`, and `networks`) without breaking existing
  scripts.
- [ ] Give destructive operations such as `leave --delete-files`, `unshare`,
  revoke, and unpin prominent confirmation and undo guidance.
- [ ] Keep global filters, progress, JSON envelopes, and error conventions
  consistent as communal-swarm commands are added.

## Documentation and release discipline

- [ ] Keep `docs/status.md` updated whenever supported, opt-in, or planned
  behavior changes.
- [ ] Add links from design docs to the applicable plan instead of presenting
  future structures as implemented APIs.
- [ ] Convert performance targets in `docs/testing.md` into reproducible
  fixtures before making them release gates.

## Completion criteria

- A new user can set up or join without knowing whether the daemon, folder,
  package, or network abstraction is involved.
- A long-lived node can be supervised, restarted, backed up, and restored
  without losing queued work or silently changing access policy.
- Destructive and resource-consuming actions are explicit, explainable, and
  scriptable.
- Documentation distinguishes current behavior from planned behavior and
  remains linked to executable command/test surfaces.
