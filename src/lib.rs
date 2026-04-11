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

pub mod memory;
pub mod gpu;
pub mod cache;
pub mod v8_integration;
pub mod network;
pub mod runtime;
pub mod driver_manager;
pub mod driver_catalog;
pub mod settings;

// Re-export commonly used types
pub use memory::{
    TabResidencyManager, ResidencyState, TabResidency, TabSnapshot,
    MemoryPressureMonitor, MemoryPressureLevel,
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

pub use runtime::{
    EngineRuntime, InputEvent, LifecycleEvent, PlatformTier, RuntimeError,
    SafeAreaInsets, SurfaceDescriptor, SurfaceId, SurfaceRotation, SurfaceSize,
};

pub use driver_manager::{
    AllowUnsignedPackages, Capability, CapabilityBroker, DriverError, DriverLease,
    DriverManifest, DriverManager, DriverRecord, DriverRegistry, DriverState,
    PackageSignature, RequireSignedPackages, TrustPolicy,
};

pub use driver_catalog::{
    DriverCatalog, DriverCatalogError, PersistentDriverManager,
};

pub use settings::{
    SettingToggle, SettingsManager,
};
