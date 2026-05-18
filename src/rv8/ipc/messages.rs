//! IPC Message Types
//!
//! Defines all message types for inter-process communication.

use serde::{Deserialize, Serialize};
use soliloquy_browser_optimizations::runtime::{SurfaceDescriptor, SurfaceId, SurfaceSize};

use crate::js::JsValue;
use crate::renderer::RenderFrame;

/// Messages from renderer to browser process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserMessage {
    /// Request navigation to URL
    Navigate { tab_id: u64, url: String },
    /// Document title changed
    TitleChanged { tab_id: u64, title: String },
    /// Page finished loading
    LoadComplete { tab_id: u64 },
    /// Software-rendered frame ready for presentation
    FrameReady { tab_id: u64, frame: RenderFrame },
    /// Request reload
    Reload { tab_id: u64 },
    /// Stop loading
    Stop { tab_id: u64 },
    /// Load progress update (0-100)
    LoadProgress { tab_id: u64, progress: u8 },
    /// Favicon updated
    FaviconChanged { tab_id: u64, url: Option<String> },
    /// Security state changed
    SecurityChanged { tab_id: u64, secure: bool },
    /// Console message from JavaScript
    ConsoleMessage {
        tab_id: u64,
        level: String,
        message: String,
    },
    /// Result of a browser-requested script evaluation
    ScriptResult {
        tab_id: u64,
        callback_id: u64,
        result: Result<JsValue, String>,
    },
    /// Request to close tab
    CloseTab { tab_id: u64 },
    /// Request to open new tab
    OpenNewTab { url: Option<String> },
    /// JavaScript dialog (alert, confirm, prompt)
    JsDialog {
        tab_id: u64,
        dialog_type: JsDialogType,
        message: String,
    },
    /// Crash report
    RendererCrashed { tab_id: u64, reason: String },
}

use ipc_channel::ipc::IpcSender;

/// Messages from browser to renderer process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RendererMessage {
    /// Initialize IPC (handshake)
    Initialize {
        browser_tx: IpcSender<BrowserMessage>,
    },
    /// Navigate to URL
    Navigate { url: String },
    /// Reload page
    Reload,
    /// Stop loading
    Stop,
    /// Go back in history
    GoBack,
    /// Go forward in history
    GoForward,
    /// Execute JavaScript
    ExecuteScript { script: String, callback_id: u64 },
    /// Resize viewport
    Resize { width: u32, height: u32 },
    /// Mouse event
    MouseEvent {
        event_type: MouseEventType,
        x: f32,
        y: f32,
        button: u8,
    },
    /// Keyboard event
    KeyEvent {
        event_type: KeyEventType,
        key: String,
        modifiers: u8,
    },
    /// Scroll event
    Scroll { delta_x: f32, delta_y: f32 },
    /// Focus changed
    Focus { focused: bool },
    /// Visibility changed
    Visibility { visible: bool },
    /// Response to JS dialog
    JsDialogResponse {
        accepted: bool,
        response: Option<String>,
    },
    /// Shutdown renderer
    Shutdown,
}

/// Messages to/from GPU process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuMessage {
    /// Create surface for tab
    CreateSurface {
        tab_id: u64,
        surface: SurfaceDescriptor,
    },
    /// Destroy surface
    DestroySurface { tab_id: u64, surface_id: SurfaceId },
    /// Resize surface
    ResizeSurface {
        tab_id: u64,
        surface_id: SurfaceId,
        size: SurfaceSize,
    },
    /// Submit frame for compositing
    SubmitFrame {
        tab_id: u64,
        surface_id: SurfaceId,
        frame_id: u64,
    },
    /// Present composited frame
    Present { tab_id: u64, surface_id: SurfaceId },
    /// Update display list
    UpdateDisplayList { tab_id: u64, data: Vec<u8> },
    /// GPU context lost
    ContextLost,
    /// GPU context restored
    ContextRestored,
}

/// Messages to/from network process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Fetch resource
    Fetch {
        request_id: u64,
        url: String,
        method: String,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    },
    /// Cancel fetch
    CancelFetch { request_id: u64 },
    /// Response headers received
    ResponseHeaders {
        request_id: u64,
        status: u16,
        headers: Vec<(String, String)>,
    },
    /// Response body chunk
    ResponseBody {
        request_id: u64,
        data: Vec<u8>,
        done: bool,
    },
    /// Response error
    ResponseError { request_id: u64, error: String },
    /// Preconnect to host
    Preconnect { url: String },
    /// DNS prefetch
    DnsPrefetch { hostname: String },
    /// Set cookie
    SetCookie { url: String, cookie: String },
    /// Get cookies for URL
    GetCookies { url: String },
    /// Cookies response
    CookiesResponse { cookies: Vec<String> },
    /// Clear browsing data
    ClearData { types: u32 },
}

/// JavaScript dialog types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JsDialogType {
    Alert,
    Confirm,
    Prompt,
    BeforeUnload,
}

/// Mouse event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseEventType {
    Move,
    Down,
    Up,
    Click,
    DoubleClick,
    Enter,
    Leave,
}

/// Keyboard event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KeyEventType {
    Down,
    Up,
    Press,
}
