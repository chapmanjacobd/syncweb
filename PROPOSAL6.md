# Proposal 6: Knowledge Objects, Annotations, and Provenance

**Status:** Proposed
**Priority:** Medium
**Depends on:** PROPOSAL2, PROPOSAL7, PROPOSAL8, PROPOSAL9

## Problem

Files and paths are insufficient as a knowledge model. Users need to connect
works, editions, people, claims, citations, translations, and derived material
without modifying the original blobs.

## Goals

- Add stable knowledge objects linked to immutable content.
- Support collaborative annotations and citations.
- Preserve provenance for every assertion and derivative.
- Use CRDT-style merging only where it matches the object type.

## Design

Define typed objects such as:

```text
Work, Edition, Document, Collection, Person, Organization
Claim, Annotation, Citation, License, Provenance
```

Relationships include `cites`, `derived_from`, `translation_of`,
`supersedes`, `duplicate_of`, and `annotates`.

Annotations use stable anchors: page/paragraph, byte range, timestamp range,
transcript span, or structured field. The underlying file remains immutable.

Use append-only signed records for publisher metadata and citations. Use
conflict-aware mutable documents for comments, tags, and collaborative
annotations. Do not apply one generic CRDT policy to arbitrary binary files.

## Scoped configuration

- **Network:** schema vocabulary, accepted publishers, annotation visibility.
- **Folder:** whether the collection is an editable knowledge workspace or
  read-only archive.
- **File:** annotation permission and allowed anchor types.

## User-facing interface

```text
syncweb annotate <file> --anchor <anchor> --text <text>
syncweb cite <source> --target <content-or-claim>
syncweb links <content-or-object>
syncweb history <object>
```

## Implementation steps

1. Define object IDs, relationship serialization, anchors, and signatures.
2. Add read-only provenance and citation records.
3. Add collaborative annotation documents and conflict UX.
4. Add collection views that combine files and knowledge objects.
5. Add export to stable interchange formats after the internal model settles.

## Acceptance criteria

- An annotation remains addressable across collection path changes.
- Readers can inspect the source and provenance of a claim or derivative.
- Concurrent comments merge without changing the referenced file.
- A publisher can expose files publicly while restricting annotations to a
  private network.
