//! Memory management subsystem for Soliloquy
//!
//! Implements tiered tab residency system for efficient memory usage
//! enabling 150+ tabs with <3GB RAM.

pub mod compression;
pub mod disk_storage;
pub mod pressure;
pub mod residency;

pub use compression::{compress, decompress, estimate_compression_ratio, DataType};
pub use disk_storage::{DiskStorage, DiskStorageStats, FrozenTabState};
pub use pressure::{MemoryPressureLevel, MemoryPressureMonitor, SystemMemoryInfo};
pub use residency::{ResidencyState, TabResidency, TabResidencyManager, TabSnapshot, TabStats};
