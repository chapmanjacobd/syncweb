# Plan 015 — Interrogate folder vs catalog (unify the iroh-docs concepts)

## Overview

Three independent code paths create an iroh-docs namespace and share a ticket, for three
concepts that may be one concept wearing three hats:

1. **Folder** — `FolderManager::create` → `docs_engine.create_namespace()` (`folder/manager.rs:52`);
   stores file entries keyed by path, sync mode, subscribe filters.
2. **Catalog** — `CatalogService::create_catalog` → `docs.create_namespace()` (`indexing/catalog.rs:284`);
   stores `CatalogRecord` metadata entries with FTS indexing.
3. **Network membership** — `network_doc_namespace()` derived namespace (`net/membership_doc.rs:114`);
   stores a signed member list.

## Goal

Determine whether a "catalog" is a distinct namespace type or a *derived view* over a folder, and
collapse the namespace-provisioning surface (`DocsEngine::create_namespace` + `share`) into one
helper used by all three.

## Current state

- `publish catalog <folder> --catalog <name>` already takes a **folder** and publishes its
  metadata into a catalog — evidence the catalog is a view of a folder, not a first-class doc.
- `indexing enable <folder>` opts a folder into indexing (`workflow/indexing.rs`:
  `indexing_enable_disable_uses_persistent_folder_namespace`).
- `CatalogService` publishes/subscribes catalogs via iroh-docs tickets; editorial channels have a
  `Catalog` backend that is a *separate* catalog-namespace concept (`editorial/channel.rs:83`).

## Open questions / alternatives

| # | Question | Alternatives |
|---|----------|--------------|
| Q1 | Is a catalog a separate namespace or a folder with a `catalog` entry type? | (a) fold catalog into folder docs (catalog = folder whose entries are `CatalogRecord`); (b) keep separate. |
| Q2 | If folded, what does `indexing enable` do? | (a) marks a folder as indexed (adds a flag/entry) instead of creating a catalog namespace; (b) unchanged. |
| Q3 | Network membership doc: keep the derived-namespace + signed-member-list approach, or express membership as a catalog/folder entry? | (a) keep; (b) reuse the folder/catalog doc for membership. |
| Q4 | `publish catalog` and editorial `channel` (catalog backend): one concept or two? | reconcile here or in 016. |

## Recommendation

- Q1: (a) — a catalog is a folder (or an entry type within one) that stores indexable records.
  This removes one `create_namespace` caller and clarifies `publish catalog`.
- Q3: (a) keep membership as its own derived doc for now; revisit only if the audit (014) finds
  network/resilience overlap.
- Provide `DocsEngine::create_or_open_namespace(...) -> (Doc, DocTicket)` and
  `DocsEngine::share_ticket(doc)` used by folder, catalog, and membership paths.

## Proposed changes

1. `syncweb-core`: add `DocsEngine::create_or_open_namespace` + `share_ticket` helper
   (`node/docs_engine.rs`).
2. Refactor `FolderManager::create`, `CatalogService::create_catalog`, and network membership
   creation to call it.
3. (Per Q1 decision) fold catalog storage into folder docs or introduce a `CatalogRecord` entry
   type; update `IndexingDatabase` FTS to index folder-sourced catalog records.
4. Tests: `folder_test.rs`, `catalog`/`indexing` integration tests, `workflow/indexing.rs`
   (`indexing_enable_disable_uses_persistent_folder_namespace`, `publish_catalog_with_tags`).

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
```

## Files affected

- `syncweb-core/src/node/docs_engine.rs`
- `syncweb-core/src/folder/manager.rs`, `src/folder/collection.rs`
- `syncweb-core/src/indexing/catalog.rs`
- `syncweb-core/src/net/membership_doc.rs`
- `syncweb-cli/src/cli/indexing.rs`, `src/main.rs`
- tests + `docs/*`

## Dependencies

- Coordinate with 014 (gossip audit) and 016 (search merge).
