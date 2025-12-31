//! Zircon kernel integration for Soliloquy
//!
//! Provides zero-copy memory sharing, channel-based IPC,
//! and capability-based security isolation.

pub mod vmo;
pub mod channels;
pub mod isolation;

pub use vmo::{ZirconVmo, MappedMemory, ZirconTabMemory};
pub use channels::{ChannelMessage, ChannelEndpoint, TabChannelManager, create_channel};
pub use isolation::{
    Capability, PermissionStatus, TabIsolation, IsolationManager,
};
