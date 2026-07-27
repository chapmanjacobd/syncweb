use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use iroh::endpoint::{Connection, EndpointHooks, AfterHandshakeOutcome, VarInt};

#[derive(Debug)]
pub struct MembershipHook {
    pub member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
}

impl EndpointHooks for MembershipHook {
    fn after_handshake<'a>(
        &'a self,
        conn: &'a Connection,
    ) -> impl std::future::Future<Output = AfterHandshakeOutcome> + Send + 'a {
        async move {
            let alpn = conn.alpn();
            match alpn {
                b"iroh-docs/1" | b"iroh-gossip/1" => {
                    let guard = self.member_keys.read().await;
                    if guard.contains(&conn.remote_id()) {
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
