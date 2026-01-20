//! Chrome-like optimization flags
//!
//! These flags control various performance optimizations similar to Chrome's
//! command-line flags and field trials.

use std::collections::HashSet;

/// Optimization flags for the browser
#[derive(Debug, Clone)]
pub struct OptimizationFlags {
    // === V8/JavaScript Optimizations ===
    /// Enable TurboFan optimizing compiler
    pub turbofan: bool,

    /// Enable Sparkplug baseline compiler (fast initial compilation)
    pub sparkplug: bool,

    /// Enable Maglev mid-tier compiler
    pub maglev: bool,

    /// Enable code caching (bytecode caching)
    pub code_caching: bool,

    /// Enable lazy compilation of functions
    pub lazy_compilation: bool,

    /// Enable concurrent compilation on background threads
    pub concurrent_compilation: bool,

    /// Maximum heap size in MB
    pub max_heap_size_mb: usize,

    /// Enable incremental marking for GC
    pub incremental_marking: bool,

    /// Enable concurrent garbage collection
    pub concurrent_gc: bool,

    // === Rendering Optimizations ===
    /// Enable GPU rasterization
    pub gpu_rasterization: bool,

    /// Enable partial tile invalidation
    pub partial_raster: bool,

    /// Enable zero-copy rasterizer output
    pub zero_copy: bool,

    /// Enable compositor threaded scrolling
    pub threaded_scrolling: bool,

    /// Enable layer caching
    pub layer_caching: bool,

    /// Number of rasterizer threads
    pub raster_threads: usize,

    /// Enable OOP (out-of-process) rasterization
    pub oop_rasterization: bool,

    /// Enable Skia GPU backend (SkiaRenderer)
    pub skia_renderer: bool,

    // === Network Optimizations ===
    /// Enable QUIC (HTTP/3)
    pub quic: bool,

    /// Enable preconnect to likely destinations
    pub preconnect: bool,

    /// Enable DNS prefetching
    pub dns_prefetch: bool,

    /// Enable preload scanning
    pub preload_scanning: bool,

    /// Enable Service Worker
    pub service_workers: bool,

    /// Maximum parallel connections per host
    pub max_connections_per_host: usize,

    // === Memory Optimizations ===
    /// Enable tab discarding under memory pressure  
    pub tab_discarding: bool,

    /// Enable compressed tab contents
    pub tab_freezing: bool,

    /// Enable back-forward cache
    pub back_forward_cache: bool,

    /// Enable memory pressure handling
    pub memory_pressure_handling: bool,

    /// Enable aggressive garbage collection under pressure
    pub aggressive_gc_on_pressure: bool,

    // === Compositor Optimizations ===
    /// Enable double buffering
    pub double_buffering: bool,

    /// Target frame rate
    pub target_fps: u32,

    /// Enable vsync
    pub vsync: bool,

    /// Enable hardware overlays
    pub hardware_overlays: bool,

    // === Experimental ===
    /// Set of enabled experimental features
    pub experiments: HashSet<String>,
}

impl Default for OptimizationFlags {
    fn default() -> Self {
        OptimizationFlags {
            // V8
            turbofan: true,
            sparkplug: true,
            maglev: true,
            code_caching: true,
            lazy_compilation: true,
            concurrent_compilation: true,
            max_heap_size_mb: 512,
            incremental_marking: true,
            concurrent_gc: true,

            // Rendering
            gpu_rasterization: true,
            partial_raster: true,
            zero_copy: false, // Platform dependent
            threaded_scrolling: true,
            layer_caching: true,
            raster_threads: 4,
            oop_rasterization: true,
            skia_renderer: true,

            // Network
            quic: true,
            preconnect: true,
            dns_prefetch: true,
            preload_scanning: true,
            service_workers: true,
            max_connections_per_host: 6,

            // Memory
            tab_discarding: true,
            tab_freezing: true,
            back_forward_cache: true,
            memory_pressure_handling: true,
            aggressive_gc_on_pressure: true,

            // Compositor
            double_buffering: true,
            target_fps: 60,
            vsync: true,
            hardware_overlays: true,

            // Experimental
            experiments: HashSet::new(),
        }
    }
}

impl OptimizationFlags {
    /// Create flags matching Chrome's default optimizations
    pub fn chrome_like() -> Self {
        Self::default()
    }

    /// Create minimal flags for low-end devices
    pub fn low_end() -> Self {
        OptimizationFlags {
            turbofan: true,
            sparkplug: true,
            maglev: false, // Skip mid-tier
            code_caching: true,
            lazy_compilation: true,
            concurrent_compilation: false, // Save threads
            max_heap_size_mb: 128,
            incremental_marking: true,
            concurrent_gc: false,

            gpu_rasterization: false, // CPU raster
            partial_raster: true,
            zero_copy: false,
            threaded_scrolling: true,
            layer_caching: true,
            raster_threads: 2,
            oop_rasterization: false,
            skia_renderer: false,

            quic: false, // Simpler stack
            preconnect: true,
            dns_prefetch: true,
            preload_scanning: true,
            service_workers: true,
            max_connections_per_host: 4,

            tab_discarding: true, // Important on low-end
            tab_freezing: true,
            back_forward_cache: false, // Save memory
            memory_pressure_handling: true,
            aggressive_gc_on_pressure: true,

            double_buffering: true,
            target_fps: 30, // Lower target
            vsync: true,
            hardware_overlays: false,

            experiments: HashSet::new(),
        }
    }

    /// Create flags for high-end devices
    pub fn high_end() -> Self {
        let mut flags = Self::default();
        flags.max_heap_size_mb = 1024;
        flags.raster_threads = 8;
        flags.target_fps = 120;
        flags.zero_copy = true;
        flags.max_connections_per_host = 10;
        flags
    }

    /// Create flags optimized for battery life
    pub fn power_save() -> Self {
        OptimizationFlags {
            concurrent_compilation: false,
            concurrent_gc: false,
            gpu_rasterization: false,
            raster_threads: 2,
            preconnect: false,
            dns_prefetch: false,
            target_fps: 30,
            ..Self::low_end()
        }
    }

    /// Convert V8-related flags to command line args
    pub fn to_v8_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();

        if self.turbofan {
            flags.push("--turbofan".to_string());
        }
        if self.sparkplug {
            flags.push("--sparkplug".to_string());
        }
        if self.maglev {
            flags.push("--maglev".to_string());
        }
        if self.lazy_compilation {
            flags.push("--lazy".to_string());
        }
        if self.concurrent_compilation {
            flags.push("--concurrent-recompilation".to_string());
        }
        if self.incremental_marking {
            flags.push("--incremental-marking".to_string());
        }
        if self.concurrent_gc {
            flags.push("--concurrent-marking".to_string());
            flags.push("--parallel-scavenge".to_string());
        }

        flags.push(format!("--max-heap-size={}", self.max_heap_size_mb));

        flags
    }

    /// Enable an experiment by name
    pub fn enable_experiment(&mut self, name: &str) {
        self.experiments.insert(name.to_string());
    }

    /// Check if an experiment is enabled
    pub fn is_experiment_enabled(&self, name: &str) -> bool {
        self.experiments.contains(name)
    }
}
