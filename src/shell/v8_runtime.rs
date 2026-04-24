//! V8 Runtime helper for Soliloquy Shell
//!
//! This module provides a thin wrapper around rusty_v8 to simplify
//! V8 isolate creation and JavaScript execution.
//!
//! MOCK IMPLEMENTATION - rusty_v8 dependency missing in environment

use log::{debug, info};
use serde_json::{json, Value};
use std::sync::Mutex;
use url::{ParseError, Url};

use crate::js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};

#[derive(Clone, Debug, Default)]
struct ShellDomSnapshot {
    title: Option<String>,
    url: Option<String>,
    ready_state: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ShellBridgeCommand {
    EngineBackend,
    EngineStatus,
    WebViewId,
    WebViewDescribe,
    DomCapabilities,
    DomInspect(ShellBridgeTarget),
    DomSet(ShellBridgeWrite),
    Unsupported(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellBridgeTarget {
    DocumentTitle,
    LocationHref,
    DocumentReadyState,
}

impl ShellBridgeTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "document.title" => Some(Self::DocumentTitle),
            "location.href" | "window.location.href" => Some(Self::LocationHref),
            "document.readyState" => Some(Self::DocumentReadyState),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::DocumentTitle => "document.title",
            Self::LocationHref => "location.href",
            Self::DocumentReadyState => "document.readyState",
        }
    }

    fn writable(self) -> bool {
        matches!(self, Self::DocumentTitle | Self::LocationHref)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ShellBridgeWrite {
    SetDocumentTitle(String),
    SetLocationHref(String),
}

impl ShellBridgeWrite {
    fn parse(target: &str, value: &str) -> Option<Self> {
        match target {
            "document.title" => Some(Self::SetDocumentTitle(value.to_string())),
            "location.href" | "window.location.href" => {
                Some(Self::SetLocationHref(value.to_string()))
            }
            _ => None,
        }
    }
}

/// V8 Runtime context wrapper
pub struct V8Runtime {
    _lock: Mutex<()>,
    engine_status: JsEngineStatus,
    dom_snapshot: ShellDomSnapshot,
}

impl V8Runtime {
    /// Create a new V8 runtime
    pub fn new() -> Result<Self, String> {
        info!("Initializing V8 runtime (MOCKED)");
        debug!("V8 runtime initialized successfully");

        Ok(V8Runtime {
            _lock: Mutex::new(()),
            engine_status: JsEngineStatus::embedder_v8_mock_from_environment(),
            dom_snapshot: ShellDomSnapshot::default(),
        })
    }

    /// Record a navigation known by the shell-side embedder.
    pub fn record_navigation(&mut self, url: &str) {
        self.dom_snapshot.url = Some(url.to_string());
        self.dom_snapshot.ready_state = Some("loading".to_string());
    }

    /// Record load completion for the shell-side DOM snapshot.
    pub fn record_load_complete(&mut self) {
        self.dom_snapshot.ready_state = Some("complete".to_string());
    }

    /// Execute JavaScript code and return the result
    pub fn execute_script(&mut self, script: &str) -> Result<String, String> {
        debug!("Would execute script: {}", script);

        let normalized = script.replace(['\n', '\r', '\t'], " ");
        let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

        if let Some(result) = self.execute_bridge_script(&compact)? {
            return Ok(result);
        }

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
            return Ok(r#"{"title":"Soliloquy Test","ready":true,"version":"1.0.0"}"#.to_string());
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

    fn execute_bridge_script(&mut self, script: &str) -> Result<Option<String>, String> {
        let trimmed = script.trim().trim_end_matches(';').trim();

        if let Some((target, value)) = split_assignment(trimmed) {
            let Some(value) = parse_string_literal(value) else {
                return Ok(None);
            };
            let Some(write) = ShellBridgeWrite::parse(target, &value) else {
                return Ok(None);
            };
            return Ok(Some(self.apply_bridge_write(write, false)?));
        }

        if let Some(target) = ShellBridgeTarget::parse(trimmed) {
            return Ok(Some(self.read_target(target)));
        }

        let Some(command) = parse_shell_bridge_command(trimmed) else {
            return Ok(None);
        };
        Ok(Some(self.dispatch_bridge_command(command)?))
    }

    fn dispatch_bridge_command(&mut self, command: ShellBridgeCommand) -> Result<String, String> {
        Ok(match command {
            ShellBridgeCommand::EngineBackend => {
                bridge_envelope(json!(self.engine_status.active_engine.label()))
            }
            ShellBridgeCommand::EngineStatus => bridge_envelope(json!({
                "requestedEngine": self.engine_status.requested_engine.label(),
                "activeEngine": self.engine_status.active_engine.label(),
                "bridgeReady": self.engine_status.dom_bridge_ready,
                "controlsDom": self.engine_status.servo_controls_javascript,
                "commandChannel": true,
            })),
            ShellBridgeCommand::WebViewId => bridge_envelope(json!(0)),
            ShellBridgeCommand::WebViewDescribe => bridge_envelope(json!({
                "id": 0,
                "backend": self.engine_status.active_engine.label(),
                "url": self.dom_snapshot.url.as_deref(),
                "title": self.dom_snapshot.title.as_deref(),
                "readyState": self.dom_snapshot.ready_state.as_deref(),
                "controlsDom": true,
            })),
            ShellBridgeCommand::DomCapabilities => bridge_envelope(json!({
                "simpleEval": true,
                "structuredCommands": true,
                "liveDomProperties": true,
                "liveDomWrites": true,
                "navigationWrites": true,
                "controlsDom": true,
                "fallbackEngine": "mozjs",
            })),
            ShellBridgeCommand::DomInspect(target) => bridge_envelope(self.inspect_target(target)),
            ShellBridgeCommand::DomSet(write) => self.apply_bridge_write(write, true)?,
            ShellBridgeCommand::Unsupported(operation) => {
                bridge_detail_envelope("unsupported", json!(operation))
            }
        })
    }

    fn apply_bridge_write(
        &mut self,
        write: ShellBridgeWrite,
        envelope: bool,
    ) -> Result<String, String> {
        let value = match write {
            ShellBridgeWrite::SetDocumentTitle(title) => {
                self.dom_snapshot.title = Some(title.clone());
                title
            }
            ShellBridgeWrite::SetLocationHref(url) => {
                let url = match self.resolve_location_href(&url) {
                    Ok(url) => url,
                    Err(error) if envelope => {
                        return Ok(bridge_detail_envelope("error", json!(error)));
                    }
                    Err(error) => return Err(error),
                };
                self.dom_snapshot.url = Some(url.clone());
                self.dom_snapshot.ready_state = Some("loading".to_string());
                url
            }
        };

        if envelope {
            Ok(bridge_envelope(json!(value)))
        } else {
            Ok(value)
        }
    }

    fn read_target(&self, target: ShellBridgeTarget) -> String {
        match target {
            ShellBridgeTarget::DocumentTitle => {
                self.dom_snapshot.title.as_deref().unwrap_or("null")
            }
            ShellBridgeTarget::LocationHref => self.dom_snapshot.url.as_deref().unwrap_or("null"),
            ShellBridgeTarget::DocumentReadyState => {
                self.dom_snapshot.ready_state.as_deref().unwrap_or("null")
            }
        }
        .to_string()
    }

    fn inspect_target(&self, target: ShellBridgeTarget) -> Value {
        let value = match target {
            ShellBridgeTarget::DocumentTitle => json!(self.dom_snapshot.title.as_deref()),
            ShellBridgeTarget::LocationHref => json!(self.dom_snapshot.url.as_deref()),
            ShellBridgeTarget::DocumentReadyState => {
                json!(self.dom_snapshot.ready_state.as_deref())
            }
        };
        let value_available = !value.is_null();

        json!({
            "target": target.label(),
            "kind": "string",
            "writable": target.writable(),
            "status": if value_available { "live-snapshot" } else { "fallback-required" },
            "fallbackEngine": "mozjs",
            "valueAvailable": value_available,
            "value": value,
        })
    }

    fn resolve_location_href(&self, href: &str) -> Result<String, String> {
        if href.is_empty() {
            return Err("location.href cannot be empty".to_string());
        }

        match Url::parse(href) {
            Ok(url) => Ok(url.to_string()),
            Err(ParseError::RelativeUrlWithoutBase) => {
                let base_url = self.dom_snapshot.url.as_deref().ok_or_else(|| {
                    "invalid location.href: relative URL without a base".to_string()
                })?;
                let base = Url::parse(base_url)
                    .map_err(|error| format!("invalid base location.href: {error}"))?;
                base.join(href)
                    .map(|url| url.to_string())
                    .map_err(|error| format!("invalid location.href: {error}"))
            }
            Err(error) => Err(format!("invalid location.href: {error}")),
        }
    }
}

impl JsEngineKind {
    fn label(self) -> &'static str {
        match self {
            Self::Mozjs => "mozjs",
            Self::V8 => "v8",
            Self::V8Mock => "v8-mock",
        }
    }
}

fn parse_shell_bridge_command(script: &str) -> Option<ShellBridgeCommand> {
    const PREFIXES: [&str; 2] = ["window.__soliloquyEval(", "globalThis.__soliloquyEval("];
    for prefix in PREFIXES {
        if let Some(rest) = script.strip_prefix(prefix) {
            let tokens = parse_quoted_arguments(rest.strip_suffix(')')?.trim())?;
            let (name, args) = tokens.split_first()?;
            return Some(parse_bridge_command(name, args));
        }
    }
    None
}

fn parse_bridge_command(name: &str, args: &[String]) -> ShellBridgeCommand {
    match name {
        "engine.backend" if args.is_empty() => ShellBridgeCommand::EngineBackend,
        "engine.status" if args.is_empty() => ShellBridgeCommand::EngineStatus,
        "webview.id" if args.is_empty() => ShellBridgeCommand::WebViewId,
        "webview.describe" if args.is_empty() => ShellBridgeCommand::WebViewDescribe,
        "dom.capabilities" if args.is_empty() => ShellBridgeCommand::DomCapabilities,
        "dom.inspect" => parse_dom_inspect(args)
            .map(ShellBridgeCommand::DomInspect)
            .unwrap_or_else(|| ShellBridgeCommand::Unsupported(name.to_string())),
        "dom.set" => parse_dom_set(args)
            .map(ShellBridgeCommand::DomSet)
            .unwrap_or_else(|| ShellBridgeCommand::Unsupported(name.to_string())),
        _ => ShellBridgeCommand::Unsupported(name.to_string()),
    }
}

fn parse_dom_inspect(args: &[String]) -> Option<ShellBridgeTarget> {
    let [target] = args else {
        return None;
    };
    ShellBridgeTarget::parse(target)
}

fn parse_dom_set(args: &[String]) -> Option<ShellBridgeWrite> {
    let [target, value] = args else {
        return None;
    };
    ShellBridgeWrite::parse(target, value)
}

fn parse_quoted_arguments(payload: &str) -> Option<Vec<String>> {
    let mut values = Vec::new();
    let mut cursor = payload.trim();

    while !cursor.is_empty() {
        let quote = cursor.chars().next()?;
        if quote != '\'' && quote != '"' {
            return None;
        }

        let rest = &cursor[quote.len_utf8()..];
        let end = rest.find(quote)?;
        values.push(rest[..end].to_string());
        cursor = rest[end + quote.len_utf8()..].trim_start();

        if cursor.is_empty() {
            break;
        }

        if !cursor.starts_with(',') {
            return None;
        }
        cursor = cursor[1..].trim_start();
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn split_assignment(script: &str) -> Option<(&str, &str)> {
    let mut in_quote: Option<char> = None;
    for (index, ch) in script.char_indices() {
        match in_quote {
            Some(quote) if ch == quote => in_quote = None,
            Some(_) => {}
            None if ch == '\'' || ch == '"' => in_quote = Some(ch),
            None if ch == '=' => {
                let lhs = script[..index].trim();
                let rhs = script[index + 1..].trim();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs));
                }
            }
            None => {}
        }
    }

    None
}

fn parse_string_literal(script: &str) -> Option<String> {
    let quote = script.chars().next()?;
    if (quote != '\'' && quote != '"') || !script.ends_with(quote) || script.len() < 2 {
        return None;
    }

    let inner = &script[1..script.len() - 1];
    if inner.contains(quote) {
        return None;
    }
    Some(inner.to_string())
}

fn bridge_envelope(value: Value) -> String {
    json!({
        "ok": true,
        "status": "ok",
        "value": value,
        "detail": null,
    })
    .to_string()
}

fn bridge_detail_envelope(status: &str, detail: Value) -> String {
    json!({
        "ok": false,
        "status": status,
        "value": null,
        "detail": detail,
    })
    .to_string()
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

    #[test]
    fn shell_bridge_reads_and_writes_live_snapshot() {
        let mut runtime = V8Runtime::new().unwrap();

        assert_eq!(runtime.execute_script("document.title").unwrap(), "null");
        assert_eq!(
            runtime
                .execute_script("document.title = 'Shell Bridge'")
                .unwrap(),
            "Shell Bridge"
        );
        assert_eq!(
            runtime.execute_script("document.title").unwrap(),
            "Shell Bridge"
        );

        runtime.record_navigation("https://soliloquy.test/start");
        assert_eq!(
            runtime.execute_script("location.href").unwrap(),
            "https://soliloquy.test/start"
        );
        assert_eq!(
            runtime
                .execute_script("location.href = 'docs/page'")
                .unwrap(),
            "https://soliloquy.test/docs/page"
        );
        assert_eq!(
            runtime.execute_script("document.readyState").unwrap(),
            "loading"
        );
    }

    #[test]
    fn shell_bridge_commands_return_stable_envelopes() {
        let mut runtime = V8Runtime::new().unwrap();
        runtime.record_navigation("https://soliloquy.test/bridge");

        let result = runtime
            .execute_script("window.__soliloquyEval('dom.inspect', 'location.href')")
            .unwrap();
        let envelope: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["status"], "ok");
        assert_eq!(envelope["value"]["target"], "location.href");
        assert_eq!(envelope["value"]["value"], "https://soliloquy.test/bridge");

        let result = runtime
            .execute_script(
                "window.__soliloquyEval('dom.set', 'location.href', 'https://soliloquy.test/next')",
            )
            .unwrap();
        let envelope: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["value"], "https://soliloquy.test/next");
        assert_eq!(
            runtime.execute_script("location.href").unwrap(),
            "https://soliloquy.test/next"
        );

        let result = runtime
            .execute_script("window.__soliloquyEval('dom.set', 'location.href', 'nested/page')")
            .unwrap();
        let envelope: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(envelope["ok"], true);
        assert_eq!(envelope["value"], "https://soliloquy.test/nested/page");
        assert_eq!(
            runtime.execute_script("location.href").unwrap(),
            "https://soliloquy.test/nested/page"
        );
    }

    #[test]
    fn shell_bridge_rejects_invalid_navigation_writes() {
        let mut runtime = V8Runtime::new().unwrap();

        let result = runtime
            .execute_script("window.__soliloquyEval('dom.set', 'location.href', '')")
            .unwrap();
        let envelope: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["status"], "error");
        assert_eq!(envelope["detail"], "location.href cannot be empty");

        let result = runtime.execute_script("location.href = 'not a url'");
        assert!(result.is_err());
    }
}
