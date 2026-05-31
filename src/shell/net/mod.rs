//! Networking module for Soliloquy browser
//!
//! This module provides high-performance networking capabilities including:
//! - HTTP/3 with QUIC transport
//! - Connection pooling and DNS caching
//! - Resource loading with redirects
//! - Speculation rules for prefetch/prerender
//! - V8 code caching for faster script execution

pub mod code_cache;
pub mod connection_manager;
pub mod quic;
pub mod resource_loader;
pub mod speculation;

// Re-export main types for convenience
pub use code_cache::CodeCache;
pub use connection_manager::ConnectionManager;
pub use quic::{QuicConfig, QuicTransport};
pub use resource_loader::{ResourceLoader, ResourceResponse};
pub use speculation::{SpeculationEngine, SpeculationRules};
