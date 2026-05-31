//! Soliloquy Shell Library
//!
//! Core library for the Soliloquy browser shell, providing:
//! - Servo browser engine integration
//! - V8 JavaScript runtime
//! - Desktop windowing integration
//! - High-performance networking with HTTP/3 and QUIC
//! - Resource loading and caching

#![allow(dead_code)]

// Core modules
pub mod browser_optimizations;
pub mod engine_bridge;
pub mod js_engine;
pub mod net;
pub mod optimizations;
pub mod servo_embedder;
pub mod v8_runtime;

// Re-export main types
pub use js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};
pub use net::{
    CodeCache, ConnectionManager, ResourceLoader, ResourceResponse, SpeculationEngine,
    SpeculationRules,
};
pub use servo_embedder::ServoEmbedder;
pub use v8_runtime::V8Runtime;
