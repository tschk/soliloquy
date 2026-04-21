use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    token: String,
    term_sessions: Arc<Mutex<HashMap<String, TermSession>>>,
}

#[derive(Clone, Debug)]
struct TermSession {
    command: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: &'static str,
}

#[derive(Debug, Serialize)]
struct NetworkStatus {
    connected: bool,
    interfaces: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BatteryStatus {
    present: bool,
    percent: Option<u8>,
    charging: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct NotifyRequest {
    title: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct TermSessionResponse {
    session_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "sold=info,axum=info".to_string()),
        )
        .init();

    let token = std::env::var("SOL_TOKEN").unwrap_or_else(|_| {
        warn!("SOL_TOKEN not set; using development token");
        "dev-token-change-me".to_string()
    });
    let ui_dir = std::env::var("SOLD_UI_DIR").unwrap_or_else(|_| "/usr/local/share/soliloquy/ui".to_string());
    let index_file = format!("{ui_dir}/index.html");
    let static_files = ServeDir::new(&ui_dir).not_found_service(ServeFile::new(index_file));

    let api = Router::new()
        .route("/healthz", get(health))
        .route("/v1/status/network", get(get_network_status))
        .route("/v1/status/battery", get(get_battery_status))
        .route("/v1/power/{action}", post(power_action))
        .route("/v1/notify", post(notify))
        .route("/v1/term/session", post(create_term_session))
        .route("/v1/term/session/{id}/ws", get(term_ws))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let app = Router::new()
        .merge(api)
        .fallback_service(static_files)
        .with_state(AppState {
            token,
            term_sessions: Arc::new(Mutex::new(HashMap::new())),
        });

    let bind = std::env::var("SOLD_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind sold to {bind}"))?;

    info!("sold listening on {}", bind);
    axum::serve(listener, app).await.context("serve failed")
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

fn check_auth(headers: &HeaderMap, state: &AppState) -> Result<(), Response> {
    let got = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    let want = format!("Bearer {}", state.token);
    if got == want {
        return Ok(());
    }
    Err((
        StatusCode::UNAUTHORIZED,
        Json(ApiError {
            error: "unauthorized",
        }),
    )
        .into_response())
}

async fn get_network_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<NetworkStatus>, Response> {
    check_auth(&headers, &state)?;

    #[cfg(target_os = "linux")]
    let interfaces = match tokio::fs::read_dir("/sys/class/net").await {
        Ok(mut dir) => {
            let mut ifaces = Vec::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                let name = entry.file_name();
                if let Some(s) = name.to_str() {
                    if s != "lo" {
                        ifaces.push(s.to_string());
                    }
                }
            }
            ifaces
        }
        Err(_) => Vec::new(),
    };

    #[cfg(not(target_os = "linux"))]
    let interfaces = Vec::new();

    Ok(Json(NetworkStatus {
        connected: !interfaces.is_empty(),
        interfaces,
    }))
}

async fn get_battery_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BatteryStatus>, Response> {
    check_auth(&headers, &state)?;

    #[cfg(target_os = "linux")]
    {
        let mut out = BatteryStatus {
            present: false,
            percent: None,
            charging: None,
        };
        if let Ok(mut dir) = tokio::fs::read_dir("/sys/class/power_supply").await {
            while let Ok(Some(entry)) = dir.next_entry().await {
                let p = entry.path();
                let kind = tokio::fs::read_to_string(p.join("type")).await.ok();
                if !matches!(kind.as_deref(), Some("Battery\n") | Some("Battery")) {
                    continue;
                }
                out.present = true;
                if let Ok(cap) = tokio::fs::read_to_string(p.join("capacity")).await {
                    out.percent = cap.trim().parse::<u8>().ok();
                }
                if let Ok(status) = tokio::fs::read_to_string(p.join("status")).await {
                    out.charging = Some(status.trim().eq_ignore_ascii_case("charging"));
                }
                break;
            }
        }
        return Ok(Json(out));
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(Json(BatteryStatus {
            present: false,
            percent: None,
            charging: None,
        }))
    }
}

async fn power_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(action): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    check_auth(&headers, &state)?;

    let _action_cmd = match action.as_str() {
        "shutdown" => vec!["poweroff"],
        "reboot" => vec!["reboot"],
        "suspend" => vec!["suspend"],
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: "invalid action",
                }),
            )
                .into_response())
        }
    };

    #[cfg(target_os = "linux")]
    {
        use std::process::Stdio;
        use tokio::process::Command;
        let status = Command::new("loginctl")
            .args(&_action_cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        if let Err(e) = status {
            warn!("power action failed: {}", e);
        }
    }

    Ok(Json(serde_json::json!({ "ok": true, "action": action })))
}

async fn notify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<NotifyRequest>,
) -> Result<Json<serde_json::Value>, Response> {
    check_auth(&headers, &state)?;
    info!("ui notification: {} — {}", req.title, req.body);
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn create_term_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TermSessionResponse>, Response> {
    check_auth(&headers, &state)?;
    let id = Uuid::new_v4().to_string();
    info!("creating terminal session {}", id);
    let mut sessions = state.term_sessions.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: "session lock failed",
            }),
        )
            .into_response()
    })?;
    sessions.insert(
        id.clone(),
        TermSession {
            command: vec![
                "sh".to_string(),
                "-lc".to_string(),
                "zellij attach -c main || zellij || /bin/ash -l".to_string(),
            ],
        },
    );
    Ok(Json(TermSessionResponse { session_id: id }))
}

async fn term_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
    Path(id): Path<String>,
) -> Response {
    if let Err(resp) = check_auth_with_token(&headers, query.token.as_deref(), &state) {
        return resp;
    }
    info!("terminal websocket requested for session {}", id);
    ws.on_upgrade(move |socket| handle_ws_term(socket, state, id))
}

fn check_auth_with_token(headers: &HeaderMap, query_token: Option<&str>, state: &AppState) -> Result<(), Response> {
    if let Some(token) = query_token {
        if token == state.token {
            return Ok(());
        }
    }
    check_auth(headers, state)
}

async fn handle_ws_term(mut socket: WebSocket, state: AppState, id: String) {
    info!("attaching terminal session {}", id);
    let session = {
        match state.term_sessions.lock() {
            Ok(mut sessions) => sessions.remove(&id),
            Err(_) => None,
        }
    };

    let Some(session) = session else {
        let _ = socket
            .send(Message::Text(
                "unknown or already used session".to_string().into(),
            ))
            .await;
        return;
    };

    info!("starting terminal command: {}", session.command.join(" "));
    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 30,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(e) => {
            error!("pty open failed: {}", e);
            let _ = socket
                .send(Message::Text("failed to open pty".to_string().into()))
                .await;
            return;
        }
    };

    let mut cmd = CommandBuilder::new(&session.command[0]);
    for arg in &session.command[1..] {
        cmd.arg(arg);
    }
    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(e) => {
            error!("spawn failed: {}", e);
            let _ = socket
                .send(Message::Text("failed to start terminal".to_string().into()))
                .await;
            return;
        }
    };

    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            error!("clone reader failed: {}", e);
            return;
        }
    };
    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            error!("take writer failed: {}", e);
            return;
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    if tx.send(s).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        tokio::select! {
            ws_msg = socket.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        let _ = writer.write_all(text.as_bytes());
                    }
                    Some(Ok(Message::Binary(data))) => {
                        let _ = writer.write_all(&data);
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("ws receive error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            pty_msg = rx.recv() => {
                match pty_msg {
                    Some(chunk) => {
                        if socket.send(Message::Text(chunk.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    info!("terminal session {} closed", id);
}
