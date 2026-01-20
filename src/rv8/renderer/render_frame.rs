//! Render frame representation

/// A rendered frame ready for compositing
#[derive(Debug, Clone)]
pub struct RenderFrame {
    /// Frame ID
    pub id: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel data (RGBA)
    pub pixels: Vec<u8>,
    /// Scroll offset X
    pub scroll_x: f32,
    /// Scroll offset Y
    pub scroll_y: f32,
}

impl RenderFrame {
    pub fn new(width: u32, height: u32) -> Self {
        RenderFrame {
            id: 0,
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }
}
