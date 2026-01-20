//! Performance monitoring and metrics

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Performance monitor for tracking browser metrics
pub struct PerformanceMonitor {
    /// Frame times for FPS calculation
    frame_times: RwLock<VecDeque<Duration>>,

    /// Total frames rendered
    frame_count: AtomicU64,

    /// Last sample time
    last_sample: RwLock<Instant>,

    /// JavaScript execution time (cumulative)
    js_time_ns: AtomicU64,

    /// Layout time (cumulative)
    layout_time_ns: AtomicU64,

    /// Paint time (cumulative)
    paint_time_ns: AtomicU64,

    /// Composite time (cumulative)
    composite_time_ns: AtomicU64,

    /// Network requests completed
    network_requests: AtomicU64,

    /// Network bytes received
    network_bytes: AtomicU64,

    /// Memory usage samples
    memory_samples: RwLock<VecDeque<usize>>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        PerformanceMonitor {
            frame_times: RwLock::new(VecDeque::with_capacity(120)),
            frame_count: AtomicU64::new(0),
            last_sample: RwLock::new(Instant::now()),
            js_time_ns: AtomicU64::new(0),
            layout_time_ns: AtomicU64::new(0),
            paint_time_ns: AtomicU64::new(0),
            composite_time_ns: AtomicU64::new(0),
            network_requests: AtomicU64::new(0),
            network_bytes: AtomicU64::new(0),
            memory_samples: RwLock::new(VecDeque::with_capacity(60)),
        }
    }

    /// Record a frame time
    pub fn record_frame(&self, duration: Duration) {
        self.frame_count.fetch_add(1, Ordering::Relaxed);

        let mut times = self.frame_times.write();
        if times.len() >= 120 {
            times.pop_front();
        }
        times.push_back(duration);
    }

    /// Record JavaScript execution time
    pub fn record_js_time(&self, duration: Duration) {
        self.js_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record layout time
    pub fn record_layout_time(&self, duration: Duration) {
        self.layout_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record paint time
    pub fn record_paint_time(&self, duration: Duration) {
        self.paint_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record composite time
    pub fn record_composite_time(&self, duration: Duration) {
        self.composite_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record network request completion
    pub fn record_network_request(&self, bytes: usize) {
        self.network_requests.fetch_add(1, Ordering::Relaxed);
        self.network_bytes
            .fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Take a sample (called periodically)
    pub fn sample(&self) {
        // Sample memory usage
        if let Some(usage) = get_memory_usage() {
            let mut samples = self.memory_samples.write();
            if samples.len() >= 60 {
                samples.pop_front();
            }
            samples.push_back(usage);
        }

        *self.last_sample.write() = Instant::now();
    }

    /// Calculate current FPS
    pub fn fps(&self) -> f64 {
        let times = self.frame_times.read();
        if times.is_empty() {
            return 0.0;
        }

        let total: Duration = times.iter().sum();
        let avg = total.as_secs_f64() / times.len() as f64;

        if avg > 0.0 {
            1.0 / avg
        } else {
            0.0
        }
    }

    /// Get average frame time in milliseconds
    pub fn avg_frame_time_ms(&self) -> f64 {
        let times = self.frame_times.read();
        if times.is_empty() {
            return 0.0;
        }

        let total: Duration = times.iter().sum();
        total.as_secs_f64() * 1000.0 / times.len() as f64
    }

    /// Get total frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }

    /// Get cumulative JS time
    pub fn js_time(&self) -> Duration {
        Duration::from_nanos(self.js_time_ns.load(Ordering::Relaxed))
    }

    /// Get cumulative layout time
    pub fn layout_time(&self) -> Duration {
        Duration::from_nanos(self.layout_time_ns.load(Ordering::Relaxed))
    }

    /// Get cumulative paint time
    pub fn paint_time(&self) -> Duration {
        Duration::from_nanos(self.paint_time_ns.load(Ordering::Relaxed))
    }

    /// Get average memory usage
    pub fn avg_memory_mb(&self) -> f64 {
        let samples = self.memory_samples.read();
        if samples.is_empty() {
            return 0.0;
        }

        let total: usize = samples.iter().sum();
        (total as f64 / samples.len() as f64) / (1024.0 * 1024.0)
    }

    /// Get network stats
    pub fn network_stats(&self) -> (u64, u64) {
        (
            self.network_requests.load(Ordering::Relaxed),
            self.network_bytes.load(Ordering::Relaxed),
        )
    }

    /// Generate a performance report
    pub fn report(&self) -> PerformanceReport {
        PerformanceReport {
            fps: self.fps(),
            avg_frame_time_ms: self.avg_frame_time_ms(),
            frame_count: self.frame_count(),
            js_time: self.js_time(),
            layout_time: self.layout_time(),
            paint_time: self.paint_time(),
            composite_time: Duration::from_nanos(self.composite_time_ns.load(Ordering::Relaxed)),
            avg_memory_mb: self.avg_memory_mb(),
            network_requests: self.network_requests.load(Ordering::Relaxed),
            network_bytes: self.network_bytes.load(Ordering::Relaxed),
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance report snapshot
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub fps: f64,
    pub avg_frame_time_ms: f64,
    pub frame_count: u64,
    pub js_time: Duration,
    pub layout_time: Duration,
    pub paint_time: Duration,
    pub composite_time: Duration,
    pub avg_memory_mb: f64,
    pub network_requests: u64,
    pub network_bytes: u64,
}

impl PerformanceReport {
    /// Check if performance is acceptable
    pub fn is_healthy(&self) -> bool {
        self.fps >= 55.0 && self.avg_frame_time_ms < 18.0
    }

    /// Get frame budget remaining (target 16.67ms for 60fps)
    pub fn frame_budget_remaining_ms(&self) -> f64 {
        16.67 - self.avg_frame_time_ms
    }
}

/// Get current process memory usage (platform-specific)
fn get_memory_usage() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(kb) = parts.get(1) {
                        if let Ok(kb) = kb.parse::<usize>() {
                            return Some(kb * 1024);
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    {
        // macOS implementation using mach APIs would go here
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}
