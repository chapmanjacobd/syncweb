use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use iroh_blobs::api::blobs::BlobStatus;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::node::blob_store::BlobStore;

use super::mime::detect_mime;
use super::range::{content_range, parse_range_header};

const fn static_header(v: &'static str) -> HeaderValue {
    HeaderValue::from_static(v)
}

#[derive(Clone)]
pub struct MediaState {
    pub blob_store: BlobStore,
}

pub async fn serve_media(
    State(state): State<Arc<MediaState>>,
    Path(hash_str): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let hash: iroh_blobs::Hash = match hash_str.parse() {
        Ok(h) => h,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "invalid hash").into_response();
        }
    };

    let blob_store = &state.blob_store;

    let total_size = match blob_store.stat(hash).await {
        Ok(BlobStatus::Complete { size }) => size,
        Ok(BlobStatus::Partial { size }) => size.unwrap_or(0),
        Ok(BlobStatus::NotFound) | Err(_) => {
            return (StatusCode::NOT_FOUND, format!("blob {hash_str} not found in store")).into_response();
        }
    };

    if total_size == 0 {
        return (StatusCode::NOT_FOUND, "blob has zero size").into_response();
    }

    let mime = if let Some(mime_str) = params.get("mime").filter(|m| !m.is_empty()) {
        mime_str.clone()
    } else {
        let magic = peek_magic_bytes(blob_store, hash, 256).await;
        detect_mime(&HashMap::new(), &magic)
    };

    let Some(range) = parse_range_header(&headers, total_size) else {
        return serve_full(blob_store, hash, total_size, &mime, &hash_str);
    };

    if range.start >= total_size {
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
        let content_range_val = format!("bytes */{total_size}");
        if let Ok(hv) = HeaderValue::from_str(&content_range_val) {
            resp.headers_mut().insert(header::CONTENT_RANGE, hv);
        }
        return resp;
    }

    let reader = blob_store.reader(hash);
    let mut ranged_reader = reader;

    if ranged_reader.seek(SeekFrom::Start(range.start)).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "seek failed").into_response();
    }

    let range_len = range.end.saturating_sub(range.start).saturating_add(1);

    let content_range_val = content_range(range.start, range.end, total_size);

    let limited_reader = ranged_reader.take(range_len);
    let stream = ReaderStream::new(limited_reader);
    let body = Body::from_stream(stream);

    let mut resp = Response::new(body);
    *resp.status_mut() = StatusCode::PARTIAL_CONTENT;

    let set_hdr = |r: &mut Response, name: header::HeaderName, value: &str| {
        if let Ok(hv) = HeaderValue::from_str(value) {
            r.headers_mut().insert(name, hv);
        }
    };

    set_hdr(&mut resp, header::CONTENT_TYPE, &mime);
    set_hdr(&mut resp, header::CONTENT_LENGTH, &range_len.to_string());
    set_hdr(&mut resp, header::CONTENT_RANGE, &content_range_val);
    resp.headers_mut().insert(header::ACCEPT_RANGES, static_header("bytes"));
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        static_header("public, max-age=31536000, immutable"),
    );
    set_hdr(&mut resp, header::ETAG, &format!("\"{hash_str}\""));

    resp
}

fn serve_full(blob_store: &BlobStore, hash: iroh_blobs::Hash, total_size: u64, mime: &str, hash_str: &str) -> Response {
    let reader = blob_store.reader(hash);
    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let mut resp = Response::new(body);
    *resp.status_mut() = StatusCode::OK;

    if let Ok(hv) = HeaderValue::from_str(mime) {
        resp.headers_mut().insert(header::CONTENT_TYPE, hv);
    }
    if let Ok(hv) = HeaderValue::from_str(&total_size.to_string()) {
        resp.headers_mut().insert(header::CONTENT_LENGTH, hv);
    }
    resp.headers_mut().insert(header::ACCEPT_RANGES, static_header("bytes"));
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        static_header("public, max-age=31536000, immutable"),
    );
    if let Ok(hv) = HeaderValue::from_str(&format!("\"{hash_str}\"")) {
        resp.headers_mut().insert(header::ETAG, hv);
    }

    resp
}

async fn peek_magic_bytes(blob_store: &BlobStore, hash: iroh_blobs::Hash, max_len: usize) -> Vec<u8> {
    let reader = blob_store.reader(hash);
    let max_len_u64 = u64::try_from(max_len).unwrap_or(u64::MAX);
    let mut limited = reader.take(max_len_u64);
    let mut buf = Vec::with_capacity(max_len);
    let _ = limited.read_to_end(&mut buf).await;
    buf
}
