//! Platform-specific implementations
//!
//! Provides windowing and graphics abstractions for different platforms.
//! This module allows Soliloquy to run on:
//! - Linux (via winit + wgpu)
//! - macOS (via winit + wgpu)
//! - Fuchsia (via Flatland compositor)

#[cfg(all(target_os = "linux", feature = "desktop"))]
pub mod linux;

#[cfg(all(target_os = "macos", feature = "desktop"))]
pub mod macos;

#[cfg(feature = "fuchsia")]
pub mod fuchsia;

// Re-export the appropriate Window implementation based on platform
#[cfg(all(target_os = "linux", feature = "desktop"))]
pub use linux::LinuxWindow as NativeWindow;

#[cfg(all(target_os = "macos", feature = "desktop"))]
pub use macos::MacOSWindow as NativeWindow;

#[cfg(feature = "fuchsia")]
pub use fuchsia::ZirconWindow as NativeWindow;

/// Platform-agnostic window trait
///
/// All platform-specific window implementations must implement this trait
/// to provide a consistent API for the embedder.
pub trait Window: Sized {
    /// Create a new window with default dimensions (1920x1080)
    fn new() -> Result<Self, String>;

    /// Create a new window with specified dimensions
    fn new_with_size(width: u32, height: u32) -> Result<Self, String>;

    /// Present the current frame to the display
    fn present(&self) -> Result<(), String>;

    /// Resize the window
    fn resize(&mut self, width: u32, height: u32);

    /// Get current window dimensions
    fn size(&self) -> (u32, u32);

    /// Check if the window should close
    fn should_close(&self) -> bool;

    /// Request a redraw
    fn request_redraw(&self);
}

/// Window events that can be received from the platform
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window was resized
    Resized { width: u32, height: u32 },
    /// User requested window close
    CloseRequested,
    /// Window needs to be redrawn
    RedrawRequested,
    /// Mouse/touch input
    MouseInput {
        x: f32,
        y: f32,
        button: MouseButton,
        pressed: bool,
    },
    /// Keyboard input
    KeyboardInput {
        key: u32,
        pressed: bool,
        modifiers: KeyModifiers,
    },
    /// Mouse moved
    MouseMoved { x: f32, y: f32 },
    /// Mouse scroll
    Scroll { delta_x: f32, delta_y: f32 },
    /// Window gained focus
    Focused,
    /// Window lost focus
    Unfocused,
}

/// Mouse button identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Command on macOS, Windows key on Windows/Linux
}

/// Graphics backend configuration
#[derive(Debug, Clone)]
pub struct GraphicsConfig {
    /// Enable VSync
    pub vsync: bool,
    /// Preferred backend (Vulkan, Metal, DX12, OpenGL)
    pub preferred_backend: Option<GraphicsBackend>,
    /// MSAA sample count (1 = disabled)
    pub msaa_samples: u32,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        GraphicsConfig {
            vsync: true,
            preferred_backend: None, // Auto-detect
            msaa_samples: 1,
        }
    }
}

/// Graphics backend options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackend {
    Vulkan,
    Metal,
    DX12,
    OpenGL,
}

/// Stub implementation for when no platform is available
#[cfg(not(any(
    all(target_os = "linux", feature = "desktop"),
    all(target_os = "macos", feature = "desktop"),
    feature = "fuchsia"
)))]
pub struct StubWindow;

#[cfg(not(any(
    all(target_os = "linux", feature = "desktop"),
    all(target_os = "macos", feature = "desktop"),
    feature = "fuchsia"
)))]
impl Window for StubWindow {
    fn new() -> Result<Self, String> {
        Ok(StubWindow)
    }

    fn new_with_size(_width: u32, _height: u32) -> Result<Self, String> {
        Ok(StubWindow)
    }

    fn present(&self) -> Result<(), String> {
        Ok(())
    }

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn size(&self) -> (u32, u32) {
        (1920, 1080)
    }

    fn should_close(&self) -> bool {
        false
    }

    fn request_redraw(&self) {}
}

#[cfg(not(any(
    all(target_os = "linux", feature = "desktop"),
    all(target_os = "macos", feature = "desktop"),
    feature = "fuchsia"
)))]
pub type NativeWindow = StubWindow;
