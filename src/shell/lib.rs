//! Soliloquy Shell Library
//!
//! Core library for the Soliloquy browser shell, providing:
//! - Servo browser engine integration
//! - V8 JavaScript runtime
//! - Desktop windowing integration
//! - High-performance networking with HTTP/3 and QUIC
//! - Resource loading and caching

// Core modules
pub mod browser_optimizations;
pub mod engine_bridge;
pub mod js_engine;
pub mod optimizations;
pub mod servo_embedder;
pub mod v8_runtime;
pub mod net;

// Re-export main types
pub use servo_embedder::ServoEmbedder;
pub use js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};
pub use v8_runtime::V8Runtime;
pub use net::{
    CodeCache, ConnectionManager, ResourceLoader, ResourceResponse,
    SpeculationEngine, SpeculationRules,
};
