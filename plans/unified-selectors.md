# Unified Content Selectors

## Problem

The CLI has grown organically — several subcommands accept overlapping
selection arguments (`--hash`, `--path-filter`, `--glob-filter`, namespace
IDs, blob tickets) but each defines its own struct.  This makes the
interface inconsistent and adds boilerplate for every new command.

## Goal

Make hash/path/glob/folder selection consistent across subcommands and
share the parsing / filter-construction logic between CLI and daemon IPC.

## Commands and their current selection args

| command     | folder   | hash   | path/prefix | glob   | tickets/min-providers |
|-------------|----------|--------|-------------|--------|-----------------------|
| verify      | ✓ (arg)  | ✓      | ✓           | ✓      | ✓ (--from)            |
| download    | ✓ (arg)  | ✓      | (via peer)  | ✗      | ✓ (--from, min-prov)  |
| subscribe   | ✓ (arg)  | ✗      | ✓           | ✓      | ✗                     |
| join        | ✓ (ticket)| ✗     | ✓           | ✓      | ✗                     |
| health      | ✓ (arg)  | ✗      | ✗           | ✗      | ✗                     |
| ls          | ✓ (arg)  | ✗      | ✗           | ✗      | ✗                     |
| import      | ✓ (arg)  | ✗      | ✗           | ✗      | ✗                     |
| watch       | ✓ (arg)  | ✗      | ✗           | ✓ excl | ✗                     |
| publish     | ✓ (arg)  | ✓      | ✗           | ✗      | ✗                     |
| index health| ✗        | ✓      | ✗           | ✗      | ✗                     |
| trust       | ✗        | ✓      | ✗           | ✗      | ✗                     |
| attest      | ✗        | ✓      | ✗           | ✗      | ✗                     |
| report      | ✗        | ✓      | ✗           | ✗      | ✗                     |
| unfollow    | ✓ sel    | ✗      | ✗           | ✗      | ✗                     |
| leave       | ✓ sel    | ✗      | ✗           | ✗      | ✗                     |
| unsubscribe | ✓ sel    | ✗      | ✗           | ✗      | ✗                     |

## Proposed clap structs

### `FolderSelector` (already exists at `cli/commands.rs:361`)

```rust
#[derive(Debug, Args)]
pub struct FolderSelector {
    #[arg(help = "Namespace ID or path to a managed folder")]
    pub folder: String,
}
```

Unchanged.  Used by `unwatch`, `leave`, `unsubscribe`.

### `ContentFilter` (new — shared by commands that select blobs by hash/path/glob)

```rust
#[derive(Debug, Args)]
pub struct ContentFilter {
    #[arg(long, help = "Content hash(es) to select (can repeat)")]
    pub hash: Vec<String>,

    #[arg(long, help = "Only entries whose path starts with this prefix")]
    pub path_prefix: Option<String>,

    #[arg(long, help = "Only entries whose path matches this glob pattern")]
    pub glob: Option<String>,
}
```

This filter selects **which entries** the command acts on.  It maps to
`syncweb_core::verify::VerifyFilter` and `syncweb_core::sync::AreaFilter`.

### `ProviderSelector` (new — for commands that fetch blobs from the network)

```rust
#[derive(Debug, Args)]
pub struct ProviderSelector {
    #[arg(long, visible_alias = "provider", help = "Blob ticket(s) for providers")]
    pub from: Vec<String>,

    #[arg(long, default_value_t = 2, help = "Minimum providers for healthy replication")]
    pub min_providers: usize,

    #[arg(long, help = "Do not share or seed downloaded content")]
    pub no_sharing: bool,
}
```

This controls **where and how** blobs are fetched.  It maps to
`ProviderLease` / `ResilienceService` in the core.

### Composition

Commands combine the structs with `#[command(flatten)]`:

```rust
#[derive(Debug, Args)]
pub struct VerifyArgs {
    pub path: PathBuf,
    #[command(flatten)]
    pub filter: ContentFilter,
    #[command(flatten)]
    pub providers: ProviderSelector,
    #[arg(long, help = "Attempt to repair corrupted blobs")]
    pub fix: bool,
}
```

```rust
#[derive(Debug, Args)]
pub struct DownloadArgs {
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    #[command(flatten)]
    pub filter: ContentFilter,
    #[command(flatten)]
    pub providers: ProviderSelector,
    // existing: threads, no_sharing
}
```

### Mapping `ContentFilter` → core filter types

Currently the core has two filter types that overlap:

| Concept            | `sync::AreaFilter`     | `verify::VerifyFilter` |
|--------------------|------------------------|------------------------|
| All                | `All`                  | (no filter)            |
| Path prefix        | `Prefix(PathBuf)`      | `path`                 |
| Glob               | `Glob(String)`         | `glob`                 |
| Hash range         | `HashRange(_, _)`      | `hashes`               |

Both are used in different contexts (`AreaFilter` for live sync
subscriptions, `VerifyFilter` for the verify command).  Instead of
merging them (which would risk breaking existing callers), the plan
adds a bridge function in each crate:

```rust
// in syncweb_core::sync (new)
impl VerifyFilter {
    pub fn to_area_filter(&self) -> Option<AreaFilter> { ... }
}

// in syncweb_core::verify (new)
// ContentFilter → VerifyFilter lives in the CLI layer already as build_verify_filter
```

The `build_verify_filter` function in `main.rs` is moved into
`syncweb_core::verify` as a public helper so the daemon IPC handler
can reuse it.

## Commands to update

### Phase 1 — adopt `ContentFilter` + `ProviderSelector` (high value)

1. **`verify`** — already done in this branch.  Flatten into `ContentFilter`
   and `ProviderSelector` once those structs exist.

2. **`health`** — add `#[command(flatten)] filter: ContentFilter`.
   The `health` command currently builds a `HealthReport` for every
   non-system entry in the folder.  With `ContentFilter` it can
   narrow to specific hashes, paths, or globs.

3. **`download`** — flatten `ContentFilter` and `ProviderSelector`.
   The existing `--hash`, `--from`, `--min-providers` fields are
   already the same concepts; just replace them with the flattened
   structs.

### Phase 2 — less urgent but nice to have

4. **`ls`** — flatten `ContentFilter`.  Currently `ls` accepts a
   path and lists files in the store.  Adding hash/path/glob
   filtering would let users call `syncweb ls --hash <hash>` to
   locate which folder contains a given blob, or `syncweb ls --glob '*.md'`
   to list markdown files across the store.

5. **`import`** — flatten `ContentFilter`.  Filter which local
   files to import into a folder.  `--glob '*.jpg'` or `--path-prefix 'photos/'`.

6. **`indexing health`** — this already takes a bare `Hash` as a
   positional argument.  It can adopt `ContentFilter` so that
   `syncweb indexing health --hash <h1> --hash <h2>` works
   consistently with other commands.

## Core changes needed

### `syncweb_core::verify::VerifyFilter`

Already has the fields.  Needs:

```rust
impl VerifyFilter {
    /// Build from a CLI ContentFilter (string list → parsed hashes).
    pub fn from_content_filter(filter: &ContentFilter) -> Self { ... }

    /// Convert to an AreaFilter for use with SubscribeParams.
    pub fn to_area_filter(&self) -> Option<sync::AreaFilter> { ... }
}
```

### `syncweb_core::sync::AreaFilter`

Already has `HashRange`.  Needs no changes — the bridge function
in `VerifyFilter` handles conversion.

### `syncweb_core::sync::FetchFilter`

Currently has `min_peers`, `max_peers`, `min_count`, `max_count`,
`paths`, `min_size`, `max_size`.  The `paths` field overlaps with
`ContentFilter.path_prefix`.  No changes needed — just use
`ContentFilter → AreaFilter` in the fetch path.

## Backward compatibility

All new fields get `#[serde(default)]` on the IPC command variants.
Old clients that send requests without them still work.

The new flattened clap structs use `#[arg(long)]` so they do not
conflict with existing positional or optional arguments.

## Implementation order

1. Extract `ContentFilter` and `ProviderSelector` structs into
   `cli/commands.rs` or a new `cli/filter.rs`.
2. Move `build_verify_filter` from `main.rs` into
   `syncweb_core::verify::VerifyFilter::from_content_filter`.
3. Refactor `verify` to use the flattened structs.
4. Refactor `health`, `download`, `ls`, `import` in order.
5. Update daemon IPC handlers for each changed command.
