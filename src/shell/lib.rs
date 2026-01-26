//! Soliloquy Shell Library
//!
//! Core library for the Soliloquy browser shell, providing:
//! - Servo browser engine integration
//! - V8 JavaScript runtime
//! - Fuchsia UI integration (Flatland compositor, Views, Input)
//! - High-performance networking with HTTP/3 and QUIC
//! - Resource loading and caching

// Core modules
pub mod engine_bridge;
pub mod optimizations;
pub mod servo_embedder;
pub mod v8_runtime;
pub mod zircon_window;
pub mod net;

// Re-export main types
pub use servo_embedder::ServoEmbedder;
pub use v8_runtime::V8Runtime;
pub use zircon_window::ZirconWindow;
pub use net::{
    CodeCache, ConnectionManager, QuicConfig, QuicTransport, ResourceLoader, ResourceResponse,
    SpeculationEngine, SpeculationRules,
};
