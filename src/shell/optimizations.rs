//! Performance optimizations for the Soliloquy shell
//!
//! This module centralizes optimization settings for:
//! - V8 JavaScript execution (turbofan, memory management)
//! - Servo rendering (multithreading, caching)
//! - Frame pacing and present timing
//! - Memory management and garbage collection

use log::{debug, info};
use std::time::{Duration, Instant};

/// V8 optimization configuration
#[derive(Debug, Clone)]
pub struct V8Optimizations {
    /// Enable turbofan optimizing compiler
    pub turbofan_enabled: bool,
    /// Maximum heap size in MB
    pub max_heap_size_mb: usize,
    /// Initial heap size in MB
    pub initial_heap_size_mb: usize,
    /// Enable lazy compilation
    pub lazy_compilation: bool,
    /// Number of optimization threads
    pub optimization_threads: usize,
    /// Enable concurrent garbage collection
    pub concurrent_gc: bool,
    /// Enable incremental marking for GC
    pub incremental_marking: bool,
    /// Enable code caching
    pub code_cache_enabled: bool,
}

impl Default for V8Optimizations {
    fn default() -> Self {
        let cpu_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);

        V8Optimizations {
            turbofan_enabled: true,
            max_heap_size_mb: 512,
            initial_heap_size_mb: 64,
            lazy_compilation: true,
            optimization_threads: cpu_count.min(4),
            concurrent_gc: true,
            incremental_marking: true,
            code_cache_enabled: true,
        }
    }
}

impl V8Optimizations {
    /// Create optimizations for low-memory environments
    pub fn low_memory() -> Self {
        V8Optimizations {
            max_heap_size_mb: 128,
            initial_heap_size_mb: 32,
            optimization_threads: 2,
            ..Default::default()
        }
    }

    /// Create optimizations for high-performance environments
    pub fn high_performance() -> Self {
        let cpu_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(8);

        V8Optimizations {
            max_heap_size_mb: 1024,
            initial_heap_size_mb: 128,
            optimization_threads: cpu_count.min(8),
            ..Default::default()
        }
    }

    /// Convert to V8 command line flags
    pub fn to_v8_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();

        if self.turbofan_enabled {
            flags.push("--turbofan".to_string());
        }

        flags.push(format!("--max-heap-size={}", self.max_heap_size_mb));
        flags.push(format!("--initial-heap-size={}", self.initial_heap_size_mb));

        if self.lazy_compilation {
            flags.push("--lazy".to_string());
        }

        if self.concurrent_gc {
            flags.push("--concurrent-marking".to_string());
            flags.push("--parallel-scavenge".to_string());
        }

        if self.incremental_marking {
            flags.push("--incremental-marking".to_string());
        }

        flags.push(format!(
            "--concurrent-recompilation-front-end-threads={}",
            self.optimization_threads
        ));

        flags
    }
}

/// Servo/WebRender optimization configuration
#[derive(Debug, Clone)]
pub struct RenderOptimizations {
    /// Enable multithreaded layout
    pub multithreaded_layout: bool,
    /// Enable multithreaded painting
    pub multithreaded_painting: bool,
    /// Target frame rate (0 = uncapped)
    pub target_fps: u32,
    /// Use low power mode when idle
    pub low_power_idle: bool,
    /// Maximum texture cache size in MB
    pub texture_cache_mb: usize,
    /// Enable picture caching
    pub picture_caching: bool,
    /// Tile size for rendering
    pub tile_size: u32,
    /// Enable GPU rendering
    pub gpu_rendering: bool,
}

impl Default for RenderOptimizations {
    fn default() -> Self {
        RenderOptimizations {
            multithreaded_layout: true,
            multithreaded_painting: true,
            target_fps: 60,
            low_power_idle: true,
            texture_cache_mb: 256,
            picture_caching: true,
            tile_size: 512,
            gpu_rendering: true,
        }
    }
}

impl RenderOptimizations {
    /// Create optimizations for battery-powered devices
    pub fn power_saving() -> Self {
        RenderOptimizations {
            target_fps: 30,
            low_power_idle: true,
            texture_cache_mb: 128,
            ..Default::default()
        }
    }

    /// Create optimizations for high-refresh displays
    pub fn high_refresh() -> Self {
        RenderOptimizations {
            target_fps: 120,
            low_power_idle: false,
            texture_cache_mb: 512,
            ..Default::default()
        }
    }
}

/// Frame pacer for consistent frame timing
pub struct FramePacer {
    target_frame_time: Duration,
    last_frame_time: Instant,
    frame_count: u64,
    frame_times: Vec<Duration>,
    max_history: usize,
}

impl FramePacer {
    /// Create a new frame pacer targeting the given FPS
    pub fn new(target_fps: u32) -> Self {
        let target_frame_time = if target_fps > 0 {
            Duration::from_secs_f64(1.0 / target_fps as f64)
        } else {
            Duration::ZERO
        };

        FramePacer {
            target_frame_time,
            last_frame_time: Instant::now(),
            frame_count: 0,
            frame_times: Vec::with_capacity(120),
            max_history: 120,
        }
    }

    /// Begin a new frame, returns time since last frame
    pub fn begin_frame(&mut self) -> Duration {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;
        delta
    }

    /// Wait for next frame timing if needed
    pub fn wait_for_frame(&mut self) {
        if self.target_frame_time == Duration::ZERO {
            return;
        }

        let elapsed = self.last_frame_time.elapsed();
        if elapsed < self.target_frame_time {
            let sleep_time = self.target_frame_time - elapsed;
            std::thread::sleep(sleep_time);
        }

        // Record frame time for statistics
        let frame_time = self.last_frame_time.elapsed();
        if self.frame_times.len() >= self.max_history {
            self.frame_times.remove(0);
        }
        self.frame_times.push(frame_time);

        self.last_frame_time = Instant::now();
        self.frame_count += 1;
    }

    /// Get total frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get average frame time over recent history
    pub fn average_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            return Duration::ZERO;
        }

        let total: Duration = self.frame_times.iter().sum();
        total / self.frame_times.len() as u32
    }

    /// Get current FPS based on recent history
    pub fn current_fps(&self) -> f64 {
        let avg = self.average_frame_time();
        if avg == Duration::ZERO {
            return 0.0;
        }
        1.0 / avg.as_secs_f64()
    }

    /// Check if frame rate is meeting target
    pub fn is_meeting_target(&self) -> bool {
        if self.target_frame_time == Duration::ZERO {
            return true;
        }
        self.average_frame_time() <= self.target_frame_time * 110 / 100 // 10% tolerance
    }
}

/// Memory manager for controlling resource usage
pub struct MemoryManager {
    max_memory_mb: usize,
    warning_threshold: f64,
    critical_threshold: f64,
    last_gc_time: Instant,
    gc_interval: Duration,
}

impl MemoryManager {
    pub fn new(max_memory_mb: usize) -> Self {
        MemoryManager {
            max_memory_mb,
            warning_threshold: 0.75,
            critical_threshold: 0.90,
            last_gc_time: Instant::now(),
            gc_interval: Duration::from_secs(30),
        }
    }

    /// Check if garbage collection should be triggered
    pub fn should_gc(&self) -> bool {
        self.last_gc_time.elapsed() >= self.gc_interval
    }

    /// Record that GC was performed
    pub fn record_gc(&mut self) {
        self.last_gc_time = Instant::now();
    }

    /// Get memory thresholds in bytes
    pub fn thresholds(&self) -> (usize, usize) {
        let max_bytes = self.max_memory_mb * 1024 * 1024;
        (
            (max_bytes as f64 * self.warning_threshold) as usize,
            (max_bytes as f64 * self.critical_threshold) as usize,
        )
    }
}

/// Combined optimization settings
#[derive(Debug, Clone, Default)]
pub struct OptimizationSettings {
    pub v8: V8Optimizations,
    pub render: RenderOptimizations,
}

impl OptimizationSettings {
    /// Create settings optimized for desktop workloads
    pub fn desktop() -> Self {
        OptimizationSettings {
            v8: V8Optimizations::high_performance(),
            render: RenderOptimizations::default(),
        }
    }

    /// Create settings optimized for embedded/low-power devices
    pub fn embedded() -> Self {
        OptimizationSettings {
            v8: V8Optimizations::low_memory(),
            render: RenderOptimizations::power_saving(),
        }
    }
}

/// Initialize all optimizations based on settings
pub fn init_optimizations(settings: &OptimizationSettings) {
    info!("Initializing Soliloquy optimizations");

    info!(
        "V8 optimizations: turbofan={}, max_heap={}MB, threads={}",
        settings.v8.turbofan_enabled,
        settings.v8.max_heap_size_mb,
        settings.v8.optimization_threads
    );

    info!(
        "Render optimizations: multithreaded={}, target_fps={}, texture_cache={}MB",
        settings.render.multithreaded_layout,
        settings.render.target_fps,
        settings.render.texture_cache_mb
    );

    debug!("V8 flags: {:?}", settings.v8.to_v8_flags());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_optimizations_default() {
        let opts = V8Optimizations::default();
        assert!(opts.turbofan_enabled);
        assert!(opts.max_heap_size_mb > 0);
    }

    #[test]
    fn test_v8_flags_generation() {
        let opts = V8Optimizations::default();
        let flags = opts.to_v8_flags();
        assert!(!flags.is_empty());
        assert!(flags.iter().any(|f| f.contains("max-heap-size")));
    }

    #[test]
    fn test_frame_pacer() {
        let mut pacer = FramePacer::new(60);
        assert_eq!(pacer.frame_count(), 0);

        pacer.wait_for_frame();
        assert_eq!(pacer.frame_count(), 1);
    }

    #[test]
    fn test_memory_manager() {
        let manager = MemoryManager::new(512);
        let (warning, critical) = manager.thresholds();

        assert!(warning < critical);
        assert!(critical <= 512 * 1024 * 1024);
    }

    #[test]
    fn test_optimization_settings() {
        let desktop = OptimizationSettings::desktop();
        let embedded = OptimizationSettings::embedded();

        assert!(desktop.v8.max_heap_size_mb > embedded.v8.max_heap_size_mb);
        assert!(desktop.render.target_fps >= embedded.render.target_fps);
    }
}
