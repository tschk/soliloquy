//! V8 Runtime stub — rv8 agent replaces with real rv8 V8 integration.
//! ponytail: rusty_v8 removed, stubs until rv8 linkage lands.

use std::time::Duration;

use log::info;

#[cfg(test)]
use serde_json::Value;

use crate::browser_optimizations::GcType;
use crate::js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

#[derive(Clone, Debug)]
pub struct GcCollectionRecord {
    pub gc_type: GcType,
    pub duration: Duration,
    pub timestamp: std::time::Instant,
}

pub struct V8Runtime {
    initialized: bool,
    gc_count: u64,
    last_gc: Option<GcCollectionRecord>,
    engine_kind: JsEngineKind,
    status: JsEngineStatus,
}

impl V8Runtime {
    pub fn new() -> Result<Self, String> {
        info!("V8Runtime stub — rv8 integration pending");
        Ok(V8Runtime {
            initialized: true,
            gc_count: 0,
            last_gc: None,
            engine_kind: JsEngineKind::V8,
            status: JsEngineStatus {
            requested_engine: JsEngineKind::V8,
            active_engine: JsEngineKind::V8Mock,
            swap_stage: JsEngineSwapStage::EmbedderV8Experiment,
            dom_bridge_ready: false,
            servo_controls_javascript: false,
        },
        })
    }

    pub fn record_navigation(&mut self, _url: &str) {}

    pub fn record_load_complete(&mut self) {}

    pub fn collect_garbage(&mut self, gc_type: GcType) -> Duration {
        self.gc_count += 1;
        self.last_gc = Some(GcCollectionRecord {
            gc_type,
            duration: Duration::ZERO,
            timestamp: std::time::Instant::now(),
        });
        Duration::ZERO
    }

    pub fn garbage_collection_count(&self) -> u64 {
        self.gc_count
    }

    pub fn last_garbage_collection(&self) -> Option<GcCollectionRecord> {
        self.last_gc.clone()
    }

    pub fn execute_script(&mut self, _script: &str) -> Result<String, String> {
        Err("V8 stub: execution unavailable until rv8 linkage lands".to_string())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn engine_kind(&self) -> JsEngineKind {
        self.engine_kind
    }

    pub fn swap_stage(&self) -> JsEngineSwapStage {
        JsEngineSwapStage::EmbedderV8Experiment
    }

    pub fn status(&self) -> JsEngineStatus {
        self.status.clone()
    }

    pub fn begin_embedder_experiment(&mut self) {
        info!("V8Runtime stub: begin_embedder_experiment");
    }

    pub fn get_version() -> String {
        format!("soliloquy-v8-stub")
    }

    #[cfg(test)]
    fn emit_bridge_response(&self, _data: &str) -> String {
        String::new()
    }

    #[cfg(test)]
    fn extract_bridge_value(&self, _json: &Value) -> Option<String> {
        None
    }

    #[cfg(test)]
    fn raw_execute_internal(&self, _script: &str) -> Result<String, String> {
        Err("stub".to_string())
    }

    #[cfg(test)]
    fn on_script_failure(&self, _script: &str, _error: &str) {}
}
