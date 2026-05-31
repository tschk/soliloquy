//! V8 JavaScript engine optimizations
//!
//! Provides snapshot caching, bytecode caching, and idle-time GC scheduling.

pub mod gc_scheduler;
pub mod snapshots;

pub use gc_scheduler::{GcScheduler, GcStats, GcStrategy, GcType};
pub use snapshots::{hash_source, BytecodeCache, SnapshotManager, V8BytecodeCache, V8Snapshot};
