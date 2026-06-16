//! Disk serialization for frozen tab state
//!
//! Provides persistent storage for tabs in Frozen state to achieve
//! near-zero memory footprint while maintaining instant restoration.

use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Serialized tab state for disk storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenTabState {
    /// Tab unique identifier
    pub tab_id: u64,
    /// Last known URL
    pub url: String,
    /// Compressed DOM snapshot
    pub dom_snapshot: Vec<u8>,
    /// Compressed render tree
    pub render_snapshot: Vec<u8>,
    /// Compressed V8 heap snapshot
    pub v8_snapshot: Vec<u8>,
    /// Scroll position (x, y)
    pub scroll_position: (f32, f32),
    /// Viewport dimensions (width, height)
    pub viewport_size: (u32, u32),
    /// Timestamp when frozen (seconds since epoch)
    pub frozen_at: u64,
    /// Size in bytes before compression
    pub original_size: usize,
    /// Size in bytes after compression
    pub compressed_size: usize,
}

impl FrozenTabState {
    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.compressed_size as f32 / self.original_size as f32
    }

    /// Estimate memory saved by freezing
    pub fn memory_saved(&self) -> usize {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

/// Disk storage manager for frozen tabs
pub struct DiskStorage {
    /// Base directory for storing frozen tabs
    storage_path: PathBuf,
    /// Maximum disk space to use (bytes)
    max_disk_usage: usize,
    /// Current disk usage (bytes)
    current_usage: usize,
}

impl DiskStorage {
    /// Create a new disk storage manager
    ///
    /// # Arguments
    /// * `storage_path` - Directory to store frozen tab data
    /// * `max_disk_usage` - Maximum disk space to use (default: 1GB)
    pub fn new<P: AsRef<Path>>(storage_path: P, max_disk_usage: usize) -> Result<Self, String> {
        let storage_path = storage_path.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_path)
            .map_err(|e| format!("Failed to create storage directory: {}", e))?;

        let mut storage = DiskStorage {
            storage_path,
            max_disk_usage,
            current_usage: 0,
        };

        // Calculate current disk usage
        storage.recalculate_usage()?;

        info!(
            "Disk storage initialized at {:?} (max: {} MB, current: {} MB)",
            storage.storage_path,
            max_disk_usage / 1024 / 1024,
            storage.current_usage / 1024 / 1024
        );

        Ok(storage)
    }

    /// Serialize and save a frozen tab to disk
    pub fn save_tab(&mut self, state: &FrozenTabState) -> Result<(), String> {
        let file_path = self.get_tab_path(state.tab_id);

        debug!("Saving frozen tab {} to {:?}", state.tab_id, file_path);

        // Serialize using bincode for efficient binary format
        let serialized = bincode::serialize(state)
            .map_err(|e| format!("Failed to serialize tab state: {}", e))?;

        let size = serialized.len();

        // Check if we have space
        if self.current_usage + size > self.max_disk_usage {
            return Err(format!(
                "Disk storage limit reached ({} MB / {} MB)",
                (self.current_usage + size) / 1024 / 1024,
                self.max_disk_usage / 1024 / 1024
            ));
        }

        // Write to disk
        let mut file =
            fs::File::create(&file_path).map_err(|e| format!("Failed to create file: {}", e))?;

        file.write_all(&serialized)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        self.current_usage += size;

        info!(
            "Saved frozen tab {} ({} KB, compression ratio: {:.2}%)",
            state.tab_id,
            size / 1024,
            state.compression_ratio() * 100.0
        );

        Ok(())
    }

    /// Load a frozen tab from disk
    pub fn load_tab(&self, tab_id: u64) -> Result<FrozenTabState, String> {
        let file_path = self.get_tab_path(tab_id);

        debug!("Loading frozen tab {} from {:?}", tab_id, file_path);

        // Read file
        let mut file =
            fs::File::open(&file_path).map_err(|e| format!("Failed to open file: {}", e))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Deserialize
        let state: FrozenTabState = bincode::deserialize(&buffer)
            .map_err(|e| format!("Failed to deserialize tab state: {}", e))?;

        info!("Loaded frozen tab {} ({} KB)", tab_id, buffer.len() / 1024);

        Ok(state)
    }

    /// Delete a frozen tab from disk
    pub fn delete_tab(&mut self, tab_id: u64) -> Result<(), String> {
        let file_path = self.get_tab_path(tab_id);

        if !file_path.exists() {
            return Ok(()); // Already deleted
        }

        // Get file size before deletion
        let size = fs::metadata(&file_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        fs::remove_file(&file_path).map_err(|e| format!("Failed to delete file: {}", e))?;

        self.current_usage = self.current_usage.saturating_sub(size);

        debug!("Deleted frozen tab {} ({} KB freed)", tab_id, size / 1024);

        Ok(())
    }

    /// Check if a tab exists on disk
    pub fn has_tab(&self, tab_id: u64) -> bool {
        self.get_tab_path(tab_id).exists()
    }

    /// List all frozen tabs
    pub fn list_tabs(&self) -> Result<Vec<u64>, String> {
        let mut tab_ids = Vec::new();

        let entries = fs::read_dir(&self.storage_path)
            .map_err(|e| format!("Failed to read storage directory: {}", e))?;

        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("tab_") && name.ends_with(".bin") {
                    if let Some(id_str) = name
                        .strip_prefix("tab_")
                        .and_then(|s| s.strip_suffix(".bin"))
                    {
                        if let Ok(id) = id_str.parse::<u64>() {
                            tab_ids.push(id);
                        }
                    }
                }
            }
        }

        Ok(tab_ids)
    }

    /// Get current disk usage statistics
    pub fn get_stats(&self) -> DiskStorageStats {
        DiskStorageStats {
            current_usage: self.current_usage,
            max_usage: self.max_disk_usage,
            usage_percentage: (self.current_usage as f32 / self.max_disk_usage as f32) * 100.0,
            tab_count: self.list_tabs().map(|t| t.len()).unwrap_or(0),
        }
    }

    /// Clear all frozen tabs from disk
    pub fn clear_all(&mut self) -> Result<usize, String> {
        let tab_ids = self.list_tabs()?;
        let count = tab_ids.len();

        for tab_id in tab_ids {
            self.delete_tab(tab_id)?;
        }

        info!("Cleared {} frozen tabs from disk", count);

        Ok(count)
    }

    /// Get file path for a tab
    fn get_tab_path(&self, tab_id: u64) -> PathBuf {
        self.storage_path.join(format!("tab_{}.bin", tab_id))
    }

    /// Recalculate current disk usage
    fn recalculate_usage(&mut self) -> Result<(), String> {
        self.current_usage = 0;

        let entries = fs::read_dir(&self.storage_path)
            .map_err(|e| format!("Failed to read storage directory: {}", e))?;

        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                self.current_usage += metadata.len() as usize;
            }
        }

        Ok(())
    }
}

impl Default for DiskStorage {
    fn default() -> Self {
        // Use system temp directory with reasonable default
        let storage_path = std::env::temp_dir().join("soliloquy_frozen_tabs");
        Self::new(storage_path, 1024 * 1024 * 1024) // 1GB default
            .expect("Failed to create default disk storage")
    }
}

/// Disk storage statistics
#[derive(Debug, Clone)]
pub struct DiskStorageStats {
    /// Current disk usage in bytes
    pub current_usage: usize,
    /// Maximum allowed disk usage in bytes
    pub max_usage: usize,
    /// Usage as percentage of maximum
    pub usage_percentage: f32,
    /// Number of frozen tabs on disk
    pub tab_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_frozen_tab_state() {
        let state = FrozenTabState {
            tab_id: 123,
            url: "https://example.com".to_string(),
            dom_snapshot: vec![1, 2, 3],
            render_snapshot: vec![4, 5, 6],
            v8_snapshot: vec![7, 8, 9],
            scroll_position: (0.0, 100.0),
            viewport_size: (1920, 1080),
            frozen_at: 1234567890,
            original_size: 1000000,
            compressed_size: 250000,
        };

        assert_eq!(state.compression_ratio(), 0.25);
        assert_eq!(state.memory_saved(), 750000);
    }

    #[test]
    fn test_disk_storage_save_load() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut storage =
            DiskStorage::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create storage");

        let state = FrozenTabState {
            tab_id: 456,
            url: "https://test.com".to_string(),
            dom_snapshot: vec![10; 1000],
            render_snapshot: vec![20; 1000],
            v8_snapshot: vec![30; 1000],
            scroll_position: (50.0, 200.0),
            viewport_size: (1280, 720),
            frozen_at: 1234567890,
            original_size: 5000000,
            compressed_size: 1000000,
        };

        // Save tab
        storage.save_tab(&state).expect("Failed to save tab");
        assert!(storage.has_tab(456));

        // Load tab
        let loaded = storage.load_tab(456).expect("Failed to load tab");
        assert_eq!(loaded.tab_id, 456);
        assert_eq!(loaded.url, "https://test.com");
        assert_eq!(loaded.dom_snapshot, vec![10; 1000]);

        // Delete tab
        storage.delete_tab(456).expect("Failed to delete tab");
        assert!(!storage.has_tab(456));
    }

    #[test]
    fn test_disk_storage_list_tabs() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut storage =
            DiskStorage::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create storage");

        // Save multiple tabs
        for i in 1..=5 {
            let state = FrozenTabState {
                tab_id: i,
                url: format!("https://test{}.com", i),
                dom_snapshot: vec![],
                render_snapshot: vec![],
                v8_snapshot: vec![],
                scroll_position: (0.0, 0.0),
                viewport_size: (1920, 1080),
                frozen_at: 1234567890,
                original_size: 1000,
                compressed_size: 500,
            };
            storage.save_tab(&state).expect("Failed to save tab");
        }

        let tabs = storage.list_tabs().expect("Failed to list tabs");
        assert_eq!(tabs.len(), 5);
        assert!(tabs.contains(&1));
        assert!(tabs.contains(&5));
    }

    #[test]
    fn test_disk_storage_stats() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut storage =
            DiskStorage::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create storage");

        let state = FrozenTabState {
            tab_id: 789,
            url: "https://stats.com".to_string(),
            dom_snapshot: vec![0; 10000],
            render_snapshot: vec![],
            v8_snapshot: vec![],
            scroll_position: (0.0, 0.0),
            viewport_size: (1920, 1080),
            frozen_at: 1234567890,
            original_size: 100000,
            compressed_size: 10000,
        };

        storage.save_tab(&state).expect("Failed to save tab");

        let stats = storage.get_stats();
        assert!(stats.current_usage > 0);
        assert_eq!(stats.tab_count, 1);
        assert!(stats.usage_percentage < 100.0);
    }

    #[test]
    fn test_disk_storage_clear_all() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut storage =
            DiskStorage::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create storage");

        // Save multiple tabs
        for i in 1..=3 {
            let state = FrozenTabState {
                tab_id: i,
                url: format!("https://clear{}.com", i),
                dom_snapshot: vec![],
                render_snapshot: vec![],
                v8_snapshot: vec![],
                scroll_position: (0.0, 0.0),
                viewport_size: (1920, 1080),
                frozen_at: 1234567890,
                original_size: 1000,
                compressed_size: 500,
            };
            storage.save_tab(&state).expect("Failed to save tab");
        }

        let cleared = storage.clear_all().expect("Failed to clear all");
        assert_eq!(cleared, 3);
        assert_eq!(storage.get_stats().tab_count, 0);
    }

    #[test]
    fn test_disk_storage_clear_all_empty() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut storage =
            DiskStorage::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create storage");

        let cleared = storage.clear_all().expect("Failed to clear all");
        assert_eq!(cleared, 0);
        assert_eq!(storage.get_stats().tab_count, 0);
    }

    #[test]
    fn test_disk_storage_clear_all_with_dummy_files() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let mut storage =
            DiskStorage::new(temp_dir.path(), 10 * 1024 * 1024).expect("Failed to create storage");

        // Save a valid tab
        let state = FrozenTabState {
            tab_id: 1,
            url: "https://clear1.com".to_string(),
            dom_snapshot: vec![],
            render_snapshot: vec![],
            v8_snapshot: vec![],
            scroll_position: (0.0, 0.0),
            viewport_size: (1920, 1080),
            frozen_at: 1234567890,
            original_size: 1000,
            compressed_size: 500,
        };
        storage.save_tab(&state).expect("Failed to save tab");

        // Create a dummy non-tab file
        let dummy_file_path = temp_dir.path().join("not_a_tab.txt");
        std::fs::write(&dummy_file_path, "dummy data").expect("Failed to write dummy file");

        // Create a dummy tab file with invalid id
        let dummy_tab_invalid_id = temp_dir.path().join("tab_invalid.bin");
        std::fs::write(&dummy_tab_invalid_id, "dummy data").expect("Failed to write invalid tab");

        // clear_all should only clear the valid tab
        let cleared = storage.clear_all().expect("Failed to clear all");
        assert_eq!(cleared, 1);
        assert_eq!(storage.get_stats().tab_count, 0);

        // Verify non-tab files still exist
        assert!(dummy_file_path.exists());
        assert!(dummy_tab_invalid_id.exists());
    }
}
