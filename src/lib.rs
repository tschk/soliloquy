//! Soliloquy Browser Optimizations
//!
//! Comprehensive optimization system for browser-as-desktop-environment
//! targeting 150+ tabs with <3GB RAM usage.
//!
//! ## Modules
//!
//! - `memory`: Tab residency management with tiered eviction
//! - `gpu`: GPU-accelerated layout and damage-based compositing
//! - `cache`: Unified caching system with LRU and texture atlasing
//! - `v8_integration`: V8 snapshot/bytecode caching and GC scheduling
//! - `network`: DNS prefetch and resource prioritization

pub mod cache;
pub mod driver_catalog;
pub mod driver_manager;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod runtime;
pub mod settings;
pub mod v8_integration;

// Re-export commonly used types
pub use memory::{
    MemoryPressureLevel, MemoryPressureMonitor, ResidencyState, TabResidency, TabResidencyManager,
    TabSnapshot,
};

pub use gpu::{CompositorLayer, DamageTracker, GpuLayoutCompute, LayoutNode};

pub use cache::{LruCache, TextureAtlasManager};

pub use v8_integration::{GcScheduler, SnapshotManager, V8BytecodeCache};

pub use network::{PrefetchManager, PriorityQueue};

pub use runtime::{
    EngineRuntime, InputEvent, LifecycleEvent, PlatformTier, RuntimeError, SafeAreaInsets,
    SurfaceDescriptor, SurfaceId, SurfaceRotation, SurfaceSize,
};

pub use driver_manager::{
    AllowUnsignedPackages, Capability, CapabilityBroker, DriverError, DriverLease, DriverManager,
    DriverManifest, DriverRecord, DriverRegistry, DriverState, PackageSignature,
    RequireSignedPackages, TrustPolicy,
};

pub use driver_catalog::{DriverCatalog, DriverCatalogError, PersistentDriverManager};

pub use settings::{SettingToggle, SettingsManager};
