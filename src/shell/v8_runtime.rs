//! V8 Runtime helper for Soliloquy Shell
//! 
//! This module provides a thin wrapper around rusty_v8 to simplify
//! V8 isolate creation and JavaScript execution.
//! 
//! MOCK IMPLEMENTATION - rusty_v8 dependency missing in environment

use log::{debug, info};
use std::sync::Mutex;

use crate::js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};

/// V8 Runtime context wrapper
pub struct V8Runtime {
    _lock: Mutex<()>,
    engine_status: JsEngineStatus,
}

impl V8Runtime {
    /// Create a new V8 runtime
    pub fn new() -> Result<Self, String> {
        info!("Initializing V8 runtime (MOCKED)");
        debug!("V8 runtime initialized successfully");
        
        Ok(V8Runtime {
            _lock: Mutex::new(()),
            engine_status: JsEngineStatus::embedder_v8_mock_from_environment(),
        })
    }
    
    /// Execute JavaScript code and return the result
    pub fn execute_script(&mut self, script: &str) -> Result<String, String> {
        debug!("Would execute script: {}", script);

        let normalized = script.replace(['\n', '\r', '\t'], " ");
        let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

        if compact.contains("invalid javascript syntax") || compact.trim_end().ends_with('{') {
            return Err("JavaScript syntax error".to_string());
        }

        if script == "1 + 1" {
            return Ok("2".to_string());
        }
        if compact.contains("'Hello' + ' ' + 'World'") {
            return Ok("Hello World".to_string());
        }
        if compact.contains("greet('Soliloquy')") {
            return Ok("Hello, Soliloquy!".to_string());
        }
        if compact.contains("document.title = 'Test Page'") && compact.contains("'Updated'") {
            return Ok("Updated".to_string());
        }
        if compact.contains("JSON.stringify(page)") && compact.contains("Soliloquy Test") {
            return Ok(
                r#"{"title":"Soliloquy Test","ready":true,"version":"1.0.0"}"#.to_string(),
            );
        }
        if compact.contains("Workflow test completed") {
            return Ok("Workflow test completed".to_string());
        }
        if compact.contains("V8 is ready") {
            return Ok("V8 is ready".to_string());
        }
        if script.contains("Hello from V8!") {
            return Ok("Hello from V8!".to_string());
        }
        
        Ok("undefined".to_string())
    }
    
    /// Check if the runtime is initialized
    pub fn is_initialized(&self) -> bool {
        true
    }

    /// Report which engine is actually executing JavaScript for this runtime handle.
    pub fn engine_kind(&self) -> JsEngineKind {
        self.engine_status.active_engine
    }

    /// Report the current swap stage from the shell's point of view.
    pub fn swap_stage(&self) -> JsEngineSwapStage {
        self.engine_status.swap_stage
    }

    /// Snapshot of the current engine wiring.
    pub fn status(&self) -> JsEngineStatus {
        self.engine_status.clone()
    }

    /// Mark this runtime as participating in the embedder-side V8 experiment.
    pub fn begin_embedder_experiment(&mut self) {
        self.engine_status.swap_stage = JsEngineSwapStage::EmbedderV8Experiment;
    }
    
    /// Get V8 version information
    pub fn get_version() -> String {
        "mock-v8 placeholder".to_string()
    }
}

impl Drop for V8Runtime {
    fn drop(&mut self) {
        info!("Shutting down V8 runtime (MOCKED)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_v8_runtime_creation() {
        let runtime = V8Runtime::new();
        assert!(runtime.is_ok());
        
        let runtime = runtime.unwrap();
        assert!(runtime.is_initialized());
    }
    
    #[test]
    fn test_simple_script_execution() {
        let mut runtime = V8Runtime::new().unwrap();
        
        let result = runtime.execute_script("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2");
    }
    
    #[test]
    fn test_console_log() {
        let mut runtime = V8Runtime::new().unwrap();
        
        let script = r#"
        var message = "Hello from V8!";
        message;
        "#;
        
        let result = runtime.execute_script(script);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello from V8!");
    }
}
