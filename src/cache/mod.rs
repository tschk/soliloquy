//! Unified caching system for Soliloquy
//!
//! Provides tiered caching across memory, disk, and GPU.

pub mod disk_cache;
pub mod texture_atlas;
pub mod unified;

pub use disk_cache::{DiskCache, DiskCacheEntry, DiskCacheStats};
pub use texture_atlas::{
    AtlasRect, TextureAtlas, TextureAtlasManager, TextureEntry, TextureHandle,
};
pub use unified::{CacheStats, CachedResource, LruCache, ResourceKey};
