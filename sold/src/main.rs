use std::net::SocketAddr;
use std::os::unix::io::{AsRawFd, BorrowedFd, IntoRawFd};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, Json};
use axum::routing::{delete, get, post, put};
use axum::Router;
use dashmap::DashMap;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::sys::signal::Signal;
use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult, Pid};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower_http::services::ServeDir;

// ── TIOCSWINSZ ioctl (module-level, required by nix macro) ───────────────────
nix::ioctl_write_ptr_bad!(pty_set_winsize, nix::libc::TIOCSWINSZ, Winsize);
#[cfg(target_os = "linux")]
nix::ioctl_write_int_bad!(pty_set_ctty, nix::libc::TIOCSCTTY);

// ── constants ────────────────────────────────────────────────────────────────

const DEFAULT_FILES_DIR: &str = "/var/lib/soliloquy/files";
const BROWSE_CACHE_TTL: Duration = Duration::from_secs(15);
/// Shell search order on Alpine.
const SHELLS: &[&str] = &["/usr/bin/zellij", "/bin/ash", "/bin/sh"];

// ── types ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct FileInfo {
    name: String,
    size: u64,
    is_dir: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    theme: String,
    cache_size_mb: u32,
    enable_javascript: bool,
    homepage: String,
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
    http: reqwest::Client,
    page_cache: Arc<DashMap<String, CachedPage>>,
    started_at: Instant,
}

// ── main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let files_dir = std::env::var_os("SOLILOQUY_FILES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FILES_DIR));
    fs::create_dir_all(&files_dir).await.unwrap();

    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        files_dir,
        http: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .user_agent("Soliloquy/0.1")
            .build()
            .unwrap(),
        page_cache: Arc::new(DashMap::new()),
        started_at: Instant::now(),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/browse", get(browse_page))
        .route("/api/device", get(device_status))
        .route("/api/device/{action}", post(device_action))
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
        // Static bundle (index.html, terminal/*, etc.)
        .fallback_service(ServeDir::new("bundle"))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("sold listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
    let cache_key = url.to_string();

    if let Some(entry) = state.page_cache.get(&cache_key) {
        if entry.stored_at.elapsed() <= BROWSE_CACHE_TTL {
            return Ok(Html(entry.html.clone()));
        }
    }

    let body = state
        .http
        .get(url.clone())
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .text()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let html = render_remote_page(url.as_str(), &body);

    state.page_cache.insert(
        cache_key,
        CachedPage {
            html: html.clone(),
            stored_at: Instant::now(),
        },
    );

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
    State(state): State<AppState>,
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
            let enabled = std::env::var("SOLILOQUY_ENABLE_POWER_ACTIONS")
                .ok()
                .as_deref()
                == Some("1");
            if enabled {
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
    fs::read_to_string(state.files_dir.join(name))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn write_file(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<FileContent>,
) -> Result<(), StatusCode> {
    fs::write(state.files_dir.join(name), payload.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_file(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<(), StatusCode> {
    fs::remove_file(state.files_dir.join(name))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

// ── settings API ─────────────────────────────────────────────────────────────

async fn get_settings(State(state): State<AppState>) -> Result<Json<Settings>, StatusCode> {
    let path = state.files_dir.join("settings.json");
    match fs::read_to_string(&path).await {
        Ok(content) => Ok(Json(serde_json::from_str(&content).unwrap_or_default())),
        Err(_) => Ok(Json(Settings::default())),
    }
}

async fn put_settings(
    State(state): State<AppState>,
    Json(settings): Json<Settings>,
) -> Result<(), StatusCode> {
    let content =
        serde_json::to_string(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    fs::write(state.files_dir.join("settings.json"), content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn parse_remote_url(raw: &str) -> Result<reqwest::Url, StatusCode> {
    let url = reqwest::Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn render_remote_page(url: &str, body: &str) -> String {
    let escaped_url = escape_html_attr(url);
    let base = format!(r#"<base href="{escaped_url}">"#);
    let lower = body.to_ascii_lowercase();

    if let Some(head_start) = lower.find("<head") {
        if let Some(head_end) = body[head_start..].find('>') {
            let insert_at = head_start + head_end + 1;
            let mut output = String::with_capacity(body.len() + base.len());
            output.push_str(&body[..insert_at]);
            output.push_str(&base);
            output.push_str(&body[insert_at..]);
            return output;
        }
    }

    format!(
        r#"<!doctype html><html><head><meta charset="utf-8">{base}<style>body{{margin:0;padding:24px;font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;color:#fff;background:#000}}pre{{white-space:pre-wrap;word-break:break-word}}</style></head><body><pre>{}</pre></body></html>"#,
        escape_html_text(body)
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
        assert!(html.contains("<title>x</title>"));
    }

    #[test]
    fn plain_remote_text_is_escaped() {
        let html = render_remote_page("https://example.com/", "<script>alert(1)</script>");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
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
}
