//! GPU texture atlas for efficient texture management
//!
//! Packs multiple small textures into larger atlases to reduce
//! draw calls and memory overhead.

use log::{debug, info};
use std::collections::HashMap;

/// Rectangle representing a region in the atlas
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AtlasRect {
    /// Create a new atlas rect
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        AtlasRect {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the area of this rect
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Get normalized texture coordinates (0.0 to 1.0)
    pub fn normalized(&self, atlas_width: u32, atlas_height: u32) -> (f32, f32, f32, f32) {
        let x1 = self.x as f32 / atlas_width as f32;
        let y1 = self.y as f32 / atlas_height as f32;
        let x2 = (self.x + self.width) as f32 / atlas_width as f32;
        let y2 = (self.y + self.height) as f32 / atlas_height as f32;
        (x1, y1, x2, y2)
    }
}

/// Texture handle for accessing textures in the atlas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    /// Atlas index
    pub atlas_id: u32,
    /// Texture ID within the atlas
    pub texture_id: u32,
}

/// Texture atlas entry
#[derive(Debug, Clone)]
pub struct TextureEntry {
    /// Handle for this texture
    pub handle: TextureHandle,
    /// Region in the atlas
    pub rect: AtlasRect,
    /// Original texture dimensions
    pub original_width: u32,
    pub original_height: u32,
}

/// Simple rectangle packing algorithm
struct RectPacker {
    width: u32,
    height: u32,
    used_regions: Vec<AtlasRect>,
}

impl RectPacker {
    fn new(width: u32, height: u32) -> Self {
        RectPacker {
            width,
            height,
            used_regions: Vec::new(),
        }
    }

    /// Try to pack a rectangle into the atlas
    ///
    /// Note: This uses a naive O(w*h) grid search with 32-pixel steps.
    /// For large atlases (2048x2048), this could iterate over 4096 positions.
    /// For production use, consider more efficient packing algorithms like:
    /// - Shelf packing (rows with increasing heights)
    /// - Guillotine (recursive space subdivision)
    /// - MaxRects (best area fit)
    ///   These provide better space utilization and O(n) or O(n log n) performance.
    fn pack(&mut self, width: u32, height: u32) -> Option<AtlasRect> {
        // Try to find a spot using simple shelf algorithm
        for y in (0..self.height).step_by(32) {
            for x in (0..self.width).step_by(32) {
                let rect = AtlasRect::new(x, y, width, height);

                // Check if this position fits
                if x + width > self.width || y + height > self.height {
                    continue;
                }

                // Check for overlaps with existing regions
                let overlaps = self.used_regions.iter().any(|used| {
                    rect.x < used.x + used.width
                        && rect.x + rect.width > used.x
                        && rect.y < used.y + used.height
                        && rect.y + rect.height > used.y
                });

                if !overlaps {
                    self.used_regions.push(rect);
                    return Some(rect);
                }
            }
        }

        None
    }
}

/// Texture atlas for managing multiple textures
pub struct TextureAtlas {
    /// Atlas dimensions
    width: u32,
    height: u32,
    /// Atlas ID
    atlas_id: u32,
    /// Packer for this atlas
    packer: RectPacker,
    /// Stored textures
    textures: HashMap<u32, TextureEntry>,
    /// Next texture ID
    next_texture_id: u32,
}

impl TextureAtlas {
    /// Create a new texture atlas
    pub fn new(atlas_id: u32, width: u32, height: u32) -> Self {
        info!("Creating texture atlas {} ({}x{})", atlas_id, width, height);

        TextureAtlas {
            width,
            height,
            atlas_id,
            packer: RectPacker::new(width, height),
            textures: HashMap::new(),
            next_texture_id: 1,
        }
    }

    /// Add a texture to the atlas
    pub fn add_texture(&mut self, width: u32, height: u32) -> Option<TextureHandle> {
        // Try to pack the texture
        let rect = self.packer.pack(width, height)?;

        let texture_id = self.next_texture_id;
        self.next_texture_id += 1;

        let handle = TextureHandle {
            atlas_id: self.atlas_id,
            texture_id,
        };

        let entry = TextureEntry {
            handle,
            rect,
            original_width: width,
            original_height: height,
        };

        self.textures.insert(texture_id, entry);

        debug!(
            "Added texture {} to atlas {} at ({}, {})",
            texture_id, self.atlas_id, rect.x, rect.y
        );

        Some(handle)
    }

    /// Get texture entry by handle
    pub fn get_texture(&self, handle: TextureHandle) -> Option<&TextureEntry> {
        if handle.atlas_id != self.atlas_id {
            return None;
        }
        self.textures.get(&handle.texture_id)
    }

    /// Remove a texture from the atlas
    pub fn remove_texture(&mut self, handle: TextureHandle) -> bool {
        if handle.atlas_id != self.atlas_id {
            return false;
        }
        self.textures.remove(&handle.texture_id).is_some()
    }

    /// Get atlas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get texture count
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Calculate utilization percentage
    pub fn utilization(&self) -> f32 {
        let total_area = (self.width * self.height) as f32;
        let used_area: u32 = self.textures.values().map(|e| e.rect.area()).sum();
        (used_area as f32 / total_area) * 100.0
    }
}

/// Manager for multiple texture atlases
pub struct TextureAtlasManager {
    /// All atlases
    atlases: HashMap<u32, TextureAtlas>,
    /// Next atlas ID
    next_atlas_id: u32,
    /// Default atlas dimensions
    default_width: u32,
    default_height: u32,
}

impl TextureAtlasManager {
    /// Create a new texture atlas manager
    pub fn new(default_width: u32, default_height: u32) -> Self {
        info!(
            "Creating texture atlas manager (default size: {}x{})",
            default_width, default_height
        );

        TextureAtlasManager {
            atlases: HashMap::new(),
            next_atlas_id: 1,
            default_width,
            default_height,
        }
    }

    /// Add a texture, creating a new atlas if necessary
    pub fn add_texture(&mut self, width: u32, height: u32) -> Option<TextureHandle> {
        // Try existing atlases first
        for atlas in self.atlases.values_mut() {
            if let Some(handle) = atlas.add_texture(width, height) {
                return Some(handle);
            }
        }

        // Create new atlas
        let atlas_id = self.next_atlas_id;
        self.next_atlas_id += 1;

        let mut atlas = TextureAtlas::new(atlas_id, self.default_width, self.default_height);
        let handle = atlas.add_texture(width, height)?;

        self.atlases.insert(atlas_id, atlas);
        Some(handle)
    }

    /// Get texture entry by handle
    pub fn get_texture(&self, handle: TextureHandle) -> Option<&TextureEntry> {
        self.atlases.get(&handle.atlas_id)?.get_texture(handle)
    }

    /// Remove a texture
    pub fn remove_texture(&mut self, handle: TextureHandle) -> bool {
        if let Some(atlas) = self.atlases.get_mut(&handle.atlas_id) {
            atlas.remove_texture(handle)
        } else {
            false
        }
    }

    /// Get total texture count across all atlases
    pub fn total_textures(&self) -> usize {
        self.atlases.values().map(|a| a.texture_count()).sum()
    }

    /// Get atlas count
    pub fn atlas_count(&self) -> usize {
        self.atlases.len()
    }
}

impl Default for TextureAtlasManager {
    fn default() -> Self {
        Self::new(2048, 2048)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_rect() {
        let rect = AtlasRect::new(10, 20, 100, 50);
        assert_eq!(rect.area(), 5000);
    }

    #[test]
    fn test_atlas_rect_normalized() {
        let rect = AtlasRect::new(0, 0, 512, 512);
        let (x1, y1, x2, y2) = rect.normalized(1024, 1024);

        assert_eq!(x1, 0.0);
        assert_eq!(y1, 0.0);
        assert_eq!(x2, 0.5);
        assert_eq!(y2, 0.5);
    }

    #[test]
    fn test_texture_atlas_creation() {
        let atlas = TextureAtlas::new(1, 1024, 1024);
        assert_eq!(atlas.dimensions(), (1024, 1024));
        assert_eq!(atlas.texture_count(), 0);
    }

    #[test]
    fn test_add_texture() {
        let mut atlas = TextureAtlas::new(1, 1024, 1024);

        let handle = atlas.add_texture(256, 256);
        assert!(handle.is_some());

        let handle = handle.unwrap();
        assert_eq!(handle.atlas_id, 1);
        assert_eq!(atlas.texture_count(), 1);
    }

    #[test]
    fn test_get_texture() {
        let mut atlas = TextureAtlas::new(1, 1024, 1024);
        let handle = atlas.add_texture(256, 256).unwrap();

        let entry = atlas.get_texture(handle);
        assert!(entry.is_some());

        let entry = entry.unwrap();
        assert_eq!(entry.original_width, 256);
        assert_eq!(entry.original_height, 256);
    }

    #[test]
    fn test_remove_texture() {
        let mut atlas = TextureAtlas::new(1, 1024, 1024);
        let handle = atlas.add_texture(256, 256).unwrap();

        assert!(atlas.remove_texture(handle));
        assert_eq!(atlas.texture_count(), 0);
    }

    #[test]
    fn test_atlas_utilization() {
        let mut atlas = TextureAtlas::new(1, 1024, 1024);

        atlas.add_texture(512, 512);
        atlas.add_texture(256, 256);

        let util = atlas.utilization();
        assert!(util > 0.0 && util <= 100.0);
    }

    #[test]
    fn test_atlas_manager() {
        let mut manager = TextureAtlasManager::new(1024, 1024);

        let handle = manager.add_texture(256, 256);
        assert!(handle.is_some());

        assert_eq!(manager.atlas_count(), 1);
        assert_eq!(manager.total_textures(), 1);
    }

    #[test]
    fn test_atlas_manager_multiple_atlases() {
        let mut manager = TextureAtlasManager::new(512, 512);

        // Add many textures to force multiple atlases
        for _ in 0..10 {
            manager.add_texture(256, 256);
        }

        assert!(manager.atlas_count() >= 1);
        assert_eq!(manager.total_textures(), 10);
    }
}
