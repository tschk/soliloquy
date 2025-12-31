//! V8 JavaScript engine optimizations
//!
//! Provides snapshot caching, bytecode caching, and idle-time GC scheduling.

pub mod snapshots;
pub mod gc_scheduler;

pub use snapshots::{
    V8Snapshot, V8BytecodeCache, SnapshotManager, BytecodeCache, hash_source,
};
pub use gc_scheduler::{GcScheduler, GcType, GcStrategy, GcStats};
