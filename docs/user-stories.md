# syncweb UX User Stories

Working document: personas, user stories, friction points, and UX improvement
plans for syncweb. Grounded in the actual CLI surface
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
- Today: `syncweb create ~/Documents` → prints a long
  `syncweb://folder/<ns>?ticket=...` URL. On the phone she must
  `syncweb join <url>`, and unless she adds `--subscribe` and `--download`,
  nothing actually lands on disk — `ls` shows entries, files stay remote. She has
  to learn lazy-fetch, tickets, subscriptions, and sync modes before her first
  successful sync.
- Friction: the happy path requires ~4 concepts (ticket, subscribe,
  download, receiveonly) that the default hides. Data dir defaults to
  `./.syncweb` (cwd-relative) while docs say `~/.config/syncweb` — confusion
  about where "my files" and "syncweb's files" live.
- Improvement: `join` enables live sync by default (persisted), so new files
  arrive on their own; existing content is bulk-downloaded only on an explicit
  `join --download-existing` (alias `--download`), so the disk is never filled
  without consent. Print a human-readable one-liner ("joined <ns> — live sync
  on; downloaded 5 files (12 GB)") instead of a bare URL.

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
  `find --type f --ext mp4 --size +500MB` work, but filters are spread across
  `find`, `sort`, `download`, and `verify` with different flag spellings
  (`--extension` in find vs `--ext` in docs, depth `+N/-N` vs `--min-depth`).
- Friction: filter vocabulary is inconsistent across commands; users must
  assemble pipelines (`find ... | download -`) to do one logical thing.
- Improvement: unify the filter flags (same `--ext/--size/--depth/--newer`
  spellings everywhere via a shared `ContentFilter` — it already exists
  internally), and add `--dry-run`/`--preview` to `download` so Ari can see what
  would be fetched before committing.

### 5. Sam onboards a new teammate to a work network

- As a coordinator, I want to invite a colleague and control what they
  can touch, so that I don't hand out write access by accident.
- Today: `network create work` → `network invite work` → teammate
  `network join <ticket>`. Capability/mode is set per-folder at `create`/`join`
  time, and there's no `accept`/`drop` command in the current CLI even though
  docs reference them.
- Friction: invite + folder sharing are two hops with no "who has what"
  dashboard; docs list `accept`/`drop` that don't exist — docs drift is itself a
  UX trap (`syncweb-mirror.1` exists for a `mirror` command that isn't
  implemented).
- Improvement: one `syncweb network invite work --folder docs --write` that
  provisions the folder + capability + ticket in a single step; remove/deprecate
  commands from docs that aren't real, or implement them.

### 6. Oli babysits a headless server

- As an operator, I want to see sync health and progress at a glance,
  so that I can trust the box without SSH-ing in repeatedly.
- Today: he must know `status`, `daemon-sync`, `stats network`,
  `transfer info`, `stats files`, `networks`, `devices` — and `--json` is only
  supported "where supported" (inconsistent).
- Friction: no single health screen; progress of a big sync is only visible
  in `transfer info`/`--verbose`. The daemon model (`start`, `shutdown`,
  `reload`, `daemon-sync`) isn't auto-managed, so "is it even running?" is an
  open question.
- Improvement: a `syncweb status` that includes folders + connected devices +
  current transfer progress + pending queue in one table, and `--json`
  guaranteed on every list/status command.

### 7. Oli recovers from a dead disk

- As an operator, I want to back up and restore the sync store, so
  that a disk failure doesn't cost me re-imports.
- Today: `db backup/check/vacuum` exists but is tucked away, and the blob
  store + doc DB + config live in separate places (data_dir, `~/.config/syncweb`).
  No single "export everything" story.
- Friction: users can't easily tell which directories constitute "syncweb
  state."
- Improvement: a `syncweb backup` that captures config + DBs + manifest
  (blobs can be excluded by flag), and a first-run wizard that detects a fresh
  `data_dir` and asks whether to restore.

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
4. Docs drift is a UX bug — `accept`, `drop`, `conflicts`, `pending`,
   `deleted`, `undelete`, `repl`, `mirror` appear in docs/man pages but not in
   `commands.rs`.
5. Safety needs friction — write tickets and `leave --delete-files`/`unshare`
   need visual weight and undo hints.
6. No unified filters/progress/JSON — shared `ContentFilter` exists in code;
   expose it consistently, make `--json` universal.

---

## Plans

*Iterated below in the planning section of this document — each plan is
validated against the actual implementation before being written, and each
action item names files/commands to touch, before/after behavior, and tests.*