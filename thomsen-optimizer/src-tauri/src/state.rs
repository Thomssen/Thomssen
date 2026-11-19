use std::sync::{Arc, Mutex};
use thomsen_optimizer_core::engine::Engine;

pub struct AppState {
    pub engine: Arc<Engine>,
    /// The update found by the last check, kept so "Install" installs exactly it.
    pub pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
}

impl AppState {
    pub fn new(engine: Arc<Engine>) -> Self {
        Self { engine, pending_update: Mutex::new(None) }
    }
}
