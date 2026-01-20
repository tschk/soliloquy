//! V8 JavaScript engine wrapper
//!
//! Provides a safe Rust interface to the V8 JavaScript engine.

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
    isolate: Option<v8::OwnedIsolate>,
}

impl JsEngine {
    /// Create a new JavaScript engine instance
    pub fn new() -> Result<Self, String> {
        init_v8();

        let isolate = v8::Isolate::new(v8::CreateParams::default());
        info!("Created new V8 isolate");

        Ok(JsEngine {
            isolate: Some(isolate),
        })
    }

    /// Execute JavaScript code and return result
    pub fn execute(&mut self, script: &str) -> Result<super::JsValue, String> {
        let isolate = self.isolate.as_mut().ok_or("Isolate not initialized")?;

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope);
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
        let isolate = self.isolate.as_mut().ok_or("Isolate not initialized")?;

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope);
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
        // Isolate is automatically dropped
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
}
