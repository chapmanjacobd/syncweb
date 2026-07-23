# Media Playback: Iroh Blobs → `<video>` Tag

## Problem

A `<video>` tag in the browser needs to play media stored as Iroh content-addressed blobs
(BLAKE3 verified, Bao-tree chunked). The video player issues byte-range requests to seek
within the file. We need to bridge the iroh blob store to HTTP Range semantics so that:

```
<video src="http://localhost:9193/media/<blob-hash>">
  -->  GET /media/<blob-hash>                         (discover content-type, size)
  -->  GET /media/<blob-hash>  Range: bytes=0-1048575  (buffered playback)
  -->  GET /media/<blob-hash>  Range: bytes=5242880-   (user seeks to 5 MB)
```

resolves to verified, content-addressed bytes from the iroh blob store — even for
**partially downloaded blobs** where on-demand fetching may still be in progress.

## Prior art

[iroh-blobs PR #72](https://github.com/n0-computer/iroh-blobs/pull/72) (draft, rklaehn,
Mar 2025) adds `entry_path_or_data()` to the store trait and an `open(hash)` method
that returns `impl std::io::Read + std::io::Seek` backed by a `BaoFile`. Key ideas:

- A blob opened this way is **independent of the store** — valid even after the store
  is dropped.
- Partial blobs are supported: reads at positions where chunks are missing return an
  `io::Error`. This is intentional ("YOLO reads").
- The `positioned_io::ReadAt` trait is used for read-at-offset without a cursor.

syncweb already has `BlobReader` from iroh-blobs 0.103 which implements
`tokio::io::AsyncRead + tokio::io::AsyncSeek` (except `SeekFrom::End`, which is a TODO).
This is the building block for HTTP range serving.

---

## Target architecture

```
┌──────────────────────────────────────────────────┐
│  Browser / Compose WebView                        │
│  ┌────────────────────────────────────────────┐  │
│  │ <video src="http://127.0.0.1:9193/media/   │  │
│  │               bafy...hash">                │  │
│  │    │                                        │  │
│  │    │ HTTP Range Requests (GET + Range hdr)  │  │
│  └────┼────────────────────────────────────────┘  │
└───────┼───────────────────────────────────────────┘
        │ http://127.0.0.1:9193
  ┌─────┴──────────────────────────────────────┐
  │  syncweb — Media HttpServer                 │
  │  ┌──────────────────────────────────────┐  │
  │  │ route GET /media/{hash}              │  │
  │  │   ├─ parse Range header              │  │
  │  │   ├─ BlobStore::reader(hash)          │  │
  │  │   │   → BlobReader (AsyncRead+Seek)  │  │
  │  │   ├─ detect MIME type                │  │
  │  │   ├─ BlobReader::seek(start)         │  │
  │  │   ├─ BlobReader::read(range_len)     │  │
  │  │   └─ respond 206 + Content-Range     │  │
│  └──────────────────────────────────────┘  │
  │  ┌──────────────────────────────────────┐  │
  │  │ IrohNode                             │  │
  │  │  ├─ BlobStore (local store)           │  │
  │  │  ├─ DocsEngine                         │  │
  │  │  └─ GossipService                      │  │
  │  └──────────────────────────────────────┘  │
  └────────────────────────────────────────────┘
```

The media HTTP server is a separate TCP listener from the WebSocket bridge.
It speaks plain HTTP/1.1 (no WebSocket upgrade, no TLS for local-only use).

Port: **9193** (`SYNCWEB_MEDIA_PORT`), one above the bridge port.

---

## HTTP endpoint specification

### `GET /media/{hash}`

Serve a blob from the iroh blob store with full HTTP Range negotiation.

#### Without `Range` header

```
HTTP/1.1 200 OK
Accept-Ranges: bytes
Content-Type: video/mp4
Content-Length: 48234496
Cache-Control: public, max-age=31536000, immutable
ETag: "bafy..."
```

The body is the full blob from byte 0. Sending the entire blob as a single
response is inefficient — the browser almost always follows up with a Range
request. We can send a small initial chunk (e.g. first 64 KiB) to let the
player detect the container format (`ftyp` box in MP4, `RIFF` in WebM, etc.),
then let the browser switch to range mode. Simpler: always treat absence of
`Range` as a full-body 200.

#### With `Range: bytes=<start>-<end>`

```
GET /media/bafy... HTTP/1.1
Range: bytes=0-1048575
```

```
HTTP/1.1 206 Partial Content
Accept-Ranges: bytes
Content-Type: video/mp4
Content-Length: 1048576
Content-Range: bytes 0-1048575/48234496
Cache-Control: public, max-age=31536000, immutable
ETag: "bafy..."

<1048576 bytes>
```

#### With `Range: bytes=<start>-` (open-ended)

```
GET /media/bafy... HTTP/1.1
Range: bytes=5242880-
```

```
HTTP/1.1 206 Partial Content
Content-Range: bytes 5242880-48234495/48234496
Content-Length: 42991616
... etc.
```

#### Multiple ranges

```
Range: bytes=0-1023,5000-5999
```

We do **not** support multiple ranges (rare in video playback). Respond with
a single range containing the full requested span, or fall back to 200 for
simplicity. The `<video>` tag does not use multi-range.

#### Range not satisfiable

```
HTTP/1.1 416 Range Not Satisfiable
Content-Range: bytes */48234496
```

#### Blob not found

```
HTTP/1.1 404 Not Found
Content-Type: text/plain

blob <hash> not found in store
```

#### Blob partially available (missing chunks via lazy sync)

If the requested byte range overlaps chunks that have not been downloaded yet,
`BlobReader::read()` will return an `io::Error`. Options:

| Strategy | Behavior |
|----------|----------|
| **Fail 503** | Return `503 Service Unavailable` with `Retry-After`. Player retries. |
| **Block & fetch** | Trigger an on-demand `BlobStore::fetch()` for the missing chunk, await arrival, then read. |
| **Serve what we have** | Return `206` with only available contiguous bytes. Short response — player may stall. |

**Recommendation: Block & fetch.** Trigger a background fetch for the missing
range if the blob is partial, await the download, then complete the read. The
player experiences latency instead of errors. This requires wiring a
`Downloader` or `BlobApi::downloader()` into the media endpoint.

---

### Content-Type detection

We need a MIME type to set `Content-Type`. Three approaches ranked by
complexity:

#### A. Store MIME in iroh-docs metadata _(preferred, future)_

When a blob is added to a collection, the docs entry includes the MIME type:

```rust
// Docs entry for a media file:
// key:   "videos/clip.mp4"
// value: ContentEntry { hash: "bafy...", mime: "video/mp4", size: 48234496, ... }
```

The media endpoint looks up the blob's metadata in the docs namespace to get
the stored MIME type. This requires a namespace_id query parameter or a
separate endpoint.

#### B. Detect from URL / query parameter _(immediate, pragmatic)_

```
GET /media/bafy...?mime=video/mp4
GET /media/video/bafy...            (extension implied by path prefix)
```

The client (mapa) already knows the MIME type from the file extension or from
the protobuf `MapEvent` payload properties. It passes it along.

#### C. Detect from magic bytes _(zero-config, fallback)_

Read the first 256 bytes of the blob, match against known signatures:

| Bytes | MIME |
|-------|------|
| `00 00 00 xx 66 74 79 70` (MP4 `ftyp` box) | `video/mp4` |
| `1A 45 DF A3` (WebM/Matroska) | `video/webm` |
| `47 40 00` (MPEG-TS) | `video/mp2t` |
| `52 49 46 46 ... 57 45 42 50` (WebP) | `image/webp` |

Fallback: `application/octet-stream`.

**Initial implementation: option B with C as fallback.** MIME passed via
`?mime=` query param; if absent, detect from magic bytes.

---

### `<video>` tag integration

The `<video>` element in a browser or WebView sends these HTTP requests in
sequence:

```
1. GET /media/bafy...?mime=video/mp4
   → 200 + full body (or small initial chunk)
   → Browser reads container header, discovers track metadata

2. GET /media/bafy...?mime=video/mp4
   Range: bytes=0-1048575
   → 206 Partial Content
   → Buffer for playback

3. User seeks to 1:30 → browser calculates byte offset
   GET /media/bafy...?mime=video/mp4
   Range: bytes=6291456-
   → 206 Partial Content
   → Seek to offset 6 MB, stream from there
```

The browser handles all of this automatically given a valid `Accept-Ranges`
response. No JavaScript needed for basic playback.

For **adaptive streaming** (HLS/DASH), the `<video>` tag needs a playlist URL:

```html
<video>
  <source src="http://localhost:9193/media/bafy.../master.m3u8" type="application/vnd.apple.mpegurl">
</video>
```

But the segment files would also be blobs. This requires an endpoint that
resolves relative URLs within a blob-based directory structure, which is
beyond scope for v1.

#### URL format

```
http://127.0.0.1:9193/media/{hash}?mime={mime_type}
```

The hash is the iroh-blobs BLAKE3 `Hash` in multibase encoding (the same format
returned by `BlobStore::add_bytes()` and displayed by `syncweb` CLI).

For blobs that are referenced by an iroh-docs entry (a named file in a
collection), an alternative URL could resolve the path to a hash:

```
http://127.0.0.1:9193/docs/{namespace_id}/media/{path}
  → look up {path} in namespace {namespace_id}
  → resolve to hash
  → serve blob with MIME from docs metadata
```

This is more ergonomic but requires docs-level lookup. Start with the hash URL.

---

### Caching semantics

Content-addressed blobs are immutable by definition. Cache headers:

```
Cache-Control: public, max-age=31536000, immutable
ETag: "<hash>"
```

The `ETag` is the blob hash. The browser may send `If-None-Match: "<hash>"`
— respond `304 Not Modified` with empty body. For immutable blobs this is
always true.

Browsers cache range-request resources differently from full resources.
The `immutable` directive helps but is non-standard for range responses.
In practice, the `<video>` tag does not issue `If-None-Match` for media
— it relies on range requests and client-side buffering.

---

### WebRTC alternative (future consideration)

For real-time streaming between peers without a local server intermediary,
WebRTC DataChannels can carry bytes directly.

```
Peer A (has blob)  ←──WebRTC DataChannel──→  Peer B (browser)
```

**How it would work:**

1. A JavaScript/WASM bridge on peer B requests a blob range from peer A
   over the WebRTC signaling channel.
2. Peer A reads the range from its blob store via `BlobReader`, sends
   chunks over a DataChannel (binary mode, `arraybuffer`).
3. Peer B reassembles chunks and feeds them to a
   [`MediaSource`](https://developer.mozilla.org/en-US/docs/Web/API/MediaSource)
   buffer.

**Challenges:**

- `<video>` does not consume WebRTC natively. You need a `MediaSource` +
  `SourceBuffer` JavaScript shim that receives chunks over a DataChannel
  and appends them.
- The JS shim must implement its own range-request-to-chunk protocol — the
  browser won't auto-negotiate byte ranges.
- Seeking requires signaling a new range request to peer A, waiting for
  response, flushing the `SourceBuffer`, and re-appending.
- WebRTC setup requires ICE/STUN/TURN — nontrivial in a local-first
  context (though LAN-only works without STUN).
- WebRTC in Android WebView requires additional permissions and API levels.

**When WebRTC makes sense:**

- Off-grid/air-gap scenarios (BLE + WiFi Direct handoff, as mapa already
  explores).
- When no local syncweb service is running and you want direct browser-to-
  peer streaming.
- When latency is critical (live streaming).

**For v1, HTTP range requests are the right primitive.** They leverage
the browser's native media pipeline, require zero JavaScript, and work
with every `<video>` tag on every platform.

---

### Completeness: what about audio?

The same endpoint works for `<audio>` tags. The MIME type is the only
difference:

```
<audio controls src="http://127.0.0.1:9193/media/bafy...?mime=audio/mpeg">
```

Audio codecs that browsers support natively: MP3 (`audio/mpeg`), Ogg
Vorbis (`audio/ogg`), Opus (`audio/opus` or `audio/ogg; codecs=opus`),
AAC (`audio/aac`), FLAC (`audio/flac`), WAV (`audio/wav`).

---

## Implementation plan — Rust side

### Dependencies

Add an HTTP framework to `syncweb-core/Cargo.toml`:

```toml
# Media HTTP server
axum = { version = "0.7", optional = true }
hyper = { version = "1", features = ["server", "http1"], optional = true }
tokio-util = { version = "0.7", features = ["io"] }
mime_guess = "2"           # or manual magic-byte detection
```

Use `axum` for routing + `hyper` for the low-level server. The
`axum::body::Body` can wrap a `BodyStream` that pipes from `BlobReader`.

### Module layout

```
syncweb-core/src/media/
├── mod.rs             # re-exports
├── server.rs          # MediaServer: binds TCP, routes requests
├── range.rs           # Range header parsing, Content-Range construction
├── mime.rs            # MIME detection (query param + magic bytes)
└── serve.rs           # Core serving logic: hash -> BlobReader -> HTTP response
```

### Key implementation: `serve_blob_range()`

```rust
// media/serve.rs

use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use crate::node::blob_store::BlobStore;

pub(crate) struct MediaState {
    pub blob_store: BlobStore,
}

pub(crate) async fn serve_media(
    axum::extract::State(state): axum::extract::State<Arc<MediaState>>,
    axum::extract::Path(hash_str): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let hash: Hash = match hash_str.parse() {
        Ok(h) => h,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid hash").into_response(),
    };

    // Check blob existence
    if !state.blob_store.has(hash).await.unwrap_or(false) {
        return (StatusCode::NOT_FOUND, "blob not found").into_response();
    }

    // Open reader & get total size
    // NOTE: BlobReader does NOT expose total size after creation.
    // We need to get size from the blob store metadata, or import it
    // during blob creation. For v1, store size alongside the hash.
    let reader = state.blob_store.reader(hash);
    let total_size = /* get from metadata */;

    // Detect MIME
    let mime = detect_mime_from_query(&params)
        .or_else(|| detect_mime_from_magic(&state.blob_store, hash))
        .unwrap_or("application/octet-stream");

    // Parse Range header
    let range = match parse_range_header(&headers, total_size) {
        Some(r) => r,
        None => {
            // No Range header — serve full body with 200
            return serve_full(reader, total_size, mime).await;
        }
    };

    // Seek to range start
    if let Err(_) = reader.seek(SeekFrom::Start(range.start)).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, "seek failed").into_response();
    }

    // Read range
    let range_len = range.end - range.start + 1;
    // For on-demand fetch of missing chunks:
    // let reader = ensure_range_available(reader, range, &state.blob_store).await?;

    let stream = reader_stream(reader, range_len);
    let body = Body::from_stream(stream);

    let mut response = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, range_len.to_string())
        .header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", range.start, range.end, total_size),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ETAG, format!("\"{}\"", hash_str))
        .body(body)
        .unwrap();

    response.into_response()
}
```

### Size metadata

`BlobReader` does not expose total blob size. The size is stored **inside**
the blob's first BLAKE3 chunk (as part of the Bao tree header). Options:

1. **Store size in iroh-docs alongside the hash** — the `ContentEntry`
   struct in the docs entry includes `content_len`. Use this.
2. **Read the Bao tree header** — the `bao_tree::BaoTree::new(size, block_size)`
   has the size, but it's not exposed through `BlobReader`.
3. **Use `iroh_blobs::store::ExportRangesProgress`** — the `export_ranges()`
   stream emits `Size(u64)` as the first item.
4. **Track size at insert time** — when the blob is added via
   `BlobStore::add_bytes()`, also store the `(hash, size)` pair in a
   local `HashMap` or SQLite table.

Option 1 is idiomatic — syncweb already tracks content size in iroh-docs
entries. The media endpoint would resolve the hash → docs entry → size.

### On-demand fetch for partial blobs

When the blob is incomplete and the requested range has missing chunks:

```rust
async fn ensure_range_available(
    blob_store: &BlobStore,
    hash: Hash,
    range: ByteRange,
) -> Result<(), Error> {
    // Check if the blob is complete
    if blob_store.has(hash).await? {
        return Ok(());
    }

    // Get missing chunks in the requested range
    let bitfield = blob_store.observe(hash).await?; // which chunks are present
    let missing = find_missing_chunks(&bitfield, range);

    // Attempt to fetch from connected peers and wait
    let downloader = blob_store.downloader(endpoint);
    let handle = downloader.queue(hash).await?;
    handle.wait_for_range(range).await?;

    Ok(())
}
```

This requires a `Downloader` that can be scoped to a byte range. The iroh-blobs
protocol already supports range-based fetching (`BlobDownloadRequest` with
`RangeSpec`). The downloader API may need wrapping for this use case.

**For v1: return 503 for missing ranges, let the player retry.** This is
simpler and still works — the browser retries failed range requests. Once
the blob is fully synced (via the normal sync process), playback works.

---

### Server startup

```bash
# Standalone media server
syncweb media --data-dir /home/user/.syncweb --bind 127.0.0.1:9193

# Combined with bridge (common case for mapa)
syncweb daemon --data-dir /home/user/.syncweb \
  --bridge-listen 127.0.0.1:9192 \
  --media-listen 127.0.0.1:9193
```

The media server shares the `Arc<IrohNode>` with the bridge and daemon.
All three share the same blob store, so blob contents are consistent.

---

## Comparison table

| Approach | Protocol | Seeking | Latency | Browser support | JS needed |
|----------|----------|---------|---------|-----------------|-----------|
| **HTTP Range (this plan)** | HTTP/1.1 | Native via `Range` header | Low (local loopback) | Universal | None |
| **WebRTC DataChannel** | SCTP/DTLS | Manual via JS signaling | Very low | Chrome, Firefox, Safari | Full shim |
| **blob: URL (full download)** | N/A | None (must download entire file first) | High (wait for full blob) | Universal | Minimal |
| **HLS via Service Worker** | HTTP + JS | Via m3u8 playlist | Low (segmented) | Chrome, Firefox, Safari | Service Worker |

**Recommendation:** HTTP Range Requests are the right v1 primitive. WebRTC is
a separate optimization for off-grid/low-latency scenarios.

---

## Open questions

1. **Size metadata:** Should the endpoint require a `?size=<bytes>` parameter
   (like it does for `?mime=`), or should we resolve size from iroh-docs
   metadata? If the blob was added outside a docs namespace (e.g., raw
   `syncweb add-file`), there is no docs metadata to consult.

2. **Partial blob semantics:** When the blob is incomplete, should the server:
   - Block and wait for the missing chunks (adds latency)?
   - Return 503 with `Retry-After` (player retries)?
   - Return as much as it can (206 with truncated range)?
   - Return a [multipart/byteranges](https://www.rfc-editor.org/rfc/rfc9110#section-14.6)
     response with available chunks?

3. **Docs-based URL routing:** Should we add a `/docs/{namespace_id}/media/{path}`
   endpoint for human-readable URLs, or stick to hash-only URLs where the
   client resolves the name→hash mapping itself?

4. **CORS:** The media endpoint runs on a different port than the map app's
   WebView origin. Do we need `Access-Control-Allow-Origin: *` headers, or is
   the WebView configured to allow cross-origin requests to `localhost`?

5. **WebSocket transport for media:** Could the existing WebSocket bridge
   (from the [mapa-p2p-api.md](./mapa-p2p-api.md) plan) serve media chunks
   instead of a separate HTTP server? A custom binary frame type for
   `read_blob_range(hash, offset, len)` would reuse the connection. But
   `<video>` tags cannot speak WebSocket — a JavaScript `MediaSource` shim
   is required, which defeats the purpose of the HTTP approach.
