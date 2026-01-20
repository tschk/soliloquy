//! Chrome-like optimizations for RV8
//!
//! This module contains performance optimizations inspired by Chrome/Chromium:
//!
//! - **V8 optimizations**: Turbofan, code caching, lazy compilation
//! - **Rendering optimizations**: Compositor threading, tile caching
//! - **Network optimizations**: Connection coalescing, preconnect
//! - **Memory optimizations**: Tab discarding, memory pressure handling

mod flags;
mod memory;
mod monitor;
mod preload;
mod render_opts;
mod v8_opts;

pub use flags::OptimizationFlags;
pub use memory::MemoryOptimizer;
pub use monitor::PerformanceMonitor;
pub use preload::{PreloadHint, Preloader};
pub use render_opts::RenderOptimizations;
pub use v8_opts::V8Optimizations;
