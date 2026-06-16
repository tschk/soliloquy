//! Session manager — owns the desktop session lifecycle.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
pub struct SessionManager {
    pub user: String,
    pub display: String,
    pub started_at: u64,
    exit_requested: AtomicU64,
}

impl SessionManager {
    pub fn new(user: String, display: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            user,
            display,
            started_at: now,
            exit_requested: AtomicU64::new(0),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(self.started_at)
    }

    pub fn request_exit(&self) {
        self.exit_requested.store(1, Ordering::Relaxed);
    }

    pub fn is_exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Relaxed) != 0
    }
}
