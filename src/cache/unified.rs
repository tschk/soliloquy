//! Unified cache system for efficient resource management
//!
//! Provides tiered caching across memory, disk, and GPU for:
//! - Compiled shaders
//! - V8 bytecode
//! - Network resources
//! - GPU textures

use log::{debug, info};
use std::collections::HashMap;

/// Cache entry with cost/benefit tracking
#[derive(Debug, Clone)]
pub struct CachedResource<T> {
    /// The cached resource
    pub data: T,
    /// Size in bytes
    pub size: usize,
    /// Access count for LRU
    pub access_count: u64,
    /// Last access timestamp (seconds since epoch)
    pub last_access: u64,
    /// Cost to recreate (computation time in ms)
    pub recreation_cost: u32,
}

impl<T> CachedResource<T> {
    /// Create a new cached resource
    pub fn new(data: T, size: usize, recreation_cost: u32) -> Self {
        CachedResource {
            data,
            size,
            access_count: 1,
            last_access: current_timestamp(),
            recreation_cost,
        }
    }

    /// Update access statistics
    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_access = current_timestamp();
    }

    /// Calculate priority score for eviction (higher = keep longer)
    pub fn priority_score(&self) -> f64 {
        let age = current_timestamp().saturating_sub(self.last_access) as f64;
        let frequency = self.access_count as f64;
        let cost = self.recreation_cost as f64;

        // Formula: (frequency * cost) / (age + 1)
        // High frequency + high cost + recent access = high priority
        (frequency * cost) / (age + 1.0)
    }
}

/// Resource key for cache lookup
pub type ResourceKey = String;

/// LRU cache with intelligent eviction
pub struct LruCache<T> {
    /// Cached entries
    entries: HashMap<ResourceKey, CachedResource<T>>,
    /// Maximum memory usage in bytes
    max_size: usize,
    /// Current memory usage
    current_size: usize,
    /// Hit count
    hits: u64,
    /// Miss count
    misses: u64,
}

impl<T: Clone> LruCache<T> {
    /// Create a new LRU cache
    pub fn new(max_size: usize) -> Self {
        info!(
            "Creating LRU cache with max size: {} MB",
            max_size / 1024 / 1024
        );

        LruCache {
            entries: HashMap::new(),
            max_size,
            current_size: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Insert an entry into the cache
    pub fn insert(&mut self, key: ResourceKey, resource: CachedResource<T>) {
        // Evict if necessary
        while self.current_size + resource.size > self.max_size && !self.entries.is_empty() {
            self.evict_one();
        }

        self.current_size += resource.size;
        self.entries.insert(key, resource);
    }

    /// Get an entry from the cache
    pub fn get(&mut self, key: &str) -> Option<&T> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.touch();
            self.hits += 1;
            Some(&entry.data)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Get a mutable entry from the cache
    pub fn get_mut(&mut self, key: &str) -> Option<&mut T> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.touch();
            self.hits += 1;
            Some(&mut entry.data)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Remove an entry
    pub fn remove(&mut self, key: &str) -> Option<CachedResource<T>> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size = self.current_size.saturating_sub(entry.size);
            Some(entry)
        } else {
            None
        }
    }

    /// Evict one entry based on priority score
    ///
    /// Note: This uses O(n) linear search to find minimum priority.
    /// For production with many entries, consider using a min-heap (BinaryHeap with Reverse)
    /// to achieve O(log n) evictions.
    fn evict_one(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        // Find entry with lowest priority
        let key_to_evict = self
            .entries
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.priority_score()
                    .partial_cmp(&b.priority_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(k, _)| k.clone());

        if let Some(key) = key_to_evict {
            debug!("Evicting cache entry: {}", key);
            self.remove(&key);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            size: self.current_size,
            max_size: self.max_size,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                (self.hits as f64) / ((self.hits + self.misses) as f64)
            } else {
                0.0
            },
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_resource_creation() {
        let resource = CachedResource::new(vec![1, 2, 3], 100, 50);
        assert_eq!(resource.size, 100);
        assert_eq!(resource.recreation_cost, 50);
        assert_eq!(resource.access_count, 1);
    }

    #[test]
    fn test_cached_resource_touch() {
        let mut resource = CachedResource::new("data", 10, 10);
        let initial_count = resource.access_count;
        let initial_time = resource.last_access;

        std::thread::sleep(std::time::Duration::from_millis(10));
        resource.touch();

        assert_eq!(resource.access_count, initial_count + 1);
        assert!(resource.last_access >= initial_time);
    }

    #[test]
    fn test_lru_cache_insert_get() {
        let mut cache = LruCache::new(1024);

        cache.insert("key1".to_string(), CachedResource::new("value1", 100, 10));

        let value = cache.get("key1");
        assert_eq!(value, Some(&"value1"));
    }

    #[test]
    fn test_lru_cache_miss() {
        let mut cache: LruCache<String> = LruCache::new(1024);
        let value = cache.get("nonexistent");
        assert_eq!(value, None);
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(250); // Space for ~2 entries

        cache.insert("key1".to_string(), CachedResource::new("value1", 100, 10));
        cache.insert("key2".to_string(), CachedResource::new("value2", 100, 10));
        cache.insert("key3".to_string(), CachedResource::new("value3", 100, 10));

        // One entry should have been evicted
        assert!(cache.entries.len() <= 2);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = LruCache::new(1024);

        cache.insert("key1".to_string(), CachedResource::new("value1", 100, 10));
        cache.get("key1");
        cache.get("key2"); // Miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.0 && stats.hit_rate < 1.0);
    }

    #[test]
    fn test_priority_score() {
        let resource1 = CachedResource::new("data", 100, 100); // High cost
        let mut resource2 = CachedResource::new("data", 100, 10); // Low cost

        // resource2 with higher access count should have higher priority
        for _ in 0..10 {
            resource2.touch();
        }

        assert!(resource2.priority_score() > resource1.priority_score());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = LruCache::new(1024);

        cache.insert("key1".to_string(), CachedResource::new("value1", 100, 10));
        cache.insert("key2".to_string(), CachedResource::new("value2", 100, 10));

        cache.clear();

        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.current_size, 0);
    }
}
