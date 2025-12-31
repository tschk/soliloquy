//! Soliloquy Browser Optimizations
//!
//! Comprehensive optimization system for browser-as-desktop-environment
//! targeting 150+ tabs with <3GB RAM usage.
//!
//! ## Modules
//!
//! - `memory`: Tab residency management with tiered eviction
//! - `zircon`: Zero-copy memory sharing and IPC primitives
//! - `gpu`: GPU-accelerated layout and damage-based compositing
//! - `cache`: Unified caching system with LRU and texture atlasing
//! - `v8_integration`: V8 snapshot/bytecode caching and GC scheduling
//! - `network`: DNS prefetch and resource prioritization

pub mod memory;
pub mod zircon;
pub mod gpu;
pub mod cache;
pub mod v8_integration;
pub mod network;

// Re-export commonly used types
pub use memory::{
    TabResidencyManager, ResidencyState, TabResidency, TabSnapshot,
    MemoryPressureMonitor, MemoryPressureLevel,
};

pub use zircon::{
    ZirconVmo, ZirconTabMemory, create_channel, ChannelEndpoint,
    Capability, IsolationManager,
};

pub use gpu::{
    GpuLayoutCompute, LayoutNode, DamageTracker, CompositorLayer,
};

pub use cache::{
    LruCache, TextureAtlasManager,
};

pub use v8_integration::{
    V8BytecodeCache, GcScheduler, SnapshotManager,
};

pub use network::{
    PrefetchManager, PriorityQueue,
};
