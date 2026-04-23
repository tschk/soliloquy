//! Soliloquy Shell main entry point
//!
//! This is the main entry point for desktop and Alpine/Linux builds.

mod browser_optimizations;
mod engine_bridge;
mod js_engine;
mod optimizations;
mod platform;
mod servo_embedder;
mod v8_runtime;

#[cfg(test)]
mod integration_tests;

use log::{error, info};
use servo_embedder::ServoEmbedder;
use std::env;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("═══════════════════════════════════════════════════");
    info!("         Soliloquy Shell v0.1.0");
    info!("         Browser Appliance Mode (Linux/macOS)");
    info!("═══════════════════════════════════════════════════");

    // Initialize the leaner runtime profile used by the browser appliance.
    let settings = optimizations::OptimizationSettings::embedded();
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

    if let Ok(start_url) = env::var("SOLILOQUY_START_URL") {
        let start_url = start_url.trim().to_string();
        if !start_url.is_empty() {
            match embedder.load_url(&start_url) {
                Ok(_) => info!("Initial URL loaded successfully"),
                Err(e) => error!("Failed to load initial URL: {}", e),
            }
        } else {
            info!("No SOLILOQUY_START_URL provided; browser will stay lazy until first navigation");
        }
    } else {
        info!("No SOLILOQUY_START_URL provided; browser will stay lazy until first navigation");
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
        use servo_embedder::InputEvent;
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

    #[cfg(all(target_os = "linux", feature = "mobile"))]
    {
        use platform::mobile::MobileWindow;
        use platform::Window;

        info!("Creating Linux mobile surface placeholder...");
        let window = match MobileWindow::new() {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create mobile window: {}", e);
                return;
            }
        };

        info!(
            "Linux mobile surface placeholder created ({}x{})",
            window.size().0,
            window.size().1
        );
        let _ = window.present();
    }

    info!("Soliloquy Shell shutdown complete");
}
