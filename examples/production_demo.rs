/// Production-ready browser optimization demo
/// 
/// Demonstrates real zstd compression, disk serialization, and disk caching.

use soliloquy_browser_optimizations::memory::{
    TabResidencyManager, DiskStorage, FrozenTabState, compress, decompress,
};
use soliloquy_browser_optimizations::cache::DiskCache;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    env_logger::init();
    
    println!("=== Soliloquy Production Browser Optimization Demo ===\n");
    
    // Initialize systems
    let mut residency = TabResidencyManager::new();
    let mut disk_storage = DiskStorage::default();
    let mut disk_cache = DiskCache::new("./demo_cache", 100 * 1024 * 1024)
        .expect("Failed to create disk cache");
    
    println!("✓ Initialized optimization systems");
    println!("  - Tab residency manager");
    println!("  - Disk storage (1GB max)");
    println!("  - Disk cache (100MB max)\n");
    
    // Demo 1: Real compression
    demo_compression();
    
    // Demo 2: Create and manage tabs
    println!("\n=== Creating 150 tabs ===");
    for i in 1..=150 {
        let url = format!("https://example{}.com", i);
        residency.register_tab(url);
    }
    println!("✓ Created 150 tabs (all Active)\n");
    
    // Demo 3: Simulate idle time and freeze tabs
    println!("=== Simulating aggressive eviction ===");
    residency.set_memory_pressure(true);
    let evicted = residency.run_eviction_pass();
    println!("✓ Evicted {} tabs to save memory\n", evicted);
    
    // Demo 4: Freeze tabs to disk
    println!("=== Freezing idle tabs to disk ===");
    let frozen_count = freeze_idle_tabs(&residency, &mut disk_storage);
    println!("✓ Froze {} tabs to disk\n", frozen_count);
    
    // Demo 5: Disk cache demo
    demo_disk_cache(&mut disk_cache);
    
    // Final statistics
    print_final_stats(&residency, &disk_storage, &disk_cache);
    
    // Cleanup
    println!("\n=== Cleanup ===");
    disk_storage.clear_all().expect("Failed to clear storage");
    disk_cache.clear().expect("Failed to clear cache");
    println!("✓ Cleaned up disk storage and cache");
}

fn demo_compression() {
    println!("\n=== Real zstd Compression Demo ===");
    
    // Simulate DOM snapshot with repetitive structure
    let dom_data = b"<div class='container'><p>Content</p></div>".repeat(1000);
    println!("Original DOM size: {} KB", dom_data.len() / 1024);
    
    let compressed = compress(&dom_data).expect("Compression failed");
    println!("Compressed size: {} KB", compressed.len() / 1024);
    
    let ratio = (compressed.len() as f64 / dom_data.len() as f64) * 100.0;
    println!("Compression ratio: {:.1}%", ratio);
    println!("Space saved: {} KB ({:.1}%)", 
        (dom_data.len() - compressed.len()) / 1024,
        100.0 - ratio
    );
    
    // Verify decompression works
    let decompressed = decompress(&compressed).expect("Decompression failed");
    assert_eq!(&dom_data[..], &decompressed[..]);
    println!("✓ Compression/decompression verified");
}

fn freeze_idle_tabs(residency: &TabResidencyManager, storage: &mut DiskStorage) -> usize {
    let stats = residency.get_stats();
    let tabs_to_freeze = stats.cold_count + stats.frozen_count;
    
    // Simulate freezing tabs
    for tab_id in 1u64..=tabs_to_freeze as u64 {
        // Create compressed snapshots
        let dom_data = b"<html>DOM content</html>".repeat(100);
        let dom_snapshot = compress(&dom_data)
            .expect("Compression failed");
        let render_snapshot = compress(&vec![0u8; 5000])
            .expect("Compression failed");
        let v8_snapshot = compress(&vec![0u8; 10000])
            .expect("Compression failed");
        
        let frozen = FrozenTabState {
            tab_id,
            url: format!("https://example{}.com", tab_id),
            dom_snapshot,
            render_snapshot,
            v8_snapshot,
            scroll_position: (0.0, 0.0),
            viewport_size: (1920, 1080),
            frozen_at: current_timestamp(),
            original_size: 20_000_000,  // 20MB
            compressed_size: 250_000,    // 250KB
        };
        
        storage.save_tab(&frozen).expect("Failed to save frozen tab");
    }
    
    tabs_to_freeze as usize
}

fn demo_disk_cache(cache: &mut DiskCache) {
    println!("\n=== Disk Cache Demo ===");
    
    // Cache compiled shader
    let shader_code = b"@compute @workgroup_size(64) fn main() { /* shader */ }";
    cache.insert("shader:layout_compute", shader_code, "wgsl")
        .expect("Failed to cache shader");
    println!("✓ Cached compiled shader ({} bytes)", shader_code.len());
    
    // Cache V8 bytecode
    let bytecode = vec![0xFFu8; 10000];
    cache.insert("v8:script_main", &bytecode, "bytecode")
        .expect("Failed to cache bytecode");
    println!("✓ Cached V8 bytecode ({} bytes)", bytecode.len());
    
    // Retrieve from cache
    let retrieved = cache.get("shader:layout_compute")
        .expect("Failed to get shader");
    assert!(retrieved.is_some());
    println!("✓ Retrieved shader from cache (cache hit)");
    
    // Cache miss
    let missing = cache.get("nonexistent")
        .expect("Failed to get");
    assert!(missing.is_none());
    println!("✓ Handled cache miss correctly");
}

fn print_final_stats(
    residency: &TabResidencyManager,
    disk_storage: &DiskStorage,
    disk_cache: &DiskCache,
) {
    println!("\n=== Final Statistics ===");
    
    let stats = residency.get_stats();
    println!("\nTab Distribution:");
    println!("  Active: {}", stats.active_count);
    println!("  Warm:   {}", stats.warm_count);
    println!("  Cold:   {}", stats.cold_count);
    println!("  Frozen: {}", stats.frozen_count);
    
    let ram_usage = stats.total_memory as f64 / 1024.0 / 1024.0;
    println!("\nMemory Usage:");
    println!("  RAM: {:.2} MB", ram_usage);
    
    let disk_stats = disk_storage.get_stats();
    println!("  Disk (frozen tabs): {:.2} MB", 
        disk_stats.current_usage as f64 / 1024.0 / 1024.0);
    
    let cache_stats = disk_cache.stats();
    println!("  Disk (cache): {:.2} MB",
        cache_stats.current_size as f64 / 1024.0 / 1024.0);
    println!("  Cache hit rate: {:.1}%", cache_stats.hit_rate * 100.0);
    
    // Calculate total resources
    let baseline_memory = 150 * 20 * 1024 * 1024; // 150 tabs × 20MB
    let actual_memory = stats.total_memory + disk_stats.current_usage;
    let reduction = ((baseline_memory - actual_memory) as f64 / baseline_memory as f64) * 100.0;
    
    println!("\nPerformance:");
    println!("  Baseline (150 tabs × 20MB): {:.2} GB", 
        baseline_memory as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("  Actual usage: {:.2} MB", 
        actual_memory as f64 / 1024.0 / 1024.0);
    println!("  Memory reduction: {:.1}%", reduction);
    
    if ram_usage < 3000.0 {
        println!("\n✅ Target achieved: 150 tabs with <3GB RAM!");
    } else {
        println!("\n⚠️  Target not achieved ({}MB > 3GB)", ram_usage);
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
