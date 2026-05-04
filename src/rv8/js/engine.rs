//! V8 JavaScript engine wrapper
//!
//! Provides a safe Rust interface to the V8 JavaScript engine.

use crate::js::bindings::{initialize_context, take_context_data, V8ContextData};
use crate::servo_embed::dom::DomEvent;
use log::{debug, error, info};
use rusty_v8 as v8;
use std::sync::Once;

static V8_INIT: Once = Once::new();

/// Initialize V8 (must be called once)
fn init_v8() {
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
        info!("V8 JavaScript engine initialized");
    });
}

/// V8 JavaScript engine
pub struct JsEngine {
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
}

impl JsEngine {
    /// Create a new JavaScript engine instance
    pub fn new() -> Result<Self, String> {
        init_v8();

        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        info!("Created new V8 isolate");

        let context = {
            let handle_scope = &mut v8::HandleScope::new(&mut isolate);
            let context = v8::Context::new(handle_scope);
            v8::Global::new(handle_scope, context)
        };

        Ok(JsEngine { isolate, context })
    }

    /// Initialize the engine with DOM and Web APIs
    pub fn initialize(&mut self, data: V8ContextData) {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);

        {
            let old_context = v8::Local::new(handle_scope, &self.context);
            let scope = &mut v8::ContextScope::new(handle_scope, old_context);
            let _ = take_context_data(scope);
        }

        let context = initialize_context(handle_scope, data);
        self.context = v8::Global::new(handle_scope, context);
        info!("JsEngine initialized with DOM and Web APIs");
    }

    /// Execute JavaScript code and return result
    pub fn execute(&mut self, script: &str) -> Result<super::JsValue, String> {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(handle_scope, &self.context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let code = v8::String::new(scope, script).ok_or("Failed to create script string")?;

        let script = v8::Script::compile(scope, code, None).ok_or("Failed to compile script")?;

        match script.run(scope) {
            Some(result) => {
                let value = Self::v8_to_js_value(scope, result);
                debug!("Script executed successfully");
                Ok(value)
            }
            None => {
                error!("Script execution returned None");
                Err("Script execution failed".to_string())
            }
        }
    }

    /// Execute JavaScript and return as string
    pub fn execute_to_string(&mut self, script: &str) -> Result<String, String> {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(handle_scope, &self.context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let code = v8::String::new(scope, script).ok_or("Failed to create script string")?;

        let script = v8::Script::compile(scope, code, None).ok_or("Failed to compile script")?;

        match script.run(scope) {
            Some(result) => {
                let result_str = result
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "undefined".to_string());
                Ok(result_str)
            }
            None => Err("Script execution failed".to_string()),
        }
    }

    /// Perform microtask checkpoint
    pub fn perform_microtask_checkpoint(&mut self) {
        self.isolate.perform_microtask_checkpoint();
    }

    /// Call a timer callback by ID
    pub fn call_timer_callback(&mut self, timer_id: u64) {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(handle_scope, &self.context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let data = crate::js::bindings::get_context_data(scope);

        let callback_global = {
            let callbacks = data.timer_callbacks.read();
            callbacks.get(&timer_id).cloned()
        };

        if let Some(callback_global) = callback_global {
            let callback: v8::Local<v8::Function> = v8::Local::new(scope, callback_global);
            let recv = context.global(scope).into();
            callback.call(scope, recv, &[]);
        }
    }

    /// Dispatch a host event to V8 DOM listeners.
    pub fn dispatch_event(&mut self, event: &DomEvent) -> usize {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(handle_scope, &self.context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        crate::js::bindings::dispatch_event(scope, event)
    }

    /// Convert V8 value to our JsValue type
    fn v8_to_js_value(scope: &mut v8::HandleScope, value: v8::Local<v8::Value>) -> super::JsValue {
        if value.is_undefined() {
            super::JsValue::Undefined
        } else if value.is_null() {
            super::JsValue::Null
        } else if value.is_boolean() {
            super::JsValue::Boolean(value.boolean_value(scope))
        } else if value.is_number() {
            super::JsValue::Number(value.number_value(scope).unwrap_or(f64::NAN))
        } else if value.is_string() {
            let s = value
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            super::JsValue::String(s)
        } else if value.is_array() {
            super::JsValue::Array
        } else if value.is_function() {
            super::JsValue::Function
        } else if value.is_object() {
            super::JsValue::Object
        } else {
            super::JsValue::Undefined
        }
    }

    /// Get V8 version string
    pub fn version() -> &'static str {
        v8::V8::get_version()
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create JsEngine")
    }
}

impl Drop for JsEngine {
    fn drop(&mut self) {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(handle_scope, &self.context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);
        let _ = take_context_data(scope);
        debug!("V8 isolate dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_execution() {
        let mut engine = JsEngine::new().unwrap();
        let result = engine.execute_to_string("1 + 1").unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_string_execution() {
        let mut engine = JsEngine::new().unwrap();
        let result = engine.execute_to_string("'hello' + ' world'").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_dom_bindings() {
        use crate::servo_embed::dom::DomTree;
        use crate::servo_embed::web_apis::{ConsoleApi, StorageApi, TimerManager};
        use parking_lot::RwLock;
        use std::sync::Arc;

        let mut engine = JsEngine::new().unwrap();
        let dom_tree = Arc::new(RwLock::new(DomTree::new()));
        let console_api = Arc::new(RwLock::new(ConsoleApi::new()));
        let timer_manager = Arc::new(RwLock::new(TimerManager::new()));
        let local_storage = Arc::new(RwLock::new(StorageApi::new(1024)));
        let session_storage = Arc::new(RwLock::new(StorageApi::new(1024)));

        engine.initialize(V8ContextData::new(
            dom_tree.clone(),
            console_api.clone(),
            timer_manager.clone(),
            local_storage.clone(),
            session_storage.clone(),
        ));

        // Test console.log
        engine.execute("console.log('test log')").unwrap();
        assert_eq!(console_api.read().get_logs().len(), 1);
        assert_eq!(console_api.read().get_logs()[0].message, "test log");

        // Test document.nodeType
        let node_type = engine.execute_to_string("document.nodeType").unwrap();
        assert_eq!(node_type, "9"); // Document

        // Test document.createElement
        engine
            .execute("var el = document.createElement('div')")
            .unwrap();
        let tag_name = engine.execute_to_string("el.tagName").unwrap();
        assert_eq!(tag_name, "DIV");
    }

    #[test]
    fn test_dom_mutation_and_event_bindings() {
        use crate::servo_embed::dom::{DomEvent, DomTree};
        use crate::servo_embed::web_apis::{ConsoleApi, StorageApi, TimerManager};
        use parking_lot::RwLock;
        use std::sync::Arc;

        let mut engine = JsEngine::new().unwrap();
        let dom_tree = Arc::new(RwLock::new(DomTree::new()));
        let console_api = Arc::new(RwLock::new(ConsoleApi::new()));
        let timer_manager = Arc::new(RwLock::new(TimerManager::new()));
        let local_storage = Arc::new(RwLock::new(StorageApi::new(1024)));
        let session_storage = Arc::new(RwLock::new(StorageApi::new(1024)));

        engine.initialize(V8ContextData::new(
            dom_tree.clone(),
            console_api,
            timer_manager,
            local_storage,
            session_storage,
        ));

        engine
            .execute(
                "var el = document.createElement('section');
                 el.setAttribute('id', 'root');
                 el.textContent = 'ready';
                 document.appendChild(el);
                 var seen = '';
                 document.addEventListener('click', function(event) {
                   seen = event.type + ':' + event.clientX + ':' + event.target.nodeType;
                 });",
            )
            .unwrap();

        assert!(dom_tree.read().query_selector("#root").is_some());
        assert_eq!(engine.execute_to_string("el.textContent").unwrap(), "ready");
        assert!(!dom_tree.write().take_mutations().is_empty());

        let event = DomEvent {
            event_type: "click".to_string(),
            target_id: dom_tree.read().document_id(),
            client_x: Some(7.0),
            client_y: Some(3.0),
            button: Some(0),
            key: None,
        };

        assert_eq!(engine.dispatch_event(&event), 1);
        assert_eq!(engine.execute_to_string("seen").unwrap(), "click:7:9");
    }
}
