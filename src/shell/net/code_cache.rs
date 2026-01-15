//! V8 Code Cache Implementation
//!
//! Provides persistent caching of compiled V8 bytecode to speed up JavaScript execution.
//! Uses SHA-256 hashing for cache validation and LRU eviction for size management.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use log::{debug, warn, error};

/// Maximum cache size in bytes (100 MB default)
const DEFAULT_MAX_CACHE_SIZE: u64 = 100 * 1024 * 1024;

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Original script URL
    pub url: String,
    /// SHA-256 hash of source content
    pub content_hash: String,
    /// Path to cached bytecode file
    pub file_path: PathBuf,
    /// Size of cached file in bytes
    pub size: u64,
    /// Last access timestamp (Unix epoch)
    pub last_accessed: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(url: String, content_hash: String, file_path: PathBuf, size: u64) -> Self {
        Self {
            url,
            content_hash,
            file_path,
            size,
            last_accessed: current_timestamp(),
        }
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = current_timestamp();
    }
}

/// V8 code cache manager
pub struct CodeCache {
    /// Directory for cache storage
    cache_dir: PathBuf,
    /// Map of URL -> CacheEntry
    entries: HashMap<String, CacheEntry>,
    /// Maximum total cache size
    max_cache_size: u64,
    /// Current total size
    current_size: u64,
    /// Metadata file path
    metadata_path: PathBuf,
}

impl CodeCache {
    /// Create a new code cache instance
    ///
    /// # Arguments
    /// * `cache_dir` - Directory to store cached bytecode
    /// * `max_cache_size` - Maximum cache size in bytes (None = default 100MB)
    ///
    /// # Returns
    /// Result containing CodeCache or error message
    pub fn new(cache_dir: PathBuf, max_cache_size: Option<u64>) -> Result<Self, String> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let metadata_path = cache_dir.join("metadata.json");
        let max_cache_size = max_cache_size.unwrap_or(DEFAULT_MAX_CACHE_SIZE);

        let mut cache = Self {
            cache_dir,
            entries: HashMap::new(),
            max_cache_size,
            current_size: 0,
            metadata_path,
        };

        // Load existing metadata
        cache.load_metadata()?;

        Ok(cache)
    }

    /// Get cached bytecode for a URL
    ///
    /// # Arguments
    /// * `url` - Script URL
    /// * `source_code` - Source code for validation
    ///
    /// # Returns
    /// Option containing cached bytecode if valid
    pub fn get(&mut self, url: &str, source_code: &str) -> Option<Vec<u8>> {
        let content_hash = Self::compute_hash(source_code);
        
        if let Some(entry) = self.entries.get_mut(url) {
            // Verify hash matches
            if entry.content_hash == content_hash {
                // Read cached file
                match fs::read(&entry.file_path) {
                    Ok(data) => {
                        entry.touch();
                        debug!("Cache hit for {}", url);
                        return Some(data);
                    }
                    Err(e) => {
                        warn!("Failed to read cache file for {}: {}", url, e);
                        self.entries.remove(url);
                    }
                }
            } else {
                debug!("Cache invalidated for {} (hash mismatch)", url);
                self.remove_entry(url);
            }
        }

        debug!("Cache miss for {}", url);
        None
    }

    /// Store compiled bytecode in cache
    ///
    /// # Arguments
    /// * `url` - Script URL
    /// * `source_code` - Original source code
    /// * `bytecode` - Compiled V8 bytecode
    ///
    /// # Returns
    /// Result indicating success or error
    pub fn put(&mut self, url: &str, source_code: &str, bytecode: &[u8]) -> Result<(), String> {
        let content_hash = Self::compute_hash(source_code);
        let cache_filename = format!("{}.cache", sanitize_filename(url));
        let file_path = self.cache_dir.join(&cache_filename);
        let size = bytecode.len() as u64;

        // Ensure we have space
        while self.current_size + size > self.max_cache_size && !self.entries.is_empty() {
            self.evict_lru()?;
        }

        // Write bytecode to file
        fs::write(&file_path, bytecode)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        // Remove old entry if exists
        if self.entries.contains_key(url) {
            self.remove_entry(url);
        }

        // Add new entry
        let entry = CacheEntry::new(url.to_string(), content_hash, file_path, size);
        self.current_size += size;
        self.entries.insert(url.to_string(), entry);

        debug!("Cached bytecode for {} ({} bytes)", url, size);

        // Save metadata
        self.save_metadata()?;

        Ok(())
    }

    /// Compute SHA-256 hash of content
    ///
    /// # Arguments
    /// * `content` - Content to hash
    ///
    /// # Returns
    /// Hex-encoded SHA-256 hash
    pub fn compute_hash(content: &str) -> String {
        // TODO: Replace placeholder with actual SHA-256 using sha2 crate
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Evict least recently used entry
    fn evict_lru(&mut self) -> Result<(), String> {
        let lru_url = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(url, _)| url.clone());

        if let Some(url) = lru_url {
            debug!("Evicting LRU entry: {}", url);
            self.remove_entry(&url);
        }

        Ok(())
    }

    /// Remove a cache entry
    fn remove_entry(&mut self, url: &str) {
        if let Some(entry) = self.entries.remove(url) {
            self.current_size = self.current_size.saturating_sub(entry.size);
            if let Err(e) = fs::remove_file(&entry.file_path) {
                warn!("Failed to remove cache file {:?}: {}", entry.file_path, e);
            }
        }
    }

    /// Clear entire cache
    pub fn clear(&mut self) -> Result<(), String> {
        for url in self.entries.keys().cloned().collect::<Vec<_>>() {
            self.remove_entry(&url);
        }
        self.entries.clear();
        self.current_size = 0;
        self.save_metadata()?;
        debug!("Cache cleared");
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

    /// Load metadata from disk
    fn load_metadata(&mut self) -> Result<(), String> {
        if !self.metadata_path.exists() {
            return Ok(());
        }

        let data = fs::read_to_string(&self.metadata_path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        // TODO: Use serde_json to parse metadata
        let entries: Vec<CacheEntry> = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse metadata: {}", e))?;

        self.entries.clear();
        self.current_size = 0;

        for entry in entries {
            if entry.file_path.exists() {
                self.current_size += entry.size;
                self.entries.insert(entry.url.clone(), entry);
            }
        }

        debug!("Loaded {} cache entries", self.entries.len());
        Ok(())
    }

    /// Save metadata to disk
    fn save_metadata(&self) -> Result<(), String> {
        let entries: Vec<&CacheEntry> = self.entries.values().collect();
        
        // TODO: Use serde_json to serialize metadata
        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        fs::write(&self.metadata_path, json)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;

        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_size: u64,
    pub max_size: u64,
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Sanitize filename by replacing invalid characters
fn sanitize_filename(url: &str) -> String {
    url.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .take(200) // Limit length
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_test_cache() -> CodeCache {
        let temp_dir = env::temp_dir().join(format!("soliloquy_cache_test_{}", current_timestamp()));
        CodeCache::new(temp_dir, Some(1024 * 1024)).unwrap()
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = setup_test_cache();
        let result = cache.get("https://example.com/script.js", "console.log('test');");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_put_and_get() {
        let mut cache = setup_test_cache();
        let url = "https://example.com/script.js";
        let source = "console.log('test');";
        let bytecode = vec![1, 2, 3, 4, 5];

        cache.put(url, source, &bytecode).unwrap();
        let result = cache.get(url, source);
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), bytecode);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = setup_test_cache();
        let url = "https://example.com/script.js";
        let source1 = "console.log('test');";
        let source2 = "console.log('modified');";
        let bytecode = vec![1, 2, 3, 4, 5];

        cache.put(url, source1, &bytecode).unwrap();
        
        // Different source should invalidate cache
        let result = cache.get(url, source2);
        assert!(result.is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let temp_dir = env::temp_dir().join(format!("soliloquy_cache_test_{}", current_timestamp()));
        let mut cache = CodeCache::new(temp_dir, Some(100)).unwrap();

        // Add entries that exceed cache size
        cache.put("url1", "code1", &vec![0; 40]).unwrap();
        cache.put("url2", "code2", &vec![0; 40]).unwrap();
        cache.put("url3", "code3", &vec![0; 40]).unwrap();

        // First entry should be evicted
        assert!(cache.entries.len() <= 2);
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = CodeCache::compute_hash("test");
        let hash2 = CodeCache::compute_hash("test");
        let hash3 = CodeCache::compute_hash("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 hex length
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = setup_test_cache();
        cache.put("url1", "code1", &vec![0; 100]).unwrap();
        cache.put("url2", "code2", &vec![0; 200]).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.total_size, 300);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = setup_test_cache();
        cache.put("url1", "code1", &vec![0; 100]).unwrap();
        cache.put("url2", "code2", &vec![0; 200]).unwrap();

        cache.clear().unwrap();
        
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("https://example.com/script.js"), "https___example.com_script.js");
        assert_eq!(sanitize_filename("test<>?:file"), "test___file");
    }

    #[test]
    fn test_metadata_persistence() {
        let temp_dir = env::temp_dir().join(format!("soliloquy_cache_test_{}", current_timestamp()));
        
        {
            let mut cache = CodeCache::new(temp_dir.clone(), Some(1024 * 1024)).unwrap();
            cache.put("url1", "code1", &vec![0; 100]).unwrap();
        }

        // Reload cache
        let cache = CodeCache::new(temp_dir, Some(1024 * 1024)).unwrap();
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key("url1"));
    }
}
