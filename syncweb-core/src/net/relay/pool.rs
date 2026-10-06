//! Syncthing relay pool discovery.
//!
//! When no relays are configured, syncweb fetches the public pool so the
//! bep-relay custom transport has somewhere to register. The pool API is a
//! simple HTTPS endpoint returning a JSON list of `relay://` URLs.

use std::time::Duration;

use serde::Deserialize;

use crate::error::{Result, SyncwebError};

use super::certs::ensure_provider_installed;

/// The public Syncthing relay pool endpoint.
pub const POOL_HTTP_ENDPOINT: &str = "https://relays.syncthing.net/endpoint";

/// HTTP user agent for pool requests.
const POOL_USER_AGENT: &str = concat!("syncweb/", env!("CARGO_PKG_VERSION"));

/// Fetch the list of pool relay urls.
///
/// # Errors
///
/// Returns an error if the endpoint is unreachable, returns a non-success
/// status, or its body is not the expected JSON shape.
pub async fn fetch_relay_pool(timeout: Duration) -> Result<Vec<String>> {
    fetch_relay_pool_from(POOL_HTTP_ENDPOINT, timeout).await
}

/// Fetch the pool from a specific endpoint (used by tests).
async fn fetch_relay_pool_from(endpoint: &str, timeout: Duration) -> Result<Vec<String>> {
    ensure_provider_installed();
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(POOL_USER_AGENT)
        .build()
        .map_err(|error| SyncwebError::operation("relay pool client", error))?;
    let response: PoolResponse = client
        .get(endpoint)
        .send()
        .await
        .map_err(|error| SyncwebError::operation("relay pool request", error))?
        .error_for_status()
        .map_err(|error| SyncwebError::operation("relay pool response status", error))?
        .json()
        .await
        .map_err(|error| SyncwebError::operation("relay pool response body", error))?;
    Ok(response.relays.into_iter().map(|relay| relay.url).collect())
}

/// The JSON shape served by the pool endpoint.
#[derive(Deserialize)]
struct PoolResponse {
    relays: Vec<PoolRelay>,
}

/// A single pool relay entry.
#[derive(Deserialize)]
struct PoolRelay {
    url: String,
}
