//! Soliloquy Shell main entry point
//!
//! This is the main entry point that supports both Fuchsia and desktop (Linux/macOS) builds.
//!
//! ## Build Modes
//!
//! - **Desktop (default)**: `cargo run --features desktop`
//! - **Fuchsia**: `cargo build --features fuchsia --target aarch64-unknown-fuchsia`

// Platform modules
#[cfg(feature = "fuchsia")]
mod zircon_window;

mod engine_bridge;
mod optimizations;
mod platform;
mod servo_embedder;
mod v8_runtime;

#[cfg(test)]
mod integration_tests;

use log::{debug, error, info};
use servo_embedder::{InputEvent, ServoEmbedder};

// Fuchsia-specific imports
#[cfg(feature = "fuchsia")]
use fidl::endpoints::ServiceMarker;
#[cfg(feature = "fuchsia")]
use fuchsia_async as fasync;
#[cfg(feature = "fuchsia")]
use fuchsia_component::server::ServiceFs;
#[cfg(feature = "fuchsia")]
use fuchsia_ui_app::fidl_fuchsia_ui_app::{
    ViewProviderMarker, ViewProviderRequest, ViewProviderRequestStream,
};
#[cfg(feature = "fuchsia")]
use futures::StreamExt;
#[cfg(feature = "fuchsia")]
use zircon_window::ZirconWindow;

#[cfg(feature = "fuchsia")]
enum IncomingService {
    ViewProvider(ViewProviderRequestStream),
}

// Desktop entry point
#[cfg(all(feature = "desktop", not(feature = "fuchsia")))]
fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("═══════════════════════════════════════════════════");
    info!("         Soliloquy Shell v0.1.0");
    info!("         Desktop Mode (Linux/macOS)");
    info!("═══════════════════════════════════════════════════");

    // Initialize optimizations
    let settings = optimizations::OptimizationSettings::desktop();
    optimizations::init_optimizations(&settings);

    // Initialize embedder
    let mut embedder = match ServoEmbedder::new() {
        Ok(embedder) => {
            info!("Servo embedder initialized successfully");
            embedder
        }
        Err(e) => {
            error!("Failed to initialize Servo embedder: {}", e);
            return;
        }
    };

    // Test URL load
    match embedder.load_url("https://example.com") {
        Ok(_) => info!("Initial URL loaded successfully"),
        Err(e) => error!("Failed to load initial URL: {}", e),
    }

    // Test V8
    match embedder.execute_js("console.log('V8 is working in Soliloquy!'); 'V8 Test Success'") {
        Ok(result) => info!("V8 test result: {}", result),
        Err(e) => error!("V8 test failed: {}", e),
    }

    info!("Embedder state: {:?}", embedder.get_state());
    if let Some(url) = embedder.get_current_url() {
        info!("Current URL: {}", url);
    }

    if let Some(webview_info) = embedder.get_webview_info() {
        info!("Webview info: {:?}", webview_info);
    }

    // Create native window
    #[cfg(target_os = "linux")]
    {
        use platform::linux::LinuxWindow;
        use platform::{Window, WindowEvent};

        info!("Creating Linux window...");

        let window = match LinuxWindow::new() {
            Ok(w) => {
                info!("Linux window created ({}x{})", w.size().0, w.size().1);
                w
            }
            Err(e) => {
                error!("Failed to create window: {}", e);
                return;
            }
        };

        let mut frame_pacer = optimizations::FramePacer::new(60);

        if let Err(e) = window.run(move |event, _| match event {
            WindowEvent::CloseRequested => {
                info!("Shutting down...");
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = embedder.present() {
                    error!("Present failed: {}", e);
                }
                frame_pacer.wait_for_frame();
            }
            WindowEvent::MouseInput { x, y, .. } => {
                embedder.handle_input(InputEvent::Touch { x, y });
            }
            WindowEvent::KeyboardInput {
                key, pressed: true, ..
            } => {
                embedder.handle_input(InputEvent::Key { code: key });
            }
            _ => {}
        }) {
            error!("Event loop error: {}", e);
        }
    }

    #[cfg(target_os = "macos")]
    {
        info!("macOS window support - use soliloquy_desktop binary for full support");
    }

    info!("Soliloquy Shell shutdown complete");
}

// Fuchsia entry point
#[cfg(feature = "fuchsia")]
#[fasync::run_singlethreaded]
async fn main() {
    fuchsia_syslog::init().unwrap();
    info!("Soliloquy Shell starting (Fuchsia mode)...");

    let mut embedder = match ServoEmbedder::new() {
        Ok(embedder) => {
            info!("Servo embedder initialized successfully");
            embedder
        }
        Err(e) => {
            error!("Failed to initialize Servo embedder: {}", e);
            return;
        }
    };

    match embedder.load_url("https://example.com") {
        Ok(_) => info!("Initial URL loaded successfully"),
        Err(e) => error!("Failed to load initial URL: {}", e),
    }

    match embedder.execute_js("console.log('V8 is working in Soliloquy!'); 'V8 Test Success'") {
        Ok(result) => info!("V8 test result: {}", result),
        Err(e) => error!("V8 test failed: {}", e),
    }

    let window = ZirconWindow::new();

    embedder.handle_input(InputEvent::Touch { x: 100.0, y: 200.0 });
    embedder.handle_input(InputEvent::Key { code: 13 });

    match embedder.present() {
        Ok(_) => debug!("Frame presented successfully"),
        Err(e) => error!("Failed to present frame: {}", e),
    }

    info!("Embedder state: {:?}", embedder.get_state());
    if let Some(url) = embedder.get_current_url() {
        info!("Current URL: {}", url);
    }

    if let Some(webview_info) = embedder.get_webview_info() {
        info!("Webview info: {:?}", webview_info);
    }

    info!("Setting up ViewProvider service");
    let mut fs = ServiceFs::new_local();

    fs.dir("svc")
        .add_fidl_service(IncomingService::ViewProvider);

    fs.take_and_serve_directory_handle()
        .expect("Failed to serve directory handle");

    info!("Soliloquy Shell running with ViewProvider service exposed");

    fs.for_each_concurrent(None, |request: IncomingService| async {
        match request {
            IncomingService::ViewProvider(stream) => {
                info!("Received ViewProvider connection");
                handle_view_provider(stream).await;
            }
        }
    })
    .await;
}

#[cfg(feature = "fuchsia")]
async fn handle_view_provider(mut stream: ViewProviderRequestStream) {
    info!("Handling ViewProvider request stream");

    while let Some(request) = stream.next().await {
        match request {
            Ok(ViewProviderRequest::CreateView {
                token,
                control_handle: _,
            }) => {
                info!("Received CreateView request (legacy)");

                let window = ZirconWindow::new_with_view_token(token);
                window.setup_scene_graph();

                info!("CreateView handled successfully");
            }
            Ok(ViewProviderRequest::CreateView2 {
                args,
                control_handle: _,
            }) => {
                info!("Received CreateView2 request");

                let window = ZirconWindow::new_with_view_token(args.view_creation_token);
                window.setup_scene_graph();

                info!("CreateView2 handled successfully");
            }
            Err(e) => {
                error!("ViewProvider request error: {:?}", e);
                break;
            }
        }
    }

    info!("ViewProvider stream closed");
}

// Fallback when neither feature is enabled
#[cfg(not(any(feature = "desktop", feature = "fuchsia")))]
fn main() {
    eprintln!("Error: No platform feature enabled.");
    eprintln!("Build with one of:");
    eprintln!("  cargo run --features desktop     # For Linux/macOS");
    eprintln!("  cargo build --features fuchsia   # For Fuchsia OS");
}
