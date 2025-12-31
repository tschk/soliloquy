//! Memory management subsystem for Soliloquy
//!
//! Implements tiered tab residency system for efficient memory usage
//! enabling 150+ tabs with <3GB RAM.

pub mod residency;
pub mod compression;
pub mod pressure;

pub use residency::{
    ResidencyState, TabResidency, TabResidencyManager, TabSnapshot, TabStats,
};
pub use compression::{compress, decompress, estimate_compression_ratio, DataType};
pub use pressure::{MemoryPressureLevel, MemoryPressureMonitor, SystemMemoryInfo};
