//! GPU rendering and layout computation subsystem
//!
//! Provides GPU-accelerated layout computation and damage-based compositing
//! for efficient rendering, with real WGPU device integration.

pub mod compositor;
pub mod layout_compute;
pub mod wgpu_integration;

pub use compositor::{
    CompositorLayer, DamageRect, DamageTracker, DisplayListCache, DisplayListEntry, RenderCommand,
};
pub use layout_compute::{style_flags, GpuLayoutCompute, LayoutNode, LayoutParams};
pub use wgpu_integration::WgpuContext;
