# Proposal 7: Ingestion, Derivatives, and Search Indexes

**Status:** Proposed
**Priority:** Medium
**Depends on:** PROPOSAL2, PROPOSAL6, PROPOSAL9

## Problem

General file sharing becomes a knowledge platform only when users can search
inside files and inspect useful derivatives such as previews, OCR, transcripts,
and normalized metadata.

## Goals

- Build an explicit, repeatable ingestion pipeline.
- Store derivatives as verified content-addressed artifacts.
- Make full-text and semantic search optional and policy-controlled.
- Record how every derivative was produced.

## Design

An ingestion job consumes a content ID and produces zero or more derivative
artifacts:

```text
Derivative {
  source_content_id
  derivative_content_id
  kind                 # text, OCR, transcript, thumbnail, embedding, metadata
  tool, tool_version
  created_at
  provenance
  signature
}
```

Indexers may index filenames, extracted text, tags, and relationships. Embeddings
are optional and must not be required for basic search. Failed extraction is a
visible job result, not a silent fallback.

## Scoped configuration

- **Network:** approved extractors, indexer destinations, model/tool policy.
- **Folder:** automatic ingestion profile and searchable metadata fields.
- **File:** opt out, select extractor, or require local-only processing.

Private files must not be sent to a hosted indexer without an explicit policy.

## User-facing interface

```text
syncweb ingest <file-or-collection>
syncweb derivatives <content-id>
syncweb search --full-text "query"
syncweb index rebuild <scope>
syncweb jobs
```

## Implementation steps

1. Define derivative manifests and provenance records.
2. Implement deterministic metadata and text extraction jobs.
3. Add local full-text indexing.
4. Add previews/transcripts and optional hosted indexers.
5. Add semantic indexing only after privacy and deletion semantics are defined.

## Acceptance criteria

- A derivative can be independently verified and traced to its source hash.
- Re-running the same deterministic extractor produces the same derivative identity.
- Full-text search returns source anchors that can be opened or fetched.
- A file-level opt-out prevents hosted ingestion and indexing.
