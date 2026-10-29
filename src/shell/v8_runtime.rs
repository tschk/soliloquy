//! V8 runtime placeholder.
//!
//! Soliloquy has no JavaScript engine linked in yet: rusty_v8 was removed and
//! the rv8 linkage has not landed. `V8Runtime::new` therefore fails in every
//! build, tests included, so callers see the missing engine instead of a stub
//! that only works under `cfg(test)`.

use std::time::Duration;

use log::info;

use crate::browser_optimizations::GcType;
use crate::js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

/// Error returned wherever JavaScript execution is requested.
pub const V8_UNIMPLEMENTED: &str =
    "V8 runtime is not implemented: no JavaScript engine is linked into soliloquy (rv8 linkage pending)";

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
        Err(V8_UNIMPLEMENTED.to_string())
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
        Err(V8_UNIMPLEMENTED.to_string())
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
        "unavailable".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_reports_missing_engine() {
        let error = V8Runtime::new().err().expect("no JS engine is linked in");
        assert!(error.contains("not implemented"), "{error}");
    }
}
