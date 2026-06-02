//! Damage-based compositor for efficient rendering
//!
//! Implements tile-based rendering with damage region tracking and occlusion culling.

use log::{debug, info};
use std::collections::HashSet;

/// Rectangular damage region
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DamageRect {
    /// Create a new damage rect
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        DamageRect {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if this rect intersects another
    pub fn intersects(&self, other: &DamageRect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Get the union of two rects
    pub fn union(&self, other: &DamageRect) -> DamageRect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);

        DamageRect {
            x,
            y,
            width: x2 - x,
            height: y2 - y,
        }
    }

    /// Get area in pixels
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

/// Compositor layer
#[derive(Debug, Clone)]
pub struct CompositorLayer {
    pub texture_id: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
    pub z_index: f32,
}

impl CompositorLayer {
    /// Create a new layer
    pub fn new(texture_id: u32, x: f32, y: f32, width: f32, height: f32) -> Self {
        CompositorLayer {
            texture_id,
            x,
            y,
            width,
            height,
            opacity: 1.0,
            z_index: 0.0,
        }
    }

    /// Check if layer is opaque
    pub fn is_opaque(&self) -> bool {
        self.opacity >= 1.0
    }

    /// Get layer bounds as damage rect
    pub fn bounds(&self) -> DamageRect {
        DamageRect {
            x: self.x as u32,
            y: self.y as u32,
            width: self.width as u32,
            height: self.height as u32,
        }
    }
}

/// Damage tracker for efficient incremental rendering
pub struct DamageTracker {
    /// Current frame damage regions
    current_damage: Vec<DamageRect>,
    /// Viewport dimensions
    viewport_width: u32,
    viewport_height: u32,
    /// Tile size for tile-based rendering
    tile_size: u32,
}

impl DamageTracker {
    /// Create a new damage tracker
    pub fn new(viewport_width: u32, viewport_height: u32, tile_size: u32) -> Self {
        info!(
            "Initializing damage tracker ({}x{}, tile size: {})",
            viewport_width, viewport_height, tile_size
        );

        DamageTracker {
            current_damage: Vec::new(),
            viewport_width,
            viewport_height,
            tile_size,
        }
    }

    /// Add a damage region
    pub fn add_damage(&mut self, rect: DamageRect) {
        debug!(
            "Adding damage: {}x{} at ({},{})",
            rect.width, rect.height, rect.x, rect.y
        );
        self.current_damage.push(rect);
    }

    /// Get all damaged tiles for this frame
    pub fn get_damaged_tiles(&self) -> HashSet<(u32, u32)> {
        let mut tiles = HashSet::new();

        for damage in &self.current_damage {
            // Calculate tile range for this damage rect
            let start_tile_x = damage.x / self.tile_size;
            let start_tile_y = damage.y / self.tile_size;
            let end_tile_x = (damage.x + damage.width).div_ceil(self.tile_size);
            let end_tile_y = (damage.y + damage.height).div_ceil(self.tile_size);

            for ty in start_tile_y..end_tile_y {
                for tx in start_tile_x..end_tile_x {
                    tiles.insert((tx, ty));
                }
            }
        }

        tiles
    }

    /// Clear damage for next frame
    pub fn clear_damage(&mut self) {
        self.current_damage.clear();
    }

    /// Get merged damage region (union of all damage)
    pub fn get_merged_damage(&self) -> Option<DamageRect> {
        if self.current_damage.is_empty() {
            return None;
        }

        let mut merged = self.current_damage[0];
        for damage in &self.current_damage[1..] {
            merged = merged.union(damage);
        }

        Some(merged)
    }

    /// Update viewport size
    pub fn set_viewport_size(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
        // Mark entire viewport as damaged on resize
        self.add_damage(DamageRect::new(0, 0, width, height));
    }
}

/// Display list entry for cached rendering
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayListEntry {
    pub layer_id: u32,
    pub command: RenderCommand,
}

/// Rendering commands
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommand {
    DrawRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    },
    DrawTexture {
        texture_id: u32,
        src_rect: DamageRect,
        dst_rect: DamageRect,
    },
    DrawText {
        text: String,
        x: f32,
        y: f32,
        font_size: f32,
    },
}

/// Display list cache with incremental diffing
pub struct DisplayListCache {
    /// Cached display list from previous frame
    cached_list: Vec<DisplayListEntry>,
    /// Current display list being built
    current_list: Vec<DisplayListEntry>,
}

impl DisplayListCache {
    /// Create a new display list cache
    pub fn new() -> Self {
        DisplayListCache {
            cached_list: Vec::new(),
            current_list: Vec::new(),
        }
    }

    /// Start building a new display list
    pub fn begin(&mut self) {
        self.current_list.clear();
    }

    /// Add an entry to the current display list
    pub fn add_entry(&mut self, entry: DisplayListEntry) {
        self.current_list.push(entry);
    }

    /// Finish building and compute differences
    pub fn finish(&mut self) -> Vec<usize> {
        let min_len = self.current_list.len().min(self.cached_list.len());
        let max_len = self.current_list.len().max(self.cached_list.len());

        let mut changed_indices = Vec::new();

        // Simple diff: find entries that changed
        for (i, (current, cached)) in self
            .current_list
            .iter()
            .zip(self.cached_list.iter())
            .enumerate()
        {
            if current != cached {
                changed_indices.push(i);
            }
        }

        if min_len < max_len {
            changed_indices.extend(min_len..max_len);
        }

        // Update cache
        std::mem::swap(&mut self.cached_list, &mut self.current_list);

        debug!(
            "Display list diff: {} entries changed",
            changed_indices.len()
        );
        changed_indices
    }
}

impl Default for DisplayListCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_rect_intersection() {
        let r1 = DamageRect::new(0, 0, 100, 100);
        let r2 = DamageRect::new(50, 50, 100, 100);
        let r3 = DamageRect::new(200, 200, 100, 100);

        assert!(r1.intersects(&r2));
        assert!(!r1.intersects(&r3));
    }

    #[test]
    fn test_damage_rect_union() {
        let r1 = DamageRect::new(0, 0, 50, 50);
        let r2 = DamageRect::new(25, 25, 50, 50);
        let union = r1.union(&r2);

        assert_eq!(union.x, 0);
        assert_eq!(union.y, 0);
        assert_eq!(union.width, 75);
        assert_eq!(union.height, 75);
    }

    #[test]
    fn test_damage_tracker() {
        let mut tracker = DamageTracker::new(1920, 1080, 64);

        tracker.add_damage(DamageRect::new(0, 0, 100, 100));
        tracker.add_damage(DamageRect::new(200, 200, 100, 100));

        let tiles = tracker.get_damaged_tiles();
        assert!(!tiles.is_empty());
    }

    #[test]
    fn test_damage_tracker_clear() {
        let mut tracker = DamageTracker::new(1920, 1080, 64);

        tracker.add_damage(DamageRect::new(0, 0, 100, 100));
        tracker.clear_damage();

        let tiles = tracker.get_damaged_tiles();
        assert!(tiles.is_empty());
    }

    #[test]
    fn test_merged_damage() {
        let mut tracker = DamageTracker::new(1920, 1080, 64);

        tracker.add_damage(DamageRect::new(0, 0, 50, 50));
        tracker.add_damage(DamageRect::new(100, 100, 50, 50));

        let merged = tracker.get_merged_damage().unwrap();
        assert_eq!(merged.x, 0);
        assert_eq!(merged.y, 0);
        assert_eq!(merged.width, 150);
        assert_eq!(merged.height, 150);
    }

    #[test]
    fn test_compositor_layer() {
        let layer = CompositorLayer::new(1, 0.0, 0.0, 800.0, 600.0);
        assert!(layer.is_opaque());
        assert_eq!(layer.bounds().width, 800);
    }

    #[test]
    fn test_display_list_cache() {
        let mut cache = DisplayListCache::new();

        cache.begin();
        cache.add_entry(DisplayListEntry {
            layer_id: 1,
            command: RenderCommand::DrawRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
                color: [1.0, 0.0, 0.0, 1.0],
            },
        });

        let changed = cache.finish();
        assert_eq!(changed.len(), 1);
    }
}
