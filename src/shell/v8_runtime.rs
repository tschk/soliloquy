//! V8 Runtime for Soliloquy Shell — backed by rusty_v8 v0.32.

use std::cell::RefCell;
use std::sync::Once;
use std::time::{Duration, Instant};

use log::{debug, info};
use rusty_v8 as v8;
use serde_json::{json, Value};
use url::{ParseError, Url};

use crate::browser_optimizations::GcType;
use crate::js_engine::{JsEngineKind, JsEngineStatus, JsEngineSwapStage};

const SOLILOQUY_BRIDGE_SCHEMA: &str =
    include_str!("../../third_party/servo/components/servo/soliloquy_bridge_schema.json");
const SOLILOQUY_BRIDGE_SCHEMA_VERSION: &str = "rv8-bridge-v1";

const SOLILOQUY_JS_ENGINE_ENV: &str = "SOLILOQUY_JS_ENGINE";

// ── V8 platform / isolate — process-global init, thread-local isolate ────────

static V8_INIT: Once = Once::new();
thread_local! {
    static V8_ISOLATE: RefCell<Option<v8::OwnedIsolate>> = const { RefCell::new(None) };
}

// ── shell-side DOM snapshot ───────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
struct ShellDomSnapshot {
    title: Option<String>,
    url: Option<String>,
    ready_state: Option<String>,
}

// ── bridge command model ──────────────────────────────────────────────────────

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

// ── V8Runtime ────────────────────────────────────────────────────────────────

/// V8 Runtime context — real rusty_v8 isolate plus shell-side DOM bridge.
pub struct V8Runtime {
    engine_status: JsEngineStatus,
    dom_snapshot: ShellDomSnapshot,
    garbage_collections: u64,
    last_garbage_collection: Option<GcCollectionRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GcCollectionRecord {
    pub gc_type: GcType,
    pub started_at: Instant,
    pub duration: Duration,
}

impl V8Runtime {
    /// Initialize the V8 platform and create a per-thread isolate.
    pub fn new() -> Result<Self, String> {
        V8_INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        V8_ISOLATE.with(|cell| {
            let mut guard = cell.borrow_mut();
            if guard.is_none() {
                *guard = Some(v8::Isolate::new(v8::CreateParams::default()));
            }
        });

        info!("V8 runtime initialized ({})", v8::V8::get_version());

        let engine_status = JsEngineStatus {
            requested_engine: if env_requests_v8() {
                JsEngineKind::V8
            } else {
                JsEngineKind::Mozjs
            },
            active_engine: JsEngineKind::V8,
            swap_stage: JsEngineSwapStage::EmbedderV8Experiment,
            dom_bridge_ready: true,
            servo_controls_javascript: false,
        };

        Ok(V8Runtime {
            engine_status,
            dom_snapshot: ShellDomSnapshot::default(),
            garbage_collections: 0,
            last_garbage_collection: None,
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

    pub fn collect_garbage(&mut self, gc_type: GcType) -> Duration {
        let started_at = Instant::now();
        V8_ISOLATE.with(|cell| {
            if let Some(isolate) = cell.borrow_mut().as_mut() {
                isolate.low_memory_notification();
            }
        });
        let duration = started_at.elapsed();
        self.garbage_collections += 1;
        self.last_garbage_collection = Some(GcCollectionRecord {
            gc_type,
            started_at,
            duration,
        });
        duration
    }

    pub fn garbage_collection_count(&self) -> u64 {
        self.garbage_collections
    }

    pub fn last_garbage_collection(&self) -> Option<GcCollectionRecord> {
        self.last_garbage_collection
    }

    /// Execute JavaScript and return its string result.
    ///
    /// The shell bridge intercepts structured commands and DOM property
    /// reads/writes first; everything else is evaluated in the real V8 isolate.
    pub fn execute_script(&mut self, script: &str) -> Result<String, String> {
        debug!("execute_script: {}", script);

        let normalized = script.replace(['\n', '\r', '\t'], " ");
        let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

        // Bridge intercepts dom.*, engine.*, __soliloquyEval, assignments to
        // document.title / location.href, and bare property reads.
        if let Some(result) = self.execute_bridge_script(&compact)? {
            return Ok(result);
        }

        // Fall through to real V8 for arbitrary scripts.
        self.eval_v8(script)
    }

    /// Check if the runtime is initialized.
    pub fn is_initialized(&self) -> bool {
        true
    }

    /// Report which engine is actually executing JavaScript.
    pub fn engine_kind(&self) -> JsEngineKind {
        self.engine_status.active_engine
    }

    /// Report the current swap stage.
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

    /// Return the V8 version string.
    pub fn get_version() -> String {
        v8::V8::get_version().to_string()
    }

    // ── V8 evaluation ─────────────────────────────────────────────────────────

    fn eval_v8(&self, script: &str) -> Result<String, String> {
        V8_ISOLATE.with(|cell| {
            let mut guard = cell.borrow_mut();
            let isolate = guard
                .as_mut()
                .ok_or_else(|| "V8 isolate not initialized".to_string())?;

            let scope = &mut v8::HandleScope::new(isolate);
            let context = v8::Context::new(scope);
            let scope = &mut v8::ContextScope::new(scope, context);

            let prelude = self.browser_prelude();
            let prelude_source = v8::String::new(scope, &prelude)
                .ok_or_else(|| "V8: failed to create prelude string".to_string())?;
            let prelude_script = v8::Script::compile(scope, prelude_source, None)
                .ok_or_else(|| "JavaScript prelude syntax error".to_string())?;
            prelude_script
                .run(scope)
                .ok_or_else(|| "JavaScript prelude evaluation failure".to_string())?;

            let source = v8::String::new(scope, script)
                .ok_or_else(|| "V8: failed to create script string".to_string())?;

            let compiled = v8::Script::compile(scope, source, None)
                .ok_or_else(|| "JavaScript syntax error".to_string())?;

            let value = compiled
                .run(scope)
                .ok_or_else(|| "JavaScript evaluation failure".to_string())?;

            Ok(v8_value_to_string(scope, value))
        })
    }

    fn browser_prelude(&self) -> String {
        let href = js_string(self.dom_snapshot.url.as_deref());
        let title = js_string(self.dom_snapshot.title.as_deref());
        let ready_state = js_string(self.dom_snapshot.ready_state.as_deref());
        format!(
            "globalThis.window = globalThis.window || {{}}; globalThis.window.location = globalThis.window.location || {{}}; globalThis.window.location.href = {href}; globalThis.document = globalThis.document || {{}}; globalThis.document.title = {title}; globalThis.document.readyState = {ready_state}; globalThis.console = globalThis.console || {{ log() {{}} }};"
        )
    }

    // ── bridge dispatch ───────────────────────────────────────────────────────

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
                "schemaVersion": SOLILOQUY_BRIDGE_SCHEMA_VERSION,
                "schema": SOLILOQUY_BRIDGE_SCHEMA,
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
                let base =
                    Url::parse(base_url).map_err(|e| format!("invalid base location.href: {e}"))?;
                base.join(href)
                    .map(|url| url.to_string())
                    .map_err(|e| format!("invalid location.href: {e}"))
            }
            Err(e) => Err(format!("invalid location.href: {e}")),
        }
    }
}

impl Drop for V8Runtime {
    fn drop(&mut self) {
        info!("V8 runtime dropped");
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Convert a V8 value to its JavaScript string representation.
fn v8_value_to_string(scope: &mut v8::HandleScope, value: v8::Local<v8::Value>) -> String {
    if value.is_undefined() {
        "undefined".to_string()
    } else if value.is_null() {
        "null".to_string()
    } else if value.is_boolean() {
        value.boolean_value(scope).to_string()
    } else if value.is_number() {
        let n = value.number_value(scope).unwrap_or(f64::NAN);
        if n.is_nan() {
            return "NaN".to_string();
        }
        if n.is_infinite() {
            return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
        }
        // Integer values: omit the decimal point (matches JS `String(2)` → "2").
        if n.fract() == 0.0 && n.abs() < 1e15 {
            format!("{}", n as i64)
        } else {
            format!("{n}")
        }
    } else {
        value
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default()
    }
}

fn js_string(value: Option<&str>) -> String {
    serde_json::to_string(value.unwrap_or_default()).unwrap_or_else(|_| "\"\"".to_string())
}

fn env_requests_v8() -> bool {
    matches!(
        std::env::var(SOLILOQUY_JS_ENGINE_ENV),
        Ok(v)
            if matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "v8" | "v8-experimental" | "v8_experimental"
            )
    )
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_runtime_creation() {
        let runtime = V8Runtime::new();
        assert!(runtime.is_ok());
        let runtime = runtime.unwrap();
        assert!(runtime.is_initialized());
        assert_eq!(runtime.engine_kind(), JsEngineKind::V8);
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
        let script = r#"var message = "Hello from V8!"; message;"#;
        let result = runtime.execute_script(script);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello from V8!");
    }

    #[test]
    fn collect_garbage_records_bookkeeping() {
        let mut runtime = V8Runtime::new().unwrap();

        let duration = runtime.collect_garbage(GcType::Minor);

        assert_eq!(runtime.garbage_collection_count(), 1);
        assert_eq!(
            runtime
                .last_garbage_collection()
                .expect("GC record should exist")
                .gc_type,
            GcType::Minor
        );
        assert_eq!(
            duration,
            runtime
                .last_garbage_collection()
                .expect("GC record should exist")
                .duration
        );
    }

    #[test]
    fn test_v8_version_non_empty() {
        let version = V8Runtime::get_version();
        assert!(!version.is_empty());
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
    fn shell_bridge_capabilities_use_servo_bridge_schema() {
        let mut runtime = V8Runtime::new().unwrap();

        let result = runtime
            .execute_script("window.__soliloquyEval('dom.capabilities')")
            .unwrap();
        let envelope: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(envelope["ok"], true);
        assert_eq!(
            envelope["value"]["schemaVersion"],
            SOLILOQUY_BRIDGE_SCHEMA_VERSION
        );
        assert_eq!(envelope["value"]["schema"], SOLILOQUY_BRIDGE_SCHEMA);

        let schema: Value =
            serde_json::from_str(envelope["value"]["schema"].as_str().unwrap()).unwrap();
        assert_eq!(schema["version"], SOLILOQUY_BRIDGE_SCHEMA_VERSION);
        assert_eq!(schema["commands"][0]["name"], "engine.backend");
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
