//! V8 snapshot and bytecode caching for fast startup
//!
//! Implements snapshot serialization and bytecode caching to reduce
//! cold start time and improve performance for revisited pages.

use log::{debug, info};
use std::collections::HashMap;

/// V8 snapshot for fast isolate creation
#[derive(Debug, Clone)]
pub struct V8Snapshot {
    /// Snapshot data (serialized V8 heap)
    data: Vec<u8>,
    /// Snapshot size in bytes
    size: usize,
    /// Creation timestamp
    created_at: u64,
}

impl V8Snapshot {
    /// Create a new snapshot
    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len();
        info!("Created V8 snapshot ({} bytes)", size);

        V8Snapshot {
            data,
            size,
            created_at: current_timestamp(),
        }
    }

    /// Get snapshot data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get snapshot size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get age in seconds
    pub fn age(&self) -> u64 {
        current_timestamp().saturating_sub(self.created_at)
    }
}

/// Bytecode cache entry
#[derive(Debug, Clone)]
pub struct BytecodeCache {
    /// Compiled bytecode
    bytecode: Vec<u8>,
    /// Source hash for invalidation
    source_hash: u64,
    /// Last access timestamp
    last_access: u64,
    /// Access count
    access_count: u32,
}

impl BytecodeCache {
    /// Create a new bytecode cache entry
    pub fn new(bytecode: Vec<u8>, source_hash: u64) -> Self {
        BytecodeCache {
            bytecode,
            source_hash,
            last_access: current_timestamp(),
            access_count: 1,
        }
    }

    /// Get bytecode data
    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    /// Check if cache is valid for given source hash
    pub fn is_valid(&self, source_hash: u64) -> bool {
        self.source_hash == source_hash
    }

    /// Update access statistics
    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_access = current_timestamp();
    }
}

/// V8 bytecode cache manager
pub struct V8BytecodeCache {
    /// Cached bytecode indexed by script URL
    cache: HashMap<String, BytecodeCache>,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current cache size
    current_size: usize,
    /// Hit count
    hits: u64,
    /// Miss count
    misses: u64,
}

impl V8BytecodeCache {
    /// Create a new bytecode cache
    pub fn new(max_size: usize) -> Self {
        info!(
            "Creating V8 bytecode cache (max: {} MB)",
            max_size / 1024 / 1024
        );

        V8BytecodeCache {
            cache: HashMap::new(),
            max_size,
            current_size: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Store bytecode for a script
    pub fn store(&mut self, url: String, bytecode: Vec<u8>, source_hash: u64) {
        let size = bytecode.len();

        // Evict if necessary
        while self.current_size + size > self.max_size && !self.cache.is_empty() {
            self.evict_lru();
        }

        let entry = BytecodeCache::new(bytecode, source_hash);
        self.cache.insert(url.clone(), entry);
        self.current_size += size;

        debug!("Stored bytecode for {} ({} bytes)", url, size);
    }

    /// Retrieve bytecode for a script
    pub fn get(&mut self, url: &str, source_hash: u64) -> Option<&[u8]> {
        // Check if entry exists and is valid
        let is_valid = if let Some(entry) = self.cache.get(url) {
            entry.is_valid(source_hash)
        } else {
            self.misses += 1;
            return None;
        };

        if !is_valid {
            // Source changed, invalidate cache
            debug!("Bytecode cache invalidated for {} (hash mismatch)", url);
            self.cache.remove(url);
            self.misses += 1;
            return None;
        }

        // Update stats and return bytecode
        if let Some(entry) = self.cache.get_mut(url) {
            entry.touch();
            self.hits += 1;
            Some(entry.bytecode())
        } else {
            None
        }
    }

    /// Evict least recently used entry
    ///
    /// Note: This uses O(n) linear search to find LRU entry.
    /// For production with many entries, consider using a more efficient data structure
    /// like a combination of HashMap + doubly-linked list, or maintaining sorted order.
    fn evict_lru(&mut self) {
        let url_ptr = self
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(k, _)| k.as_str() as *const str);

        if let Some(ptr) = url_ptr {
            let url = unsafe { &*ptr };
            debug!("Evicting bytecode cache entry: {}", url);
            if let Some(entry) = self.cache.remove(url) {
                self.current_size = self.current_size.saturating_sub(entry.bytecode.len());
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
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

    /// Clear all cached bytecode
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size = 0;
        debug!("Cleared bytecode cache");
    }
}

impl Default for V8BytecodeCache {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024) // 64MB default
    }
}

/// Snapshot manager for V8 isolates
pub struct SnapshotManager {
    /// Cached snapshots indexed by context name
    snapshots: HashMap<String, V8Snapshot>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new() -> Self {
        SnapshotManager {
            snapshots: HashMap::new(),
        }
    }

    /// Store a snapshot
    pub fn store_snapshot(&mut self, name: String, snapshot: V8Snapshot) {
        info!("Storing V8 snapshot: {} ({} bytes)", name, snapshot.size());
        self.snapshots.insert(name, snapshot);
    }

    /// Retrieve a snapshot
    pub fn get_snapshot(&self, name: &str) -> Option<&V8Snapshot> {
        self.snapshots.get(name)
    }

    /// Remove a snapshot
    pub fn remove_snapshot(&mut self, name: &str) -> bool {
        self.snapshots.remove(name).is_some()
    }

    /// Get total snapshot count
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Get total snapshot size
    pub fn total_size(&self) -> usize {
        self.snapshots.values().map(|s| s.size()).sum()
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
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

/// Compute hash of source code using std::hash
pub fn hash_source(source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_snapshot_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let snapshot = V8Snapshot::new(data.clone());

        assert_eq!(snapshot.size(), 5);
        assert_eq!(snapshot.data(), &data[..]);
    }

    #[test]
    fn test_v8_snapshot_creation_empty() {
        let data: Vec<u8> = vec![];
        let snapshot = V8Snapshot::new(data.clone());

        assert_eq!(snapshot.size(), 0);
        assert!(snapshot.data().is_empty());
    }

    #[test]
    fn test_bytecode_cache_creation() {
        let bytecode = vec![1, 2, 3];
        let cache = BytecodeCache::new(bytecode.clone(), 12345);

        assert_eq!(cache.bytecode(), &bytecode[..]);
        assert!(cache.is_valid(12345));
        assert!(!cache.is_valid(54321));
    }

    #[test]
    fn test_bytecode_cache_creation_empty() {
        let bytecode: Vec<u8> = vec![];
        let cache = BytecodeCache::new(bytecode.clone(), 12345);

        assert!(cache.bytecode().is_empty());
        assert!(cache.is_valid(12345));
        assert!(!cache.is_valid(54321));
    }

    #[test]
    fn test_bytecode_cache_manager() {
        let mut manager = V8BytecodeCache::new(1024);

        let bytecode = vec![1, 2, 3, 4];
        let hash = hash_source("console.log('test')");

        manager.store("test.js".to_string(), bytecode.clone(), hash);

        let cached = manager.get("test.js", hash);
        assert_eq!(cached, Some(&bytecode[..]));
    }

    #[test]
    fn test_bytecode_cache_invalidation() {
        let mut manager = V8BytecodeCache::new(1024);

        let bytecode = vec![1, 2, 3];
        let hash1 = hash_source("code version 1");
        let hash2 = hash_source("code version 2");

        manager.store("test.js".to_string(), bytecode, hash1);

        // Different hash should return None
        let cached = manager.get("test.js", hash2);
        assert_eq!(cached, None);
    }

    #[test]
    fn test_bytecode_cache_stats() {
        let mut manager = V8BytecodeCache::new(1024);

        manager.store("test.js".to_string(), vec![1, 2, 3], 123);
        manager.get("test.js", 123); // Hit
        manager.get("missing.js", 456); // Miss

        let stats = manager.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_snapshot_manager() {
        let mut manager = SnapshotManager::new();

        let snapshot = V8Snapshot::new(vec![1, 2, 3, 4, 5]);
        manager.store_snapshot("default".to_string(), snapshot);

        assert_eq!(manager.snapshot_count(), 1);
        assert!(manager.get_snapshot("default").is_some());
    }

    #[test]
    fn test_snapshot_manager_remove() {
        let mut manager = SnapshotManager::new();

        let snapshot = V8Snapshot::new(vec![1, 2, 3]);
        manager.store_snapshot("test".to_string(), snapshot);

        assert!(manager.remove_snapshot("test"));
        assert_eq!(manager.snapshot_count(), 0);
    }

    #[test]
    fn test_snapshot_manager_total_size() {
        let mut manager = SnapshotManager::new();
        assert_eq!(manager.total_size(), 0);

        manager.store_snapshot("snap1".to_string(), V8Snapshot::new(vec![1, 2, 3, 4]));
        assert_eq!(manager.total_size(), 4);

        manager.store_snapshot("snap2".to_string(), V8Snapshot::new(vec![5, 6, 7, 8, 9]));
        assert_eq!(manager.total_size(), 9);
    }

    #[test]
    fn test_hash_source() {
        let hash1 = hash_source("console.log('hello')");
        let hash2 = hash_source("console.log('hello')");
        let hash3 = hash_source("console.log('world')");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
