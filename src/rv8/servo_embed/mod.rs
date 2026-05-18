//! Servo embedding for RV8
//!
//! This module provides an interface to integrate Servo's rendering
//! capabilities while using V8 for JavaScript execution.
//!
//! Architecture:
//! - Servo handles HTML/CSS parsing, layout, and painting
//! - V8 (from rv8) handles JavaScript execution
//! - This module bridges the two engines

use log::{debug, error, info};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::js::bindings::V8ContextData;
use crate::js::{JsEngine, JsValue};
use crate::renderer::RenderFrame;

pub mod dom;
#[cfg(feature = "software-paint")]
pub mod paint;
pub mod parser;
#[cfg(feature = "servo-render")]
mod servo_renderer;
pub mod web_apis;

use self::dom::{DomEvent, DomTree};
use self::web_apis::{ConsoleApi, StorageApi, TimerManager};

/// Servo embedding configuration
#[derive(Debug, Clone)]
pub struct ServoConfig {
    /// Initial viewport width
    pub width: u32,
    /// Initial viewport height
    pub height: u32,
    /// Enable hardware acceleration
    pub hardware_acceleration: bool,
    /// Enable WebGL
    pub webgl: bool,
    /// Enable WebGPU
    pub webgpu: bool,
    /// User agent string
    pub user_agent: String,
    /// Resource directory path
    pub resources_path: Option<String>,
}

impl Default for ServoConfig {
    fn default() -> Self {
        ServoConfig {
            width: 1280,
            height: 800,
            hardware_acceleration: true,
            webgl: true,
            webgpu: false,
            user_agent: crate::user_agent(),
            resources_path: None,
        }
    }
}

/// Servo embedder for RV8
pub struct ServoEmbedder {
    /// Configuration
    config: ServoConfig,
    /// V8 JavaScript engine
    pub js_engine: Arc<Mutex<JsEngine>>,
    /// DOM Tree
    dom_tree: Arc<RwLock<DomTree>>,
    /// Console API
    console_api: Arc<RwLock<ConsoleApi>>,
    /// Timer Manager
    timer_manager: Arc<RwLock<TimerManager>>,
    /// Local Storage
    local_storage: Arc<RwLock<StorageApi>>,
    /// Session Storage
    session_storage: Arc<RwLock<StorageApi>>,
    /// Current document URL
    current_url: String,
    /// Document title
    title: String,
    /// Is currently loading
    loading: bool,
    /// Load progress (0-100)
    load_progress: u8,
    /// Monotonic frame id for embed consumers
    frame_generation: u64,
    #[cfg(feature = "servo-render")]
    servo: Option<servo_renderer::ServoRenderer>,
}

impl ServoEmbedder {
    /// Create a new Servo embedder
    pub async fn new(config: ServoConfig) -> Result<Self, String> {
        info!("Initializing Servo embedder with V8");

        let mut js_engine =
            JsEngine::new().map_err(|e| format!("Failed to create V8 engine: {}", e))?;

        info!("V8 JavaScript engine version: {}", JsEngine::version());

        let dom_tree = Arc::new(RwLock::new(DomTree::new()));
        let console_api = Arc::new(RwLock::new(ConsoleApi::new()));
        let timer_manager = Arc::new(RwLock::new(TimerManager::new()));
        let local_storage = Arc::new(RwLock::new(StorageApi::new(5 * 1024 * 1024)));
        let session_storage = Arc::new(RwLock::new(StorageApi::new(5 * 1024 * 1024)));

        // Initialize JsEngine with DOM and Web APIs
        js_engine.initialize(V8ContextData::new(
            dom_tree.clone(),
            console_api.clone(),
            timer_manager.clone(),
            local_storage.clone(),
            session_storage.clone(),
        ));

        #[cfg(feature = "servo-render")]
        let servo = Some(
            servo_renderer::ServoRenderer::new(config.width, config.height)
                .map_err(|e| format!("Servo renderer init failed: {e}"))?,
        );

        Ok(ServoEmbedder {
            config,
            js_engine: Arc::new(Mutex::new(js_engine)),
            dom_tree,
            console_api,
            timer_manager,
            local_storage,
            session_storage,
            current_url: String::new(),
            title: String::new(),
            loading: false,
            load_progress: 0,
            frame_generation: 0,
            #[cfg(feature = "servo-render")]
            servo,
        })
    }

    /// Navigate to a URL
    pub async fn navigate(&mut self, url: &str) -> Result<(), String> {
        info!("Navigating to: {}", url);

        self.current_url = url.to_string();
        self.loading = true;
        self.load_progress = 0;

        #[cfg(feature = "servo-render")]
        {
            let servo = self
                .servo
                .as_mut()
                .ok_or_else(|| "Servo renderer not initialized".to_string())?;
            let result = tokio::task::block_in_place(|| servo.navigate(url));
            self.loading = false;
            self.load_progress = 100;
            self.frame_generation = self.frame_generation.saturating_add(1);
            self.title = servo.title();
            return result;
        }

        #[cfg(feature = "software-paint")]
        {
        // Fetch HTML
        info!("Fetching URL: {}", url);
        match reqwest::get(url).await {
            Ok(response) => {
                if !response.status().is_success() {
                    error!("Failed to fetch URL {}: Status {}", url, response.status());
                    self.loading = false;
                    return Err(format!("HTTP error: {}", response.status()));
                }

                match response.text().await {
                    Ok(html) => {
                        info!("Parsing HTML...");
                        self.load_progress = 50;

                        {
                            let mut dom = self.dom_tree.write();
                            // Reset DOM tree
                            *dom = DomTree::new();

                            parser::parse_html(&html, &mut *dom);
                            self.title = dom
                                .document_title()
                                .filter(|t| !t.is_empty())
                                .unwrap_or_else(|| paint::display_host(url));
                        }
                        info!("HTML parsing complete");
                    }
                    Err(e) => {
                        error!("Failed to read response text: {}", e);
                        self.loading = false;
                        return Err(format!("Failed to read response: {}", e));
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch URL {}: {}", url, e);
                self.loading = false;
                return Err(format!("Network error: {}", e));
            }
        }

        // Execute any inline scripts via V8
        // self.execute_document_scripts().await?;

        self.loading = false;
        self.load_progress = 100;
        self.frame_generation = self.frame_generation.saturating_add(1);

        Ok(())
        }

        #[cfg(all(not(feature = "servo-render"), not(feature = "software-paint")))]
        {
            let _ = url;
            Err("rv8 built without servo-render or software-paint".to_string())
        }
    }

    /// Execute JavaScript in the context of the current document
    pub async fn execute_script(&self, script: &str) -> Result<String, String> {
        let mut engine = self.js_engine.lock().await;
        engine.execute_to_string(script)
    }

    /// Execute JavaScript and return a typed transport value.
    pub async fn execute_script_value(&self, script: &str) -> Result<JsValue, String> {
        let mut engine = self.js_engine.lock().await;
        engine.execute(script)
    }

    /// Get the current render frame from Servo (or software paint fallback).
    pub fn get_render_frame(&mut self) -> Option<RenderFrame> {
        #[cfg(feature = "servo-render")]
        if let Some(servo) = self.servo.as_mut() {
            return servo.capture_frame(self.frame_generation);
        }

        #[cfg(feature = "software-paint")]
        {
            let mut frame = RenderFrame::new(self.config.width, self.config.height);
            frame.id = self.frame_generation;
            let dom = self.dom_tree.read();
            paint::paint_document_frame(
                &mut frame,
                &dom,
                &paint::PaintContext {
                    url: &self.current_url,
                    title: &self.title,
                    loading: self.loading,
                },
            );
            return Some(frame);
        }

        #[allow(unreachable_code)]
        None
    }

    pub fn frame_generation(&self) -> u64 {
        self.frame_generation
    }

    /// Resize the viewport
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.frame_generation = self.frame_generation.saturating_add(1);
        #[cfg(feature = "servo-render")]
        if let Some(servo) = self.servo.as_mut() {
            servo.resize(width, height);
        }
        debug!("Viewport resized to {}x{}", width, height);
    }

    /// Handle mouse event
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) {
        // TODO: Forward to Servo's event handling
        debug!("Mouse move: ({}, {})", x, y);
    }

    /// Handle mouse click
    pub async fn handle_mouse_click(&mut self, x: f32, y: f32, button: MouseButton) {
        debug!("Mouse click: ({}, {}) button={:?}", x, y, button);
        let target_id = self.dom_tree.read().document_id();
        let event = DomEvent::mouse("click", target_id, x, y, button);
        self.dom_tree.write().record_event(event.clone());
        let mut engine = self.js_engine.lock().await;
        engine.dispatch_event(&event);
    }

    /// Handle key event
    pub async fn handle_key(&mut self, key: &str, pressed: bool) {
        debug!("Key event: {} pressed={}", key, pressed);
        let target_id = self.dom_tree.read().document_id();
        let event_type = if pressed { "keydown" } else { "keyup" };
        let event = DomEvent::key(event_type, target_id, key);
        self.dom_tree.write().record_event(event.clone());
        let mut engine = self.js_engine.lock().await;
        engine.dispatch_event(&event);
    }

    /// Handle scroll event
    pub fn handle_scroll(&mut self, delta_x: f32, delta_y: f32) {
        // TODO: Forward to Servo's event handling
        debug!("Scroll: ({}, {})", delta_x, delta_y);
    }

    /// Poll and execute ready timers
    pub async fn poll_timers(&self) {
        let ready_timers = {
            let mut manager = self.timer_manager.write();
            manager.poll_ready_timers()
        };

        if !ready_timers.is_empty() {
            let mut engine = self.js_engine.lock().await;
            for timer in ready_timers {
                engine.call_timer_callback(timer.id);
            }
        }
    }

    // Getters
    pub fn current_url(&self) -> &str {
        &self.current_url
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn is_loading(&self) -> bool {
        self.loading
    }
    pub fn load_progress(&self) -> u8 {
        self.load_progress
    }
    pub fn viewport_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}

/// Mouse button type
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

/// DOM element representation
#[derive(Debug, Clone)]
pub struct DomElement {
    /// Tag name (e.g., "div", "p", "a")
    pub tag: String,
    /// Element ID
    pub id: Option<String>,
    /// CSS classes
    pub classes: Vec<String>,
    /// Bounding box (x, y, width, height)
    pub bounds: (f32, f32, f32, f32),
}

/// Document information
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    /// Document URL
    pub url: String,
    /// Document title
    pub title: String,
    /// Content type
    pub content_type: String,
    /// Character encoding
    pub charset: String,
    /// Is secure (HTTPS)
    pub secure: bool,
}
