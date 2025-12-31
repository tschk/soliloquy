//! GPU rendering and layout computation subsystem
//!
//! Provides GPU-accelerated layout computation and damage-based compositing
//! for efficient rendering.

pub mod layout_compute;
pub mod compositor;

pub use layout_compute::{LayoutNode, LayoutParams, GpuLayoutCompute, style_flags};
pub use compositor::{
    DamageRect, DamageTracker, CompositorLayer, DisplayListCache,
    DisplayListEntry, RenderCommand,
};
