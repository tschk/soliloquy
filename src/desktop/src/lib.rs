//! Soliloquy Desktop Environment Daemon
//!
//! The DE daemon runs as a background service (dinit on Alpenglow),
//! consumes OS daemon state from alpenglow, and exposes an HTTP API
//! for the Svelte UI to query and control the desktop environment.

#![allow(dead_code)]

pub mod app_registry;
pub mod session;
pub mod status;

pub use app_registry::{AppEntry, AppInstance, AppRegistry, AppState};
pub use session::SessionManager;
pub use status::{AppInfo, DesktopStatus, SessionInfo};


