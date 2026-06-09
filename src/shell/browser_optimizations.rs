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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pressure_monitor() {
        let monitor = MemoryPressureMonitor::default();

        // 1. Initial State
        // Reset state to ensure clean test
        MONITORING_ACTIVE.store(false, Ordering::SeqCst);
        CURRENT_USAGE.store(0, Ordering::SeqCst);

        assert!(!monitor.is_monitoring_active());
        assert!(!monitor.is_under_pressure());
        assert_eq!(monitor.get_usage_percentage(), 0.0);

        // 2. Activation
        monitor.start_monitoring();
        assert!(monitor.is_monitoring_active());

        // 3. Normal Usage (below warning)
        let safe_usage = 1 * 1024 * 1024 * 1024; // 1 GB
        monitor.update_usage(safe_usage);
        assert!(!monitor.is_under_pressure());
        assert_eq!(monitor.get_usage_percentage(), (safe_usage as f64 / CRITICAL_THRESHOLD as f64) * 100.0);

        // 4. Warning Threshold
        monitor.update_usage(WARNING_THRESHOLD); // 2 GB
        assert!(monitor.is_under_pressure());
        assert_eq!(monitor.get_usage_percentage(), (WARNING_THRESHOLD as f64 / CRITICAL_THRESHOLD as f64) * 100.0);

        // 5. Critical Threshold
        monitor.update_usage(CRITICAL_THRESHOLD); // 3 GB
        assert!(monitor.is_under_pressure());
        assert_eq!(monitor.get_usage_percentage(), 100.0);

        // 6. Above Critical
        let extreme_usage = 4 * 1024 * 1024 * 1024; // 4 GB
        monitor.update_usage(extreme_usage);
        assert!(monitor.is_under_pressure());
        assert!(monitor.get_usage_percentage() > 100.0);
    }
}
