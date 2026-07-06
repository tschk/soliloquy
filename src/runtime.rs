// ponytail: stub — replace with rv8::optimizations::runtime when rv8 linkage lands
// Another agent handles real rv8 integration.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceDescriptor {
    pub id: SurfaceId,
    pub width: u32,
    pub height: u32,
    pub platform_tier: PlatformTier,
}

impl SurfaceDescriptor {
    pub fn new(id: u32, width: u32, height: u32, platform_tier: PlatformTier) -> Self {
        Self { id: SurfaceId(id as u64), width, height, platform_tier }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformTier {
    Desktop,
    Mobile,
    ArmLinux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Suspended,
    Foregrounded,
    Backgrounded,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseMove(f64, f64),
    MouseClick(u32, f64, f64),
    KeyPress(String),
    KeyRelease(String),
    Touch(u32, f64, f64),
    Scroll(f64, f64),
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    InvalidSurface(String),
    SurfaceNotFound(u64),
    Unsupported(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::InvalidSurface(msg) => write!(f, "invalid surface: {msg}"),
            RuntimeError::SurfaceNotFound(id) => write!(f, "surface {id} not found"),
            RuntimeError::Unsupported(msg) => write!(f, "unsupported: {msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

pub trait EngineRuntime: Send + Sync {
    fn attach_surface(&self, surface: SurfaceDescriptor) -> Result<(), RuntimeError>;
    fn present_frame(&self, surface_id: SurfaceId) -> Result<(), RuntimeError>;
    fn handle_input(&self, event: InputEvent) -> Result<(), RuntimeError>;
    fn handle_lifecycle(&self, event: LifecycleEvent) -> Result<(), RuntimeError>;
}
