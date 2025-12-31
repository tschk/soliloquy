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
//! - `cache`: Unified caching system (planned)
//! - `v8_integration`: V8 optimizations (planned)
//! - `network`: Network stack optimizations (planned)

pub mod memory;
pub mod zircon;
pub mod gpu;

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
