//! Soliloquy Browser Optimizations
//!
//! ponytail: stubs until rv8 merges runtime/memory/v8_integration/driver_manager.
//! Another agent handles rv8 linkages — these will become re-exports from rv8 crate.

pub mod runtime;
pub mod memory;
pub mod v8_integration;
pub mod driver_manager;

#[cfg(test)]
pub mod test_utils;
