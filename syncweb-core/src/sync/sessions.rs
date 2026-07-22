use std::sync::{Mutex, OnceLock};

use iroh_docs::NamespaceId;
use tokio::sync::mpsc;

use super::SyncCommand;

type CancelSender = mpsc::UnboundedSender<SyncCommand>;
type Registry = Vec<(NamespaceId, CancelSender)>;

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

pub struct ActiveSession {
    namespace: NamespaceId,
}

impl ActiveSession {
    #[must_use]
    pub fn register(namespace: NamespaceId, cancel: CancelSender) -> Self {
        if let Ok(mut guard) = registry().lock() {
            guard.retain(|(id, _)| *id != namespace);
            guard.push((namespace, cancel));
        }
        Self { namespace }
    }
}

impl Drop for ActiveSession {
    fn drop(&mut self) {
        if let Ok(mut guard) = registry().lock() {
            guard.retain(|(id, _)| *id != self.namespace);
        }
    }
}

#[must_use]
pub fn cancel_session(namespace: NamespaceId) -> bool {
    if let Ok(mut guard) = registry().lock()
        && let Some(pos) = guard.iter().position(|(id, _)| *id == namespace)
    {
        let (_, cancel) = guard.remove(pos);
        return cancel.send(SyncCommand::Cancel).is_ok();
    }
    false
}

#[must_use]
pub fn is_active(namespace: NamespaceId) -> bool {
    registry()
        .lock()
        .is_ok_and(|guard| guard.iter().any(|(id, _)| *id == namespace))
}
