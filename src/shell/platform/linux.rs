//! Linux platform windowing implementation using winit
//!
//! Provides a window and graphics context for Linux using:
//! - winit for window creation and event handling
//! - wgpu for Vulkan/OpenGL rendering
//!
//! Supports both X11 and Wayland display servers.

use log::{debug, error, info, warn};
use std::sync::Arc;

#[cfg(feature = "desktop")]
use winit::application::ApplicationHandler;
#[cfg(feature = "desktop")]
use winit::dpi::{LogicalSize, PhysicalSize};
#[cfg(feature = "desktop")]
use winit::event::{
    ElementState, MouseButton as WinitMouseButton, WindowEvent as WinitWindowEvent,
};
#[cfg(feature = "desktop")]
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(feature = "desktop")]
use winit::platform::scancode::PhysicalKeyExtScancode;
#[cfg(feature = "desktop")]
use winit::window::{Window as WinitWindow, WindowId};

use super::{GraphicsConfig, KeyModifiers, MouseButton, Window, WindowEvent};

/// Linux window implementation using winit
pub struct LinuxWindow {
    #[cfg(feature = "desktop")]
    window: Option<Arc<WinitWindow>>,
    width: u32,
    height: u32,
    should_close: bool,
    pending_events: Vec<WindowEvent>,
    graphics_config: GraphicsConfig,
}

impl Window for LinuxWindow {
    fn new() -> Result<Self, String> {
        Self::new_with_size(1920, 1080)
    }

    fn new_with_size(width: u32, height: u32) -> Result<Self, String> {
        info!("Creating Linux window ({}x{})", width, height);

        Ok(LinuxWindow {
            #[cfg(feature = "desktop")]
            window: None, // Will be created when event loop starts
            width,
            height,
            should_close: false,
            pending_events: Vec::new(),
            graphics_config: GraphicsConfig::default(),
        })
    }

    fn present(&self) -> Result<(), String> {
        debug!("Presenting frame on Linux");
        // Frame presentation is handled by wgpu surface
        // This is called after rendering is complete
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) {
        debug!("Resizing Linux window to {}x{}", width, height);
        self.width = width;
        self.height = height;
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_redraw(&self) {
        #[cfg(feature = "desktop")]
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

impl LinuxWindow {
    /// Create a new LinuxWindow with custom graphics configuration
    pub fn new_with_config(
        width: u32,
        height: u32,
        config: GraphicsConfig,
    ) -> Result<Self, String> {
        info!(
            "Creating Linux window with custom config ({}x{})",
            width, height
        );

        Ok(LinuxWindow {
            #[cfg(feature = "desktop")]
            window: None,
            width,
            height,
            should_close: false,
            pending_events: Vec::new(),
            graphics_config: config,
        })
    }

    /// Run the event loop with a callback for each event
    /// This consumes the window and runs until the window is closed
    #[cfg(feature = "desktop")]
    pub fn run<F>(self, mut event_handler: F) -> Result<(), String>
    where
        F: FnMut(WindowEvent, &WinitWindow) + 'static,
    {
        info!("Starting Linux window event loop");

        let event_loop =
            EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;

        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = LinuxApp {
            window: None,
            requested_size: (self.width, self.height),
            event_handler: Box::new(event_handler),
        };

        event_loop
            .run_app(&mut app)
            .map_err(|e| format!("Event loop error: {}", e))?;

        info!("Linux window event loop ended");
        Ok(())
    }

    /// Poll for pending events without blocking
    pub fn poll_events(&mut self) -> Vec<WindowEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Get the raw window handle for graphics integration
    #[cfg(feature = "desktop")]
    pub fn raw_window(&self) -> Option<&Arc<WinitWindow>> {
        self.window.as_ref()
    }

    /// Get the display server type (X11 or Wayland)
    pub fn display_server(&self) -> DisplayServer {
        // Check environment for display server type
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            DisplayServer::Wayland
        } else if std::env::var("DISPLAY").is_ok() {
            DisplayServer::X11
        } else {
            DisplayServer::Unknown
        }
    }
}

/// Linux display server types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

/// Application handler for winit event loop
#[cfg(feature = "desktop")]
struct LinuxApp<F>
where
    F: FnMut(WindowEvent, &WinitWindow) + 'static,
{
    window: Option<Arc<WinitWindow>>,
    requested_size: (u32, u32),
    event_handler: Box<F>,
}

#[cfg(feature = "desktop")]
impl<F> ApplicationHandler for LinuxApp<F>
where
    F: FnMut(WindowEvent, &WinitWindow) + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            info!("Creating window on resume");

            let window_attrs = WinitWindow::default_attributes()
                .with_title("Soliloquy")
                .with_inner_size(LogicalSize::new(
                    self.requested_size.0 as f64,
                    self.requested_size.1 as f64,
                ))
                .with_min_inner_size(LogicalSize::new(800.0, 600.0));

            match event_loop.create_window(window_attrs) {
                Ok(window) => {
                    info!("Window created successfully");
                    self.window = Some(Arc::new(window));
                }
                Err(e) => {
                    error!("Failed to create window: {}", e);
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        match event {
            WinitWindowEvent::CloseRequested => {
                info!("Close requested");
                (self.event_handler)(WindowEvent::CloseRequested, window);
                event_loop.exit();
            }

            WinitWindowEvent::Resized(PhysicalSize { width, height }) => {
                debug!("Window resized to {}x{}", width, height);
                (self.event_handler)(WindowEvent::Resized { width, height }, window);
            }

            WinitWindowEvent::RedrawRequested => {
                (self.event_handler)(WindowEvent::RedrawRequested, window);
            }

            WinitWindowEvent::MouseInput { state, button, .. } => {
                let mouse_button = match button {
                    WinitMouseButton::Left => MouseButton::Left,
                    WinitMouseButton::Right => MouseButton::Right,
                    WinitMouseButton::Middle => MouseButton::Middle,
                    WinitMouseButton::Other(n) => MouseButton::Other(n),
                    _ => MouseButton::Other(0),
                };

                // Note: We'd need to track cursor position separately
                (self.event_handler)(
                    WindowEvent::MouseInput {
                        x: 0.0,
                        y: 0.0,
                        button: mouse_button,
                        pressed: state == ElementState::Pressed,
                    },
                    window,
                );
            }

            WinitWindowEvent::CursorMoved { position, .. } => {
                (self.event_handler)(
                    WindowEvent::MouseMoved {
                        x: position.x as f32,
                        y: position.y as f32,
                    },
                    window,
                );
            }

            WinitWindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                (self.event_handler)(
                    WindowEvent::Scroll {
                        delta_x: dx,
                        delta_y: dy,
                    },
                    window,
                );
            }

            WinitWindowEvent::KeyboardInput { event, .. } => {
                // Convert physical key to u32 code
                let key_code = event.physical_key.to_scancode().unwrap_or(0);

                (self.event_handler)(
                    WindowEvent::KeyboardInput {
                        key: key_code,
                        pressed: event.state == ElementState::Pressed,
                        modifiers: KeyModifiers::default(), // Would need to track modifiers
                    },
                    window,
                );
            }

            WinitWindowEvent::Focused(focused) => {
                if focused {
                    (self.event_handler)(WindowEvent::Focused, window);
                } else {
                    (self.event_handler)(WindowEvent::Unfocused, window);
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_window_creation() {
        let window = LinuxWindow::new();
        assert!(window.is_ok());

        let window = window.unwrap();
        assert_eq!(window.size(), (1920, 1080));
        assert!(!window.should_close());
    }

    #[test]
    fn test_linux_window_resize() {
        let mut window = LinuxWindow::new().unwrap();
        window.resize(1280, 720);
        assert_eq!(window.size(), (1280, 720));
    }

    #[test]
    fn test_display_server_detection() {
        let window = LinuxWindow::new().unwrap();
        let _server = window.display_server();
        // Just check it doesn't panic
    }
}
