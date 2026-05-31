use std::collections::VecDeque;
use std::time::{Duration, Instant};

use log::{debug, info};

use super::{GraphicsBackend, GraphicsConfig, Window, WindowEvent};

pub struct MobileWindow {
    width: u32,
    height: u32,
    scale_factor: f32,
    should_close: bool,
    graphics_config: GraphicsConfig,
    pending_events: VecDeque<WindowEvent>,
    last_presented: Option<Instant>,
}

impl Window for MobileWindow {
    fn new() -> Result<Self, String> {
        Self::new_with_size(default_width(), default_height())
    }

    fn new_with_size(width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("mobile surface dimensions must be non-zero".to_string());
        }

        info!("Creating Linux mobile surface ({}x{})", width, height);

        Ok(Self {
            width,
            height,
            scale_factor: default_scale_factor(),
            should_close: false,
            graphics_config: mobile_graphics_config(),
            pending_events: VecDeque::new(),
            last_presented: None,
        })
    }

    fn present(&self) -> Result<(), String> {
        debug!(
            "Presenting Linux mobile surface at {}x{} @{}x",
            self.width, self.height, self.scale_factor
        );
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.pending_events
            .push_back(WindowEvent::Resized { width, height });
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_redraw(&self) {
        debug!("Mobile redraw requested");
    }
}

impl MobileWindow {
    pub fn graphics_config(&self) -> &GraphicsConfig {
        &self.graphics_config
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn mark_presented(&mut self) {
        self.last_presented = Some(Instant::now());
    }

    pub fn elapsed_since_present(&self) -> Option<Duration> {
        self.last_presented.map(|instant| instant.elapsed())
    }

    pub fn queue_event(&mut self, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            self.should_close = true;
        }
        self.pending_events.push_back(event);
    }

    pub fn poll_events(&mut self) -> Vec<WindowEvent> {
        self.pending_events.drain(..).collect()
    }
}

fn default_width() -> u32 {
    parse_u32_env("SOLILOQUY_MOBILE_WIDTH").unwrap_or(1080)
}

fn default_height() -> u32 {
    parse_u32_env("SOLILOQUY_MOBILE_HEIGHT").unwrap_or(2400)
}

fn default_scale_factor() -> f32 {
    std::env::var("SOLILOQUY_MOBILE_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(2.0)
}

fn parse_u32_env(name: &str) -> Option<u32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
}

fn mobile_graphics_config() -> GraphicsConfig {
    GraphicsConfig {
        vsync: true,
        preferred_backend: Some(GraphicsBackend::OpenGL),
        msaa_samples: 1,
    }
}
