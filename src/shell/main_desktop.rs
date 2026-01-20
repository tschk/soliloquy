//! Desktop main entry point for Soliloquy shell
//!
//! This is the main entry point for running Soliloquy on Linux and macOS.
//! It uses winit for windowing and wgpu for graphics, with Servo for
//! rendering and V8 for JavaScript execution.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin soliloquy_desktop --features desktop
//! ```

mod engine_bridge;
mod optimizations;
mod platform;
mod servo_embedder;
mod v8_runtime;

use engine_bridge::EngineBridge;
use log::{debug, error, info, warn};
use optimizations::{init_optimizations, FramePacer, OptimizationSettings};
use servo_embedder::{InputEvent, ServoEmbedder};

#[cfg(feature = "desktop")]
use platform::{Window, WindowEvent};

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("Soliloquy Desktop v0.1");

    // Initialize optimizations
    let settings = OptimizationSettings::desktop();
    init_optimizations(&settings);

    // Initialize V8 runtime first
    info!("Initializing V8 JavaScript runtime...");
    let v8_runtime = match v8_runtime::V8Runtime::new() {
        Ok(runtime) => {
            info!(
                "V8 runtime initialized: version {}",
                v8_runtime::V8Runtime::get_version()
            );
            runtime
        }
        Err(e) => {
            error!("Failed to initialize V8 runtime: {}", e);
            return;
        }
    };

    // Create engine bridge
    info!("Creating Servo + V8 engine bridge...");
    let engine_bridge = match EngineBridge::new(v8_runtime) {
        Ok(bridge) => {
            info!("Engine bridge created successfully");
            bridge
        }
        Err(e) => {
            error!("Failed to create engine bridge: {}", e);
            return;
        }
    };

    // Initialize bridge (sets up DOM bindings, Web APIs, Soliloquy APIs)
    if let Err(e) = engine_bridge.initialize() {
        error!("Failed to initialize engine bridge: {}", e);
        return;
    }
    info!("Engine bridge initialized with DOM, Web APIs, and Soliloquy APIs");

    // Test V8 with desktop APIs
    match engine_bridge.execute_script("soliloquy.version") {
        Ok(version) => info!("Soliloquy API version: {}", version),
        Err(e) => warn!("Failed to query Soliloquy version: {}", e),
    }

    // Create native window
    #[cfg(feature = "desktop")]
    {
        info!("Creating native window...");

        #[cfg(target_os = "linux")]
        {
            use platform::linux::LinuxWindow;

            let window = match LinuxWindow::new() {
                Ok(w) => {
                    info!("Linux window created ({}x{})", w.size().0, w.size().1);
                    info!("Display server: {:?}", w.display_server());
                    w
                }
                Err(e) => {
                    error!("Failed to create Linux window: {}", e);
                    return;
                }
            };

            // Create frame pacer for 60 FPS
            let mut frame_pacer = FramePacer::new(settings.render.target_fps);
            let mut frame_count: u64 = 0;

            info!(
                "Starting main event loop at {} FPS target",
                settings.render.target_fps
            );

            // Run the event loop
            if let Err(e) = window.run(move |event, _window| {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Close requested, shutting down...");
                        info!("Total frames rendered: {}", frame_count);
                        info!("Average FPS: {:.1}", frame_pacer.current_fps());
                    }

                    WindowEvent::Resized { width, height } => {
                        debug!("Window resized to {}x{}", width, height);
                        // TODO: Notify Servo of resize
                    }

                    WindowEvent::RedrawRequested => {
                        // Begin frame timing
                        let _delta = frame_pacer.begin_frame();

                        // TODO: Render Servo content here
                        // servo.render();

                        // Wait for frame pacing
                        frame_pacer.wait_for_frame();
                        frame_count = frame_pacer.frame_count();

                        // Log FPS every second
                        if frame_count % 60 == 0 {
                            debug!("FPS: {:.1}", frame_pacer.current_fps());
                        }
                    }

                    WindowEvent::MouseInput {
                        x,
                        y,
                        button,
                        pressed,
                    } => {
                        debug!(
                            "Mouse {:?} at ({}, {}): {:?}",
                            if pressed { "pressed" } else { "released" },
                            x,
                            y,
                            button
                        );
                    }

                    WindowEvent::MouseMoved { x, y } => {
                        // High-frequency event, don't log
                    }

                    WindowEvent::KeyboardInput { key, pressed, .. } => {
                        if pressed {
                            debug!("Key pressed: {}", key);
                        }
                    }

                    WindowEvent::Focused => {
                        debug!("Window focused");
                    }

                    WindowEvent::Unfocused => {
                        debug!("Window unfocused");
                    }

                    _ => {}
                }
            }) {
                error!("Event loop error: {}", e);
            }
        }

        #[cfg(target_os = "macos")]
        {
            use platform::macos::MacOSWindow;

            let window = match MacOSWindow::new() {
                Ok(w) => {
                    info!("macOS window created ({}x{})", w.size().0, w.size().1);
                    if MacOSWindow::is_apple_silicon() {
                        info!("Running on Apple Silicon");
                    }
                    w
                }
                Err(e) => {
                    error!("Failed to create macOS window: {}", e);
                    return;
                }
            };

            info!("macOS event loop not yet implemented");
            // TODO: Implement macOS event loop similar to Linux
        }
    }

    #[cfg(not(feature = "desktop"))]
    {
        error!("Desktop feature not enabled. Build with: cargo build --features desktop");
    }

    info!("Soliloquy Desktop Shell shutdown complete");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_initialization() {
        let runtime = v8_runtime::V8Runtime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_engine_bridge() {
        let runtime = v8_runtime::V8Runtime::new().unwrap();
        let bridge = EngineBridge::new(runtime);
        assert!(bridge.is_ok());
    }
}
