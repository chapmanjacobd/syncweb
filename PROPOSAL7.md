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

## Grounded UX example

Ingestion should be inspectable before it becomes automatic:

```console
$ syncweb ingest climate-report.pdf --profile local-text
JOB       KIND       STATUS     OUTPUT
job_81a   metadata   complete   b3:12a9...
job_81b   text       complete   b3:446e... (184 pages)
job_81c   thumbnail  skipped    not enabled by profile

$ syncweb search --full-text '"annual mean temperature"'
CONTENT       PAGE  EXCERPT
b3:8e7a...    73    ...the annual mean temperature increased...

$ syncweb derivatives b3:8e7a...
KIND  TOOL                POLICY  ID
text  pdftotext 24.02     local   b3:446e...
```

A failed extractor should report its command/tool version, bounded stderr, and
whether retrying could help. Search results should open the source at page,
timestamp, or text offsets, not expose the derivative as if it were the original.

## Code patterns and library candidates

Make jobs content-addressed and idempotent:

```rust
struct JobKey {
    source: ContentId,
    extractor: ExtractorId,
    extractor_version: String,
    config_hash: ContentId,
}

trait Extractor {
    fn supports(&self, media_type: &str) -> bool;
    async fn run(&self, input: VerifiedBlob, limits: Limits)
        -> Result<Vec<DerivativeDraft>>;
}
```

- Start with in-process, memory-safe extractors where practical: `symphonia` for
  media metadata, `image` for bounded thumbnails, and `quick-xml`/`zip` for
  selected document containers. External tools such as `pdftotext`, Tesseract,
  or FFmpeg should run behind an explicit adapter.
- Isolate external extractors with time, output, memory, and input-size limits.
  `tokio::process::Command` plus kill-on-drop handles timeouts, but stronger OS
  sandboxing should be a deployment option for untrusted files.
- Use SQLite FTS5 initially so full-text search shares the catalog storage
  approach. Consider `tantivy` only after benchmarks show SQLite is inadequate.
- Hash normalized output bytes and a provenance manifest; do not promise
  identical IDs across platforms for tools that are nondeterministic.
- Persist job state and uniqueness by `JobKey`, allowing failed jobs to be
  retried without producing duplicate successful records.

## Pros and cons

**Pros**

- High usefulness for document-heavy collections: full-text results and previews
  make catalogs far more navigable.
- Content-addressed derivatives are cacheable, shareable, and traceable without
  changing source files.
- A local-only baseline provides value without sending private data to a service.

**Cons**

- High operational and security complexity: parsers and external media tools
  process hostile inputs and require resource isolation and patching.
- Reproducibility varies by extractor and platform, weakening simple claims of
  deterministic derivative identity.
- OCR, transcription, and embeddings can consume substantial CPU, storage, and
  implementation effort; each should remain an optional profile.
