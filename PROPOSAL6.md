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

## Grounded UX example

A reader can annotate immutable content without creating a modified copy:

```console
$ syncweb annotate b3:8e7a... \
    --anchor 'text-quote:annual mean temperature' \
    --text 'Methodology is clarified in the 2025 edition.'
Created annotation ann_42f... in workspace research

$ syncweb cite ann_42f... \
    --target syncweb://collection/col_methodology@b3:77ad...
Added signed citation cite_a19...

$ syncweb links b3:8e7a...
TYPE          TARGET       AUTHOR       VISIBILITY
annotated-by  ann_42f...   node5abc...  research
derived-from  b3:901e...   node8def...  public
```

When an exact text anchor no longer maps to a new edition, the UI should show it
as `orphaned` and retain the original content ID and quote. It must not silently
attach the annotation to nearby text. Concurrent edits to annotation text should
show both versions when an automatic merge is ambiguous.

## Code patterns and library candidates

Use immutable event records for provenance and a materialized view for queries:

```rust
enum KnowledgeEvent {
    Assert { subject: ObjectId, predicate: Relation, object: ObjectRef },
    Annotate { target: ContentId, anchor: Anchor, body: ContentId },
    Retract { event: EventId, reason: String },
}

enum Anchor {
    Bytes { start: u64, end: u64 },
    Time { start_ms: u64, end_ms: u64 },
    TextQuote { exact: String, prefix: Option<String>, suffix: Option<String> },
}
```

- Serialize and sign immutable events using the same canonical envelope as
  collection and catalog records. Store bodies as blobs and event indexes in
  `iroh-docs`.
- Follow W3C Web Annotation concepts for selectors and bodies where they fit,
  rather than inventing every anchor vocabulary. Keep the internal enum stricter
  than arbitrary JSON-LD.
- Use `yrs` only for annotation bodies that truly need shared text editing.
  Citations, provenance, and retractions are better represented as append-only
  signed events.
- Build reverse-link and history views in SQLite; they are derived indexes that
  can be rebuilt from verified events.
- Require object-type-specific validation, such as `end >= start` and bounds
  checks against known media metadata.

## Pros and cons

**Pros**

- Medium-to-high usefulness for research and knowledge workflows: annotations
  and provenance add value without mutating or duplicating source files.
- Append-only signed events provide an auditable history and work offline.
- Reusing established annotation concepts improves interoperability.

**Cons**

- High domain complexity: object vocabularies, stable anchoring, permissions,
  retractions, and conflict UX are each substantial features.
- CRDT dependencies and state growth are difficult to justify for simple
  comments, so collaborative editing should be narrowly scoped.
- This is not required for the core sharing loop and should follow collection,
  policy, and discovery stability.
