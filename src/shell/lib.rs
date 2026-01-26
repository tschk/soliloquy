//! Soliloquy Shell - Desktop environment with Servo rendering and V8 JavaScript
//!
//! This library provides the core components for the Soliloquy desktop shell:
//!
//! - **Platform abstraction** - Unified windowing across Linux, macOS, and Fuchsia
//! - **Servo embedding** - Web content rendering via Servo browser engine
//! - **V8 runtime** - JavaScript execution replacing SpiderMonkey
//! - **Engine bridge** - Integration layer between Servo and V8
//! - **Optimizations** - Performance tuning for V8 and rendering
//!
//! ## Feature Flags
//!
//! - `desktop` - Enable Linux/macOS windowing with winit + wgpu
//! - `fuchsia` - Enable Fuchsia OS support with Flatland compositor
//!
//! ## Example
//!
//! ```no_run
//! use soliloquy_shell::servo_embedder::ServoEmbedder;
//!
//! let mut embedder = ServoEmbedder::new().expect("Failed to create embedder");
//! embedder.load_url("https://example.com").expect("Failed to load URL");
//! ```

// Core modules
pub mod engine_bridge;
pub mod optimizations;
pub mod servo_embedder;
pub mod v8_runtime;
pub mod zircon_window;
pub mod net;