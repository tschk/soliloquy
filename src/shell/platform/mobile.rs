//! Mobile Linux platform windowing scaffolding.
//!
//! This is a lightweight placeholder backend for Linux mobile targets such as
//! postmarketOS, Ubuntu Touch, and Phosh-based devices. It keeps the platform
//! contract intact while the actual native surface bridge is brought up.

use log::info;

use super::{GraphicsConfig, Window};

/// Mobile window placeholder.
pub struct MobileWindow {
    width: u32,
    height: u32,
    should_close: bool,
    graphics_config: GraphicsConfig,
}

impl Window for MobileWindow {
    fn new() -> Result<Self, String> {
        Self::new_with_size(1920, 1080)
    }

    fn new_with_size(width: u32, height: u32) -> Result<Self, String> {
        info!(
            "Creating Linux mobile surface placeholder ({}x{})",
            width, height
        );

        Ok(Self {
            width,
            height,
            should_close: false,
            graphics_config: GraphicsConfig::default(),
        })
    }

    fn present(&self) -> Result<(), String> {
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_redraw(&self) {}
}

impl MobileWindow {
    pub fn graphics_config(&self) -> &GraphicsConfig {
        &self.graphics_config
    }
}
