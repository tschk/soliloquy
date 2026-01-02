//! Unified caching system for Soliloquy
//!
//! Provides tiered caching across memory, disk, and GPU.

pub mod unified;
pub mod texture_atlas;
pub mod disk_cache;

pub use unified::{LruCache, CachedResource, ResourceKey, CacheStats};
pub use texture_atlas::{
    TextureAtlas, TextureAtlasManager, TextureHandle, TextureEntry, AtlasRect,
};
pub use disk_cache::{DiskCache, DiskCacheEntry, DiskCacheStats};
