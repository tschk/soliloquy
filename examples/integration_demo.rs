//! Full integration demo showcasing WGPU GPU rendering and Zircon VMO integration
//!
//! This example demonstrates:
//! - Real WGPU device initialization
//! - GPU compute and render pipeline creation
//! - Zircon VMO creation and operations (placeholder on non-Fuchsia)
//! - Complete tab memory management flow with GPU integration

use soliloquy_browser_optimizations::*;
use soliloquy_browser_optimizations::gpu::WgpuContext;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    println!("=== Soliloquy Browser Integration Demo ===\n");

    // 1. Initialize WGPU GPU device
    println!("1. Initializing WGPU GPU device...");
    #[cfg(feature = "wgpu_rendering")]
    {
        let ctx = pollster::block_on(async {
            WgpuContext::new().await
        });
        
        match ctx {
            Ok(ctx) => {
                println!("✓ WGPU device initialized:");
                println!("  - Adapter: {} ({:?})", ctx.adapter_info().name, ctx.adapter_info().backend);
                println!("  - Device type: {:?}", ctx.adapter_info().device_type);
                
                // Create compute pipeline for layout
                println!("\n2. Creating GPU compute pipeline for layout...");
                match ctx.create_layout_pipeline() {
                    Ok(_pipeline) => {
                        println!("✓ Layout compute pipeline created");
                        println!("  - Entry point: layout_pass");
                        println!("  - Workgroup size: 64");
                    }
                    Err(e) => println!("✗ Failed to create layout pipeline: {}", e),
                }
                
                // Create render pipeline for compositing
                println!("\n3. Creating GPU render pipeline for compositing...");
                match ctx.create_compositor_pipeline(wgpu::TextureFormat::Bgra8Unorm) {
                    Ok(_pipeline) => {
                        println!("✓ Compositor render pipeline created");
                        println!("  - Vertex entry: vs_main");
                        println!("  - Fragment entry: fs_main");
                        println!("  - Blend mode: Alpha blending");
                    }
                    Err(e) => println!("✗ Failed to create compositor pipeline: {}", e),
                }
            }
            Err(e) => {
                println!("✗ Failed to initialize WGPU: {}", e);
                println!("  (This is expected in headless CI environments)");
            }
        }
    }
    
    #[cfg(not(feature = "wgpu_rendering"))]
    {
        println!("✓ WGPU rendering disabled (using stub implementation)");
        let ctx = WgpuContext::new()?;
        println!("  - Placeholder context created for development");
    }

    // 2. Zircon VMO operations
    println!("\n4. Creating Zircon VMO for tab memory...");
    let tab_size = 20 * 1024 * 1024; // 20MB
    let vmo = ZirconVmo::create(tab_size, "demo_tab")?;
    
    #[cfg(feature = "fuchsia")]
    println!("✓ Real Zircon VMO created:");
    
    #[cfg(not(feature = "fuchsia"))]
    println!("✓ Placeholder VMO created (fuchsia feature not enabled):");
    
    println!("  - Name: {}", vmo.name());
    println!("  - Size: {} bytes ({}MB)", vmo.size(), vmo.size() / 1024 / 1024);
    
    // Test COW cloning
    println!("\n5. Creating copy-on-write clone (for tab forking)...");
    let vmo_clone = vmo.create_cow_clone()?;
    println!("✓ COW clone created:");
    println!("  - Name: {}", vmo_clone.name());
    println!("  - Size: {} bytes", vmo_clone.size());
    
    // Test memory mapping
    println!("\n6. Mapping VMO into address space...");
    let _mapped = vmo.map()?;
    println!("✓ VMO mapped successfully");
    #[cfg(feature = "fuchsia")]
    println!("  - Would use real zx_vmar_map on Fuchsia");
    #[cfg(not(feature = "fuchsia"))]
    println!("  - Using placeholder mapping");
    
    // 3. Complete tab memory flow
    println!("\n7. Creating ZirconTabMemory for tab isolation...");
    let mut tab_mem = ZirconTabMemory::new(tab_size, 12345)?;
    println!("✓ Tab memory created");
    
    tab_mem.map()?;
    println!("✓ Tab memory mapped");
    
    #[cfg(all(feature = "wgpu_rendering", not(feature = "fuchsia")))]
    {
        let _gpu_buf = tab_mem.import_to_gpu()?;
        println!("✓ Tab memory imported to GPU (placeholder)");
        println!("  - Zero-copy sharing with GPU would work on Fuchsia");
    }
    
    // 4. Tab residency management
    println!("\n8. Demonstrating tab residency management...");
    let mut residency = TabResidencyManager::new();
    
    let tab1 = residency.register_tab("https://example.com".to_string());
    let tab2 = residency.register_tab("https://github.com".to_string());
    let tab3 = residency.register_tab("https://rust-lang.org".to_string());
    
    println!("✓ Registered 3 tabs");
    println!("  - All start in Active state");
    
    // Touch tab1 to keep it active
    residency.touch_tab(tab1);
    
    // Simulate time passing
    println!("\n9. Simulating idle time and automatic eviction...");
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    residency.run_eviction_pass();
    let stats = residency.get_stats();
    
    println!("✓ Eviction pass complete:");
    println!("  - Active: {} tabs", stats.active_count);
    println!("  - Warm: {} tabs", stats.warm_count);
    println!("  - Cold: {} tabs", stats.cold_count);
    println!("  - Frozen: {} tabs", stats.frozen_count);
    
    // 5. Memory pressure monitoring
    println!("\n10. Checking system memory pressure...");
    let monitor = MemoryPressureMonitor::default(); // Uses default thresholds
    
    // Simulate some memory usage
    monitor.update_usage(1024 * 1024 * 1024); // 1GB
    
    let level = monitor.get_pressure_level();
    println!("✓ Memory pressure level: {:?}", level);
    println!("  - Current usage: {}MB", monitor.get_current_usage() / 1024 / 1024);
    println!("  - Usage: {:.1}%", monitor.get_usage_percentage());
    
    if monitor.is_under_pressure() {
        println!("  ⚠ System under memory pressure");
    } else {
        println!("  ✓ Memory pressure normal");
    }
    
    // 6. Compression demo
    println!("\n11. Testing real zstd compression...");
    let test_data = "Hello ".repeat(1000); // Repetitive data
    let compressed = memory::compress(test_data.as_bytes())?;
    let ratio = (compressed.len() as f64 / test_data.len() as f64) * 100.0;
    
    println!("✓ Compression successful:");
    println!("  - Original: {} bytes", test_data.len());
    println!("  - Compressed: {} bytes", compressed.len());
    println!("  - Ratio: {:.1}%", ratio);
    
    let decompressed = memory::decompress(&compressed)?;
    assert_eq!(decompressed, test_data.as_bytes());
    println!("  - Roundtrip verified ✓");

    // 7. Disk storage
    println!("\n12. Testing disk-backed frozen tab storage...");
    let max_disk = 100 * 1024 * 1024; // 100MB
    let mut disk_storage = memory::DiskStorage::new("/tmp/soliloquy_demo_disk", max_disk)?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let frozen_state = memory::FrozenTabState {
        tab_id: 999,
        url: "https://example.org".to_string(),
        dom_snapshot: vec![1, 2, 3, 4, 5],
        render_snapshot: vec![6, 7, 8, 9, 10],
        v8_snapshot: vec![11, 12, 13, 14, 15],
        scroll_position: (0.0, 100.0),
        viewport_size: (1920, 1080),
        frozen_at: now,
        original_size: 20 * 1024 * 1024, // 20MB original
        compressed_size: 2 * 1024 * 1024, // 2MB compressed
    };
    
    disk_storage.save_tab(&frozen_state)?;
    println!("✓ Frozen tab saved to disk");
    
    let loaded = disk_storage.load_tab(999)?;
    assert_eq!(loaded.url, frozen_state.url);
    println!("  - Loaded back successfully ✓");
    
    let stats = disk_storage.get_stats();
    println!("  - Disk usage: {} bytes ({}% of max)", stats.current_usage, stats.usage_percentage as u32);
    println!("  - Tab count: {}", stats.tab_count);

    println!("\n=== Integration Demo Complete ===");
    println!("\nFeatures demonstrated:");
    println!("✓ WGPU GPU device initialization");
    println!("✓ GPU compute and render pipeline creation");
    println!("✓ Zircon VMO creation and COW cloning");
    println!("✓ Tab memory isolation with zero-copy potential");
    println!("✓ Tab residency management with automatic eviction");
    println!("✓ Memory pressure monitoring");
    println!("✓ Real zstd compression (90%+ reduction)");
    println!("✓ Disk-backed frozen tab storage");
    
    #[cfg(feature = "fuchsia")]
    println!("\n🚀 Running with real Fuchsia kernel APIs!");
    
    #[cfg(not(feature = "fuchsia"))]
    println!("\n💡 Enable 'fuchsia' feature for real kernel integration");
    
    #[cfg(feature = "wgpu_rendering")]
    println!("🎨 WGPU GPU rendering enabled");
    
    #[cfg(not(feature = "wgpu_rendering"))]
    println!("💡 Enable 'wgpu_rendering' feature for real GPU pipelines");

    Ok(())
}
