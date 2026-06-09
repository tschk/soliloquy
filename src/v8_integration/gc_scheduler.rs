//! Idle-time garbage collection scheduler for V8
//!
//! Schedules GC to run during browser idle periods to minimize
//! impact on user interactions.

use log::{debug, info};
use std::time::{Duration, Instant};

/// GC scheduling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcStrategy {
    /// Schedule GC during idle time
    Idle,
    /// Force GC immediately
    Immediate,
    /// Defer GC until memory pressure
    Deferred,
}

/// GC type to perform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcType {
    /// Minor (scavenge) collection - fast, young generation only
    Minor,
    /// Major (mark-sweep) collection - slower, full heap
    Major,
    /// Incremental marking step
    IncrementalMarking,
}

impl GcType {
    /// Estimated duration for this GC type
    pub fn estimated_duration(&self) -> Duration {
        match self {
            GcType::Minor => Duration::from_millis(10),
            GcType::Major => Duration::from_millis(100),
            GcType::IncrementalMarking => Duration::from_millis(5),
        }
    }
}

/// GC statistics
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    /// Number of minor GCs performed
    pub minor_gc_count: u64,
    /// Number of major GCs performed
    pub major_gc_count: u64,
    /// Total time spent in GC (milliseconds)
    pub total_gc_time_ms: u64,
    /// Last GC timestamp
    pub last_gc: Option<Instant>,
}

/// Idle-time GC scheduler
pub struct GcScheduler {
    /// GC statistics
    stats: GcStats,
    /// Minimum idle time required for GC (milliseconds)
    min_idle_time: u64,
    /// Time since last interaction
    last_interaction: Instant,
    /// Idle threshold before triggering GC
    idle_threshold: Duration,
    /// Enable automatic scheduling
    auto_schedule: bool,
    /// Pending GC requests
    pending_gc: Option<GcType>,
}

impl GcScheduler {
    /// Create a new GC scheduler
    pub fn new() -> Self {
        GcScheduler {
            stats: GcStats::default(),
            min_idle_time: 50, // 50ms minimum idle time
            last_interaction: Instant::now(),
            idle_threshold: Duration::from_secs(1), // 1 second idle threshold
            auto_schedule: true,
            pending_gc: None,
        }
    }

    /// Set minimum idle time required for GC
    pub fn set_min_idle_time(&mut self, ms: u64) {
        self.min_idle_time = ms;
    }

    /// Set idle threshold
    pub fn set_idle_threshold(&mut self, threshold: Duration) {
        self.idle_threshold = threshold;
    }

    /// Enable/disable automatic scheduling
    pub fn set_auto_schedule(&mut self, enabled: bool) {
        self.auto_schedule = enabled;
        info!(
            "GC auto-scheduling: {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// Record user interaction (resets idle timer)
    pub fn record_interaction(&mut self) {
        self.last_interaction = Instant::now();
    }

    /// Get time since last interaction
    pub fn idle_duration(&self) -> Duration {
        self.last_interaction.elapsed()
    }

    /// Check if browser is idle
    pub fn is_idle(&self) -> bool {
        self.idle_duration() >= self.idle_threshold
    }

    /// Request a GC to be scheduled
    pub fn request_gc(&mut self, gc_type: GcType) {
        debug!("GC requested: {:?}", gc_type);
        self.pending_gc = Some(gc_type);
    }

    /// Check if GC should run now
    pub fn should_run_gc(&self) -> Option<GcType> {
        if !self.auto_schedule {
            return self.pending_gc;
        }

        // Check for pending GC
        if let Some(gc_type) = self.pending_gc {
            // Only run if we have enough idle time
            if self.is_idle() {
                let estimated = gc_type.estimated_duration();
                if self.idle_duration() >= estimated {
                    return Some(gc_type);
                }
            }
        }

        // Auto-schedule based on idle time
        if self.is_idle() {
            let idle = self.idle_duration();

            // Long idle period - do major GC
            if idle >= Duration::from_secs(5) {
                return Some(GcType::Major);
            }

            // Medium idle - do minor GC
            if idle >= Duration::from_secs(2) {
                return Some(GcType::Minor);
            }

            // Short idle - incremental marking
            if idle >= Duration::from_millis(100) {
                return Some(GcType::IncrementalMarking);
            }
        }

        None
    }

    /// Record GC execution
    pub fn record_gc(&mut self, gc_type: GcType, duration: Duration) {
        match gc_type {
            GcType::Minor => self.stats.minor_gc_count += 1,
            GcType::Major => self.stats.major_gc_count += 1,
            GcType::IncrementalMarking => {}
        }

        self.stats.total_gc_time_ms += duration.as_millis() as u64;
        self.stats.last_gc = Some(Instant::now());
        self.pending_gc = None;

        debug!(
            "GC completed: {:?} in {}ms (total GCs: {} minor, {} major)",
            gc_type,
            duration.as_millis(),
            self.stats.minor_gc_count,
            self.stats.major_gc_count
        );
    }

    /// Get GC statistics
    pub fn stats(&self) -> &GcStats {
        &self.stats
    }

    /// Get average GC time
    pub fn average_gc_time(&self) -> f64 {
        let total_gcs = self.stats.minor_gc_count + self.stats.major_gc_count;
        if total_gcs > 0 {
            self.stats.total_gc_time_ms as f64 / total_gcs as f64
        } else {
            0.0
        }
    }
}

impl Default for GcScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_gc_scheduler_creation() {
        let scheduler = GcScheduler::new();
        assert!(scheduler.auto_schedule);
        assert_eq!(scheduler.min_idle_time, 50);
    }

    #[test]
    fn test_gc_type_duration() {
        assert!(GcType::Minor.estimated_duration() < GcType::Major.estimated_duration());
        assert!(
            GcType::IncrementalMarking.estimated_duration() < GcType::Minor.estimated_duration()
        );
    }

    #[test]
    fn test_idle_detection() {
        let mut scheduler = GcScheduler::new();
        scheduler.set_idle_threshold(Duration::from_millis(50));

        assert!(!scheduler.is_idle());

        sleep(Duration::from_millis(100));
        assert!(scheduler.is_idle());

        scheduler.record_interaction();
        assert!(!scheduler.is_idle());
    }

    #[test]
    fn test_gc_request() {
        let mut scheduler = GcScheduler::new();
        scheduler.request_gc(GcType::Minor);

        assert_eq!(scheduler.pending_gc, Some(GcType::Minor));
    }

    #[test]
    fn test_should_run_gc() {
        let mut scheduler = GcScheduler::new();
        scheduler.set_idle_threshold(Duration::from_millis(50));
        scheduler.request_gc(GcType::Minor);

        // Not idle yet
        assert!(scheduler.should_run_gc().is_none());

        // Wait for idle
        sleep(Duration::from_millis(100));
        assert_eq!(scheduler.should_run_gc(), Some(GcType::Minor));
    }

    #[test]
    fn test_record_gc() {
        let mut scheduler = GcScheduler::new();

        scheduler.record_gc(GcType::Minor, Duration::from_millis(10));
        assert_eq!(scheduler.stats().minor_gc_count, 1);
        assert_eq!(scheduler.stats().total_gc_time_ms, 10);

        scheduler.record_gc(GcType::Major, Duration::from_millis(50));
        assert_eq!(scheduler.stats().major_gc_count, 1);
        assert_eq!(scheduler.stats().total_gc_time_ms, 60);
    }

    #[test]
    fn test_average_gc_time() {
        let mut scheduler = GcScheduler::new();

        scheduler.record_gc(GcType::Minor, Duration::from_millis(10));
        scheduler.record_gc(GcType::Major, Duration::from_millis(50));

        let avg = scheduler.average_gc_time();
        assert_eq!(avg, 30.0); // (10 + 50) / 2
    }

    #[test]
    fn test_auto_schedule_disabled() {
        let mut scheduler = GcScheduler::new();
        scheduler.set_auto_schedule(false);
        scheduler.set_idle_threshold(Duration::from_millis(10));

        sleep(Duration::from_millis(50));

        // Should not auto-schedule
        assert!(scheduler.should_run_gc().is_none());
    }

    #[test]
    fn test_should_run_gc_auto_schedule() {
        let mut scheduler = GcScheduler::new();
        scheduler.set_idle_threshold(Duration::from_millis(50));

        // Test Incremental Marking (100ms threshold)
        scheduler.last_interaction = Instant::now() - Duration::from_millis(150);
        assert_eq!(scheduler.should_run_gc(), Some(GcType::IncrementalMarking));

        // Test Minor GC (2s threshold)
        scheduler.last_interaction = Instant::now() - Duration::from_secs(3);
        assert_eq!(scheduler.should_run_gc(), Some(GcType::Minor));

        // Test Major GC (5s threshold)
        scheduler.last_interaction = Instant::now() - Duration::from_secs(6);
        assert_eq!(scheduler.should_run_gc(), Some(GcType::Major));
    }
}
