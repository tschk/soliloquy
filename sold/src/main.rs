use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::{delete, get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tokio::fs;

const FILES_DIR: &str = "/var/lib/soliloquy/files";

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

#[tokio::main]
async fn main() {
    // Ensure files directory exists
    fs::create_dir_all(FILES_DIR).await.unwrap();

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/files", get(list_files))
        .route("/api/files/:name", get(read_file))
        .route("/api/files/:name", put(write_file))
        .route("/api/files/:name", delete(delete_file))
        .route("/api/settings", get(get_settings))
        .route("/api/settings", put(put_settings))
        .nest_service("/", ServeDir::new("bundle"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("sold listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn list_files() -> Result<Json<Vec<FileInfo>>, StatusCode> {
    let mut entries = fs::read_dir(FILES_DIR).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut files = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        let metadata = entry.metadata().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        files.push(FileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
        });
    }

    Ok(Json(files))
}

async fn read_file(Path(name): Path<String>) -> Result<String, StatusCode> {
    let path = format!("{}/{}", FILES_DIR, name);
    fs::read_to_string(&path).await.map_err(|_| StatusCode::NOT_FOUND)
}

async fn write_file(
    Path(name): Path<String>,
    Json(payload): Json<FileContent>,
) -> Result<(), StatusCode> {
    let path = format!("{}/{}", FILES_DIR, name);
    fs::write(&path, payload.content).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_file(Path(name): Path<String>) -> Result<(), StatusCode> {
    let path = format!("{}/{}", FILES_DIR, name);
    fs::remove_file(&path).await.map_err(|_| StatusCode::NOT_FOUND)
}

async fn get_settings() -> Result<Json<Settings>, StatusCode> {
    let path = format!("{}/settings.json", FILES_DIR);
    match fs::read_to_string(&path).await {
        Ok(content) => {
            let settings: Settings = serde_json::from_str(&content).unwrap_or_default();
            Ok(Json(settings))
        }
        Err(_) => Ok(Json(Settings::default())),
    }
}

async fn put_settings(Json(settings): Json<Settings>) -> Result<(), StatusCode> {
    let path = format!("{}/settings.json", FILES_DIR);
    let content = serde_json::to_string(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    fs::write(&path, content).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
