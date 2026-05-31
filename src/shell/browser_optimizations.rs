use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub use soliloquy_browser_optimizations::memory::{TabResidencyManager, TabStats};
pub use soliloquy_browser_optimizations::v8_integration::{GcScheduler, GcType};

const WARNING_THRESHOLD: usize = 2 * 1024 * 1024 * 1024;
const CRITICAL_THRESHOLD: usize = 3 * 1024 * 1024 * 1024;

static CURRENT_USAGE: AtomicUsize = AtomicUsize::new(0);
static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct MemoryPressureMonitor;

impl MemoryPressureMonitor {
    pub fn start_monitoring(&self) {
        MONITORING_ACTIVE.store(true, Ordering::SeqCst);
    }

    pub fn is_under_pressure(&self) -> bool {
        CURRENT_USAGE.load(Ordering::SeqCst) >= WARNING_THRESHOLD
    }

    pub fn update_usage(&self, usage: usize) {
        CURRENT_USAGE.store(usage, Ordering::SeqCst);
    }

    pub fn get_usage_percentage(&self) -> f64 {
        (CURRENT_USAGE.load(Ordering::SeqCst) as f64 / CRITICAL_THRESHOLD as f64) * 100.0
    }

    pub fn is_monitoring_active(&self) -> bool {
        MONITORING_ACTIVE.load(Ordering::SeqCst)
    }
}
