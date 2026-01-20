//! Core browser process and coordination
//!
//! The browser process is the main process that:
//! - Manages the UI (tabs, address bar, menus)
//! - Coordinates renderer processes
//! - Handles navigation and security
//! - Manages profiles and settings

mod browser;
mod config;
mod navigation;
mod process_manager;
mod tab;

pub use browser::Browser;
pub use config::BrowserConfig;
pub use navigation::{NavigationController, NavigationEntry};
pub use process_manager::ProcessManager;
pub use tab::{Tab, TabId, TabState};
