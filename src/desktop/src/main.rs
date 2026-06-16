//! Soliloquy Desktop Environment daemon entry point.
//!
//! Runs as a background service (dinit on Alpenglow), consuming alpenglow
//! OS daemon state and exposing an HTTP API for the Svelte UI.
//!
//! Usage:
//!   soliloquy-daemon                 # default: listen on 127.0.0.1:9842
//!   soliloquy-daemon --port 9843
//!   soliloquy-daemon --once          # run once, report status, exit

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::State, http::Method, routing::get, Router};
use log::info;
use soliloquy_daemon::{status::AlpenglowPaths, AppRegistry, DesktopStatus, SessionManager};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct DaemonState {
    alpenglow_paths: AlpenglowPaths,
    session: Arc<SessionManager>,
    apps: Arc<std::sync::Mutex<AppRegistry>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("info".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    let port = if let Ok(v) = std::env::var("SOLILOQUY_DAEMON_PORT") {
        v.parse::<u16>().unwrap_or(9842)
    } else if let Ok(v) = std::env::var("SOLD_BIND") {
        // Backwards compat: sold OpenRC scripts use SOLD_BIND=127.0.0.1:8080
        v.split(':')
            .nth(1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9842)
    } else {
        9842
    };

    let port_arg = std::env::args()
        .position(|a| a == "--port")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|v| v.parse::<u16>().ok());
    let port = port_arg.unwrap_or(port);
    let once = std::env::args().any(|a| a == "--once");

    let state = DaemonState {
        alpenglow_paths: AlpenglowPaths::default(),
        session: Arc::new(SessionManager::new(
            whoami::username(),
            std::env::var("WAYLAND_DISPLAY")
                .or_else(|_| std::env::var("DISPLAY"))
                .unwrap_or_else(|_| "tty1".into()),
        )),
        apps: Arc::new(std::sync::Mutex::new(AppRegistry::new())),
    };

    // Seed default apps
    {
        let mut reg = state.apps.lock().unwrap();
        reg.register(soliloquy_daemon::AppEntry {
            id: "terminal".into(),
            name: "Terminal".into(),
            description: "System terminal".into(),
            command: "foot".into(),
            icon: "terminal".into(),
            categories: vec!["System".into()],
        });
        reg.register(soliloquy_daemon::AppEntry {
            id: "browser".into(),
            name: "Browser".into(),
            description: "Soliloquy web browser".into(),
            command: "soliloquy-shell".into(),
            icon: "browser".into(),
            categories: vec!["Network".into()],
        });
        reg.register(soliloquy_daemon::AppEntry {
            id: "settings".into(),
            name: "Settings".into(),
            description: "Desktop settings".into(),
            command: "soliloquy-settings".into(),
            icon: "settings".into(),
            categories: vec!["System".into()],
        });
    }

    if once {
        let status = DesktopStatus::collect(&state.alpenglow_paths);
        println!("{}", serde_json::to_string_pretty(&status).unwrap());
        return;
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/status", get(handle_status))
        .route("/apps", get(handle_apps))
        .route("/session", get(handle_session))
        .route("/health", get(|| async { "ok" }))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Soliloquy DE daemon listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_status(State(state): State<DaemonState>) -> String {
    let mut status = DesktopStatus::collect(&state.alpenglow_paths);
    status.session = soliloquy_daemon::SessionInfo {
        user: state.session.user.clone(),
        display: state.session.display.clone(),
        uptime_secs: state.session.uptime_secs(),
    };
    status.apps = state
        .apps
        .lock()
        .unwrap()
        .instances()
        .into_iter()
        .map(|inst| soliloquy_daemon::AppInfo {
            id: inst.entry.id,
            name: inst.entry.name,
            pid: inst.pid,
            state: format!("{:?}", inst.state),
        })
        .collect();
    serde_json::to_string_pretty(&status).unwrap()
}

async fn handle_apps(State(state): State<DaemonState>) -> String {
    let apps = state.apps.lock().unwrap();
    let entries = apps.entries();
    serde_json::to_string_pretty(&entries).unwrap()
}

async fn handle_session(State(state): State<DaemonState>) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "user": state.session.user,
        "display": state.session.display,
        "uptime_secs": state.session.uptime_secs(),
        "started_at": state.session.started_at,
    }))
    .unwrap()
}
