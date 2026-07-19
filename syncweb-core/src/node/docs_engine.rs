use iroh_docs::engine::Engine;
use std::sync::Arc;

pub struct DocsEngine {
    engine: Arc<Engine>,
}

impl DocsEngine {
    pub fn new(engine: Arc<Engine>) -> Self {
        Self { engine }
    }

    pub fn inner(&self) -> &Engine {
        &self.engine
    }
}
