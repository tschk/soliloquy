use std::net::SocketAddr;
use std::os::unix::io::{AsRawFd, BorrowedFd, IntoRawFd};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, Json};
use axum::routing::{delete, get, post, put};
use axum::Router;
use dashmap::DashMap;
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::pty::{OpenptyResult, Winsize, openpty};
use nix::sys::signal::Signal;
use nix::unistd::{ForkResult, Pid, close, dup2, execvp, fork, setsid};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower_http::services::ServeDir;

// ── TIOCSWINSZ ioctl (module-level, required by nix macro) ───────────────────
nix::ioctl_write_ptr_bad!(pty_set_winsize, nix::libc::TIOCSWINSZ, Winsize);
#[cfg(target_os = "linux")]
nix::ioctl_write_int_bad!(pty_set_ctty, nix::libc::TIOCSCTTY);

// ── constants ────────────────────────────────────────────────────────────────

const FILES_DIR: &str = "/var/lib/soliloquy/files";
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
}

// ── main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    fs::create_dir_all(FILES_DIR).await.unwrap();

    let state = AppState {
        sessions: Arc::new(DashMap::new()),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
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
        .nest_service("/", ServeDir::new("bundle"))
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

/// POST /v1/term/session — fork PTY + shell, return session_id.
async fn create_term_session(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let shell = SHELLS
        .iter()
        .find(|&&p| std::path::Path::new(p).exists())
        .copied()
        .unwrap_or("/bin/sh");

    let win = Winsize { ws_row: 24, ws_col: 80, ws_xpixel: 0, ws_ypixel: 0 };

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
            unsafe { let _ = pty_set_ctty(slave_raw, 0); }

            // Redirect stdin/stdout/stderr to slave PTY.
            let _ = dup2(slave_raw, 0);
            let _ = dup2(slave_raw, 1);
            let _ = dup2(slave_raw, 2);
            if slave_raw > 2 { let _ = close(slave_raw); }

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
            state.sessions.insert(id.clone(), PtySession {
                master_fd: master_raw,
                child_pid: child,
                cols: 80,
                rows: 24,
            });

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

async fn list_files() -> Result<Json<Vec<FileInfo>>, StatusCode> {
    let mut entries = fs::read_dir(FILES_DIR)
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

async fn read_file(Path(name): Path<String>) -> Result<String, StatusCode> {
    fs::read_to_string(format!("{}/{}", FILES_DIR, name))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn write_file(
    Path(name): Path<String>,
    Json(payload): Json<FileContent>,
) -> Result<(), StatusCode> {
    fs::write(format!("{}/{}", FILES_DIR, name), payload.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_file(Path(name): Path<String>) -> Result<(), StatusCode> {
    fs::remove_file(format!("{}/{}", FILES_DIR, name))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

// ── settings API ─────────────────────────────────────────────────────────────

async fn get_settings() -> Result<Json<Settings>, StatusCode> {
    let path = format!("{}/settings.json", FILES_DIR);
    match fs::read_to_string(&path).await {
        Ok(content) => Ok(Json(serde_json::from_str(&content).unwrap_or_default())),
        Err(_) => Ok(Json(Settings::default())),
    }
}

async fn put_settings(Json(settings): Json<Settings>) -> Result<(), StatusCode> {
    let content =
        serde_json::to_string(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    fs::write(format!("{}/settings.json", FILES_DIR), content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
