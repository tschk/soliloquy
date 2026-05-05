use std::net::IpAddr;
use std::net::SocketAddr;
use std::os::unix::io::{AsRawFd, BorrowedFd, IntoRawFd};
use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, Json};
use axum::routing::{delete, get, post, put};
use axum::Router;
use dashmap::DashMap;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::sys::signal::Signal;
use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult, Pid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

// ── TIOCSWINSZ ioctl (module-level, required by nix macro) ───────────────────
nix::ioctl_write_ptr_bad!(pty_set_winsize, nix::libc::TIOCSWINSZ, Winsize);
#[cfg(target_os = "linux")]
nix::ioctl_write_int_bad!(pty_set_ctty, nix::libc::TIOCSCTTY);

// ── constants ────────────────────────────────────────────────────────────────

const DEFAULT_FILES_DIR: &str = "/var/lib/soliloquy/files";
const LOCAL_FILES_DIR: &str = ".soliloquy/files";
/// Shell search order on Alpine.
const SHELLS: &[&str] = &["/usr/bin/zellij", "/bin/ash", "/bin/sh"];

// ── types ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct FileInfo {
    name: String,
    size: u64,
    is_dir: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
struct Settings {
    theme: String,
    cache_size_mb: u32,
    enable_javascript: bool,
    homepage: String,
    browser_layout: String,
    tab_style: String,
    chrome_density: String,
    default_zoom: u16,
    sidebar_autohide: bool,
    page_proxy_cache_seconds: u32,
    block_private_network_proxy: bool,
    restore_tabs: bool,
    show_device_status: bool,
    terminal_font_size: u16,
    terminal_cursor: String,
    js_engine: String,
    v8_turbofan_enabled: bool,
    v8_max_heap_size_mb: u32,
    v8_initial_heap_size_mb: u32,
    v8_lazy_compilation: bool,
    v8_concurrent_gc: bool,
    v8_incremental_marking: bool,
    v8_code_cache_enabled: bool,
    renderer_process_limit: u16,
    site_isolation: bool,
    sandbox: bool,
    gpu_compositing: bool,
    hardware_acceleration: bool,
    http3_enabled: bool,
    display_backend: String,
    wayland_required: bool,
    low_power_idle: bool,
    target_fps: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            cache_size_mb: 100,
            enable_javascript: true,
            homepage: "os://terminal".to_string(),
            browser_layout: "vertical".to_string(),
            tab_style: "compact".to_string(),
            chrome_density: "comfortable".to_string(),
            default_zoom: 100,
            sidebar_autohide: false,
            page_proxy_cache_seconds: 15,
            block_private_network_proxy: true,
            restore_tabs: true,
            show_device_status: true,
            terminal_font_size: 14,
            terminal_cursor: "software".to_string(),
            js_engine: "v8-experimental".to_string(),
            v8_turbofan_enabled: true,
            v8_max_heap_size_mb: 512,
            v8_initial_heap_size_mb: 64,
            v8_lazy_compilation: true,
            v8_concurrent_gc: true,
            v8_incremental_marking: true,
            v8_code_cache_enabled: true,
            renderer_process_limit: 4,
            site_isolation: true,
            sandbox: true,
            gpu_compositing: true,
            hardware_acceleration: true,
            http3_enabled: true,
            display_backend: "wayland".to_string(),
            wayland_required: true,
            low_power_idle: true,
            target_fps: 60,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct SystemConfig {
    filesystem: FilesystemPolicy,
    browser: BrowserPolicy,
    package_manager: PackageManagerPolicy,
    plugins: Vec<PluginConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct FilesystemPolicy {
    immutable_root: bool,
    user_home_root: String,
    user_writable_scope: String,
    tmp_policy: TmpPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct TmpPolicy {
    path: String,
    mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct BrowserPolicy {
    profile_management: String,
    profiles_root: String,
    cache_root: String,
    state_root: String,
    logs_root: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PackageManagerPolicy {
    id: String,
    mode: String,
    binary: String,
    root: String,
    developer_mode_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PackageManagerConfig {
    id: String,
    display_name: String,
    mode: String,
    binary: String,
    state_root: String,
    developer_mode_required: bool,
    manages: Vec<String>,
    does_not_manage: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PluginConfig {
    id: String,
    display_name: String,
    kind: String,
    enabled: bool,
    sync: SyncFeatureFlags,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct SyncFeatureFlags {
    files: bool,
    photos: bool,
    clipboard: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct PluginStateUpdate {
    enabled: Option<bool>,
    sync: Option<SyncFeatureFlags>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PersistedPluginState {
    plugins: Vec<PluginConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PluginManifest {
    id: String,
    display_name: String,
    kind: String,
    entrypoint: String,
    capabilities: Vec<String>,
    sync_features: SyncFeatureFlags,
    #[serde(default)]
    packages: Vec<PluginPackage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PluginPackage {
    version: String,
    filename: String,
    sha256: String,
    signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PersistedPluginInstallState {
    plugins: Vec<PluginInstallRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PluginInstallRecord {
    id: String,
    installed: bool,
    version: Option<String>,
    source_path: Option<String>,
    sha256: Option<String>,
    verified: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct PluginInstallRequest {
    version: String,
    source_path: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct PluginInventoryEntry {
    manifest: PluginManifest,
    install: Option<PluginInstallRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct UpdatePolicy {
    strategy: String,
    rollback_enabled: bool,
    channels: Vec<String>,
    generation_root: String,
    retained_generations: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct UpdateState {
    active_generation: String,
    staged_generation: Option<String>,
    rollback_generation: Option<String>,
    last_result: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct UpdateStatus {
    policy: UpdatePolicy,
    state: UpdateState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct ServiceRegistry {
    services: Vec<ServiceDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct ServiceDefinition {
    id: String,
    display_name: String,
    run_as: String,
    restart: String,
    dependencies: Vec<String>,
    #[serde(default)]
    optional: bool,
    state_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NotifyRequest {
    title: String,
    body: String,
}

#[derive(Serialize)]
struct NotifyResult {
    delivered: bool,
    message: &'static str,
}

#[derive(Deserialize)]
struct FileContent {
    content: String,
}

#[derive(Clone)]
struct CachedPage {
    html: String,
    stored_at: Instant,
}

#[derive(Deserialize)]
struct BrowseQuery {
    url: String,
}

#[derive(Serialize)]
struct NetworkStatus {
    connected: bool,
    interfaces: Vec<String>,
}

#[derive(Serialize)]
struct BatteryStatus {
    present: bool,
    percent: Option<u8>,
    charging: Option<bool>,
}

#[derive(Serialize)]
struct DeviceStatus {
    service: &'static str,
    now_unix_ms: u128,
    uptime_ms: u128,
    files_dir: String,
    terminal_sessions: usize,
    terminal_shells: &'static [&'static str],
}

#[derive(Serialize)]
struct DeviceActionResult {
    action: String,
    accepted: bool,
    applied: bool,
    message: &'static str,
}

#[derive(Serialize)]
struct RuntimeStatus {
    service: &'static str,
    vinix: VinixReferenceStatus,
    javascript: JavascriptRuntimeStatus,
    display: DisplayRuntimeStatus,
    optimizations: Vec<RuntimeOptimizationStatus>,
}

#[derive(Serialize)]
struct VinixReferenceStatus {
    mode: &'static str,
    license: &'static str,
    url: &'static str,
}

#[derive(Serialize)]
struct JavascriptRuntimeStatus {
    requested_engine: String,
    active_engine: &'static str,
    bridge_ready: bool,
    servo_controls_javascript: bool,
    swap_stage: &'static str,
    restart_required: bool,
}

#[derive(Serialize)]
struct DisplayRuntimeStatus {
    configured_backend: String,
    active_backend: String,
    wayland_required: bool,
    x11_fallback: bool,
    headless: bool,
}

#[derive(Serialize)]
struct RuntimeOptimizationStatus {
    id: &'static str,
    label: &'static str,
    configured: bool,
    active: bool,
    status: &'static str,
}

#[derive(Serialize)]
struct OsStatus {
    config: SystemConfig,
    package_manager: PackageManagerConfig,
    services: ServiceRegistry,
    updates: UpdateStatus,
    network: NetworkStatus,
    battery: BatteryStatus,
    power: PowerCapabilityStatus,
    display: DisplayRuntimeStatus,
}

#[derive(Serialize)]
struct PowerCapabilityStatus {
    actions: Vec<&'static str>,
    enabled: bool,
}

/// Live PTY session.
struct PtySession {
    /// Master fd (raw — owned by this struct; closed on Remove).
    master_fd: i32,
    /// Child PID (shell).
    child_pid: Pid,
    cols: u16,
    rows: u16,
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Close the master fd when session is removed from the map.
        let _ = close(self.master_fd);
    }
}

type SessionMap = Arc<DashMap<String, PtySession>>;

// ── WebSocket message (client → server) ──────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsClientMsg {
    Resize { cols: u16, rows: u16 },
}

#[derive(Deserialize)]
struct WsQuery {
    #[allow(dead_code)]
    token: Option<String>,
}

// ── app state ────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    sessions: SessionMap,
    files_dir: PathBuf,
    runtime_env_path: PathBuf,
    token: Option<String>,
    http: reqwest::Client,
    page_cache: Arc<DashMap<String, CachedPage>>,
    started_at: Instant,
    system_config: Arc<RwLock<SystemConfig>>,
    plugin_state_path: Arc<PathBuf>,
    plugin_install_state_path: Arc<PathBuf>,
    plugin_manifests: Arc<Vec<PluginManifest>>,
    service_registry: Arc<ServiceRegistry>,
    update_policy: Arc<UpdatePolicy>,
    update_state_path: Arc<PathBuf>,
}

// ── main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let files_dir = std::env::var_os("SOLILOQUY_FILES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FILES_DIR));
    let files_dir = prepare_files_dir(files_dir).await;
    let runtime_env_path = std::env::var_os("SOLILOQUY_RUNTIME_ENV")
        .map(PathBuf::from)
        .unwrap_or_else(|| files_dir.join("runtime.env"));
    let token = std::env::var("SOL_TOKEN").ok().and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() || trimmed == "dev-token-change-me" {
            None
        } else {
            Some(trimmed)
        }
    });
    let plugin_state_path = Arc::new(system_plugin_state_path());
    let plugin_install_state_path = Arc::new(system_plugin_install_state_path());
    let system_config = Arc::new(RwLock::new(load_system_config(plugin_state_path.as_ref())));
    let plugin_manifests = Arc::new(load_plugin_manifests());
    let service_registry = Arc::new(load_service_registry());
    let update_policy = Arc::new(load_update_policy());
    let update_state_path = Arc::new(system_update_state_path());

    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        files_dir,
        runtime_env_path,
        token,
        http: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .user_agent("Soliloquy/0.1")
            .build()
            .unwrap(),
        page_cache: Arc::new(DashMap::new()),
        started_at: Instant::now(),
        system_config,
        plugin_state_path,
        plugin_install_state_path,
        plugin_manifests,
        service_registry,
        update_policy,
        update_state_path,
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/healthz",
            get(|| async { Json(serde_json::json!({ "ok": true })) }),
        )
        .route("/browse", get(browse_page))
        .route("/api/device", get(device_status))
        .route("/api/device/{action}", post(device_action))
        .route("/api/runtime", get(runtime_status))
        .route("/api/os", get(os_status))
        // os://terminal landing + PTY bridge
        .route("/terminal", get(serve_terminal_page))
        .route("/v1/term/session", post(create_term_session))
        .route("/v1/term/session/{id}/ws", get(term_ws))
        // Files API
        .route("/api/files", get(list_files))
        .route("/api/files/{name}", get(read_file))
        .route("/api/files/{name}", put(write_file))
        .route("/api/files/{name}", delete(delete_file))
        // Settings API
        .route("/api/settings", get(get_settings))
        .route("/api/settings", put(put_settings))
        .route("/v1/system/config", get(get_system_config))
        .route(
            "/v1/system/package-manager",
            get(get_package_manager_config),
        )
        .route("/v1/system/services", get(get_service_registry))
        .route("/v1/system/updates", get(get_update_status))
        .route("/v1/plugins", get(get_plugins))
        .route("/v1/plugins/manifests", get(get_plugin_manifests))
        .route("/v1/plugins/{id}/state", post(update_plugin_state))
        .route("/v1/plugins/{id}/install", post(stage_plugin_install))
        .route("/v1/status/network", get(get_network_status))
        .route("/v1/status/battery", get(get_battery_status))
        .route("/v1/power/{action}", post(power_action))
        .route("/v1/notify", post(notify))
        // Static bundle (index.html, terminal/*, etc.)
        .fallback_service(ServeDir::new("bundle"))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("sold listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn prepare_files_dir(files_dir: PathBuf) -> PathBuf {
    match fs::create_dir_all(&files_dir).await {
        Ok(()) => files_dir,
        Err(error) if files_dir == FsPath::new(DEFAULT_FILES_DIR) => {
            let fallback = PathBuf::from(LOCAL_FILES_DIR);
            eprintln!(
                "sold: cannot use {DEFAULT_FILES_DIR}: {error}; using {}",
                fallback.display()
            );
            fs::create_dir_all(&fallback)
                .await
                .expect("failed to create local Soliloquy files directory");
            fallback
        }
        Err(error) => panic!(
            "failed to create Soliloquy files directory {}: {error}",
            files_dir.display()
        ),
    }
}

// ── terminal handlers ─────────────────────────────────────────────────────────

async fn serve_terminal_page() -> Html<&'static str> {
    Html(include_str!("../../bundle/terminal/index.html"))
}

async fn browse_page(
    Query(query): Query<BrowseQuery>,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let url = parse_remote_url(&query.url)?;
    let settings = load_settings(&state).await;
    if settings.block_private_network_proxy && !remote_url_allowed(&url) {
        return Ok(Html(render_browser_message_page(
            url.as_str(),
            "blocked",
            "private network proxy blocked",
        )));
    }
    let cache_key = url.to_string();
    let ttl = Duration::from_secs(settings.page_proxy_cache_seconds.into());

    if let Some(entry) = state.page_cache.get(&cache_key) {
        if ttl > Duration::ZERO && entry.stored_at.elapsed() <= ttl {
            return Ok(Html(entry.html.clone()));
        }
    }

    let response = match state.http.get(url.clone()).send().await {
        Ok(response) => response,
        Err(_) => {
            return Ok(Html(render_browser_message_page(
                url.as_str(),
                "unavailable",
                "remote page did not respond",
            )));
        }
    };
    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(_) => {
            return Ok(Html(render_browser_message_page(
                url.as_str(),
                "unavailable",
                "remote page returned an error",
            )));
        }
    };
    let body = match response.text().await {
        Ok(body) => body,
        Err(_) => {
            return Ok(Html(render_browser_message_page(
                url.as_str(),
                "unavailable",
                "remote page body could not be read",
            )));
        }
    };
    let html = render_remote_page(url.as_str(), &body);

    if ttl > Duration::ZERO {
        state.page_cache.insert(
            cache_key,
            CachedPage {
                html: html.clone(),
                stored_at: Instant::now(),
            },
        );
    }

    Ok(Html(html))
}

async fn device_status(State(state): State<AppState>) -> Json<DeviceStatus> {
    let now_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    Json(DeviceStatus {
        service: "sold",
        now_unix_ms,
        uptime_ms: state.started_at.elapsed().as_millis(),
        files_dir: state.files_dir.to_string_lossy().to_string(),
        terminal_sessions: state.sessions.len(),
        terminal_shells: SHELLS,
    })
}

async fn device_action(
    Path(action): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<DeviceActionResult>, StatusCode> {
    check_mutation_auth(&headers, &state)?;
    apply_device_action(action, &state)
}

async fn runtime_status(State(state): State<AppState>) -> Json<RuntimeStatus> {
    let settings = load_settings(&state).await;
    Json(build_runtime_status(&settings))
}

async fn os_status(State(state): State<AppState>) -> Json<OsStatus> {
    let config = state.system_config.read().await.clone();
    let package_manager = load_package_manager_config();
    let services = (*state.service_registry).clone();
    let updates = UpdateStatus {
        policy: (*state.update_policy).clone(),
        state: load_update_state(state.update_state_path.as_ref()),
    };
    let network = read_network_status().await;
    let battery = read_battery_status().await;
    let settings = load_settings(&state).await;
    let runtime = build_runtime_status(&settings);
    Json(OsStatus {
        config,
        package_manager,
        services,
        updates,
        network,
        battery,
        power: PowerCapabilityStatus {
            actions: vec!["shutdown", "sleep", "logout"],
            enabled: power_actions_enabled(),
        },
        display: runtime.display,
    })
}

/// POST /v1/term/session — fork PTY + shell, return session_id.
async fn create_term_session(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let shell = SHELLS
        .iter()
        .find(|&&p| std::path::Path::new(p).exists())
        .copied()
        .unwrap_or("/bin/sh");

    let win = Winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let OpenptyResult { master, slave } =
        openpty(Some(&win), None).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract raw fds before fork so RAII doesn't run in child.
    let master_raw: i32 = master.into_raw_fd();
    let slave_raw: i32 = slave.into_raw_fd();

    // SAFETY: fork/exec pattern. No async code between fork and exec in child.
    let fork_result = unsafe { fork() }.map_err(|_| {
        // Clean up fds on fork failure.
        let _ = close(master_raw);
        let _ = close(slave_raw);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match fork_result {
        ForkResult::Child => {
            // ── child ────────────────────────────────────────────────────────
            let _ = setsid();
            let _ = close(master_raw);

            // Make slave the controlling terminal.
            #[cfg(target_os = "linux")]
            unsafe {
                let _ = pty_set_ctty(slave_raw, 0);
            }

            // Redirect stdin/stdout/stderr to slave PTY.
            let _ = dup2(slave_raw, 0);
            let _ = dup2(slave_raw, 1);
            let _ = dup2(slave_raw, 2);
            if slave_raw > 2 {
                let _ = close(slave_raw);
            }

            // Set TERM so shell/programs render correctly.
            std::env::set_var("TERM", "xterm-256color");
            std::env::set_var("COLORTERM", "truecolor");

            let shell_cstr = std::ffi::CString::new(shell).unwrap();
            let _ = execvp(&shell_cstr, &[&shell_cstr]);
            // execvp only returns on error.
            std::process::exit(1);
        }
        ForkResult::Parent { child } => {
            // ── parent ───────────────────────────────────────────────────────
            let _ = close(slave_raw); // parent doesn't need slave end

            // Set master fd non-blocking for async I/O.
            if let Ok(flags) = fcntl(master_raw, FcntlArg::F_GETFL) {
                let _ = fcntl(
                    master_raw,
                    FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK),
                );
            }

            let id = uuid::Uuid::new_v4().to_string();
            state.sessions.insert(
                id.clone(),
                PtySession {
                    master_fd: master_raw,
                    child_pid: child,
                    cols: 80,
                    rows: 24,
                },
            );

            Ok(Json(serde_json::json!({ "session_id": id })))
        }
    }
}

/// GET /v1/term/session/:id/ws — WebSocket PTY bridge.
async fn term_ws(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    Query(_q): Query<WsQuery>,
    State(state): State<AppState>,
) -> Result<axum::response::Response, StatusCode> {
    // Verify session exists; read master_fd without taking ownership.
    let master_fd = state
        .sessions
        .get(&id)
        .map(|s| s.master_fd)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(ws.on_upgrade(move |socket| handle_term_ws(socket, id, master_fd, state)))
}

async fn handle_term_ws(mut socket: WebSocket, id: String, master_fd: i32, state: AppState) {
    // Wrap master_fd in tokio's AsyncFd for non-blocking async I/O.
    // SAFETY: master_fd is valid and non-blocking (set above). PtySession.Drop
    // owns the close(), so AsyncFd must NOT close it — we use ManuallyDrop below.
    let async_fd = match tokio::io::unix::AsyncFd::new(master_fd) {
        Ok(fd) => fd,
        Err(_) => return,
    };

    let mut read_buf = vec![0u8; 4096];

    loop {
        tokio::select! {
            // PTY → WebSocket
            guard = async_fd.readable() => {
                match guard {
                    Err(_) => break,
                    Ok(mut g) => {
                        // SAFETY: master_fd is valid.
                        let result = unsafe {
                            let bfd = BorrowedFd::borrow_raw(master_fd);
                            nix::unistd::read(bfd.as_raw_fd(), &mut read_buf)
                        };
                        g.clear_ready();
                        match result {
                            Err(nix::errno::Errno::EAGAIN) => continue,
                            Err(_) | Ok(0) => break,
                            Ok(n) => {
                                let data = read_buf[..n].to_vec();
                                if socket.send(Message::Binary(data.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            // WebSocket → PTY
            msg = socket.recv() => {
                match msg {
                    None | Some(Err(_)) => break,
                    Some(Ok(Message::Binary(data))) => {
                        // Raw bytes → PTY stdin (keystrokes).
                        // SAFETY: master_fd valid for the session lifetime.
                        unsafe {
                            let bfd = BorrowedFd::borrow_raw(master_fd);
                            nix::unistd::write(bfd, &data).ok();
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        // JSON control message.
                        if let Ok(WsClientMsg::Resize { cols, rows }) =
                            serde_json::from_str::<WsClientMsg>(&text)
                        {
                            let win = Winsize {
                                ws_row: rows, ws_col: cols,
                                ws_xpixel: 0, ws_ypixel: 0,
                            };
                            // SAFETY: master_fd valid.
                            unsafe { let _ = pty_set_winsize(master_fd, &win); }

                            if let Some(mut s) = state.sessions.get_mut(&id) {
                                s.cols = cols;
                                s.rows = rows;
                            }
                            // SIGWINCH to shell so it reflows output.
                            if let Some(s) = state.sessions.get(&id) {
                                let _ = nix::sys::signal::kill(s.child_pid, Signal::SIGWINCH);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }

    // Cleanup: SIGHUP shell, remove session (Drop closes master_fd).
    if let Some((_, session)) = state.sessions.remove(&id) {
        let _ = nix::sys::signal::kill(session.child_pid, Signal::SIGHUP);
        // `session` dropped here → PtySession::drop() closes master_fd.
        // AsyncFd wraps the raw int (not OwnedFd), so no double-close.
        // Explicitly forget the AsyncFd so it doesn't close.
        std::mem::forget(async_fd);
    }
}

// ── files API ────────────────────────────────────────────────────────────────

async fn list_files(State(state): State<AppState>) -> Result<Json<Vec<FileInfo>>, StatusCode> {
    let mut entries = fs::read_dir(&state.files_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut files = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let metadata = entry
            .metadata()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        files.push(FileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
        });
    }
    Ok(Json(files))
}

async fn read_file(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<String, StatusCode> {
    fs::read_to_string(safe_file_path(&state.files_dir, &name)?)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn write_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<FileContent>,
) -> Result<(), StatusCode> {
    check_mutation_auth(&headers, &state)?;
    fs::write(safe_file_path(&state.files_dir, &name)?, payload.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<(), StatusCode> {
    check_mutation_auth(&headers, &state)?;
    fs::remove_file(safe_file_path(&state.files_dir, &name)?)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

// ── settings API ─────────────────────────────────────────────────────────────

async fn get_settings(State(state): State<AppState>) -> Result<Json<Settings>, StatusCode> {
    Ok(Json(load_settings(&state).await))
}

async fn put_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(settings): Json<Settings>,
) -> Result<(), StatusCode> {
    check_mutation_auth(&headers, &state)?;
    let content =
        serde_json::to_string(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    fs::write(state.files_dir.join("settings.json"), content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    write_runtime_env(&state.runtime_env_path, &settings).await?;
    Ok(())
}

async fn load_settings(state: &AppState) -> Settings {
    let path = state.files_dir.join("settings.json");
    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

async fn write_runtime_env(path: &FsPath, settings: &Settings) -> Result<(), StatusCode> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let flags = v8_flags(settings).join(" ");
    let content = format!(
        "SOLILOQUY_JS_ENGINE={}\nSOLILOQUY_V8_FLAGS={}\nSOLILOQUY_RENDERER_PROCESS_LIMIT={}\nSOLILOQUY_SITE_ISOLATION={}\nSOLILOQUY_SANDBOX={}\nSOLILOQUY_GPU_COMPOSITING={}\nSOLILOQUY_HARDWARE_ACCELERATION={}\nSOLILOQUY_HTTP3={}\nSOLILOQUY_CODE_CACHE={}\nSOLILOQUY_TARGET_FPS={}\nSOLILOQUY_LOW_POWER_IDLE={}\nSERVO_DISPLAY_BACKEND={}\nWINIT_UNIX_BACKEND={}\nEGL_PLATFORM={}\n",
        shell_escape(&settings.js_engine),
        shell_escape(&flags),
        settings.renderer_process_limit,
        bool_env(settings.site_isolation),
        bool_env(settings.sandbox),
        bool_env(settings.gpu_compositing),
        bool_env(settings.hardware_acceleration),
        bool_env(settings.http3_enabled),
        bool_env(settings.v8_code_cache_enabled),
        settings.target_fps,
        bool_env(settings.low_power_idle),
        shell_escape(&settings.display_backend),
        shell_escape(&settings.display_backend),
        shell_escape(&settings.display_backend),
    );
    fs::write(path, content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn v8_flags(settings: &Settings) -> Vec<String> {
    let mut flags = Vec::new();
    if settings.v8_turbofan_enabled {
        flags.push("--turbofan".to_string());
    }
    flags.push(format!("--max-heap-size={}", settings.v8_max_heap_size_mb));
    flags.push(format!(
        "--initial-heap-size={}",
        settings.v8_initial_heap_size_mb
    ));
    if settings.v8_lazy_compilation {
        flags.push("--lazy".to_string());
    }
    if settings.v8_concurrent_gc {
        flags.push("--concurrent-marking".to_string());
        flags.push("--parallel-scavenge".to_string());
    }
    if settings.v8_incremental_marking {
        flags.push("--incremental-marking".to_string());
    }
    if settings.v8_code_cache_enabled {
        flags.push("--serialize-toplevel".to_string());
    }
    flags
}

fn bool_env(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn build_runtime_status(settings: &Settings) -> RuntimeStatus {
    let env_engine = std::env::var("SOLILOQUY_JS_ENGINE").unwrap_or_default();
    let requested_engine = if env_engine.trim().is_empty() {
        settings.js_engine.clone()
    } else {
        env_engine
    };
    let active_backend = std::env::var("XDG_SESSION_TYPE")
        .or_else(|_| std::env::var("SERVO_DISPLAY_BACKEND"))
        .unwrap_or_else(|_| settings.display_backend.clone());
    RuntimeStatus {
        service: "sold",
        vinix: VinixReferenceStatus {
            mode: "reference-only",
            license: "GPL-2.0",
            url: "https://github.com/vlang/vinix",
        },
        javascript: JavascriptRuntimeStatus {
            requested_engine: requested_engine.clone(),
            active_engine: "mozjs",
            bridge_ready: false,
            servo_controls_javascript: true,
            swap_stage: "dual-runtime-preparation",
            restart_required: requested_engine != settings.js_engine,
        },
        display: DisplayRuntimeStatus {
            configured_backend: settings.display_backend.clone(),
            active_backend,
            wayland_required: settings.wayland_required,
            x11_fallback: false,
            headless: env_flag("SOL_SERVO_HEADLESS"),
        },
        optimizations: vec![
            RuntimeOptimizationStatus {
                id: "v8-flags",
                label: "V8 optimization flags",
                configured: true,
                active: false,
                status: "restart-required",
            },
            RuntimeOptimizationStatus {
                id: "http3",
                label: "HTTP/3 transport",
                configured: settings.http3_enabled,
                active: false,
                status: "configured",
            },
            RuntimeOptimizationStatus {
                id: "site-isolation",
                label: "Site isolation",
                configured: settings.site_isolation,
                active: false,
                status: "unsupported",
            },
            RuntimeOptimizationStatus {
                id: "renderer-limit",
                label: "Renderer process limit",
                configured: settings.renderer_process_limit > 0,
                active: false,
                status: "configured",
            },
            RuntimeOptimizationStatus {
                id: "gpu-compositing",
                label: "GPU compositing",
                configured: settings.gpu_compositing,
                active: false,
                status: "display-dependent",
            },
            RuntimeOptimizationStatus {
                id: "code-cache",
                label: "Code cache",
                configured: settings.v8_code_cache_enabled,
                active: false,
                status: "configured",
            },
        ],
    }
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name),
        Ok(value)
            if matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
    )
}

fn default_system_config() -> SystemConfig {
    SystemConfig {
        filesystem: FilesystemPolicy {
            immutable_root: true,
            user_home_root: "/home".to_string(),
            user_writable_scope: "home-only".to_string(),
            tmp_policy: TmpPolicy {
                path: "/tmp".to_string(),
                mode: "system-only".to_string(),
            },
        },
        browser: BrowserPolicy {
            profile_management: "system".to_string(),
            profiles_root: "/var/lib/soliloquy/browser/profiles".to_string(),
            cache_root: "/var/lib/soliloquy/browser/cache".to_string(),
            state_root: "/var/lib/soliloquy/browser/state".to_string(),
            logs_root: "/var/lib/soliloquy/browser/logs".to_string(),
        },
        package_manager: PackageManagerPolicy {
            id: "wax".to_string(),
            mode: "system-packages".to_string(),
            binary: "/usr/local/bin/wax".to_string(),
            root: "/var/lib/soliloquy/wax".to_string(),
            developer_mode_required: false,
        },
        plugins: vec![PluginConfig {
            id: "remote-sync".to_string(),
            display_name: "Remote Sync".to_string(),
            kind: "optional-download".to_string(),
            enabled: false,
            sync: SyncFeatureFlags {
                files: false,
                photos: false,
                clipboard: false,
            },
        }],
    }
}

fn default_package_manager_config() -> PackageManagerConfig {
    PackageManagerConfig {
        id: "wax".to_string(),
        display_name: "Wax".to_string(),
        mode: "system-packages".to_string(),
        binary: "/usr/local/bin/wax".to_string(),
        state_root: "/var/lib/soliloquy/wax".to_string(),
        developer_mode_required: false,
        manages: vec![
            "system-packages".to_string(),
            "userland-packages".to_string(),
            "generations".to_string(),
            "manifests".to_string(),
        ],
        does_not_manage: vec!["browser-profile-vault".to_string()],
    }
}

fn load_package_manager_config() -> PackageManagerConfig {
    let path = std::env::var("SOLILOQUY_PACKAGE_MANAGER_FILE")
        .unwrap_or_else(|_| "/etc/soliloquy/package-manager.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| default_package_manager_config()),
        Err(_) => default_package_manager_config(),
    }
}

fn default_service_registry() -> ServiceRegistry {
    ServiceRegistry {
        services: vec![
            ServiceDefinition {
                id: "sold".to_string(),
                display_name: "Soliloquy Local Server".to_string(),
                run_as: "sold".to_string(),
                restart: "always".to_string(),
                dependencies: vec!["networking".to_string()],
                optional: false,
                state_paths: vec![
                    "/var/lib/soliloquy/system".to_string(),
                    "/var/log/soliloquy".to_string(),
                ],
            },
            ServiceDefinition {
                id: "sol-session".to_string(),
                display_name: "Soliloquy Session".to_string(),
                run_as: "root".to_string(),
                restart: "always".to_string(),
                dependencies: vec!["sold".to_string(), "seatd".to_string()],
                optional: false,
                state_paths: vec![
                    "/run/user/0".to_string(),
                    "/var/lib/soliloquy/browser".to_string(),
                ],
            },
            ServiceDefinition {
                id: "remote-sync".to_string(),
                display_name: "Remote Sync Plugin".to_string(),
                run_as: "sold".to_string(),
                restart: "on-failure".to_string(),
                dependencies: vec!["sold".to_string()],
                optional: true,
                state_paths: vec!["/var/lib/soliloquy/system/plugins".to_string()],
            },
        ],
    }
}

fn default_plugin_manifests() -> Vec<PluginManifest> {
    vec![PluginManifest {
        id: "remote-sync".to_string(),
        display_name: "Remote Sync".to_string(),
        kind: "optional-download".to_string(),
        entrypoint: "/var/lib/soliloquy/system/plugins/remote-sync".to_string(),
        capabilities: vec![
            "profile-sync".to_string(),
            "encrypted-relay".to_string(),
            "cross-device-sync".to_string(),
        ],
        sync_features: SyncFeatureFlags {
            files: false,
            photos: false,
            clipboard: false,
        },
        packages: Vec::new(),
    }]
}

fn default_update_policy() -> UpdatePolicy {
    UpdatePolicy {
        strategy: "atomic-generations".to_string(),
        rollback_enabled: true,
        channels: vec!["stable".to_string()],
        generation_root: "/sysroot/soliloquy".to_string(),
        retained_generations: 2,
    }
}

fn default_update_state() -> UpdateState {
    UpdateState {
        active_generation: "soliloquy-0001".to_string(),
        staged_generation: None,
        rollback_generation: None,
        last_result: "bootstrapped".to_string(),
    }
}

fn system_plugin_state_path() -> PathBuf {
    std::env::var("SOLILOQUY_PLUGIN_STATE_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/soliloquy/system/plugin-state.json"))
}

fn system_plugin_install_state_path() -> PathBuf {
    std::env::var("SOLILOQUY_PLUGIN_INSTALL_STATE_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/soliloquy/system/plugin-installs.json"))
}

fn system_update_state_path() -> PathBuf {
    std::env::var("SOLILOQUY_UPDATE_STATE_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/soliloquy/system/update-state.json"))
}

fn load_system_config(plugin_state_path: &FsPath) -> SystemConfig {
    let path = std::env::var("SOLILOQUY_SYSTEM_CONFIG")
        .unwrap_or_else(|_| "/etc/soliloquy/system.json".to_string());
    let base = match std::fs::read_to_string(&path) {
        Ok(raw) => {
            serde_json::from_str::<SystemConfig>(&raw).unwrap_or_else(|_| default_system_config())
        }
        Err(_) => default_system_config(),
    };
    apply_persisted_plugin_state(base, plugin_state_path)
}

fn load_plugin_manifests() -> Vec<PluginManifest> {
    let manifest_dir = std::env::var("SOLILOQUY_PLUGIN_MANIFEST_DIR")
        .unwrap_or_else(|_| "/etc/soliloquy/plugins".to_string());
    let Ok(entries) = std::fs::read_dir(&manifest_dir) else {
        return default_plugin_manifests();
    };
    let mut manifests = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&raw) {
            manifests.push(manifest);
        }
    }
    if manifests.is_empty() {
        default_plugin_manifests()
    } else {
        manifests.sort_by(|a, b| a.id.cmp(&b.id));
        manifests
    }
}

fn load_service_registry() -> ServiceRegistry {
    let path = std::env::var("SOLILOQUY_SERVICE_REGISTRY")
        .unwrap_or_else(|_| "/etc/soliloquy/services.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| default_service_registry()),
        Err(_) => default_service_registry(),
    }
}

fn load_update_policy() -> UpdatePolicy {
    let path = std::env::var("SOLILOQUY_UPDATE_POLICY_FILE")
        .unwrap_or_else(|_| "/etc/soliloquy/update-policy.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| default_update_policy()),
        Err(_) => default_update_policy(),
    }
}

fn load_update_state(path: &FsPath) -> UpdateState {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| default_update_state()),
        Err(_) => default_update_state(),
    }
}

fn load_plugin_install_state(path: &FsPath) -> PersistedPluginInstallState {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| PersistedPluginInstallState {
            plugins: Vec::new(),
        }),
        Err(_) => PersistedPluginInstallState {
            plugins: Vec::new(),
        },
    }
}

fn persist_plugin_state(
    config: &SystemConfig,
    plugin_state_path: &FsPath,
) -> Result<(), StatusCode> {
    if let Some(parent) = plugin_state_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let persisted = PersistedPluginState {
        plugins: config.plugins.clone(),
    };
    let encoded =
        serde_json::to_string_pretty(&persisted).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(plugin_state_path, encoded).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn persist_plugin_install_state(
    state: &PersistedPluginInstallState,
    path: &FsPath,
) -> Result<(), StatusCode> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let encoded =
        serde_json::to_string_pretty(state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(path, encoded).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn apply_persisted_plugin_state(
    mut config: SystemConfig,
    plugin_state_path: &FsPath,
) -> SystemConfig {
    let Ok(raw) = std::fs::read_to_string(plugin_state_path) else {
        return config;
    };
    let Ok(persisted) = serde_json::from_str::<PersistedPluginState>(&raw) else {
        return config;
    };
    for persisted_plugin in persisted.plugins {
        if let Some(plugin) = config
            .plugins
            .iter_mut()
            .find(|plugin| plugin.id == persisted_plugin.id)
        {
            plugin.enabled = persisted_plugin.enabled;
            plugin.sync = persisted_plugin.sync;
        }
    }
    config
}

fn sha256_file(path: &FsPath) -> Result<String, StatusCode> {
    let bytes = std::fs::read(path).map_err(|_| StatusCode::BAD_REQUEST)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{:x}", digest))
}

fn safe_file_path(root: &FsPath, name: &str) -> Result<PathBuf, StatusCode> {
    let requested = FsPath::new(name);
    if requested.is_absolute() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut path = root.to_path_buf();
    let mut saw_component = false;
    for component in requested.components() {
        match component {
            Component::Normal(part) => {
                path.push(part);
                saw_component = true;
            }
            Component::CurDir => {}
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    if saw_component {
        Ok(path)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn check_mutation_auth(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    if !origin_allowed(headers) {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(token) = &state.token {
        if header_token(headers).as_deref() != Some(token.as_str()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(())
}

fn origin_allowed(headers: &HeaderMap) -> bool {
    for header in ["origin", "referer"] {
        let Some(value) = headers.get(header).and_then(|value| value.to_str().ok()) else {
            continue;
        };
        let Ok(url) = reqwest::Url::parse(value) else {
            return false;
        };
        let Some(host) = url.host_str() else {
            return false;
        };
        if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
            return false;
        }
    }
    true
}

fn header_token(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get("x-sol-token")
        .and_then(|value| value.to_str().ok())
    {
        return Some(value.to_string());
    }
    let value = headers.get("authorization")?.to_str().ok()?.trim();
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .map(ToString::to_string)
}

async fn get_system_config(State(state): State<AppState>) -> Json<SystemConfig> {
    Json(state.system_config.read().await.clone())
}

async fn get_package_manager_config() -> Json<PackageManagerConfig> {
    Json(load_package_manager_config())
}

async fn get_service_registry(State(state): State<AppState>) -> Json<ServiceRegistry> {
    Json((*state.service_registry).clone())
}

async fn get_update_status(State(state): State<AppState>) -> Json<UpdateStatus> {
    Json(UpdateStatus {
        policy: (*state.update_policy).clone(),
        state: load_update_state(state.update_state_path.as_ref()),
    })
}

async fn get_plugins(State(state): State<AppState>) -> Json<Vec<PluginInventoryEntry>> {
    let install_state = load_plugin_install_state(state.plugin_install_state_path.as_ref());
    let entries = state
        .plugin_manifests
        .iter()
        .cloned()
        .map(|manifest| {
            let install = install_state
                .plugins
                .iter()
                .find(|install| install.id == manifest.id)
                .cloned();
            PluginInventoryEntry { manifest, install }
        })
        .collect();
    Json(entries)
}

async fn get_plugin_manifests(State(state): State<AppState>) -> Json<Vec<PluginManifest>> {
    Json((*state.plugin_manifests).clone())
}

async fn update_plugin_state(
    Path(id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(update): Json<PluginStateUpdate>,
) -> Result<Json<PluginConfig>, StatusCode> {
    check_mutation_auth(&headers, &state)?;
    let mut config = state.system_config.write().await;
    let Some(index) = config.plugins.iter().position(|plugin| plugin.id == id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    if let Some(enabled) = update.enabled {
        config.plugins[index].enabled = enabled;
    }
    if let Some(sync) = update.sync {
        config.plugins[index].sync = sync;
    }
    let updated = config.plugins[index].clone();
    persist_plugin_state(&config, state.plugin_state_path.as_ref())?;
    Ok(Json(updated))
}

async fn stage_plugin_install(
    Path(id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<PluginInstallRequest>,
) -> Result<Json<PluginInstallRecord>, StatusCode> {
    check_mutation_auth(&headers, &state)?;
    let Some(manifest) = state
        .plugin_manifests
        .iter()
        .find(|manifest| manifest.id == id)
    else {
        return Err(StatusCode::NOT_FOUND);
    };
    let Some(package) = manifest
        .packages
        .iter()
        .find(|package| package.version == request.version)
    else {
        return Err(StatusCode::NOT_FOUND);
    };
    let package_root = std::env::var("SOLILOQUY_PLUGIN_PACKAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/soliloquy/system/plugin-packages"));
    let source_path = safe_file_path(&package_root, &request.source_path)?;
    let digest = sha256_file(&source_path)?;
    if digest != package.sha256 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut install_state = load_plugin_install_state(state.plugin_install_state_path.as_ref());
    let record = PluginInstallRecord {
        id: manifest.id.clone(),
        installed: true,
        version: Some(package.version.clone()),
        source_path: Some(source_path.to_string_lossy().to_string()),
        sha256: Some(digest),
        verified: true,
    };
    if let Some(existing) = install_state
        .plugins
        .iter_mut()
        .find(|install| install.id == record.id)
    {
        *existing = record.clone();
    } else {
        install_state.plugins.push(record.clone());
    }
    persist_plugin_install_state(&install_state, state.plugin_install_state_path.as_ref())?;
    Ok(Json(record))
}

async fn get_network_status() -> Json<NetworkStatus> {
    Json(read_network_status().await)
}

async fn get_battery_status() -> Json<BatteryStatus> {
    Json(read_battery_status().await)
}

async fn power_action(
    Path(action): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<DeviceActionResult>, StatusCode> {
    check_mutation_auth(&headers, &state)?;
    apply_device_action(action, &state)
}

async fn notify(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<NotifyRequest>,
) -> Result<Json<NotifyResult>, StatusCode> {
    check_mutation_auth(&headers, &state)?;
    if payload.title.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    match std::process::Command::new("notify-send")
        .arg(payload.title)
        .arg(payload.body)
        .spawn()
    {
        Ok(_) => Ok(Json(NotifyResult {
            delivered: true,
            message: "notification sent",
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Json(NotifyResult {
            delivered: false,
            message: "notification service unavailable",
        })),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn read_network_status() -> NetworkStatus {
    #[cfg(target_os = "linux")]
    {
        let mut interfaces = Vec::new();
        let mut connected = false;
        if let Ok(mut entries) = fs::read_dir("/sys/class/net").await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "lo" {
                    continue;
                }
                let state_path = entry.path().join("operstate");
                if let Ok(state) = fs::read_to_string(state_path).await {
                    if state.trim() == "up" {
                        connected = true;
                    }
                }
                interfaces.push(name);
            }
        }
        NetworkStatus {
            connected,
            interfaces,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        NetworkStatus {
            connected: false,
            interfaces: Vec::new(),
        }
    }
}

async fn read_battery_status() -> BatteryStatus {
    #[cfg(target_os = "linux")]
    {
        if let Ok(mut entries) = fs::read_dir("/sys/class/power_supply").await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let type_path = entry.path().join("type");
                let Ok(kind) = fs::read_to_string(type_path).await else {
                    continue;
                };
                if kind.trim() != "Battery" {
                    continue;
                }
                let percent = fs::read_to_string(entry.path().join("capacity"))
                    .await
                    .ok()
                    .and_then(|value| value.trim().parse::<u8>().ok());
                let charging = fs::read_to_string(entry.path().join("status"))
                    .await
                    .ok()
                    .map(|value| {
                        matches!(
                            value.trim().to_ascii_lowercase().as_str(),
                            "charging" | "full"
                        )
                    });
                return BatteryStatus {
                    present: true,
                    percent,
                    charging,
                };
            }
        }
        BatteryStatus {
            present: false,
            percent: None,
            charging: None,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        BatteryStatus {
            present: false,
            percent: None,
            charging: None,
        }
    }
}

fn apply_device_action(
    action: String,
    state: &AppState,
) -> Result<Json<DeviceActionResult>, StatusCode> {
    match action.as_str() {
        "logout" => {
            for session in state.sessions.iter() {
                let _ = nix::sys::signal::kill(session.child_pid, Signal::SIGHUP);
            }
            state.sessions.clear();
            Ok(Json(DeviceActionResult {
                action,
                accepted: true,
                applied: true,
                message: "sessions closed",
            }))
        }
        "shutdown" | "sleep" => {
            if power_actions_enabled() {
                run_power_action(&action)?;
                Ok(Json(DeviceActionResult {
                    action,
                    accepted: true,
                    applied: true,
                    message: "power action sent",
                }))
            } else {
                Ok(Json(DeviceActionResult {
                    action,
                    accepted: true,
                    applied: false,
                    message: "power action disabled",
                }))
            }
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn power_actions_enabled() -> bool {
    std::env::var("SOLILOQUY_ENABLE_POWER_ACTIONS")
        .ok()
        .as_deref()
        == Some("1")
}

fn parse_remote_url(raw: &str) -> Result<reqwest::Url, StatusCode> {
    let url = reqwest::Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn remote_url_allowed(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized == "localhost" || normalized.ends_with(".localhost") {
        return false;
    }
    if normalized.ends_with(".local") || normalized.ends_with(".internal") {
        return false;
    }
    if let Ok(ip) = normalized.parse::<IpAddr>() {
        return public_ip_allowed(ip);
    }
    true
}

fn public_ip_allowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || (octets[0] == 100 && (64..=127).contains(&octets[1])))
        }
        IpAddr::V6(ip) => {
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.segments()[0] & 0xffc0 == 0xfe80)
        }
    }
}

fn render_remote_page(url: &str, body: &str) -> String {
    let escaped_url = escape_html_attr(url);
    let base = format!(r#"<base href="{escaped_url}">"#);
    let security = r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self' http: https: data: blob:; script-src 'none'; connect-src http: https:; form-action 'none'; frame-ancestors 'none'">"#;
    let head_insert = format!("{base}{security}");
    let lower = body.to_ascii_lowercase();

    if let Some(head_start) = lower.find("<head") {
        if let Some(head_end) = body[head_start..].find('>') {
            let insert_at = head_start + head_end + 1;
            let mut output = String::with_capacity(body.len() + head_insert.len());
            output.push_str(&body[..insert_at]);
            output.push_str(&head_insert);
            output.push_str(&body[insert_at..]);
            return output;
        }
    }

    format!(
        r#"<!doctype html><html><head><meta charset="utf-8">{head_insert}<style>body{{margin:0;padding:24px;font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;color:#fff;background:#000}}pre{{white-space:pre-wrap;word-break:break-word}}</style></head><body><pre>{}</pre></body></html>"#,
        escape_html_text(body)
    )
}

fn render_browser_message_page(url: &str, title: &str, message: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="color-scheme" content="dark"><style>html,body{{height:100%;margin:0;background:#000;color:#fff;font:15px/1.45 -apple-system,BlinkMacSystemFont,"SF Pro Display","Segoe UI",sans-serif}}main{{min-height:100%;display:grid;place-items:center;padding:32px;box-sizing:border-box}}section{{width:min(520px,100%);text-align:left}}p{{margin:0;color:rgba(255,255,255,.62)}}h1{{margin:0 0 10px;font-size:28px;line-height:1.05;letter-spacing:-.04em;font-weight:650}}code{{display:block;margin-top:22px;color:rgba(255,255,255,.44);font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;word-break:break-all}}</style></head><body><main><section><h1>{}</h1><p>{}</p><code>{}</code></section></main></body></html>"#,
        escape_html_text(title),
        escape_html_text(message),
        escape_html_text(url)
    )
}

fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn run_power_action(action: &str) -> Result<(), StatusCode> {
    let Some((program, args)) = power_command(action) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    std::process::Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

fn power_command(action: &str) -> Option<(&'static str, &'static [&'static str])> {
    match action {
        "shutdown" => Some(("/sbin/poweroff", &[])),
        "sleep" => Some(("/usr/bin/systemctl", &["suspend"])),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_url_accepts_http_and_https_only() {
        assert!(parse_remote_url("https://example.com").is_ok());
        assert!(parse_remote_url("http://example.com").is_ok());
        assert!(parse_remote_url("file:///etc/passwd").is_err());
        assert!(parse_remote_url("os://terminal").is_err());
    }

    #[test]
    fn remote_html_gets_base_tag() {
        let html = render_remote_page(
            "https://example.com/path/",
            "<html><head><title>x</title></head></html>",
        );
        assert!(html.contains(r#"<base href="https://example.com/path/">"#));
        assert!(html.contains(r#"script-src 'none'"#));
        assert!(html.contains("<title>x</title>"));
    }

    #[test]
    fn plain_remote_text_is_escaped() {
        let html = render_remote_page("https://example.com/", "<script>alert(1)</script>");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn browser_message_page_is_dark_and_escaped() {
        let html = render_browser_message_page("http://127.0.0.1/<x>", "blocked", "<no>");
        assert!(html.contains("background:#000"));
        assert!(html.contains("blocked"));
        assert!(html.contains("&lt;no&gt;"));
        assert!(html.contains("http://127.0.0.1/&lt;x&gt;"));
    }

    #[test]
    fn remote_proxy_rejects_private_destinations() {
        for raw in [
            "http://127.0.0.1/",
            "http://localhost/",
            "http://10.0.0.5/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.1.1/",
            "http://100.64.1.1/",
            "http://device.local/",
        ] {
            let url = reqwest::Url::parse(raw).unwrap();
            assert!(!remote_url_allowed(&url), "{raw}");
        }
        let url = reqwest::Url::parse("https://example.com/").unwrap();
        assert!(remote_url_allowed(&url));
    }

    #[test]
    fn power_actions_are_limited_to_known_commands() {
        assert_eq!(
            power_command("shutdown").map(|command| command.0),
            Some("/sbin/poweroff")
        );
        assert_eq!(
            power_command("sleep").map(|command| command.0),
            Some("/usr/bin/systemctl")
        );
        assert!(power_command("format").is_none());
    }

    #[test]
    fn settings_accept_missing_new_fields() {
        let settings: Settings = serde_json::from_str(
            r#"{"theme":"dark","cache_size_mb":64,"enable_javascript":true,"homepage":"os://terminal"}"#,
        )
        .unwrap();
        assert_eq!(settings.cache_size_mb, 64);
        assert_eq!(settings.browser_layout, "vertical");
        assert_eq!(settings.terminal_cursor, "software");
        assert_eq!(settings.js_engine, "v8-experimental");
        assert_eq!(settings.display_backend, "wayland");
    }

    #[test]
    fn file_paths_reject_escape_attempts() {
        let root = FsPath::new("/var/lib/soliloquy/files");
        assert_eq!(
            safe_file_path(root, "notes/today.txt").unwrap(),
            PathBuf::from("/var/lib/soliloquy/files/notes/today.txt")
        );
        assert!(safe_file_path(root, "../etc/passwd").is_err());
        assert!(safe_file_path(root, "/etc/passwd").is_err());
        assert!(safe_file_path(root, "").is_err());
    }

    #[test]
    fn mutation_origin_allows_loopback_only() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", "http://127.0.0.1:8080".parse().unwrap());
        assert!(origin_allowed(&headers));
        headers.insert("origin", "https://example.com".parse().unwrap());
        assert!(!origin_allowed(&headers));
    }

    #[test]
    fn authorization_bearer_token_is_accepted() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer local-secret".parse().unwrap());
        assert_eq!(header_token(&headers).as_deref(), Some("local-secret"));
        headers.insert("x-sol-token", "header-secret".parse().unwrap());
        assert_eq!(header_token(&headers).as_deref(), Some("header-secret"));
    }

    #[test]
    fn package_manager_identity_is_wax() {
        let package_manager = default_package_manager_config();
        assert_eq!(package_manager.id, "wax");
        assert_eq!(package_manager.mode, "system-packages");
    }

    #[test]
    fn runtime_status_is_honest_about_v8_bridge() {
        let runtime = build_runtime_status(&Settings::default());
        assert_eq!(runtime.javascript.requested_engine, "v8-experimental");
        assert_eq!(runtime.javascript.active_engine, "mozjs");
        assert!(!runtime.javascript.bridge_ready);
        assert!(runtime.javascript.servo_controls_javascript);
        assert!(!runtime.display.x11_fallback);
    }

    #[tokio::test]
    async fn runtime_env_exports_wayland_and_optimization_flags() {
        let path = std::env::temp_dir().join(format!(
            "soliloquy-runtime-env-{}.env",
            uuid::Uuid::new_v4()
        ));
        let settings = Settings::default();
        write_runtime_env(&path, &settings).await.unwrap();
        let content = fs::read_to_string(&path).await.unwrap();
        let _ = fs::remove_file(&path).await;
        assert!(content.contains("SOLILOQUY_JS_ENGINE='v8-experimental'"));
        assert!(content.contains("SOLILOQUY_V8_FLAGS='--turbofan --max-heap-size=512"));
        assert!(content.contains("SERVO_DISPLAY_BACKEND='wayland'"));
        assert!(content.contains("WINIT_UNIX_BACKEND='wayland'"));
        assert!(content.contains("EGL_PLATFORM='wayland'"));
    }
}
