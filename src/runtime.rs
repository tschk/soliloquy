//! Shared runtime contracts for Soliloquy shell and RV8.
//!
//! These types define the common vocabulary for surfaces, input, and lifecycle
//! handling so the shell, browser engine, and future mobile entry points can
//! evolve together instead of diverging into separate ad hoc interfaces.

use serde::{Deserialize, Serialize};

/// High-level platform tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformTier {
    Desktop,
    ArmLinux,
    Mobile,
    Unknown,
}

/// Surface rotation in quarter turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SurfaceRotation {
    #[default]
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

/// Physical pixel size for a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

/// Insets reserved for notches, rounded corners, or system chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SafeAreaInsets {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

/// Stable identifier for a render surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SurfaceId(pub u64);

/// Description of a surface handed to the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceDescriptor {
    pub id: SurfaceId,
    pub size: SurfaceSize,
    pub scale_factor: f32,
    pub tier: PlatformTier,
    pub rotation: SurfaceRotation,
    pub safe_area: SafeAreaInsets,
    pub touch_enabled: bool,
    pub keyboard_enabled: bool,
}

impl SurfaceDescriptor {
    pub fn new(id: u64, width: u32, height: u32, tier: PlatformTier) -> Self {
        Self {
            id: SurfaceId(id),
            size: SurfaceSize { width, height },
            scale_factor: 1.0,
            tier,
            rotation: SurfaceRotation::Deg0,
            safe_area: SafeAreaInsets::default(),
            touch_enabled: matches!(tier, PlatformTier::Mobile | PlatformTier::ArmLinux),
            keyboard_enabled: true,
        }
    }
}

impl Default for SurfaceDescriptor {
    fn default() -> Self {
        Self::new(0, 1920, 1080, PlatformTier::Desktop)
    }
}

/// Lifecycle events surfaced to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleEvent {
    Starting,
    Resumed,
    Suspended,
    Backgrounded,
    Foregrounded,
    LowMemory,
    Shutdown,
}

/// Simplified input events shared by the shell and browser engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    Touch { x: f32, y: f32 },
    PointerMove { x: f32, y: f32 },
    Scroll { delta_x: f32, delta_y: f32 },
    Key { code: u32 },
    Text { value: String },
    Lifecycle(LifecycleEvent),
}

/// Error type for runtime contract operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    SurfaceNotFound(u64),
    InvalidSurface(String),
    Unsupported(String),
}

impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SurfaceNotFound(id) => write!(f, "surface {} not found", id),
            Self::InvalidSurface(msg) => write!(f, "invalid surface: {}", msg),
            Self::Unsupported(msg) => write!(f, "unsupported operation: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Shared runtime contract for the shell/engine boundary.
pub trait EngineRuntime {
    fn attach_surface(&self, surface: SurfaceDescriptor) -> Result<(), RuntimeError>;
    fn present_frame(&self, surface_id: SurfaceId) -> Result<(), RuntimeError>;
    fn handle_input(&self, event: InputEvent) -> Result<(), RuntimeError>;
    fn handle_lifecycle(&self, event: LifecycleEvent) -> Result<(), RuntimeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_surface_is_desktop_like() {
        let surface = SurfaceDescriptor::default();
        assert_eq!(surface.id, SurfaceId(0));
        assert_eq!(surface.size.width, 1920);
        assert!(surface.keyboard_enabled);
    }

    #[test]
    fn touch_is_enabled_for_mobile() {
        let surface = SurfaceDescriptor::new(7, 1080, 2400, PlatformTier::Mobile);
        assert!(surface.touch_enabled);
        assert_eq!(surface.tier, PlatformTier::Mobile);
    }
}
