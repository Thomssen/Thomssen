use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::detection::DetectionEngine;
use crate::firewall::FirewallManager;
use crate::license::LicenseManager;

pub const ACTIVITY_HISTORY_LEN: usize = 30;

/// Shared across every Tauri command. `firewall` and `license` are cheap to
/// share (an internal channel handle and a plain device-id string,
/// respectively); `detection` and `activity_points` are genuinely mutable
/// state the background monitor loop (`monitor.rs`) updates every poll.
pub struct AppState {
    pub firewall: FirewallManager,
    pub license: Arc<LicenseManager>,
    pub detection: Mutex<DetectionEngine>,
    pub activity_points: Mutex<VecDeque<u32>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            firewall: FirewallManager::new(),
            license: Arc::new(LicenseManager::new()),
            detection: Mutex::new(DetectionEngine::new()),
            activity_points: Mutex::new(VecDeque::with_capacity(ACTIVITY_HISTORY_LEN)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
