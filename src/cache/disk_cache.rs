//! Disk-backed cache using sled database
//!
//! Provides persistent caching for resources that can be reused across sessions.

use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Disk cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskCacheEntry {
    /// Resource key
    pub key: String,
    /// Size in bytes
    pub size: usize,
    /// Access count
    pub access_count: u64,
    /// Last access timestamp
    pub last_access: u64,
    /// Content type/category
    pub content_type: String,
}

/// Disk-backed cache using sled embedded database
pub struct DiskCache {
    /// Sled database instance
    db: sled::Db,
    /// Current cache size in bytes
    current_size: usize,
    /// Maximum cache size
    max_size: usize,
    /// Hit count
    hits: u64,
    /// Miss count
    misses: u64,
}

impl DiskCache {
    /// Create a new disk cache
    ///
    /// # Arguments
    /// * `path` - Directory for the database
    /// * `max_size` - Maximum cache size in bytes
    pub fn new<P: AsRef<Path>>(path: P, max_size: usize) -> Result<Self, String> {
        let db = sled::open(path.as_ref())
            .map_err(|e| format!("Failed to open sled database: {}", e))?;

        let mut cache = DiskCache {
            db,
            current_size: 0,
            max_size,
            hits: 0,
            misses: 0,
        };

        // Calculate current size
        cache.recalculate_size()?;

        info!(
            "Disk cache initialized at {:?} (max: {} MB, current: {} MB)",
            path.as_ref(),
            max_size / 1024 / 1024,
            cache.current_size / 1024 / 1024
        );

        Ok(cache)
    }

    /// Insert or update a cache entry
    pub fn insert(&mut self, key: &str, data: &[u8], content_type: &str) -> Result<(), String> {
        if data.len() > self.max_size {
            return Err("Resource too large for cache".to_string());
        }

        let size = data.len();

        // Check if we need to evict
        while self.current_size + size > self.max_size && !self.db.is_empty() {
            let size_before = self.current_size;
            self.evict_one()?;
            if self.current_size == size_before {
                break;
            }
        }

        if self.current_size + size > self.max_size {
            return Err("Cache is full and cannot evict more entries".to_string());
        }

        // Create metadata
        let metadata = DiskCacheEntry {
            key: key.to_string(),
            size,
            access_count: 1,
            last_access: current_timestamp(),
            content_type: content_type.to_string(),
        };

        let metadata_key = format!("meta:{}", key);
        let metadata_bytes = bincode::serialize(&metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        // Insert data and metadata
        self.db
            .insert(key.as_bytes(), data)
            .map_err(|e| format!("Failed to insert data: {}", e))?;

        self.db
            .insert(metadata_key.as_bytes(), metadata_bytes)
            .map_err(|e| format!("Failed to insert metadata: {}", e))?;

        self.current_size += size;

        debug!("Cached {} ({} bytes, type: {})", key, size, content_type);

        Ok(())
    }

    /// Retrieve a cache entry
    pub fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let result = self
            .db
            .get(key.as_bytes())
            .map_err(|e| format!("Failed to get data: {}", e))?;

        if let Some(data) = result {
            self.hits += 1;

            // Update metadata
            self.update_access(key)?;

            debug!("Cache hit: {}", key);
            Ok(Some(data.to_vec()))
        } else {
            self.misses += 1;
            debug!("Cache miss: {}", key);
            Ok(None)
        }
    }

    /// Check if a key exists in cache
    pub fn contains(&self, key: &str) -> bool {
        self.db.contains_key(key.as_bytes()).unwrap_or(false)
    }

    /// Remove an entry from cache
    pub fn remove(&mut self, key: &str) -> Result<(), String> {
        if let Some(data) = self
            .db
            .remove(key.as_bytes())
            .map_err(|e| format!("Failed to remove data: {}", e))?
        {
            let size = data.len();
            self.current_size = self.current_size.saturating_sub(size);

            // Remove metadata
            let metadata_key = format!("meta:{}", key);
            self.db
                .remove(metadata_key.as_bytes())
                .map_err(|e| format!("Failed to remove metadata: {}", e))?;

            debug!("Removed {} from cache ({} bytes freed)", key, size);
        }

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> Result<(), String> {
        self.db
            .clear()
            .map_err(|e| format!("Failed to clear database: {}", e))?;

        self.current_size = 0;
        self.hits = 0;
        self.misses = 0;

        info!("Cache cleared");

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> DiskCacheStats {
        DiskCacheStats {
            current_size: self.current_size,
            max_size: self.max_size,
            usage_percentage: (self.current_size as f32 / self.max_size as f32) * 100.0,
            entry_count: self.count_entries(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f32 / (self.hits + self.misses) as f32
            } else {
                0.0
            },
        }
    }

    /// Flush to disk
    pub fn flush(&self) -> Result<(), String> {
        self.db
            .flush()
            .map_err(|e| format!("Failed to flush database: {}", e))?;
        Ok(())
    }

    /// Update access metadata for a key
    fn update_access(&mut self, key: &str) -> Result<(), String> {
        let metadata_key = format!("meta:{}", key);

        if let Some(meta_bytes) = self
            .db
            .get(metadata_key.as_bytes())
            .map_err(|e| format!("Failed to get metadata: {}", e))?
        {
            let mut metadata: DiskCacheEntry = bincode::deserialize(&meta_bytes)
                .map_err(|e| format!("Failed to deserialize metadata: {}", e))?;

            metadata.access_count += 1;
            metadata.last_access = current_timestamp();

            let updated_bytes = bincode::serialize(&metadata)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

            self.db
                .insert(metadata_key.as_bytes(), updated_bytes)
                .map_err(|e| format!("Failed to update metadata: {}", e))?;
        }

        Ok(())
    }

    /// Evict one entry (LRU)
    fn evict_one(&mut self) -> Result<(), String> {
        let mut oldest_key: Option<String> = None;
        let mut oldest_time = u64::MAX;

        // Find entry with oldest access time
        for item in self.db.iter() {
            if let Ok((key, value)) = item {
                if key.starts_with(b"meta:") {
                    if let Ok(metadata) = bincode::deserialize::<DiskCacheEntry>(&value) {
                        if metadata.last_access < oldest_time {
                            oldest_time = metadata.last_access;
                            oldest_key = Some(metadata.key.clone());
                        }
                    }
                }
            }
        }

        if let Some(key) = oldest_key {
            self.remove(&key)?;
            debug!("Evicted cache entry: {}", key);
        }

        Ok(())
    }

    /// Recalculate current cache size
    fn recalculate_size(&mut self) -> Result<(), String> {
        self.current_size = 0;

        for item in self.db.iter() {
            if let Ok((key, value)) = item {
                // Only count data entries, not metadata
                if !key.starts_with(b"meta:") {
                    self.current_size += value.len();
                }
            }
        }

        Ok(())
    }

    /// Count total entries
    fn count_entries(&self) -> usize {
        self.db
            .iter()
            .filter_map(|item| item.ok())
            .filter(|(key, _)| !key.starts_with(b"meta:"))
            .count()
    }
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Disk cache statistics
#[derive(Debug, Clone)]
pub struct DiskCacheStats {
    /// Current cache size in bytes
    pub current_size: usize,
    /// Maximum cache size in bytes
    pub max_size: usize,
    /// Usage as percentage
    pub usage_percentage: f32,
    /// Number of entries
    pub entry_count: usize,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_disk_cache_insert_get() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache =
            DiskCache::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create cache");

        let data = b"Hello, disk cache!";
        cache
            .insert("test_key", data, "text/plain")
            .expect("Failed to insert");

        let retrieved = cache.get("test_key").expect("Failed to get");
        assert_eq!(retrieved, Some(data.to_vec()));
    }

    #[test]
    fn test_disk_cache_insert_too_large() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache = DiskCache::new(temp_dir.path(), 100).expect("Failed to create cache");

        let data = vec![0u8; 150];
        let result = cache.insert("too_large", &data, "application/octet-stream");

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Resource too large for cache");
    }

    #[test]
    fn test_disk_cache_insert_full_error() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache = DiskCache::new(temp_dir.path(), 100).expect("Failed to create cache");

        // Simulate corrupted state: insert data directly without metadata
        cache.db.insert(b"orphan_key", vec![0u8; 60]).unwrap();
        cache.current_size += 60;

        // Now try to insert 50 bytes. 60 + 50 = 110 > 100.
        // evict_one will be called but won't find any meta: keys to remove.
        let data = vec![0u8; 50];
        let result = cache.insert("key1", &data, "application/octet-stream");

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cache is full and cannot evict more entries"
        );
    }

    #[test]
    fn test_disk_cache_miss() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache =
            DiskCache::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create cache");

        let result = cache.get("nonexistent").expect("Failed to get");
        assert_eq!(result, None);
    }

    #[test]
    fn test_disk_cache_remove() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache =
            DiskCache::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create cache");

        cache
            .insert("remove_me", b"data", "test")
            .expect("Failed to insert");
        assert!(cache.contains("remove_me"));

        cache.remove("remove_me").expect("Failed to remove");
        assert!(!cache.contains("remove_me"));
    }

    #[test]
    fn test_disk_cache_eviction() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache = DiskCache::new(temp_dir.path(), 1024).expect("Failed to create cache");

        // Fill cache
        for i in 0..10 {
            let data = vec![i; 200];
            cache
                .insert(&format!("key_{}", i), &data, "test")
                .expect("Failed to insert");
        }

        // Should have evicted older entries
        assert!(cache.current_size <= 1024);
    }

    #[test]
    fn test_disk_cache_stats() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache =
            DiskCache::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create cache");

        cache
            .insert("key1", b"data1", "test")
            .expect("Failed to insert");
        cache.get("key1").expect("Failed to get");
        cache.get("nonexistent").expect("Failed to get");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_disk_cache_clear() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut cache =
            DiskCache::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create cache");

        cache
            .insert("key1", b"data1", "test")
            .expect("Failed to insert");
        cache
            .insert("key2", b"data2", "test")
            .expect("Failed to insert");

        cache.clear().expect("Failed to clear");
        assert_eq!(cache.stats().entry_count, 0);
        assert_eq!(cache.current_size, 0);
    }
}
