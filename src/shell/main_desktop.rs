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

#![allow(dead_code, unused_imports)]

#[cfg(not(feature = "gpui"))]
mod browser_optimizations;
#[cfg(not(feature = "gpui"))]
mod engine_bridge;
#[cfg(feature = "gpui")]
mod gpui_app;
#[cfg(not(feature = "gpui"))]
mod js_engine;
mod optimizations;
#[cfg(not(feature = "gpui"))]
mod platform;
#[cfg(not(feature = "gpui"))]
mod servo_embedder;
#[cfg(not(feature = "gpui"))]
mod v8_runtime;

#[cfg(not(feature = "gpui"))]
use engine_bridge::EngineBridge;
use log::info;
#[cfg(not(feature = "gpui"))]
use log::{debug, error, warn};
#[cfg(not(feature = "gpui"))]
use optimizations::FramePacer;
use optimizations::{init_optimizations, OptimizationSettings};
#[cfg(not(feature = "gpui"))]
use soliloquy_browser_optimizations::runtime::{
    EngineRuntime, InputEvent as RuntimeInputEvent, LifecycleEvent, PlatformTier, SurfaceDescriptor,
};
#[cfg(not(feature = "gpui"))]
use std::sync::Arc;

#[cfg(all(feature = "desktop", not(feature = "gpui")))]
use platform::{Window, WindowEvent};

fn start_daemon() {
    // Start the DE daemon in background unless SOLILOQUY_NO_DAEMON is set.
    if std::env::var("SOLILOQUY_NO_DAEMON").is_ok() {
        info!("DE daemon disabled by SOLILOQUY_NO_DAEMON");
        return;
    }
    let port = if let Ok(v) = std::env::var("SOLILOQUY_DAEMON_PORT") {
        v.parse::<u16>().unwrap_or(9842)
    } else if let Ok(v) = std::env::var("SOLD_BIND") {
        v.split(':')
            .nth(1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9842)
    } else {
        9842
    };
    std::thread::spawn(move || {
        let daemon_path = std::env::current_exe()
            .ok()
            .map(|p| {
                let mut d = p.clone();
                d.set_file_name("soliloquy-daemon");
                d
            })
            .unwrap_or_else(|| std::path::PathBuf::from("soliloquy-daemon"));
        info!(
            "Launching DE daemon: {} --port {}",
            daemon_path.display(),
            port
        );
        let _ = std::process::Command::new(&daemon_path)
            .arg("--port")
            .arg(port.to_string())
            .spawn();
    });
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("Soliloquy Desktop v0.1");

    // Start DE daemon
    start_daemon();

    // Initialize optimizations
    let settings = OptimizationSettings::desktop();
    init_optimizations(&settings);

    #[cfg(all(feature = "desktop", feature = "gpui"))]
    {
        info!("Launching Crepuscularity GPUI Shell...");
        gpui_app::run();
        info!("Soliloquy Desktop Shell shutdown complete");
        return;
    }

    #[cfg(not(feature = "gpui"))]
    {
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
        run_native_window(engine_bridge, &settings);

        #[cfg(not(feature = "desktop"))]
        {
            error!("Desktop feature not enabled. Build with: cargo build --features desktop");
        }

        info!("Soliloquy Desktop Shell shutdown complete");
    }
}

#[cfg(all(feature = "desktop", not(feature = "gpui")))]
#[allow(unused_variables)]
fn run_native_window(engine_bridge: EngineBridge, settings: &OptimizationSettings) {
    info!("Creating native window...");

    #[cfg(target_os = "linux")]
    {
        use platform::linux::LinuxWindow;
        // ... rest of linux window code ...

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

        let platform_tier = if cfg!(all(target_os = "linux", feature = "mobile")) {
            PlatformTier::Mobile
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            PlatformTier::ArmLinux
        } else {
            PlatformTier::Desktop
        };
        let surface = SurfaceDescriptor::new(1, window.size().0, window.size().1, platform_tier);
        if let Err(e) = engine_bridge.attach_surface(surface) {
            error!("Failed to attach surface: {}", e);
            return;
        }

        let engine_bridge = Arc::new(engine_bridge);
        let runtime_bridge = Arc::clone(&engine_bridge);

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
                    let _ = runtime_bridge.handle_lifecycle(LifecycleEvent::Shutdown);
                    info!("Total frames rendered: {}", frame_count);
                    info!("Average FPS: {:.1}", frame_pacer.current_fps());
                }

                WindowEvent::Resized { width, height } => {
                    debug!("Window resized to {}x{}", width, height);
                    let resized_surface = SurfaceDescriptor::new(1, width, height, platform_tier);
                    if let Err(e) = runtime_bridge.attach_surface(resized_surface) {
                        warn!("Failed to update attached surface after resize: {}", e);
                    }
                }

                WindowEvent::RedrawRequested => {
                    // Begin frame timing
                    let _delta = frame_pacer.begin_frame();

                    if let Err(e) = runtime_bridge
                        .present_frame(soliloquy_browser_optimizations::runtime::SurfaceId(1))
                    {
                        warn!("Failed to present frame through runtime contract: {}", e);
                    }

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
                    let input = if pressed {
                        RuntimeInputEvent::Touch { x, y }
                    } else {
                        RuntimeInputEvent::PointerMove { x, y }
                    };
                    if let Err(e) = runtime_bridge.handle_input(input) {
                        warn!("Failed to forward pointer input: {}", e);
                    }
                }

                WindowEvent::MouseMoved { x, y } => {
                    if let Err(e) =
                        runtime_bridge.handle_input(RuntimeInputEvent::PointerMove { x, y })
                    {
                        warn!("Failed to forward pointer move: {}", e);
                    }
                }

                WindowEvent::KeyboardInput { key, pressed, .. } => {
                    if pressed {
                        debug!("Key pressed: {}", key);
                        if let Err(e) =
                            runtime_bridge.handle_input(RuntimeInputEvent::Key { code: key })
                        {
                            warn!("Failed to forward key input: {}", e);
                        }
                    }
                }

                WindowEvent::Focused => {
                    debug!("Window focused");
                    let _ = runtime_bridge.handle_lifecycle(LifecycleEvent::Foregrounded);
                }

                WindowEvent::Unfocused => {
                    debug!("Window unfocused");
                    let _ = runtime_bridge.handle_lifecycle(LifecycleEvent::Backgrounded);
                }

                WindowEvent::Scroll { delta_x, delta_y } => {
                    if let Err(e) =
                        runtime_bridge.handle_input(RuntimeInputEvent::Scroll { delta_x, delta_y })
                    {
                        warn!("Failed to forward scroll: {}", e);
                    }
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

        let platform_tier = PlatformTier::Desktop;
        let surface = SurfaceDescriptor::new(1, window.size().0, window.size().1, platform_tier);
        if let Err(e) = engine_bridge.attach_surface(surface) {
            error!("Failed to attach macOS surface: {}", e);
            return;
        }

        let runtime_bridge = Arc::new(engine_bridge);
        let mut frame_pacer = FramePacer::new(settings.render.target_fps);
        let frame_limit = std::env::var("SOLILOQUY_DESKTOP_FRAME_LIMIT")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1);

        info!(
            "Starting macOS frame pump at {} FPS target for {} frame(s)",
            settings.render.target_fps, frame_limit
        );

        for _ in 0..frame_limit {
            let _delta = frame_pacer.begin_frame();

            if let Err(e) =
                runtime_bridge.present_frame(soliloquy_browser_optimizations::runtime::SurfaceId(1))
            {
                warn!(
                    "Failed to present macOS frame through runtime contract: {}",
                    e
                );
            }

            window.request_redraw();
            frame_pacer.wait_for_frame();
        }

        info!("Total frames rendered: {}", frame_pacer.frame_count());
        info!("Average FPS: {:.1}", frame_pacer.current_fps());
    }
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
