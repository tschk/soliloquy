//! RV8 Browser Engine
//!
//! A modern browser engine combining Servo's rendering with V8's JavaScript execution.
//! Designed with Chrome-like architecture and optimizations.
//!
//! ## Architecture
//!
//! RV8 uses a multi-process architecture similar to Chrome:
//!
//! - **Browser Process**: UI, profile management, navigation
//! - **Renderer Processes**: HTML/CSS parsing, layout, painting (sandboxed)
//! - **GPU Process**: Compositing and hardware acceleration
//! - **Network Process**: HTTP/HTTPS, caching, cookies
//!
//! ## Modules
//!
//! - [`core`] - Browser process and coordination
//! - [`renderer`] - Web content rendering (Servo-based)
//! - [`js`] - JavaScript execution (V8-based)
//! - [`compositor`] - GPU compositing and layer management
//! - [`networking`] - Network stack and resource loading
//! - [`storage`] - IndexedDB, cookies, cache
//! - [`ipc`] - Inter-process communication
//! - [`optimizations`] - Performance optimizations
//!
//! ## Example
//!
//! ```no_run
//! use rv8::{Browser, BrowserConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut browser = Browser::new(BrowserConfig::default()).await.expect("Failed to create browser");
//!     browser.new_tab("https://example.com").await.expect("Navigation failed");
//!     browser.run().await;
//! }
//! ```

// Core browser process
pub mod core;

// Rendering engine
pub mod renderer;

// JavaScript engine
pub mod js;

// GPU compositor
pub mod compositor;

// Network stack
pub mod networking;

// Storage subsystems
pub mod storage;

// Inter-process communication
pub mod ipc;

// Performance optimizations
pub mod optimizations;

// Servo embedding and V8 integration
pub mod servo_embed;

// Re-exports
pub use compositor::Compositor;
pub use core::{Browser, BrowserConfig, Tab, TabId};
pub use js::{JsEngine, JsValue};
pub use networking::{NetworkManager, Request, Response};
pub use optimizations::{OptimizationFlags, PerformanceMonitor};
pub use renderer::{RenderFrame, WebView};
pub use storage::{CookieJar, StorageManager};

/// RV8 version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// User agent string
pub fn user_agent() -> String {
    format!(
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) RV8/{} Chrome/120.0.0.0 Safari/537.36",
        VERSION
    )
}
