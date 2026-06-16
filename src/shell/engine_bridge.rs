//! Engine Bridge - Links Servo rendering with V8 JavaScript runtime
//!
//! This module provides the integration layer for replacing Servo's SpiderMonkey
//! JavaScript engine with V8. This is a significant architectural change that:
//!
//! - Uses Servo purely for HTML/CSS rendering (layout, styling, painting)
//! - Replaces SpiderMonkey entirely with V8 for JavaScript execution
//! - Bridges DOM access from V8 to Servo's internal DOM representation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      V8 Runtime                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │ JavaScript Execution Context                        │   │
//! │  │  - Web APIs (DOM, Fetch, etc.)                      │   │
//! │  │  - Desktop APIs (Soliloquy system integration)      │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └──────────────────────────┬──────────────────────────────────┘
//!                            │
//!                    ┌───────▼───────┐
//!                    │ Engine Bridge │
//!                    │ (DOM Bindings)│
//!                    └───────┬───────┘
//!                            │
//! ┌──────────────────────────▼──────────────────────────────────┐
//! │                   Servo Rendering                           │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │ HTML/CSS Processing → Layout → Paint → Composite    │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use log::{debug, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use soliloquy_browser_optimizations::runtime::{
    EngineRuntime, InputEvent, LifecycleEvent, RuntimeError, SurfaceDescriptor, SurfaceId,
};

use crate::js_engine::{JsEngineStatus, JsEngineSwapStage};
use crate::v8_runtime::V8Runtime;

/// DOM Node types matching Servo's internal representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Element,
    Text,
    Comment,
    Document,
    DocumentFragment,
}

/// Simplified DOM node for V8 binding
#[derive(Debug, Clone)]
pub struct DOMNode {
    pub id: u64,
    pub node_type: NodeType,
    pub tag_name: Option<String>,
    pub text_content: Option<String>,
    pub attributes: HashMap<String, String>,
    pub parent_id: Option<u64>,
    pub child_ids: Vec<u64>,
}

/// DOM mutation types for synchronization
#[derive(Debug, Clone)]
pub enum DOMMutation {
    /// Insert a new node
    InsertNode {
        parent_id: u64,
        node: DOMNode,
        before_id: Option<u64>,
    },
    /// Remove a node
    RemoveNode { node_id: u64 },
    /// Update node text content
    UpdateText { node_id: u64, text: String },
    /// Set attribute
    SetAttribute {
        node_id: u64,
        name: String,
        value: String,
    },
    /// Remove attribute
    RemoveAttribute { node_id: u64, name: String },
    /// Update style
    UpdateStyle {
        node_id: u64,
        property: String,
        value: String,
    },
}

/// Engine bridge connecting Servo rendering with V8 JavaScript
pub struct EngineBridge {
    /// V8 runtime instance
    v8_runtime: Arc<Mutex<V8Runtime>>,

    /// DOM tree mirror for V8 access
    dom_tree: Arc<RwLock<HashMap<u64, DOMNode>>>,

    /// Next available node ID
    next_node_id: Arc<Mutex<u64>>,

    /// Pending DOM mutations to send to Servo
    pending_mutations: Arc<Mutex<Vec<DOMMutation>>>,

    /// Event listeners registered from JavaScript
    event_listeners: EventListeners,

    /// Registered native functions callable from V8
    native_functions: Arc<RwLock<HashMap<String, NativeFunction>>>,

    /// Bridge state
    state: Arc<RwLock<BridgeState>>,

    /// Surfaces attached through the shared runtime contract.
    surfaces: Arc<RwLock<HashMap<SurfaceId, SurfaceDescriptor>>>,

    /// Most recent input event forwarded from the platform.
    last_input_event: Arc<RwLock<Option<InputEvent>>>,

    /// Most recent lifecycle event forwarded from the platform.
    last_lifecycle_event: Arc<RwLock<Option<LifecycleEvent>>>,

    /// Surfaces that have been presented most recently.
    presented_frames: Arc<Mutex<Vec<SurfaceId>>>,

    /// Current JavaScript engine linkage status for the shell.
    js_engine_status: Arc<RwLock<JsEngineStatus>>,
}

/// Native function type for V8 callbacks
pub type NativeFunction = Box<dyn Fn(Vec<String>) -> Result<String, String> + Send + Sync>;
type EventListeners = Arc<RwLock<HashMap<(u64, String), Vec<u32>>>>;

/// Bridge state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeState {
    Uninitialized,
    Initializing,
    Ready,
    Error(String),
}

impl EngineBridge {
    /// Create a new engine bridge with the given V8 runtime
    pub fn new(v8_runtime: V8Runtime) -> Result<Self, String> {
        info!("Creating engine bridge for Servo + V8 integration");
        let js_engine_status = v8_runtime.status();

        let bridge = EngineBridge {
            v8_runtime: Arc::new(Mutex::new(v8_runtime)),
            dom_tree: Arc::new(RwLock::new(HashMap::new())),
            next_node_id: Arc::new(Mutex::new(1)),
            pending_mutations: Arc::new(Mutex::new(Vec::new())),
            event_listeners: Arc::new(RwLock::new(HashMap::new())),
            native_functions: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(BridgeState::Uninitialized)),
            surfaces: Arc::new(RwLock::new(HashMap::new())),
            last_input_event: Arc::new(RwLock::new(None)),
            last_lifecycle_event: Arc::new(RwLock::new(None)),
            presented_frames: Arc::new(Mutex::new(Vec::new())),
            js_engine_status: Arc::new(RwLock::new(js_engine_status)),
        };

        Ok(bridge)
    }

    /// Initialize the bridge and set up V8 bindings
    pub fn initialize(&self) -> Result<(), String> {
        info!("Initializing engine bridge");

        {
            let mut state = self
                .state
                .write()
                .map_err(|e| format!("Failed to lock state: {}", e))?;
            *state = BridgeState::Initializing;
        }

        // Initialize DOM bindings in V8
        self.init_dom_bindings()?;

        // Initialize Web APIs
        self.init_web_apis()?;

        // Initialize Soliloquy-specific APIs
        self.init_soliloquy_apis()?;

        {
            let mut state = self
                .state
                .write()
                .map_err(|e| format!("Failed to lock state: {}", e))?;
            *state = BridgeState::Ready;
        }

        info!("Engine bridge initialized successfully");
        Ok(())
    }

    /// Initialize DOM bindings in V8
    fn init_dom_bindings(&self) -> Result<(), String> {
        debug!("Initializing DOM bindings in V8");

        self.init_node_binding()?;
        self.init_style_binding()?;
        self.init_element_binding()?;
        self.init_document_binding()?;

        let global_script = r#"
            // Global node registry
            const __nodeRegistry = new Map();

            function __registerNode(nodeId, node) {
                __nodeRegistry.set(nodeId, node);
            }

            function __getNodeById(nodeId) {
                return __nodeRegistry.get(nodeId) || null;
            }

            // Create global document
            globalThis.document = new Document();

            console.log('DOM bindings initialized');
            'DOM bindings ready';
        "#;

        self.execute_script(global_script)?;
        Ok(())
    }

    fn init_node_binding(&self) -> Result<(), String> {
        let script = r#"
            // DOM Node implementation
            class Node {
                constructor(nodeId, nodeType) {
                    this._nodeId = nodeId;
                    this._nodeType = nodeType;
                    this._childNodes = [];
                    this._parentNode = null;
                }
                
                get nodeType() { return this._nodeType; }
                get parentNode() { return this._parentNode; }
                get childNodes() { return this._childNodes; }
                get firstChild() { return this._childNodes[0] || null; }
                get lastChild() { return this._childNodes[this._childNodes.length - 1] || null; }
                
                appendChild(child) {
                    this._childNodes.push(child);
                    child._parentNode = this;
                    __native_dom_appendChild(this._nodeId, child._nodeId);
                    return child;
                }
                
                removeChild(child) {
                    const idx = this._childNodes.indexOf(child);
                    if (idx !== -1) {
                        this._childNodes.splice(idx, 1);
                        child._parentNode = null;
                        __native_dom_removeChild(this._nodeId, child._nodeId);
                    }
                    return child;
                }
            }
        "#;
        self.execute_script(script)?;
        Ok(())
    }

    fn init_style_binding(&self) -> Result<(), String> {
        let script = r#"
            // CSSStyleDeclaration for element.style
            class CSSStyleDeclaration {
                constructor(nodeId) {
                    this._nodeId = nodeId;
                    this._properties = {};
                }

                setProperty(name, value) {
                    this._properties[name] = value;
                    __native_dom_setStyle(this._nodeId, name, value);
                }

                getPropertyValue(name) {
                    return this._properties[name] || '';
                }

                removeProperty(name) {
                    delete this._properties[name];
                    __native_dom_removeStyle(this._nodeId, name);
                }
            }
        "#;
        self.execute_script(script)?;
        Ok(())
    }

    fn init_element_binding(&self) -> Result<(), String> {
        let script = r#"
            // Element implementation
            class Element extends Node {
                constructor(nodeId, tagName) {
                    super(nodeId, 1); // ELEMENT_NODE = 1
                    this._tagName = tagName.toUpperCase();
                    this._attributes = {};
                    this._style = new CSSStyleDeclaration(nodeId);
                    this._eventListeners = {};
                }
                
                get tagName() { return this._tagName; }
                get id() { return this._attributes.id || ''; }
                set id(value) { this.setAttribute('id', value); }
                get className() { return this._attributes.class || ''; }
                set className(value) { this.setAttribute('class', value); }
                get style() { return this._style; }
                
                get textContent() {
                    return __native_dom_getTextContent(this._nodeId);
                }
                
                set textContent(value) {
                    __native_dom_setTextContent(this._nodeId, value);
                }
                
                get innerHTML() {
                    return __native_dom_getInnerHTML(this._nodeId);
                }
                
                set innerHTML(value) {
                    __native_dom_setInnerHTML(this._nodeId, value);
                }
                
                getAttribute(name) {
                    return this._attributes[name] || null;
                }
                
                setAttribute(name, value) {
                    this._attributes[name] = value;
                    __native_dom_setAttribute(this._nodeId, name, value);
                }
                
                removeAttribute(name) {
                    delete this._attributes[name];
                    __native_dom_removeAttribute(this._nodeId, name);
                }
                
                hasAttribute(name) {
                    return name in this._attributes;
                }
                
                addEventListener(type, listener, options) {
                    if (!this._eventListeners[type]) {
                        this._eventListeners[type] = [];
                    }
                    this._eventListeners[type].push(listener);
                    __native_dom_addEventListener(this._nodeId, type);
                }
                
                removeEventListener(type, listener) {
                    if (this._eventListeners[type]) {
                        const idx = this._eventListeners[type].indexOf(listener);
                        if (idx !== -1) {
                            this._eventListeners[type].splice(idx, 1);
                        }
                    }
                }
                
                querySelector(selector) {
                    const nodeId = __native_dom_querySelector(this._nodeId, selector);
                    return nodeId ? __getNodeById(nodeId) : null;
                }
                
                querySelectorAll(selector) {
                    const nodeIds = __native_dom_querySelectorAll(this._nodeId, selector);
                    return nodeIds.map(id => __getNodeById(id));
                }
                
                getBoundingClientRect() {
                    return __native_dom_getBoundingClientRect(this._nodeId);
                }
            }
        "#;
        self.execute_script(script)?;
        Ok(())
    }

    fn init_document_binding(&self) -> Result<(), String> {
        let script = r#"
            // Document implementation
            class Document extends Node {
                constructor() {
                    super(0, 9); // DOCUMENT_NODE = 9
                    this._documentElement = null;
                    this._body = null;
                    this._head = null;
                }
                
                get documentElement() { return this._documentElement; }
                get body() { return this._body; }
                get head() { return this._head; }
                
                createElement(tagName) {
                    const nodeId = __native_dom_createElement(tagName);
                    const element = new Element(nodeId, tagName);
                    __registerNode(nodeId, element);
                    return element;
                }
                
                createTextNode(text) {
                    const nodeId = __native_dom_createTextNode(text);
                    const node = new Node(nodeId, 3); // TEXT_NODE = 3
                    node._textContent = text;
                    __registerNode(nodeId, node);
                    return node;
                }
                
                getElementById(id) {
                    const nodeId = __native_dom_getElementById(id);
                    return nodeId ? __getNodeById(nodeId) : null;
                }
                
                getElementsByClassName(className) {
                    const nodeIds = __native_dom_getElementsByClassName(className);
                    return nodeIds.map(id => __getNodeById(id));
                }
                
                getElementsByTagName(tagName) {
                    const nodeIds = __native_dom_getElementsByTagName(tagName);
                    return nodeIds.map(id => __getNodeById(id));
                }
                
                querySelector(selector) {
                    const nodeId = __native_dom_querySelector(0, selector);
                    return nodeId ? __getNodeById(nodeId) : null;
                }
                
                querySelectorAll(selector) {
                    const nodeIds = __native_dom_querySelectorAll(0, selector);
                    return nodeIds.map(id => __getNodeById(id));
                }
            }
        "#;
        self.execute_script(script)?;
        Ok(())
    }

    /// Initialize Web APIs in V8
    fn init_web_apis(&self) -> Result<(), String> {
        debug!("Initializing Web APIs in V8");

        let web_apis_script = r#"
            // Console API
            globalThis.console = {
                log: function(...args) {
                    __native_console_log(args.map(String).join(' '));
                },
                error: function(...args) {
                    __native_console_error(args.map(String).join(' '));
                },
                warn: function(...args) {
                    __native_console_warn(args.map(String).join(' '));
                },
                info: function(...args) {
                    __native_console_info(args.map(String).join(' '));
                },
                debug: function(...args) {
                    __native_console_debug(args.map(String).join(' '));
                },
            };
            
            // Window API (subset)
            globalThis.window = globalThis;
            
            globalThis.setTimeout = function(callback, delay, ...args) {
                return __native_setTimeout(callback, delay, args);
            };
            
            globalThis.setInterval = function(callback, delay, ...args) {
                return __native_setInterval(callback, delay, args);
            };
            
            globalThis.clearTimeout = function(id) {
                __native_clearTimeout(id);
            };
            
            globalThis.clearInterval = function(id) {
                __native_clearInterval(id);
            };
            
            globalThis.requestAnimationFrame = function(callback) {
                return __native_requestAnimationFrame(callback);
            };
            
            globalThis.cancelAnimationFrame = function(id) {
                __native_cancelAnimationFrame(id);
            };
            
            // Fetch API (basic implementation)
            globalThis.fetch = async function(url, options = {}) {
                const response = await __native_fetch(url, JSON.stringify(options));
                return JSON.parse(response);
            };
            
            // Location API
            globalThis.location = {
                get href() { return __native_location_href(); },
                set href(value) { __native_location_navigate(value); },
                get protocol() { return __native_location_protocol(); },
                get host() { return __native_location_host(); },
                get pathname() { return __native_location_pathname(); },
                reload: function() { __native_location_reload(); },
            };
            
            // Navigator API
            globalThis.navigator = {
                userAgent: 'Soliloquy/0.1.0 (Linux; Servo+V8)',
                platform: 'Linux',
                language: 'en-US',
                languages: ['en-US', 'en'],
                onLine: true,
            };
            
            console.log('Web APIs initialized');
            'Web APIs ready';
        "#;

        self.execute_script(web_apis_script)?;
        Ok(())
    }

    /// Initialize Soliloquy-specific APIs
    fn init_soliloquy_apis(&self) -> Result<(), String> {
        debug!("Initializing Soliloquy APIs in V8");

        let soliloquy_script = r#"
            // Soliloquy Desktop APIs
            globalThis.soliloquy = {
                version: '0.1.0',
                platform: 'linux',
                engine: {
                    name: 'Servo+V8',
                    servoVersion: '0.0.1',
                    v8Version: __native_v8_version(),
                },
                
                // Window management
                window: {
                    minimize: function() { return __native_window_minimize(); },
                    maximize: function() { return __native_window_maximize(); },
                    restore: function() { return __native_window_restore(); },
                    close: function() { return __native_window_close(); },
                    setTitle: function(title) { return __native_window_setTitle(title); },
                    setSize: function(w, h) { return __native_window_setSize(w, h); },
                    getSize: function() { return __native_window_getSize(); },
                    setPosition: function(x, y) { return __native_window_setPosition(x, y); },
                    getPosition: function() { return __native_window_getPosition(); },
                    setFullscreen: function(fs) { return __native_window_setFullscreen(fs); },
                    isFullscreen: function() { return __native_window_isFullscreen(); },
                },
                
                // IPC for multi-window communication
                ipc: {
                    send: function(channel, data) { return __native_ipc_send(channel, data); },
                    on: function(channel, callback) { return __native_ipc_on(channel, callback); },
                    once: function(channel, callback) { return __native_ipc_once(channel, callback); },
                    removeListener: function(channel, callback) { return __native_ipc_removeListener(channel, callback); },
                },
                
                // Clipboard
                clipboard: {
                    readText: async function() { return await __native_clipboard_readText(); },
                    writeText: async function(text) { return await __native_clipboard_writeText(text); },
                },
                
                // Notifications
                notification: {
                    show: function(title, body, options) { return __native_notification_show(title, body, options); },
                },
            };
            
            console.log('Soliloquy APIs initialized');
            'Soliloquy APIs ready';
        "#;

        self.execute_script(soliloquy_script)?;
        Ok(())
    }

    /// Execute JavaScript in V8
    pub fn execute_script(&self, script: &str) -> Result<String, String> {
        let mut runtime = self
            .v8_runtime
            .lock()
            .map_err(|e| format!("Failed to lock V8 runtime: {}", e))?;

        runtime.execute_script(script)
    }

    /// Register a native function callable from V8
    pub fn register_native_function<F>(&self, name: &str, func: F) -> Result<(), String>
    where
        F: Fn(Vec<String>) -> Result<String, String> + Send + Sync + 'static,
    {
        info!("Registering native function: {}", name);

        let mut functions = self
            .native_functions
            .write()
            .map_err(|e| format!("Failed to lock native functions: {}", e))?;

        functions.insert(name.to_string(), Box::new(func));
        Ok(())
    }

    /// Create a new DOM element and add to tree
    pub fn create_element(&self, tag_name: &str) -> Result<u64, String> {
        let node_id = {
            let mut id = self
                .next_node_id
                .lock()
                .map_err(|e| format!("Failed to lock node ID: {}", e))?;
            let current = *id;
            *id += 1;
            current
        };

        let node = DOMNode {
            id: node_id,
            node_type: NodeType::Element,
            tag_name: Some(tag_name.to_uppercase()),
            text_content: None,
            attributes: HashMap::new(),
            parent_id: None,
            child_ids: Vec::new(),
        };

        {
            let mut tree = self
                .dom_tree
                .write()
                .map_err(|e| format!("Failed to lock DOM tree: {}", e))?;
            tree.insert(node_id, node);
        }

        debug!("Created element <{}> with ID {}", tag_name, node_id);
        Ok(node_id)
    }

    /// Queue a DOM mutation for Servo to process
    pub fn queue_mutation(&self, mutation: DOMMutation) -> Result<(), String> {
        let mut mutations = self
            .pending_mutations
            .lock()
            .map_err(|e| format!("Failed to lock mutations: {}", e))?;

        mutations.push(mutation);
        Ok(())
    }

    /// Get and clear pending mutations
    pub fn drain_mutations(&self) -> Result<Vec<DOMMutation>, String> {
        let mut mutations = self
            .pending_mutations
            .lock()
            .map_err(|e| format!("Failed to lock mutations: {}", e))?;

        Ok(std::mem::take(&mut *mutations))
    }

    /// Dispatch an event to JavaScript
    pub fn dispatch_event(
        &self,
        node_id: u64,
        event_type: &str,
        event_data: &str,
    ) -> Result<(), String> {
        let safe_event_type = serde_json::to_string(event_type)
            .map_err(|e| format!("Failed to serialize event type: {}", e))?;
        let safe_event_data = serde_json::to_string(event_data)
            .map_err(|e| format!("Failed to serialize event data: {}", e))?;

        let script = format!(
            r#"
            (function() {{
                const node = __getNodeById({});
                if (node && node._eventListeners && node._eventListeners[{}]) {{
                    const event = JSON.parse({});
                    node._eventListeners[{}].forEach(listener => listener(event));
                }}
            }})();
            "#,
            node_id, safe_event_type, safe_event_data, safe_event_type
        );

        self.execute_script(&script)?;
        Ok(())
    }

    pub fn listener_count(&self) -> Result<usize, String> {
        let listeners = self
            .event_listeners
            .read()
            .map_err(|e| format!("Failed to lock event listeners: {}", e))?;
        Ok(listeners.values().map(Vec::len).sum())
    }

    /// Get the current bridge state
    pub fn state(&self) -> Result<BridgeState, String> {
        let state = self
            .state
            .read()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        Ok(state.clone())
    }

    pub fn surface(&self, surface_id: SurfaceId) -> Result<Option<SurfaceDescriptor>, String> {
        let surfaces = self
            .surfaces
            .read()
            .map_err(|e| format!("Failed to lock surfaces: {}", e))?;
        Ok(surfaces.get(&surface_id).cloned())
    }

    pub fn last_input_event(&self) -> Result<Option<InputEvent>, String> {
        let input = self
            .last_input_event
            .read()
            .map_err(|e| format!("Failed to lock input event: {}", e))?;
        Ok(input.clone())
    }

    pub fn last_lifecycle_event(&self) -> Result<Option<LifecycleEvent>, String> {
        let lifecycle = self
            .last_lifecycle_event
            .read()
            .map_err(|e| format!("Failed to lock lifecycle event: {}", e))?;
        Ok(*lifecycle)
    }

    pub fn presented_frames(&self) -> Result<Vec<SurfaceId>, String> {
        let frames = self
            .presented_frames
            .lock()
            .map_err(|e| format!("Failed to lock presented frames: {}", e))?;
        Ok(frames.clone())
    }

    pub fn js_engine_status(&self) -> Result<JsEngineStatus, String> {
        let status = self
            .js_engine_status
            .read()
            .map_err(|e| format!("Failed to lock JS engine status: {}", e))?;
        Ok(status.clone())
    }

    pub fn begin_v8_swap_preparation(&self) -> Result<JsEngineStatus, String> {
        let mut status = self
            .js_engine_status
            .write()
            .map_err(|e| format!("Failed to lock JS engine status: {}", e))?;
        status.swap_stage = JsEngineSwapStage::DualRuntimePreparation;
        Ok(status.clone())
    }
}

impl EngineRuntime for EngineBridge {
    fn attach_surface(&self, surface: SurfaceDescriptor) -> Result<(), RuntimeError> {
        if surface.size.width == 0 || surface.size.height == 0 {
            return Err(RuntimeError::InvalidSurface(
                "surface dimensions must be non-zero".to_string(),
            ));
        }

        let mut surfaces = self.surfaces.write().map_err(|e| {
            RuntimeError::Unsupported(format!("surface registry lock failed: {}", e))
        })?;
        surfaces.insert(surface.id, surface);
        Ok(())
    }

    fn present_frame(&self, surface_id: SurfaceId) -> Result<(), RuntimeError> {
        let surfaces = self.surfaces.read().map_err(|e| {
            RuntimeError::Unsupported(format!("surface registry lock failed: {}", e))
        })?;
        let surface = surfaces
            .get(&surface_id)
            .ok_or(RuntimeError::SurfaceNotFound(surface_id.0))?
            .clone();
        drop(surfaces);

        let mut presented = self
            .presented_frames
            .lock()
            .map_err(|e| RuntimeError::Unsupported(format!("present queue lock failed: {}", e)))?;
        presented.push(surface.id);
        debug!(
            "presenting surface {} at {}x{} ({:?})",
            surface.id.0, surface.size.width, surface.size.height, surface.tier
        );
        Ok(())
    }

    fn handle_input(&self, event: InputEvent) -> Result<(), RuntimeError> {
        let mut last_input = self
            .last_input_event
            .write()
            .map_err(|e| RuntimeError::Unsupported(format!("input lock failed: {}", e)))?;
        *last_input = Some(event);
        Ok(())
    }

    fn handle_lifecycle(&self, event: LifecycleEvent) -> Result<(), RuntimeError> {
        let mut last_lifecycle = self
            .last_lifecycle_event
            .write()
            .map_err(|e| RuntimeError::Unsupported(format!("lifecycle lock failed: {}", e)))?;
        *last_lifecycle = Some(event);

        if matches!(event, LifecycleEvent::Shutdown) {
            let mut state = self
                .state
                .write()
                .map_err(|e| RuntimeError::Unsupported(format!("state lock failed: {}", e)))?;
            *state = BridgeState::Ready;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soliloquy_browser_optimizations::runtime::{
        EngineRuntime, InputEvent, LifecycleEvent, PlatformTier, SurfaceDescriptor, SurfaceId,
    };

    #[test]
    fn test_engine_bridge_creation() {
        let runtime = V8Runtime::new().expect("Failed to create V8 runtime");
        let bridge = EngineBridge::new(runtime);
        assert!(bridge.is_ok());

        let bridge = bridge.unwrap();
        assert_eq!(bridge.state().unwrap(), BridgeState::Uninitialized);
    }

    #[test]
    fn test_create_element() {
        let runtime = V8Runtime::new().expect("Failed to create V8 runtime");
        let bridge = EngineBridge::new(runtime).unwrap();

        let node_id = bridge.create_element("div");
        assert!(node_id.is_ok());
        assert!(node_id.unwrap() > 0);
    }

    #[test]
    fn test_queue_mutation() {
        let runtime = V8Runtime::new().expect("Failed to create V8 runtime");
        let bridge = EngineBridge::new(runtime).unwrap();

        let node = DOMNode {
            id: 1,
            node_type: NodeType::Element,
            tag_name: Some("DIV".to_string()),
            text_content: None,
            attributes: HashMap::new(),
            parent_id: None,
            child_ids: Vec::new(),
        };

        let result = bridge.queue_mutation(DOMMutation::InsertNode {
            parent_id: 0,
            node,
            before_id: None,
        });

        assert!(result.is_ok());

        let mutations = bridge.drain_mutations().unwrap();
        assert_eq!(mutations.len(), 1);
    }

    #[test]
    fn test_runtime_contract_surface_and_input() {
        let runtime = V8Runtime::new().expect("Failed to create V8 runtime");
        let bridge = EngineBridge::new(runtime).unwrap();

        let surface = SurfaceDescriptor::new(42, 1280, 720, PlatformTier::Desktop);
        bridge.attach_surface(surface.clone()).unwrap();
        assert_eq!(bridge.surface(SurfaceId(42)).unwrap(), Some(surface));

        bridge
            .handle_input(InputEvent::PointerMove { x: 11.0, y: 22.0 })
            .unwrap();
        assert_eq!(
            bridge.last_input_event().unwrap(),
            Some(InputEvent::PointerMove { x: 11.0, y: 22.0 })
        );

        bridge.handle_lifecycle(LifecycleEvent::Suspended).unwrap();
        assert_eq!(
            bridge.last_lifecycle_event().unwrap(),
            Some(LifecycleEvent::Suspended)
        );
    }

    #[test]
    fn test_js_engine_status_reports_mock_v8_preparation() {
        let runtime = V8Runtime::new().expect("Failed to create V8 runtime");
        let bridge = EngineBridge::new(runtime).unwrap();

        let status = bridge.js_engine_status().unwrap();
        // Real V8 runtime: embedder owns JS execution, Servo no longer controls it.
        assert!(!status.servo_controls_javascript);
        assert_eq!(status.active_engine, crate::js_engine::JsEngineKind::V8);
        assert_eq!(status.swap_stage, JsEngineSwapStage::EmbedderV8Experiment);
    }
}
