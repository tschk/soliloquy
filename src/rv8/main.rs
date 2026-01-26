//! RV8 Browser Main Entry Point
//!
//! Launches the RV8 browser with multi-process architecture.

use log::{error, info};
use roverate::compositor;
use roverate::networking;
use std::env;

use roverate::renderer;
use roverate::core::{Browser, BrowserConfig};
// storage and ipc are not used directly vs via other modules, but if needed:
// use roverate::ipc;

fn main() {
    // Initialize logging with tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rv8=info".parse().unwrap())
                .add_directive("wgpu=warn".parse().unwrap()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();

    info!("roverate v0.1");

    // Parse command line args
    let args: Vec<String> = env::args().collect();
    let initial_url = args.get(1).map(|s| s.as_str()).unwrap_or("about:blank");

    // Check for special process types (Chrome-like multi-process)
    if let Some(process_type) = args.iter().find(|a| a.starts_with("--type=")) {
        let ptype = process_type.strip_prefix("--type=").unwrap();
        match ptype {
            "renderer" => run_renderer_process(&args),
            "gpu" => run_gpu_process(&args),
            "network" => run_network_process(&args),
            "utility" => run_utility_process(&args),
            _ => {
                error!("Unknown process type: {}", ptype);
                std::process::exit(1);
            }
        }
        return;
    }

    // Main browser process
    run_browser_process(initial_url);
}

// mod ui;

/// Run the main browser process (UI, coordination)
fn run_browser_process(initial_url: &str) {
    info!("Starting browser process...");

    let url = initial_url.to_string();
    // Spawn backend logic in background thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        rt.block_on(async {
            info!("Backend initialized");
            let config = BrowserConfig::headless();
            match Browser::new(config).await {
                Ok(mut browser) => {
                    if let Err(e) = browser.new_tab(&url).await {
                        error!("Failed to open new tab: {}", e);
                    }
                    browser.run().await;
                }
                Err(e) => {
                    error!("Failed to create browser: {}", e);
                }
            }
        });
    });

    // GPUI integration disabled due to dependency conflict (core-graphics)
    // ui::run_app(gpui::App::new());
    info!("GPUI disabled. Browser process running in headless mode.");
    std::thread::park();

    info!("Browser process terminated");
}

/// Run a renderer subprocess (sandboxed)
fn run_renderer_process(args: &[String]) {
    info!("Starting renderer process...");

    // Extract IPC channel from args
    let channel_id = args
        .iter()
        .find(|a| a.starts_with("--channel-id="))
        .map(|a| a.strip_prefix("--channel-id=").unwrap())
        .expect("Renderer process requires --channel-id");

    info!("Renderer process connecting to channel: {}", channel_id);

    // Create renderer with IPC connection
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    rt.block_on(async {
        // TODO: Implement real IPC connection using ipc-channel
        // For now, we use dummy channels to satisfy the build
        let (to_browser_tx, _to_browser_rx) = tokio::sync::mpsc::unbounded_channel();
        let (_from_browser_tx, from_browser_rx) = tokio::sync::mpsc::unbounded_channel();

        let config = roverate::servo_embed::ServoConfig {
            user_agent: "RV8/0.1.0".to_string(),
            ..Default::default()
        };

        use std::process;
        let mut renderer =
            renderer::RendererProcess::new(process::id() as u64, to_browser_tx, config)
                .await
                .expect("Failed to create renderer process");

        renderer.run(from_browser_rx).await;
    });
}

/// Run the GPU process (compositing)
fn run_gpu_process(args: &[String]) {
    info!("Starting GPU process...");

    let channel_id = args
        .iter()
        .find(|a| a.starts_with("--channel-id="))
        .map(|a| a.strip_prefix("--channel-id=").unwrap())
        .expect("GPU process requires --channel-id");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let gpu = compositor::GpuProcess::new(channel_id).await;
        gpu.run().await;
    });
}

/// Run the network process
fn run_network_process(args: &[String]) {
    info!("Starting network process...");

    let channel_id = args
        .iter()
        .find(|a| a.starts_with("--channel-id="))
        .map(|a| a.strip_prefix("--channel-id=").unwrap())
        .expect("Network process requires --channel-id");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let network = networking::NetworkProcess::new(channel_id).await;
        network.run().await;
    });
}

/// Run a utility process
fn run_utility_process(args: &[String]) {
    info!("Starting utility process...");

    let channel_id = args
        .iter()
        .find(|a| a.starts_with("--channel-id="))
        .map(|a| a.strip_prefix("--channel-id=").unwrap())
        .expect("Utility process requires --channel-id");

    info!("Utility process running with channel: {}", channel_id);

    // Utility processes handle various tasks like extensions, etc.
    std::thread::park();
}
