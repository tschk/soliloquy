//! macOS platform windowing implementation using winit
//!
//! Provides a window and graphics context for macOS using:
//! - winit for window creation and event handling
//! - wgpu with Metal backend for GPU rendering

use log::{debug, info};

use super::{GraphicsConfig, Window};

/// macOS window implementation using winit
///
/// This implementation mirrors LinuxWindow but with macOS-specific
/// optimizations and Metal backend preference.
pub struct MacOSWindow {
    width: u32,
    height: u32,
    should_close: bool,
    graphics_config: GraphicsConfig,
}

impl Window for MacOSWindow {
    fn new() -> Result<Self, String> {
        Self::new_with_size(1920, 1080)
    }

    fn new_with_size(width: u32, height: u32) -> Result<Self, String> {
        info!("Creating macOS window ({}x{})", width, height);

        // Configure for Metal backend by default on macOS
        let mut config = GraphicsConfig::default();
        config.preferred_backend = Some(super::GraphicsBackend::Metal);

        Ok(MacOSWindow {
            width,
            height,
            should_close: false,
            graphics_config: config,
        })
    }

    fn present(&self) -> Result<(), String> {
        debug!("Presenting frame on macOS");
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        debug!("Resizing macOS window to {}x{}", width, height);
        self.width = width;
        self.height = height;
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_redraw(&self) {
        // Will be implemented with winit integration
    }
}

impl MacOSWindow {
    /// Check if running on Apple Silicon
    pub fn is_apple_silicon() -> bool {
        #[cfg(target_arch = "aarch64")]
        return true;

        #[cfg(not(target_arch = "aarch64"))]
        return false;
    }

    /// Get the graphics config
    pub fn graphics_config(&self) -> &GraphicsConfig {
        &self.graphics_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_window_creation() {
        let window = MacOSWindow::new();
        assert!(window.is_ok());

        let window = window.unwrap();
        assert_eq!(window.size(), (1920, 1080));
    }
}
