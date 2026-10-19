# syncweb UX User Stories

Working document: personas, user stories, friction points, and UX improvement
themes for syncweb. Grounded in the actual CLI surface
(`syncweb-cli/src/cli/commands.rs`), the grouped help, and the multi-device
configs in the repo (`config-laptop.toml`, `config-phone.toml`,
`config-server.toml`).

## Personas

- Maya — brand new, just wants laptop ⇄ phone sync
- Dev — ships datasets/collections to others (package/link/publish)
- Ari — media archivist, huge folders, many peers, wants selective fetching
- Sam — small-team coordinator using networks
- Oli — headless server ops (schedules, bandwidth, backups)

---

## Stories

### 1. Maya first-run: "I just want my Documents on my phone"

- As a new user, I want to create a folder and have it sync with my phone
  automatically, so that I never think about the plumbing.
- Today: `syncweb folders create ~/Documents` → prints a long
  `syncweb://folder/<ns>?ticket=...` URL. On the phone she must
  `syncweb folders join <url>`, and unless she adds `--subscribe` and `--download`,
  nothing actually lands on disk — `ls` shows entries, files stay remote. She has
  to learn lazy-fetch, tickets, subscriptions, and sync modes before her first
  successful sync.
- Friction: the happy path requires ~4 concepts (ticket, subscribe,
  download, receiveonly) that the default hides. Data dir defaults to
  `./.syncweb` (cwd-relative) while docs say `~/.config/syncweb` — confusion
  about where "my files" and "syncweb's files" live.
- Improvement: `folders join` tracks the folder and shows metadata immediately (a
  human-readable one-liner "joined <ns>"), with live sync an explicit
  `folders join --subscribe` and existing content bulk-downloaded only on an explicit
  `folders join --download-existing` (alias `--download`), so the disk is never filled
  or spammed without consent.

### 2. Maya shares a folder without nuking it

- As a new user, I want to stop sharing a folder so that I can be
  sure friends can't write to it.
- Today: `create` defaults to a read-only ticket, but `--write` is one flag
  away; there's `share --write`, `unshare`, `leave --delete-files`, plus
  per-device revocation via networks — many overlapping controls with no guidance
  on which one "closes the door."
- Friction: no visual warning when handing out a write ticket vs. read-only;
  URL is unreadable so she can't tell which one she pasted. No clear "who
  currently has access" view (`share --list` is partial; `networks` covers
  members only).
- Improvement: make write tickets visually distinct (WRITE prefix / different
  color / a confirm prompt), and add a single `syncweb shares` view listing each
  folder's URL + capability + who joined.

### 3. Dev publishes a dataset to the world

- As a developer, I want to version and distribute a package, so that
  users can install and upgrade reliably.
- Today: the surface for this is `package add/bump/publish/install/upgrade`,
  `publish catalog`, `link create/resolve/revoke`, `provider add`, and
  `share --blob` — ~13 commands across 4 subcommand trees for one job.
- Friction: several overlapping concepts (package manifest vs. link vs. blob
  ticket vs. catalog publish) with no docs-level guidance on which fits which
  job. `snapshot` and `package` both snapshot/restore content and overlap
  conceptually.
- Improvement: collapse to a single `publish`/`install` verb pair with
  sub-flags (`publish --version --private --expires`), keep `package`/`link` as
  aliases or advanced. One man page per verb instead of 35.

### 4. Ari selectively fetches a huge media library

- As an archivist, I want to fetch only the rare, least-seeded files
  first, so that the network heals and I don't fill my disk.
- Today: `sort --by niche`, `download --max-peers 2`, `--min/max-count`, and
  `find --type f --ext mp4 --size +500MB` work. `ls`, `find`, `sort`,
  `download`, and `verify` share the same content-filter vocabulary:
  `--ext`, `--size`, `--depth`, `--type`, `--modified-*`, and
  `--remote-only`.
- Friction: compatibility aliases preserve older spellings, so help output has
  more names than a new user needs to learn.
- Improvement: keep the canonical spellings prominent and retain aliases only
  for compatibility; `download --dry-run`/`--preview` provides a safe fetch
  preview before committing.

### 5. Sam onboards a new teammate to a work network

- As a coordinator, I want to invite a colleague and control what they
  can touch, so that I don't hand out write access by accident.
- Today: `network create work` → `network invite work` → teammate
  `network join <ticket>`. Capability/mode is set per-folder at `create`/`join`
  time, and `access` shows persisted shares, devices, and network membership.
- Friction: invite + folder sharing are still two hops; there is no single
  provisioning command for a network member and folder capability.
- Improvement: one `syncweb network invite work --folder docs --write` that
  provisions the folder + capability + ticket in a single step; remove/deprecate
  commands from docs that aren't real, or implement them.

### 6. Oli babysits a headless server

- As an operator, I want to see sync health and progress at a glance,
  so that I can trust the box without SSH-ing in repeatedly.
- Today: `syncweb status` shows daemon, folders, devices, networks, transfer
  progress, and the pending queue in one view; `status --json` exposes the same
  sections as one stable object. Other command handlers accept the global
  `--json` output mode, with streaming network stats documented as NDJSON.
- Friction: the daemon model (`start`, `stop`, `reload`, `sync`) is still
  manually managed, so status reports whether it is running but does not
  supervise it.
- Improvement: add optional daemon supervision for headless deployments without
  changing the status contract.

### 7. Oli recovers from a dead disk

- As an operator, I want to back up and restore the sync store, so
  that a disk failure doesn't cost me re-imports.
- Today: `syncweb backup` captures config, identity, databases, and a manifest
  in one directory; `--include-blobs` includes the local blob store, while
  `db check/vacuum` remains available for database maintenance.
- Friction: users still lack a verified restore flow and can't easily tell how
  restored identity material affects network membership.
- Improvement: a first-run wizard that detects a fresh `data_dir`, verifies a
  backup, and asks whether to restore it.

---

## Cross-cutting themes

1. Two products wearing one CLI — folder-sync (create/join/ls) vs.
   distribution (package/link/publish/snapshot). Consider a folder-sync-focused
   default with distribution tucked under an explicit flow.
2. Lazy fetch surprises everyone — the biggest mental-model gap. Default
   behaviors (join downloads everything, live sync on) fix most of it.
3. 35+ top-level commands → ~12 verbs — collapse to
   `setup, create, join, ls, find, download, share, publish, install, status,
   config, networks` with advanced subcommands.
4. Docs drift is a UX bug — generated help and manpages must remain derived from
   `commands.rs`; design-only proposals must be clearly marked as future work.
5. Safety needs friction — write tickets and `leave --delete-files`/`unshare`
   need visual weight and undo hints.
6. Filters/progress/JSON are now consistent for the supported command surface;
   keep the shared `ContentFilter` and stable JSON envelopes when adding
   commands.

---

This document remains the source for user stories, friction points, and product
language. Current executable support is defined by [status.md](status.md).
