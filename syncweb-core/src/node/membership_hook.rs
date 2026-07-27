use iroh::endpoint::{AfterHandshakeOutcome, Connection, EndpointHooks, VarInt};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug)]
#[non_exhaustive]
pub struct MembershipHook {
    pub member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
}

impl EndpointHooks for MembershipHook {
    async fn after_handshake<'a>(&'a self, conn: &'a Connection) -> AfterHandshakeOutcome {
        let Ok(guard) = self.member_keys.read() else {
            return AfterHandshakeOutcome::Accept;
        };
        if guard.is_empty() {
            return AfterHandshakeOutcome::Accept;
        }
        let is_member = guard.contains(&conn.remote_id());
        drop(guard);
        if is_member {
            AfterHandshakeOutcome::Accept
        } else {
            AfterHandshakeOutcome::Reject {
                error_code: VarInt::from_u32(0),
                reason: b"not a network member".to_vec(),
            }
        }
    }
}
