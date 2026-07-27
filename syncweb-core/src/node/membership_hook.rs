use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use iroh::endpoint::{Connection, EndpointHooks, AfterHandshakeOutcome, VarInt};

#[derive(Debug)]
pub struct MembershipHook {
    pub member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
    pub has_public_network: Arc<AtomicBool>,
}

impl EndpointHooks for MembershipHook {
    fn after_handshake<'a>(
        &'a self,
        conn: &'a Connection,
    ) -> impl std::future::Future<Output = AfterHandshakeOutcome> + Send + 'a {
        async move {
            let alpn = conn.alpn();
            let guard = self.member_keys.read().await;
            // No networks configured — no gating to apply.
            if guard.is_empty() {
                return AfterHandshakeOutcome::Accept;
            }
            let is_member = guard.contains(&conn.remote_id());
            drop(guard);
            match alpn {
                b"iroh-docs/1" | b"iroh-gossip/1" => {
                    if is_member {
                        AfterHandshakeOutcome::Accept
                    } else {
                        AfterHandshakeOutcome::Reject {
                            error_code: VarInt::from_u32(0),
                            reason: b"not a network member".to_vec(),
                        }
                    }
                }
                b"/iroh-bytes/4" => {
                    if is_member || self.has_public_network.load(Ordering::Relaxed) {
                        AfterHandshakeOutcome::Accept
                    } else {
                        AfterHandshakeOutcome::Reject {
                            error_code: VarInt::from_u32(0),
                            reason: b"not a network member".to_vec(),
                        }
                    }
                }
                _ => AfterHandshakeOutcome::Accept,
            }
        }
    }
}
