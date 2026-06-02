//! Tab Residency Manager - Tiered Memory Management System
//!
//! Implements aggressive memory optimization inspired by bodegaOS to enable
//! 150+ tabs with <3GB RAM usage through tiered state transitions.
//!
//! State transitions:
//! - Active (0-30s idle): Full rendering, all buffers allocated
//! - Warm (30s-5min): Compressed snapshot, quick restore (<100ms)
//! - Cold (5min-15min): Minimal footprint, slower restore
//! - Frozen (>15min): Serialized to disk, near-zero memory

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Residency state for a browser tab
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidencyState {
    /// Full rendering active, all GPU buffers allocated, DOM materialized
    Active,
    /// Compressed snapshot in memory, quick restore (<100ms)
    /// GPU buffers deallocated, DOM compressed with zstd
    Warm,
    /// Minimal memory footprint, slower restore (200-500ms)
    /// Only essential metadata retained
    Cold,
    /// Serialized to disk, near-zero memory footprint
    /// Full restore required (500ms-2s)
    Frozen,
}

/// Memory snapshot of tab state for quick restoration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TabSnapshot {
    /// Compressed DOM state (zstd level 3 for speed/compression balance)
    pub dom_snapshot: Option<Vec<u8>>,
    /// Compressed render tree state
    pub render_snapshot: Option<Vec<u8>>,
    /// V8 heap snapshot for JavaScript state
    pub v8_heap_snapshot: Option<Vec<u8>>,
    /// Scroll position for viewport restoration
    pub scroll_position: (f32, f32),
    /// Viewport dimensions
    pub viewport_size: (u32, u32),
}

/// Tab residency tracking information
pub struct TabResidency {
    /// Current residency state
    pub state: ResidencyState,
    /// Tab identifier (unique across all tabs)
    pub tab_id: u64,
    /// URL of the page (for reload if needed)
    pub url: String,
    /// Last user interaction timestamp
    pub last_interaction: Instant,
    /// Compressed snapshot for Warm/Cold states
    pub snapshot: Option<TabSnapshot>,
    /// GPU buffer handles (empty when deallocated)
    pub gpu_buffer_handles: Vec<u32>,
    /// Estimated memory usage in bytes
    pub memory_usage: usize,
    /// Disk storage path for Frozen state
    pub frozen_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FrozenTabState {
    tab_id: u64,
    url: String,
    snapshot: Option<TabSnapshot>,
    memory_usage: usize,
}

impl TabResidency {
    /// Create a new tab residency tracker in Active state
    pub fn new(tab_id: u64, url: String) -> Self {
        TabResidency {
            state: ResidencyState::Active,
            tab_id,
            url,
            last_interaction: Instant::now(),
            snapshot: None,
            gpu_buffer_handles: Vec::new(),
            memory_usage: 0,
            frozen_path: None,
        }
    }

    /// Update last interaction timestamp
    pub fn touch(&mut self) {
        self.last_interaction = Instant::now();
    }

    /// Get time since last interaction
    pub fn idle_duration(&self) -> Duration {
        self.last_interaction.elapsed()
    }

    /// Check if tab should be evicted to next state
    pub fn should_evict(&self) -> bool {
        let idle = self.idle_duration();
        match self.state {
            ResidencyState::Active => idle > Duration::from_secs(30),
            ResidencyState::Warm => idle > Duration::from_secs(300), // 5 minutes
            ResidencyState::Cold => idle > Duration::from_secs(900), // 15 minutes
            ResidencyState::Frozen => false,                         // Already at lowest state
        }
    }
}

/// Tab Residency Manager - Coordinates memory optimization across all tabs
pub struct TabResidencyManager {
    /// All tracked tabs indexed by tab_id
    tabs: HashMap<u64, TabResidency>,
    /// Next available tab ID
    next_tab_id: u64,
    /// Memory pressure threshold (bytes) - trigger aggressive eviction
    memory_pressure_threshold: usize,
    /// Current total memory usage estimate
    current_memory_usage: usize,
    /// Enable aggressive eviction mode
    aggressive_mode: bool,
}

impl TabResidencyManager {
    /// Create a new tab residency manager
    pub fn new() -> Self {
        TabResidencyManager {
            tabs: HashMap::new(),
            next_tab_id: 1,
            memory_pressure_threshold: 3 * 1024 * 1024 * 1024, // 3GB default
            current_memory_usage: 0,
            aggressive_mode: false,
        }
    }

    /// Register a new tab and return its ID
    pub fn register_tab(&mut self, url: String) -> u64 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let tab = TabResidency::new(tab_id, url);
        info!("Registered new tab {}: {}", tab_id, tab.url);
        self.tabs.insert(tab_id, tab);

        tab_id
    }

    /// Unregister and cleanup a tab
    pub fn unregister_tab(&mut self, tab_id: u64) -> Result<(), String> {
        if let Some(tab) = self.tabs.remove(&tab_id) {
            info!("Unregistered tab {}: {}", tab_id, tab.url);
            self.current_memory_usage = self.current_memory_usage.saturating_sub(tab.memory_usage);
            Ok(())
        } else {
            Err(format!("Tab {} not found", tab_id))
        }
    }

    /// Mark tab as interacted with (keeps it Active)
    pub fn touch_tab(&mut self, tab_id: u64) -> Result<(), String> {
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            tab.touch();
            // Restore to Active if not already
            if tab.state != ResidencyState::Active {
                self.restore_tab(tab_id)?;
            }
            Ok(())
        } else {
            Err(format!("Tab {} not found", tab_id))
        }
    }

    /// Transition tab to next eviction state
    pub fn evict_tab(&mut self, tab_id: u64) -> Result<ResidencyState, String> {
        let tab = self
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;

        let old_state = tab.state.clone();
        let new_state = match old_state {
            ResidencyState::Active => {
                // Create compressed snapshot
                self.create_snapshot(tab_id)?;
                ResidencyState::Warm
            }
            ResidencyState::Warm => {
                // Reduce snapshot to minimal state
                self.compress_snapshot(tab_id)?;
                ResidencyState::Cold
            }
            ResidencyState::Cold => {
                // Serialize to disk
                self.freeze_tab(tab_id)?;
                ResidencyState::Frozen
            }
            ResidencyState::Frozen => {
                // Already at lowest state
                return Ok(ResidencyState::Frozen);
            }
        };

        let tab = self.tabs.get_mut(&tab_id).unwrap();
        tab.state = new_state.clone();

        info!(
            "Tab {} transitioned from {:?} to {:?}",
            tab_id, old_state, new_state
        );
        Ok(new_state)
    }

    /// Restore tab to Active state
    pub fn restore_tab(&mut self, tab_id: u64) -> Result<(), String> {
        let tab = self
            .tabs
            .get(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;

        let old_state = tab.state.clone();
        if old_state == ResidencyState::Active {
            return Ok(()); // Already active
        }

        match old_state {
            ResidencyState::Warm => {
                // Quick restore from compressed snapshot (<100ms target)
                self.restore_from_warm(tab_id)?;
            }
            ResidencyState::Cold => {
                // Restore from minimal state (200-500ms)
                self.restore_from_cold(tab_id)?;
            }
            ResidencyState::Frozen => {
                // Full restore from disk (500ms-2s)
                self.restore_from_frozen(tab_id)?;
            }
            ResidencyState::Active => {}
        }

        let tab = self.tabs.get_mut(&tab_id).unwrap();
        tab.state = ResidencyState::Active;
        tab.touch();

        info!("Tab {} restored from {:?} to Active", tab_id, old_state);
        Ok(())
    }

    /// Run eviction pass - check all tabs and evict idle ones
    pub fn run_eviction_pass(&mut self) -> usize {
        let mut evicted_count = 0;

        let to_evict: Vec<u64> = self
            .tabs
            .iter()
            .filter(|(_, tab)| tab.should_evict())
            .map(|(&tab_id, _)| tab_id)
            .collect();

        for tab_id in to_evict {
            debug!("Evicting idle tab {}", tab_id);
            if self.evict_tab(tab_id).is_ok() {
                evicted_count += 1;
            }
        }

        if evicted_count > 0 {
            info!("Eviction pass completed: {} tabs evicted", evicted_count);
        }

        evicted_count
    }

    /// Enable aggressive eviction mode when memory pressure detected
    pub fn set_memory_pressure(&mut self, pressure: bool) {
        if (pressure || self.current_memory_usage >= self.memory_pressure_threshold)
            && !self.aggressive_mode
        {
            warn!("Memory pressure detected - enabling aggressive eviction");
            self.aggressive_mode = true;
            // Immediately evict all non-active tabs
            self.aggressive_eviction();
        } else if !pressure && self.aggressive_mode {
            info!("Memory pressure relieved - disabling aggressive eviction");
            self.aggressive_mode = false;
        }
    }

    /// Aggressive eviction - immediately evict all eligible tabs
    fn aggressive_eviction(&mut self) {
        let tab_ids: Vec<u64> = self.tabs.keys().copied().collect();
        let mut evicted = 0;

        for tab_id in tab_ids {
            if let Some(tab) = self.tabs.get(&tab_id) {
                // Evict everything that's not currently active
                if tab.idle_duration() > Duration::from_secs(5) && self.evict_tab(tab_id).is_ok() {
                    evicted += 1;
                }
            }
        }

        warn!("Aggressive eviction: {} tabs evicted", evicted);
    }

    /// Get current memory usage estimate
    pub fn get_memory_usage(&self) -> usize {
        self.current_memory_usage
    }

    /// Get tab count by state
    pub fn get_stats(&self) -> TabStats {
        let mut stats = TabStats::default();
        for tab in self.tabs.values() {
            match tab.state {
                ResidencyState::Active => stats.active_count += 1,
                ResidencyState::Warm => stats.warm_count += 1,
                ResidencyState::Cold => stats.cold_count += 1,
                ResidencyState::Frozen => stats.frozen_count += 1,
            }
            stats.total_memory += tab.memory_usage;
        }
        stats
    }

    // Private helper methods for state transitions

    fn create_snapshot(&mut self, tab_id: u64) -> Result<(), String> {
        let tab = self
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;

        let dom_state = format!(
            "tab:{}\nurl:{}\nscroll:{},{}\nviewport:{},{}\n",
            tab.tab_id, tab.url, 0.0, 0.0, 1920, 1080
        );
        let render_state = encode_u32s(&tab.gpu_buffer_handles);
        let v8_state = format!("tab:{}\norigin:{}\n", tab.tab_id, origin_key(&tab.url));

        tab.snapshot = Some(TabSnapshot {
            dom_snapshot: Some(super::compression::compress(dom_state.as_bytes())?),
            render_snapshot: Some(super::compression::compress(&render_state)?),
            v8_heap_snapshot: Some(super::compression::compress(v8_state.as_bytes())?),
            scroll_position: (0.0, 0.0),
            viewport_size: (1920, 1080),
        });

        let old_usage = tab.memory_usage;
        tab.memory_usage = tab
            .snapshot
            .as_ref()
            .map(snapshot_memory_usage)
            .unwrap_or_default();
        self.current_memory_usage = self
            .current_memory_usage
            .saturating_sub(old_usage)
            .saturating_add(tab.memory_usage);

        debug!("Created snapshot for tab {}", tab_id);
        Ok(())
    }

    fn compress_snapshot(&mut self, tab_id: u64) -> Result<(), String> {
        let tab = self
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;

        let old_usage = tab.memory_usage;
        tab.memory_usage = tab
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot_memory_usage(snapshot) / 2)
            .unwrap_or(1024)
            .max(1024);
        self.current_memory_usage = self
            .current_memory_usage
            .saturating_sub(old_usage)
            .saturating_add(tab.memory_usage);

        debug!("Compressed snapshot for tab {}", tab_id);
        Ok(())
    }

    fn freeze_tab(&mut self, tab_id: u64) -> Result<(), String> {
        let tab = self
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;

        let path = frozen_tab_path(tab_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create frozen tab directory: {}", e))?;
        }

        let frozen = FrozenTabState {
            tab_id,
            url: tab.url.clone(),
            snapshot: tab.snapshot.clone(),
            memory_usage: tab.memory_usage,
        };
        let bytes = bincode::serialize(&frozen)
            .map_err(|e| format!("Failed to encode tab state: {}", e))?;
        fs::write(&path, bytes).map_err(|e| format!("Failed to write frozen tab state: {}", e))?;
        tab.frozen_path = Some(path.to_string_lossy().into_owned());

        let old_usage = tab.memory_usage;
        tab.snapshot = None;
        tab.memory_usage = 1024;
        self.current_memory_usage = self
            .current_memory_usage
            .saturating_sub(old_usage)
            .saturating_add(tab.memory_usage);

        debug!("Froze tab {} to disk", tab_id);
        Ok(())
    }

    fn restore_from_warm(&mut self, tab_id: u64) -> Result<(), String> {
        let tab = self
            .tabs
            .get(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;
        validate_snapshot(tab.snapshot.as_ref())?;
        debug!("Restoring tab {} from Warm state", tab_id);
        Ok(())
    }

    fn restore_from_cold(&mut self, tab_id: u64) -> Result<(), String> {
        let tab = self
            .tabs
            .get(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;
        validate_snapshot(tab.snapshot.as_ref())?;
        debug!("Restoring tab {} from Cold state", tab_id);
        Ok(())
    }

    fn restore_from_frozen(&mut self, tab_id: u64) -> Result<(), String> {
        let frozen_path = self
            .tabs
            .get(&tab_id)
            .and_then(|tab| tab.frozen_path.clone())
            .ok_or_else(|| format!("Tab {} has no frozen state path", tab_id))?;
        let bytes = fs::read(&frozen_path)
            .map_err(|e| format!("Failed to read frozen tab state: {}", e))?;
        let frozen: FrozenTabState = bincode::deserialize(&bytes)
            .map_err(|e| format!("Failed to decode frozen tab state: {}", e))?;
        validate_snapshot(frozen.snapshot.as_ref())?;

        let tab = self
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id))?;
        let old_usage = tab.memory_usage;
        tab.snapshot = frozen.snapshot;
        tab.url = frozen.url;
        tab.frozen_path = None;
        tab.memory_usage = estimate_active_memory_usage(&tab.snapshot);
        self.current_memory_usage = self
            .current_memory_usage
            .saturating_sub(old_usage)
            .saturating_add(tab.memory_usage);
        let _ = fs::remove_file(&frozen_path);
        debug!("Restoring tab {} from Frozen state", tab_id);
        Ok(())
    }
}

fn snapshot_memory_usage(snapshot: &TabSnapshot) -> usize {
    snapshot
        .dom_snapshot
        .as_ref()
        .map(Vec::len)
        .unwrap_or_default()
        + snapshot
            .render_snapshot
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
        + snapshot
            .v8_heap_snapshot
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
}

fn estimate_active_memory_usage(snapshot: &Option<TabSnapshot>) -> usize {
    snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot_memory_usage(snapshot)
                .saturating_mul(4)
                .max(1024 * 1024)
        })
        .unwrap_or(1024 * 1024)
}

fn validate_snapshot(snapshot: Option<&TabSnapshot>) -> Result<(), String> {
    let snapshot = snapshot.ok_or_else(|| "Tab snapshot is missing".to_string())?;
    for compressed in [
        snapshot.dom_snapshot.as_deref(),
        snapshot.render_snapshot.as_deref(),
        snapshot.v8_heap_snapshot.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let _ = super::compression::decompress(compressed)?;
    }
    Ok(())
}

fn encode_u32s(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn origin_key(url: &str) -> &str {
    url.split('/').nth(2).unwrap_or(url)
}

fn frozen_tab_path(tab_id: u64) -> PathBuf {
    std::env::temp_dir()
        .join("soliloquy")
        .join(format!("frozen_tab_{}.bin", tab_id))
}

/// Statistics about tab memory states
#[derive(Debug, Default)]
pub struct TabStats {
    pub active_count: usize,
    pub warm_count: usize,
    pub cold_count: usize,
    pub frozen_count: usize,
    pub total_memory: usize,
}

impl Default for TabResidencyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_tab_registration() {
        let mut manager = TabResidencyManager::new();
        let tab_id = manager.register_tab("https://example.com".to_string());
        assert_eq!(tab_id, 1);

        let tab = manager.tabs.get(&tab_id).unwrap();
        assert_eq!(tab.state, ResidencyState::Active);
        assert_eq!(tab.url, "https://example.com");
    }

    #[test]
    fn test_tab_unregistration() {
        let mut manager = TabResidencyManager::new();
        let tab_id = manager.register_tab("https://example.com".to_string());

        let result = manager.unregister_tab(tab_id);
        assert!(result.is_ok());
        assert!(!manager.tabs.contains_key(&tab_id));
    }

    #[test]
    fn test_tab_touch() {
        let mut manager = TabResidencyManager::new();
        let tab_id = manager.register_tab("https://example.com".to_string());

        sleep(Duration::from_millis(100));
        let result = manager.touch_tab(tab_id);
        assert!(result.is_ok());

        let tab = manager.tabs.get(&tab_id).unwrap();
        assert!(tab.idle_duration() < Duration::from_millis(50));
    }

    #[test]
    fn test_state_transitions() {
        let mut manager = TabResidencyManager::new();
        let tab_id = manager.register_tab("https://example.com".to_string());

        // Active -> Warm
        let state = manager.evict_tab(tab_id).unwrap();
        assert_eq!(state, ResidencyState::Warm);

        // Warm -> Cold
        let state = manager.evict_tab(tab_id).unwrap();
        assert_eq!(state, ResidencyState::Cold);

        // Cold -> Frozen
        let state = manager.evict_tab(tab_id).unwrap();
        assert_eq!(state, ResidencyState::Frozen);
    }

    #[test]
    fn test_tab_restoration() {
        let mut manager = TabResidencyManager::new();
        let tab_id = manager.register_tab("https://example.com".to_string());

        // Evict to Cold
        manager.evict_tab(tab_id).unwrap();
        manager.evict_tab(tab_id).unwrap();

        // Restore to Active
        let result = manager.restore_tab(tab_id);
        assert!(result.is_ok());

        let tab = manager.tabs.get(&tab_id).unwrap();
        assert_eq!(tab.state, ResidencyState::Active);
    }

    #[test]
    fn test_frozen_tab_persists_snapshot_and_restores_it() {
        let mut manager = TabResidencyManager::new();
        let tab_id = manager.register_tab("https://example.com/stateful".to_string());
        manager.tabs.get_mut(&tab_id).unwrap().memory_usage = 4 * 1024 * 1024;

        manager.evict_tab(tab_id).unwrap();
        manager.evict_tab(tab_id).unwrap();
        manager.evict_tab(tab_id).unwrap();

        let frozen_path = manager
            .tabs
            .get(&tab_id)
            .unwrap()
            .frozen_path
            .clone()
            .unwrap();
        assert!(std::path::Path::new(&frozen_path).exists());
        manager.tabs.get_mut(&tab_id).unwrap().snapshot = None;

        manager.restore_tab(tab_id).unwrap();

        let tab = manager.tabs.get(&tab_id).unwrap();
        assert_eq!(tab.state, ResidencyState::Active);
        assert!(tab.snapshot.is_some());
        assert!(tab.frozen_path.is_none());
        assert!(!std::path::Path::new(&frozen_path).exists());
    }

    #[test]
    fn test_should_evict_timing() {
        let tab = TabResidency::new(1, "https://example.com".to_string());

        // Should not evict immediately
        assert!(!tab.should_evict());
    }

    #[test]
    fn test_stats_collection() {
        let mut manager = TabResidencyManager::new();

        manager.register_tab("https://example1.com".to_string());
        let tab2 = manager.register_tab("https://example2.com".to_string());
        let tab3 = manager.register_tab("https://example3.com".to_string());

        manager.evict_tab(tab2).unwrap(); // Warm
        manager.evict_tab(tab3).unwrap(); // Warm
        manager.evict_tab(tab3).unwrap(); // Cold

        let stats = manager.get_stats();
        assert_eq!(stats.active_count, 1);
        assert_eq!(stats.warm_count, 1);
        assert_eq!(stats.cold_count, 1);
        assert_eq!(stats.frozen_count, 0);
    }

    #[test]
    fn test_memory_pressure() {
        let mut manager = TabResidencyManager::new();

        for i in 0..5 {
            manager.register_tab(format!("https://example{}.com", i));
        }

        manager.set_memory_pressure(true);
        assert!(manager.aggressive_mode);

        manager.set_memory_pressure(false);
        assert!(!manager.aggressive_mode);
    }
}
