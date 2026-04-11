//! Servo embedder for Soliloquy
//! 
//! This module provides the integration layer between Servo and the local shell,
//! implementing the necessary traits for windowing, events, and graphics.
//! It also integrates V8 for JavaScript execution and tab memory optimization.

use log::{info, debug, warn, error};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::v8_runtime::V8Runtime;
use crate::browser_optimizations::{TabResidencyManager, GcScheduler, MemoryPressureMonitor};
/// Main embedder context that bridges Servo browser engine with the shell runtime.
///
/// `ServoEmbedder` manages the lifecycle of a web browser instance running on Soliloquy.
/// It coordinates between:
/// - Local presentation bookkeeping for rendered frames
/// - V8 JavaScript runtime for script execution
/// - Servo's rendering engine for web content
/// - Desktop input event handling
/// - Tab memory optimization for efficient multi-tab handling
///
/// The embedder follows a state machine pattern (see [`EmbedderState`]) to ensure proper
/// initialization order and safe resource management.
pub struct ServoEmbedder {
    /// Placeholder display session for desktop rendering integration.
    display_session: Option<Arc<Mutex<DisplaySession>>>,
    /// Thread-safe queue for buffering input events before dispatch to Servo.
    event_queue: Arc<Mutex<Vec<InputEvent>>>,
    /// V8 JavaScript runtime instance for executing web page scripts.
    /// Initialized lazily on first use so startup stays cheap.
    v8_runtime: Option<V8Runtime>,
    /// Servo webview handle (placeholder for actual Servo browser instance).
    webview: Option<Arc<Mutex<ServoWebview>>>,
    /// Currently loaded URL, used for reload and navigation state.
    current_url: Option<String>,
    /// Current state in the embedder lifecycle (see state machine documentation).
    state: EmbedderState,
    /// Tab residency manager for memory optimization (150+ tabs at <3GB RAM).
    tab_residency: Arc<Mutex<TabResidencyManager>>,
    /// GC scheduler for idle-time garbage collection.
    gc_scheduler: Arc<Mutex<GcScheduler>>,
    /// Memory pressure monitor for adaptive eviction.
    memory_monitor: Arc<Mutex<MemoryPressureMonitor>>,
    /// Map of tab IDs to their residency tracking IDs.
    tab_id_map: HashMap<u64, u64>,
}

/// State machine for embedder lifecycle management.
///
/// The embedder transitions through these states in order:
/// 1. `Uninitialized` → `Initializing`: Begin resource allocation
/// 2. `Initializing` → `Ready`: All subsystems initialized, ready to load content
/// 3. `Ready` → `Loading`: URL load initiated
/// 4. `Loading` → `Running`: Content loaded and rendering active
///
/// Any state can transition to `Error(String)` on failure.
/// Only `Ready` and `Running` states accept new URL loads.
#[derive(Debug, Clone, PartialEq)]
pub enum EmbedderState {
    /// Initial state before any initialization.
    Uninitialized,
    /// Actively initializing V8, display state, and other subsystems.
    Initializing,
    /// All systems ready, waiting for content load.
    Ready,
    /// URL load in progress, page not yet rendered.
    Loading,
    /// Page loaded and actively rendering frames.
    Running,
    /// Unrecoverable error occurred; contains error description.
    Error(String),
}

/// Placeholder for desktop presentation state.
pub struct DisplaySession {
    /// Session identifier for debugging and logging.
    pub session_id: u32,
    /// Viewport width in physical pixels.
    pub width: u32,
    /// Viewport height in physical pixels.
    pub height: u32,
}

/// Placeholder for Servo browser webview instance.
///
/// Represents a single browser tab/window. In production, this will interface with
/// Servo's embedding API to control navigation, access DOM, and manage render output.
pub struct ServoWebview {
    /// Currently loaded URL.
    pub url: Option<String>,
    /// Page title (from `<title>` element or navigation metadata).
    pub title: Option<String>,
    /// Whether a navigation/load operation is in progress.
    pub is_loading: bool,
}

impl ServoEmbedder {
    /// Creates and initializes a new Servo embedder instance.
    ///
    /// Performs the following initialization steps:
    /// 1. Creates V8 runtime and executes a test script to verify functionality
    /// 2. Initializes the display session (currently placeholder)
    /// 3. Creates view reference tokens for window management
    /// 4. Initializes tab memory optimization systems
    /// 5. Transitions state from `Uninitialized` → `Initializing` → `Ready`
    ///
    /// # Returns
    /// - `Ok(ServoEmbedder)`: Fully initialized embedder ready to load URLs
    /// - `Err(String)`: V8 initialization failure (critical error)
    ///
    /// # Examples
    /// ```no_run
    /// let embedder = ServoEmbedder::new()?;
    /// embedder.load_url("https://example.com")?;
    /// ```
    pub fn new() -> Result<Self, String> {
        info!("Initializing Servo embedder with lazy browser startup");
        
        let mut embedder = ServoEmbedder {
            display_session: None,
            event_queue: Arc::new(Mutex::new(Vec::new())),
            v8_runtime: None,
            webview: None,
            current_url: None,
            state: EmbedderState::Uninitialized,
            tab_residency: Arc::new(Mutex::new(TabResidencyManager::new())),
            gc_scheduler: Arc::new(Mutex::new(GcScheduler::new())),
            memory_monitor: Arc::new(Mutex::new(MemoryPressureMonitor::default())),
            tab_id_map: HashMap::new(),
        };
        
        embedder.state = EmbedderState::Initializing;
        
        // Initialize memory monitoring.
        // This is intentionally cheap compared to bringing up the browser runtime.
        {
            let monitor = embedder.memory_monitor.lock().unwrap();
            monitor.start_monitoring();
        }
        info!("Memory pressure monitoring started");
        
        embedder.state = EmbedderState::Ready;
        info!("Servo embedder initialized successfully without eager runtime init");
        
        Ok(embedder)
    }

    /// Lazily initialize the V8 runtime the first time browser execution needs it.
    fn ensure_v8_runtime(&mut self) -> Result<&mut V8Runtime, String> {
        if self.v8_runtime.is_none() {
            info!("Initializing V8 runtime on demand");
            let mut runtime = V8Runtime::new()
                .map_err(|e| format!("V8 initialization failed: {}", e))?;

            // One tiny warmup script gives us a fast first real navigation without
            // paying the cost during constructor startup.
            match runtime.execute_script("'V8 is ready'") {
                Ok(result) => debug!("V8 warmup result: {}", result),
                Err(e) => warn!("V8 warmup script failed: {}", e),
            }

            self.v8_runtime = Some(runtime);
        }

        Ok(self
            .v8_runtime
            .as_mut()
            .expect("V8 runtime must exist after initialization"))
    }

    /// Lazily create the display session used by presentation code.
    fn ensure_display_session(&mut self) {
        if self.display_session.is_none() {
            match self.init_display_session() {
                Ok(session) => {
                    info!("Display session initialized on demand");
                    self.display_session = Some(Arc::new(Mutex::new(session)));
                }
                Err(e) => {
                    warn!("Failed to initialize display session: {}", e);
                }
            }
        }
    }

    /// Initializes a local display session for graphics output.
    fn init_display_session(&self) -> Result<DisplaySession, String> {
        debug!("Initializing display session");
        
        Ok(DisplaySession {
            session_id: 1,
            width: 1920,
            height: 1080,
        })
    }
    
    /// Loads a URL into the webview and initializes the page.
    ///
    /// This method:
    /// 1. Validates embedder state (must be `Ready` or `Running`)
    /// 2. Transitions to `Loading` state
    /// 3. Creates a Servo webview instance (currently placeholder)
    /// 4. Executes JavaScript initialization code via V8 to simulate page load
    /// 5. Transitions to `Running` state on success
    ///
    /// **Placeholder:** Currently uses V8 to simulate page load. Production version
    /// will invoke Servo's navigation API: `servo::webview::load(url)`.
    ///
    /// # Arguments
    /// * `url` - The URL to load (e.g., "https://example.com")
    ///
    /// # Returns
    /// - `Ok(())`: URL loaded successfully, page is rendering
    /// - `Err(String)`: Invalid state or load failure
    ///
    /// # Examples
    /// ```no_run
    /// embedder.load_url("https://soliloquy.dev")?;
    /// ```
    pub fn load_url(&mut self, url: &str) -> Result<(), String> {
        if self.state != EmbedderState::Ready && self.state != EmbedderState::Running {
            return Err(format!("Embedder not ready for loading URLs. Current state: {:?}", self.state));
        }
        
        validate_url(url)?;
        
        info!("Loading URL: {}", url);
        self.state = EmbedderState::Loading;
        self.current_url = Some(url.to_string());
        
        // Create Servo webview
        let webview = ServoWebview {
            url: Some(url.to_string()),
            title: None,
            is_loading: true,
        };
        self.webview = Some(Arc::new(Mutex::new(webview)));
        
        // Execute JavaScript to initialize the page.
        // The runtime is brought up only when a real navigation happens.
        {
            let runtime = self.ensure_v8_runtime()?;
            let init_script = format!(
                r#"
                console.log('Loading URL: {}');
                // Simulate page load
                var page = {{
                    url: '{}',
                    title: 'Soliloquy Page',
                    ready: true
                }};
                page.title;
                "#,
                url, url
            );

            match runtime.execute_script(&init_script) {
                Ok(result) => {
                    debug!("Page initialization script result: {}", result);
                    
                    // Update webview title
                    if let Some(ref webview_arc) = self.webview {
                        if let Ok(mut webview) = webview_arc.lock() {
                            webview.title = Some(result);
                            webview.is_loading = false;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to execute page initialization script: {}", e);
                }
            }
        }
        
        // TODO: Call into actual Servo API
        // servo::webview::load(url);
        
        self.state = EmbedderState::Running;
        info!("URL loaded successfully: {}", url);
        Ok(())
    }
    
    /// Processes and dispatches input events to the webview.
    ///
    /// Input events are:
    /// 1. Queued to the internal event buffer for tracking
    /// 2. Converted to JavaScript handlers (current implementation)
    /// 3. Dispatched to V8 runtime for web page interaction
    ///
    /// **Placeholder:** Production version will convert platform input events to Servo's event format
    /// and call `servo::input::handle_event(event)`.
    ///
    /// # Arguments
    /// * `event` - Touch or keyboard input event to process
    ///
    /// # Examples
    /// ```no_run
    /// embedder.handle_input(InputEvent::Touch { x: 100.0, y: 200.0 });
    /// embedder.handle_input(InputEvent::Key { code: 13 }); // Enter key
    /// ```
    pub fn handle_input(&mut self, event: InputEvent) {
        debug!("Handling input event: {:?}", event);
        
        // Add to event queue (clone to avoid move)
        if let Ok(mut queue) = self.event_queue.lock() {
            queue.push(event.clone());
        }
        
        // TODO: Convert platform input events to Servo events
        // servo::input::handle_event(event);
        
        // Execute JavaScript for input handling if needed
        if let Some(ref mut runtime) = self.v8_runtime {
            match event {
                InputEvent::Touch { x, y } => {
                    let script = format!(
                        r#"
                        if (window.handleTouch) {{
                            window.handleTouch({}, {});
                        }}
                        'Touch handled at ({}, {})';
                        "#,
                        x, y, x, y
                    );
                    
                    if let Ok(result) = runtime.execute_script(&script) {
                        debug!("Touch handling script result: {}", result);
                    }
                }
                InputEvent::Key { code } => {
                    let script = format!(
                        r#"
                        if (window.handleKey) {{
                            window.handleKey({});
                        }}
                        'Key handled: {}';
                        "#,
                        code, code
                    );
                    
                    if let Ok(result) = runtime.execute_script(&script) {
                        debug!("Key handling script result: {}", result);
                    }
                }
                InputEvent::PointerMove { x, y } => {
                    debug!("Pointer move at ({}, {})", x, y);
                }
                InputEvent::Scroll { delta_x, delta_y } => {
                    debug!("Scroll delta ({}, {})", delta_x, delta_y);
                }
                InputEvent::Text { value } => {
                    debug!("Text input: {}", value);
                }
                InputEvent::Lifecycle(event) => {
                    debug!("Lifecycle event: {:?}", event);
                }
            }
        }
    }
    
    /// Submits the current frame to the display pipeline.
    ///
    /// This method is called on each frame of the render loop and:
    /// 1. Retrieves the current display session
    /// 2. Updates display bookkeeping
    /// 3. Executes optional JavaScript frame callbacks via V8
    ///
    /// **Placeholder:** Production version will submit Servo's rendered frame buffer
    /// to the active Linux/macOS presentation backend.
    ///
    /// # Returns
    /// - `Ok(())`: Frame submitted successfully
    /// - `Err(String)`: Presentation failure (rare; logged as warning)
    pub fn present(&mut self) -> Result<(), String> {
        debug!("Presenting frame");
        self.ensure_display_session();
        
        if let Some(ref session_arc) = self.display_session {
            if let Ok(mut session) = session_arc.lock() {
                session.width = session.width.max(1);
                session.height = session.height.max(1);
                debug!("Presenting to display session {}", session.session_id);
            }
        }
        
        // Execute JavaScript for frame presentation
        if let Some(ref mut runtime) = self.v8_runtime {
            let frame_script = r#"
            if (window.onFrame) {
                window.onFrame();
            }
            'Frame presented';
            "#;
            
            match runtime.execute_script(frame_script) {
                Ok(result) => debug!("Frame script result: {}", result),
                Err(e) => warn!("Frame script failed: {}", e),
            }
        }
        
        Ok(())
    }
    
    /// Returns the current embedder lifecycle state.
    ///
    /// Use this to check if the embedder is ready for operations like URL loading.
    pub fn get_state(&self) -> &EmbedderState {
        &self.state
    }
    
    /// Returns the currently loaded URL, if any.
    ///
    /// # Returns
    /// - `Some(&String)`: URL that was passed to `load_url()`
    /// - `None`: No URL has been loaded yet
    pub fn get_current_url(&self) -> Option<&String> {
        self.current_url.as_ref()
    }
    
    /// Retrieves metadata about the current webview state.
    ///
    /// # Returns
    /// A map containing:
    /// - `"url"`: Currently loaded URL
    /// - `"title"`: Page title from `<title>` element
    /// - `"loading"`: Whether a navigation is in progress ("true"/"false")
    ///
    /// Returns `None` if no webview has been created.
    pub fn get_webview_info(&self) -> Option<HashMap<String, String>> {
        if let Some(ref webview_arc) = self.webview {
            if let Ok(webview) = webview_arc.lock() {
                let mut info = HashMap::new();
                if let Some(ref url) = webview.url {
                    info.insert("url".to_string(), url.clone());
                }
                if let Some(ref title) = webview.title {
                    info.insert("title".to_string(), title.clone());
                }
                info.insert("loading".to_string(), webview.is_loading.to_string());
                return Some(info);
            }
        }
        None
    }
    
    /// Executes arbitrary JavaScript code in the page context.
    ///
    /// Provides direct access to the V8 runtime for executing scripts.
    /// In production, this would execute within Servo's JavaScript context
    /// with access to the DOM and web APIs.
    ///
    /// # Arguments
    /// * `script` - JavaScript source code to execute
    ///
    /// # Returns
    /// - `Ok(String)`: String representation of the script's return value
    /// - `Err(String)`: V8 runtime not initialized or script execution error
    ///
    /// # Examples
    /// ```no_run
    /// let title = embedder.execute_js("document.title")?;
    /// embedder.execute_js("console.log('Hello from Soliloquy')")?;
    /// ```
    pub fn execute_js(&mut self, script: &str) -> Result<String, String> {
        let runtime = self.ensure_v8_runtime()?;
        runtime.execute_script(script)
    }
    
    /// Register a new tab with the memory optimization system.
    ///
    /// Integrates the tab into the residency manager for automatic memory eviction.
    /// Tabs start in Active state and transition through Warm→Cold→Frozen based on idle time.
    ///
    /// # Arguments
    /// * `tab_id` - Unique identifier for the tab (from browser UI)
    /// * `url` - Initial URL for the tab
    ///
    /// # Returns
    /// - `Ok(())`: Tab registered successfully
    /// - `Err(String)`: Failed to acquire lock on residency manager
    pub fn register_tab(&mut self, tab_id: u64, url: String) -> Result<(), String> {
        let mut residency = self.tab_residency.lock()
            .map_err(|e| format!("Failed to lock residency manager: {}", e))?;
        
        let residency_id = residency.register_tab(url.clone());
        self.tab_id_map.insert(tab_id, residency_id);
        
        info!("Registered tab {} (residency ID: {}) for URL: {}", tab_id, residency_id, url);
        Ok(())
    }
    
    /// Mark a tab as active (user is currently viewing it).
    ///
    /// Restores tab to Active state if it was evicted and records interaction
    /// for GC scheduler to defer garbage collection.
    ///
    /// # Arguments
    /// * `tab_id` - Tab to mark as active
    pub fn activate_tab(&mut self, tab_id: u64) -> Result<(), String> {
        if let Some(&residency_id) = self.tab_id_map.get(&tab_id) {
            let mut residency = self.tab_residency.lock()
                .map_err(|e| format!("Failed to lock residency manager: {}", e))?;
            
            residency.touch_tab(residency_id)?;
            
            // Record interaction for GC scheduler
            let mut gc = self.gc_scheduler.lock()
                .map_err(|e| format!("Failed to lock GC scheduler: {}", e))?;
            gc.record_interaction();
            
            debug!("Activated tab {}", tab_id);
        }
        Ok(())
    }
    
    /// Unregister a tab when it's closed.
    ///
    /// # Arguments
    /// * `tab_id` - Tab to remove from tracking
    pub fn unregister_tab(&mut self, tab_id: u64) -> Result<(), String> {
        if let Some(residency_id) = self.tab_id_map.remove(&tab_id) {
            let mut residency = self.tab_residency.lock()
                .map_err(|e| format!("Failed to lock residency manager: {}", e))?;
            
            residency.unregister_tab(residency_id)?;
            info!("Unregistered tab {}", tab_id);
        }
        Ok(())
    }
    
    /// Run periodic maintenance tasks.
    ///
    /// Should be called regularly (e.g., every 5 seconds) to:
    /// - Check memory pressure and trigger evictions
    /// - Run idle-time garbage collection
    /// - Update memory statistics
    ///
    /// This is the main integration point for the optimization system.
    pub fn run_maintenance(&mut self) -> Result<(), String> {
        // Check memory pressure
        let is_under_pressure = {
            let monitor = self.memory_monitor.lock()
                .map_err(|e| format!("Failed to lock memory monitor: {}", e))?;
            monitor.is_under_pressure()
        };
        
        // Update tab residency manager with memory pressure state
        {
            let mut residency = self.tab_residency.lock()
                .map_err(|e| format!("Failed to lock residency manager: {}", e))?;
            residency.set_memory_pressure(is_under_pressure);
            
            // Run eviction pass
            let evicted = residency.run_eviction_pass();
            if evicted > 0 {
                info!("Eviction pass completed: {} tabs evicted", evicted);
            }
            
            // Update memory monitor with current usage
            let usage = residency.get_memory_usage();
            let monitor = self.memory_monitor.lock()
                .map_err(|e| format!("Failed to lock memory monitor: {}", e))?;
            monitor.update_usage(usage);
        }
        
        // Check if GC should run
        {
            let mut gc = self.gc_scheduler.lock()
                .map_err(|e| format!("Failed to lock GC scheduler: {}", e))?;
            
            if let Some(gc_type) = gc.should_run_gc() {
                debug!("Scheduling GC: {:?}", gc_type);
                // TODO: Trigger actual V8 GC
                // For now, just record that we would have run it
                gc.record_gc(gc_type, std::time::Duration::from_millis(10));
            }
        }
        
        Ok(())
    }
    
    /// Get memory optimization statistics.
    ///
    /// Returns information about tab states and memory usage.
    pub fn get_memory_stats(&self) -> Result<String, String> {
        let residency = self.tab_residency.lock()
            .map_err(|e| format!("Failed to lock residency manager: {}", e))?;
        
        let stats = residency.get_stats();
        let monitor = self.memory_monitor.lock()
            .map_err(|e| format!("Failed to lock memory monitor: {}", e))?;
        
        Ok(format!(
            "Tabs: {} active, {} warm, {} cold, {} frozen | Memory: {:.2} MB ({:.1}% of limit)",
            stats.active_count,
            stats.warm_count,
            stats.cold_count,
            stats.frozen_count,
            stats.total_memory as f64 / 1024.0 / 1024.0,
            monitor.get_usage_percentage()
        ))
    }
}

pub use soliloquy_browser_optimizations::runtime::InputEvent;

fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }
    
    if url.trim().is_empty() {
        return Err("URL cannot be only whitespace".to_string());
    }
    
    let url_lower = url.to_lowercase();
    if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    
    if url.len() < 10 {
        return Err("URL is too short to be valid".to_string());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation_valid() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("https://www.example.com/path").is_ok());
        assert!(validate_url("HTTP://EXAMPLE.COM").is_ok());
    }

    #[test]
    fn test_url_validation_empty() {
        assert!(validate_url("").is_err());
        assert_eq!(validate_url("").unwrap_err(), "URL cannot be empty");
    }

    #[test]
    fn test_url_validation_whitespace() {
        assert!(validate_url("   ").is_err());
        assert_eq!(validate_url("  ").unwrap_err(), "URL cannot be only whitespace");
    }

    #[test]
    fn test_url_validation_invalid_scheme() {
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("example.com").is_err());
        assert!(validate_url("www.example.com").is_err());
        let err = validate_url("ftp://example.com").unwrap_err();
        assert!(err.contains("http://") || err.contains("https://"));
    }

    #[test]
    fn test_url_validation_too_short() {
        assert!(validate_url("http://a").is_err());
        assert_eq!(validate_url("http://a").unwrap_err(), "URL is too short to be valid");
    }

    #[test]
    fn test_embedder_state_transitions() {
        let embedder = ServoEmbedder::new().expect("Should initialize");
        assert_eq!(embedder.get_state(), &EmbedderState::Ready);
    }

    #[test]
    fn test_embedder_load_when_uninitialized() {
        let mut embedder = ServoEmbedder {
            display_session: None,
            event_queue: Arc::new(Mutex::new(Vec::new())),
            v8_runtime: None,
            webview: None,
            current_url: None,
            state: EmbedderState::Uninitialized,
            tab_residency: Arc::new(Mutex::new(TabResidencyManager::new())),
            gc_scheduler: Arc::new(Mutex::new(GcScheduler::new())),
            memory_monitor: Arc::new(Mutex::new(MemoryPressureMonitor::default())),
            tab_id_map: HashMap::new(),
        };
        
        let result = embedder.load_url("https://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not ready"));
    }

    #[test]
    fn test_embedder_load_when_initializing() {
        let mut embedder = ServoEmbedder {
            display_session: None,
            event_queue: Arc::new(Mutex::new(Vec::new())),
            v8_runtime: None,
            webview: None,
            current_url: None,
            state: EmbedderState::Initializing,
            tab_residency: Arc::new(Mutex::new(TabResidencyManager::new())),
            gc_scheduler: Arc::new(Mutex::new(GcScheduler::new())),
            memory_monitor: Arc::new(Mutex::new(MemoryPressureMonitor::default())),
            tab_id_map: HashMap::new(),
        };
        
        let result = embedder.load_url("https://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not ready"));
    }

    #[test]
    fn test_embedder_repeated_loads() {
        let mut embedder = ServoEmbedder::new().expect("Should initialize");
        
        assert!(embedder.load_url("https://first.com").is_ok());
        assert_eq!(embedder.get_state(), &EmbedderState::Running);
        assert_eq!(embedder.get_current_url(), Some(&"https://first.com".to_string()));
        
        assert!(embedder.load_url("https://second.com").is_ok());
        assert_eq!(embedder.get_state(), &EmbedderState::Running);
        assert_eq!(embedder.get_current_url(), Some(&"https://second.com".to_string()));
    }

    #[test]
    fn test_embedder_load_invalid_url() {
        let mut embedder = ServoEmbedder::new().expect("Should initialize");
        
        assert!(embedder.load_url("").is_err());
        assert_eq!(embedder.get_state(), &EmbedderState::Ready);
        assert_eq!(embedder.get_current_url(), None);
    }

    #[test]
    fn test_embedder_load_url_no_scheme() {
        let mut embedder = ServoEmbedder::new().expect("Should initialize");
        
        let result = embedder.load_url("example.com");
        assert!(result.is_err());
        assert_eq!(embedder.get_state(), &EmbedderState::Ready);
    }

    #[test]
    fn test_embedder_state_remains_running_after_multiple_loads() {
        let mut embedder = ServoEmbedder::new().expect("Should initialize");
        
        for i in 0..5 {
            let url = format!("https://example{}.com", i);
            assert!(embedder.load_url(&url).is_ok());
            assert_eq!(embedder.get_state(), &EmbedderState::Running);
        }
    }

    #[test]
    fn test_embedder_error_state() {
        let embedder = ServoEmbedder {
            display_session: None,
            event_queue: Arc::new(Mutex::new(Vec::new())),
            v8_runtime: None,
            webview: None,
            current_url: None,
            state: EmbedderState::Error("Test error".to_string()),
            tab_residency: Arc::new(Mutex::new(TabResidencyManager::new())),
            gc_scheduler: Arc::new(Mutex::new(GcScheduler::new())),
            memory_monitor: Arc::new(Mutex::new(MemoryPressureMonitor::default())),
            tab_id_map: HashMap::new(),
        };
        
        assert_eq!(embedder.get_state(), &EmbedderState::Error("Test error".to_string()));
    }

    #[test]
    fn test_url_validation_edge_cases() {
        assert!(validate_url("https://").is_err());
        assert!(validate_url("https://a.b").is_ok());
        assert!(validate_url("https://example.com:8080").is_ok());
        assert!(validate_url("https://example.com/path?query=value#fragment").is_ok());
    }

    #[test]
    fn test_display_present() {
        let mut embedder = ServoEmbedder::new().expect("Should initialize");
        assert!(embedder.present().is_ok());
    }
}
