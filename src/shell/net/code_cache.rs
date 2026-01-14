//! V8 Code Cache - Bytecode caching for faster script execution
//!
//! This module implements V8 code caching to skip parsing and compilation:
//! - Save compiled bytecode after first script execution
//! - Load cached bytecode on subsequent page loads
//! - LRU eviction to limit cache size
//! - SHA-256 hashing for cache keys

use log::{info, debug, warn};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache entry metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Script URL
    pub url: String,
    /// SHA-256 hash of script content
    pub content_hash: String,
    /// Path to cached bytecode file
    pub file_path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Last access timestamp (for LRU)
    pub last_accessed: u64,
}

/// V8 code cache for bytecode storage
pub struct CodeCache {
    /// Cache directory path
    cache_dir: PathBuf,
    /// Cache metadata: URL + hash -> entry
    entries: HashMap<String, CacheEntry>,
    /// Maximum cache size in bytes (default 100MB)
    max_cache_size: u64,
    /// Current cache size in bytes
    current_size: u64,
}

impl CodeCache {
    /// Create a new code cache
    ///
    /// # Arguments
    /// * `cache_dir` - Directory to store cached files
    ///
    /// # Returns
    /// * `Ok(CodeCache)` - Initialized cache
    /// * `Err(String)` - Failed to create cache directory
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Result<Self, String> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
            info!("Created code cache directory: {:?}", cache_dir);
        }

        let mut cache = Self {
            cache_dir,
            entries: HashMap::new(),
            max_cache_size: 100 * 1024 * 1024, // 100MB
            current_size: 0,
        };

        // Load existing cache entries
        cache.load_metadata()?;

        info!("Initialized V8 code cache ({} entries, {} bytes)", 
            cache.entries.len(), cache.current_size);

        Ok(cache)
    }

    /// Set maximum cache size in bytes
    pub fn set_max_size(&mut self, max_size: u64) {
        self.max_cache_size = max_size;
        info!("Set code cache max size to {} bytes", max_size);
    }

    /// Get cached bytecode for a script
    ///
    /// # Arguments
    /// * `url` - The script URL
    /// * `content_hash` - SHA-256 hash of script content
    ///
    /// # Returns
    /// * `Some(Vec<u8>)` - Cached bytecode
    /// * `None` - No cache entry found
    pub fn get(&mut self, url: &str, content_hash: &str) -> Option<Vec<u8>> {
        let key = self.make_key(url, content_hash);
        
        if let Some(entry) = self.entries.get_mut(&key) {
            debug!("Cache hit for: {} ({})", url, content_hash);
            
            // Update last accessed time
            entry.last_accessed = Self::current_timestamp();
            
            // Read bytecode from file
            match fs::read(&entry.file_path) {
                Ok(bytecode) => {
                    debug!("Loaded {} bytes of cached bytecode", bytecode.len());
                    return Some(bytecode);
                }
                Err(e) => {
                    warn!("Failed to read cache file {:?}: {}", entry.file_path, e);
                    // Remove invalid entry
                    self.entries.remove(&key);
                    return None;
                }
            }
        }

        debug!("Cache miss for: {} ({})", url, content_hash);
        None
    }

    /// Store bytecode in cache
    ///
    /// # Arguments
    /// * `url` - The script URL
    /// * `content_hash` - SHA-256 hash of script content
    /// * `bytecode` - V8 compiled bytecode
    ///
    /// # Returns
    /// * `Ok(())` - Successfully cached
    /// * `Err(String)` - Cache write failure
    pub fn put(&mut self, url: &str, content_hash: &str, bytecode: Vec<u8>) -> Result<(), String> {
        let key = self.make_key(url, content_hash);
        let size = bytecode.len() as u64;
        
        debug!("Caching bytecode for: {} ({} bytes)", url, size);

        // Check if we need to evict entries
        while self.current_size + size > self.max_cache_size && !self.entries.is_empty() {
            self.evict_lru()?;
        }

        // Generate filename
        let filename = format!("{}.bin", self.sanitize_filename(&key));
        let file_path = self.cache_dir.join(filename);

        // Write bytecode to file
        let mut file = fs::File::create(&file_path)
            .map_err(|e| format!("Failed to create cache file: {}", e))?;
        file.write_all(&bytecode)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        // Create cache entry
        let entry = CacheEntry {
            url: url.to_string(),
            content_hash: content_hash.to_string(),
            file_path: file_path.clone(),
            size,
            last_accessed: Self::current_timestamp(),
        };

        // Update metadata
        if let Some(old_entry) = self.entries.insert(key, entry) {
            // Replaced existing entry, adjust size
            self.current_size = self.current_size.saturating_sub(old_entry.size);
            // Delete old file
            let _ = fs::remove_file(old_entry.file_path);
        }
        
        self.current_size += size;

        info!("Cached bytecode for: {} (cache size: {} bytes)", url, self.current_size);
        self.save_metadata()?;

        Ok(())
    }

    /// Compute SHA-256 hash of script content
    pub fn compute_hash(content: &[u8]) -> String {
        // TODO: Use sha2 crate for actual SHA-256
        // For now, use simple placeholder
        format!("{:x}", content.len())
    }

    /// Evict least recently used entry
    fn evict_lru(&mut self) -> Result<(), String> {
        // Find LRU entry
        let lru_key = self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            if let Some(entry) = self.entries.remove(&key) {
                info!("Evicting LRU cache entry: {} ({} bytes)", entry.url, entry.size);
                
                // Delete file
                if let Err(e) = fs::remove_file(&entry.file_path) {
                    warn!("Failed to delete cache file {:?}: {}", entry.file_path, e);
                }
                
                self.current_size = self.current_size.saturating_sub(entry.size);
            }
        }

        Ok(())
    }

    /// Make cache key from URL and content hash
    fn make_key(&self, url: &str, content_hash: &str) -> String {
        format!("{}:{}", url, content_hash)
    }

    /// Sanitize filename for filesystem
    fn sanitize_filename(&self, name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Get current Unix timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Load cache metadata from disk
    fn load_metadata(&mut self) -> Result<(), String> {
        let metadata_path = self.cache_dir.join("metadata.json");
        
        if !metadata_path.exists() {
            debug!("No cache metadata found, starting fresh");
            return Ok(());
        }

        match fs::File::open(&metadata_path) {
            Ok(mut file) => {
                let mut contents = String::new();
                if let Err(e) = file.read_to_string(&mut contents) {
                    warn!("Failed to read metadata: {}", e);
                    return Ok(());
                }

                // TODO: Use serde_json to parse metadata
                // For now, just log that we found it
                debug!("Found cache metadata file");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to open metadata file: {}", e);
                Ok(())
            }
        }
    }

    /// Save cache metadata to disk
    fn save_metadata(&self) -> Result<(), String> {
        let metadata_path = self.cache_dir.join("metadata.json");
        
        // TODO: Use serde_json to serialize metadata
        // For now, just create empty file
        if let Err(e) = fs::File::create(&metadata_path) {
            warn!("Failed to save metadata: {}", e);
        }

        Ok(())
    }

    /// Clear all cached entries
    pub fn clear(&mut self) -> Result<(), String> {
        info!("Clearing code cache");
        
        // Delete all cached files
        for entry in self.entries.values() {
            if let Err(e) = fs::remove_file(&entry.file_path) {
                warn!("Failed to delete cache file {:?}: {}", entry.file_path, e);
            }
        }

        self.entries.clear();
        self.current_size = 0;
        self.save_metadata()?;

        info!("Code cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.entries.len(),
            total_size: self.current_size,
            max_size: self.max_cache_size,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cache entries
    pub entry_count: usize,
    /// Total size of cached data in bytes
    pub total_size: u64,
    /// Maximum cache size in bytes
    pub max_size: u64,
}

impl CacheStats {
    /// Calculate cache utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.total_size as f64 / self.max_size as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn get_test_cache_dir() -> PathBuf {
        let mut dir = env::temp_dir();
        dir.push(format!("soliloquy_test_cache_{}", std::process::id()));
        dir
    }

    fn cleanup_test_cache(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_code_cache_creation() {
        let cache_dir = get_test_cache_dir();
        let cache = CodeCache::new(&cache_dir);
        assert!(cache.is_ok());
        cleanup_test_cache(&cache_dir);
    }

    #[test]
    fn test_cache_put_get() {
        let cache_dir = get_test_cache_dir();
        let mut cache = CodeCache::new(&cache_dir).unwrap();
        
        let url = "https://example.com/script.js";
        let hash = "abc123";
        let bytecode = vec![1, 2, 3, 4, 5];
        
        // Put bytecode
        let result = cache.put(url, hash, bytecode.clone());
        assert!(result.is_ok());
        
        // Get bytecode
        let cached = cache.get(url, hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), bytecode);
        
        cleanup_test_cache(&cache_dir);
    }

    #[test]
    fn test_cache_miss() {
        let cache_dir = get_test_cache_dir();
        let mut cache = CodeCache::new(&cache_dir).unwrap();
        
        let cached = cache.get("https://example.com/script.js", "xyz789");
        assert!(cached.is_none());
        
        cleanup_test_cache(&cache_dir);
    }

    #[test]
    fn test_cache_eviction() {
        let cache_dir = get_test_cache_dir();
        let mut cache = CodeCache::new(&cache_dir).unwrap();
        cache.set_max_size(100); // Very small cache
        
        // Add entries that exceed cache size
        for i in 0..10 {
            let url = format!("https://example.com/script{}.js", i);
            let hash = format!("hash{}", i);
            let bytecode = vec![i as u8; 50]; // 50 bytes each
            
            let _ = cache.put(&url, &hash, bytecode);
        }
        
        // Cache should have evicted some entries
        assert!(cache.current_size <= cache.max_cache_size);
        
        cleanup_test_cache(&cache_dir);
    }

    #[test]
    fn test_cache_stats() {
        let cache_dir = get_test_cache_dir();
        let mut cache = CodeCache::new(&cache_dir).unwrap();
        
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
        
        // Add entry
        let _ = cache.put("https://example.com/script.js", "abc", vec![1, 2, 3]);
        
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.total_size, 3);
        
        cleanup_test_cache(&cache_dir);
    }

    #[test]
    fn test_cache_clear() {
        let cache_dir = get_test_cache_dir();
        let mut cache = CodeCache::new(&cache_dir).unwrap();
        
        // Add entries
        let _ = cache.put("https://example.com/script.js", "abc", vec![1, 2, 3]);
        assert_eq!(cache.entries.len(), 1);
        
        // Clear cache
        let result = cache.clear();
        assert!(result.is_ok());
        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.current_size, 0);
        
        cleanup_test_cache(&cache_dir);
    }

    #[test]
    fn test_compute_hash() {
        let content = b"console.log('Hello');";
        let hash = CodeCache::compute_hash(content);
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_cache_stats_utilization() {
        let stats = CacheStats {
            entry_count: 10,
            total_size: 50,
            max_size: 100,
        };
        assert_eq!(stats.utilization(), 50.0);
    }
}
