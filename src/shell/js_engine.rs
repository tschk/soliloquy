//! Shared JavaScript engine status types for the Soliloquy shell.

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

/// The JavaScript engine the shell is trying to use or report on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsEngineKind {
    Mozjs,
    V8,
    V8Mock,
}

/// Coarse-grained status for the ongoing Servo-to-V8 engine transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsEngineSwapStage {
    ServoOwnsJavascript,
    DualRuntimePreparation,
    EmbedderV8Experiment,
    FullSwap,
}

/// Snapshot of how JavaScript execution is currently wired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsEngineStatus {
    pub requested_engine: JsEngineKind,
    pub active_engine: JsEngineKind,
    pub swap_stage: JsEngineSwapStage,
    pub dom_bridge_ready: bool,
    pub servo_controls_javascript: bool,
}

impl JsEngineStatus {
    pub fn servo_mozjs() -> Self {
        Self {
            requested_engine: JsEngineKind::Mozjs,
            active_engine: JsEngineKind::Mozjs,
            swap_stage: JsEngineSwapStage::ServoOwnsJavascript,
            dom_bridge_ready: false,
            servo_controls_javascript: true,
        }
    }

    pub fn embedder_v8_mock() -> Self {
        Self {
            requested_engine: JsEngineKind::V8,
            active_engine: JsEngineKind::V8Mock,
            swap_stage: JsEngineSwapStage::DualRuntimePreparation,
            dom_bridge_ready: false,
            servo_controls_javascript: true,
        }
    }

    pub fn servo_managed_from_environment() -> Self {
        if env_requests_v8() {
            Self {
                requested_engine: JsEngineKind::V8,
                active_engine: JsEngineKind::Mozjs,
                swap_stage: JsEngineSwapStage::DualRuntimePreparation,
                dom_bridge_ready: false,
                servo_controls_javascript: true,
            }
        } else {
            Self::servo_mozjs()
        }
    }

    pub fn embedder_v8_mock_from_environment() -> Self {
        let mut status = Self::embedder_v8_mock();
        if !env_requests_v8() {
            status.requested_engine = JsEngineKind::Mozjs;
        }
        status
    }
}

fn env_requests_v8() -> bool {
    matches!(
        std::env::var(SOLILOQUY_JS_ENGINE_ENV),
        Ok(value)
            if matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "v8" | "v8-experimental" | "v8_experimental"
            )
    )
}

#[cfg(test)]
mod tests {
    use super::{JsEngineKind, JsEngineStatus, JsEngineSwapStage, SOLILOQUY_JS_ENGINE_ENV};

    #[test]
    fn servo_status_defaults_to_mozjs_without_override() {
        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);
        let status = JsEngineStatus::servo_managed_from_environment();

        assert_eq!(status.requested_engine, JsEngineKind::Mozjs);
        assert_eq!(status.active_engine, JsEngineKind::Mozjs);
        assert_eq!(status.swap_stage, JsEngineSwapStage::ServoOwnsJavascript);
    }

    #[test]
    fn servo_status_reports_v8_request_when_env_is_set() {
        std::env::set_var(SOLILOQUY_JS_ENGINE_ENV, "v8");
        let status = JsEngineStatus::servo_managed_from_environment();
        std::env::remove_var(SOLILOQUY_JS_ENGINE_ENV);

        assert_eq!(status.requested_engine, JsEngineKind::V8);
        assert_eq!(status.active_engine, JsEngineKind::Mozjs);
        assert_eq!(status.swap_stage, JsEngineSwapStage::DualRuntimePreparation);
    }
}
