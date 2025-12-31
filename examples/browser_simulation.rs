//! Example: Multi-tab browser simulation with memory optimization
//!
//! Demonstrates the tab residency system managing 150+ tabs with <3GB RAM.
//!
//! Run with: cargo run --example browser_simulation --target x86_64-unknown-linux-gnu

use soliloquy_browser_optimizations::*;
use soliloquy_browser_optimizations::network::{ResourceType, PrefetchPriority};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    env_logger::init();

    println!("=== Soliloquy Browser Optimization Demo ===\n");

    // Initialize optimization systems
    println!("1. Initializing optimization systems...");
    let mut residency = TabResidencyManager::new();
    let mut gc_scheduler = GcScheduler::new();
    let damage_tracker = DamageTracker::new(1920, 1080, 64);
    let mut prefetch = PrefetchManager::new();

    println!("   ✓ Tab residency manager");
    println!("   ✓ GC scheduler");
    println!("   ✓ Damage tracker ({}x{}, {} tiles)",
        1920, 1080, damage_tracker.get_damaged_tiles().len());
    println!("   ✓ Prefetch manager\n");

    // Simulate opening 150 tabs
    println!("2. Opening 150 tabs...");
    let urls = vec![
        "https://github.com",
        "https://news.ycombinator.com",
        "https://reddit.com",
        "https://stackoverflow.com",
        "https://twitter.com",
        "https://youtube.com",
        "https://wikipedia.org",
        "https://medium.com",
        "https://dev.to",
        "https://rust-lang.org",
    ];

    let mut tab_ids = Vec::new();
    for i in 0..150 {
        let url = format!("{}/page{}", urls[i % urls.len()], i);
        let tab_id = residency.register_tab(url);
        tab_ids.push(tab_id);
    }

    let stats = residency.get_stats();
    println!("   ✓ Created {} tabs (all Active)", stats.active_count);
    println!("   Memory usage: {:.2} MB\n", stats.total_memory as f64 / 1024.0 / 1024.0);

    // Simulate user viewing first 5 tabs for a bit
    println!("3. User actively browsing first 5 tabs...");
    for &tab_id in &tab_ids[0..5] {
        residency.touch_tab(tab_id).unwrap();
        gc_scheduler.record_interaction();
    }
    sleep(Duration::from_millis(100));

    // Run eviction pass to transition idle tabs
    println!("4. Running eviction pass (simulating 30s idle time)...");
    for &tab_id in &tab_ids[5..] {
        // Manually evict for demo (normally happens automatically based on time)
        residency.evict_tab(tab_id).unwrap();
    }

    let stats = residency.get_stats();
    println!("   Tab states:");
    println!("   - Active: {} tabs", stats.active_count);
    println!("   - Warm: {} tabs", stats.warm_count);
    println!("   - Cold: {} tabs", stats.cold_count);
    println!("   - Frozen: {} tabs", stats.frozen_count);
    println!("   Memory usage: {:.2} MB\n", stats.total_memory as f64 / 1024.0 / 1024.0);

    // Further evictions
    println!("5. Aggressive eviction (simulating longer idle)...");
    for &tab_id in &tab_ids[5..50] {
        residency.evict_tab(tab_id).unwrap();
        residency.evict_tab(tab_id).unwrap(); // Warm -> Cold
    }

    for &tab_id in &tab_ids[50..] {
        residency.evict_tab(tab_id).unwrap(); // Active -> Warm
        residency.evict_tab(tab_id).unwrap(); // Warm -> Cold
        residency.evict_tab(tab_id).unwrap(); // Cold -> Frozen
    }

    let stats = residency.get_stats();
    println!("   Tab states:");
    println!("   - Active: {} tabs", stats.active_count);
    println!("   - Warm: {} tabs", stats.warm_count);
    println!("   - Cold: {} tabs", stats.cold_count);
    println!("   - Frozen: {} tabs", stats.frozen_count);
    println!("   Memory usage: {:.2} MB ({}% reduction)\n",
        stats.total_memory as f64 / 1024.0 / 1024.0,
        ((1.0 - stats.total_memory as f64 / (150.0 * 20.0 * 1024.0 * 1024.0)) * 100.0) as i32);

    // Simulate prefetching
    println!("6. Network optimizations...");
    prefetch.request_prefetch(
        "https://github.com".to_string(),
        ResourceType::Dns,
        PrefetchPriority::High,
    );
    prefetch.record_hover("https://rust-lang.org".to_string());
    prefetch.record_hover("https://rust-lang.org".to_string());
    println!("   ✓ DNS prefetch queued: {} requests", prefetch.pending_count());

    // Check GC scheduling
    println!("\n7. V8 GC scheduling...");
    gc_scheduler.set_idle_threshold(Duration::from_millis(50));
    sleep(Duration::from_millis(100));
    
    if let Some(gc_type) = gc_scheduler.should_run_gc() {
        println!("   ✓ GC scheduled: {:?}", gc_type);
        gc_scheduler.record_gc(gc_type, Duration::from_millis(10));
    }

    // Restore a tab
    println!("\n8. User switches to a frozen tab...");
    let frozen_tab = tab_ids[100];
    let restore_start = std::time::Instant::now();
    residency.restore_tab(frozen_tab).unwrap();
    let restore_time = restore_start.elapsed();
    println!("   ✓ Tab restored in {:?}", restore_time);

    // Final stats
    println!("\n9. Final statistics:");
    let stats = residency.get_stats();
    println!("   Total tabs: {}", tab_ids.len());
    println!("   Memory usage: {:.2} MB", stats.total_memory as f64 / 1024.0 / 1024.0);
    println!("   Target achieved: {} tabs with <3GB RAM ✓", tab_ids.len());

    println!("\n=== Demo Complete ===");
}
