use axum::{
    extract::{Path, Query},
    routing::get,
    Router,
    response::{IntoResponse, Json},
    http::StatusCode,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;
use std::fs;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared::health::{HealthMonitor, ResourceStatus};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let health_monitor = Arc::new(Mutex::new(HealthMonitor::new()));

    // Create the router
    let app = Router::new()
        // API routes
        .route("/api/wiki", get(list_wiki_files))
        .route("/api/wiki/:filename", get(get_wiki_content))
        .route("/api/codewiki/page", get(get_mock_codewiki_page))
        .route("/api/health", get(get_health_status))
        .with_state(health_monitor)
        // Static files
        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3015));
    tracing::info!("🌌 Antigravity Management Console listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct WikiQuery {
    #[allow(dead_code)]
    slug: String,
}

/// Simulated CodeWiki SDK Logic
/// In a real scenario, this would call the Google CodeWiki API
async fn get_mock_codewiki_page(
    _state: axum::extract::State<Arc<Mutex<HealthMonitor>>>,
    Query(params): Query<WikiQuery>
) -> impl IntoResponse {
    let content = match params.slug.as_str() {
        "api-usage" => "# 🚀 API Usage Guide\n\nThis documentation is pulled directly from **CodeWiki**.\n\n## Authentication\nUse the `Bearer` token in the header...\n\n```bash\ncurl -H \"Authorization: Bearer $TOKEN\" http://localhost:3015/api/wiki\n```",
        "philosophy" => "# 🧠 Antigravity Philosophy\n\n## 1. 「魔法」の可視化\nブラックボックス化を阻止し、構造を一発で図解します。\n\n## 2. コンテキストスイッチの削減\nエディタを離れずに仕様を確認。\n\n## 3. 嘘つきドキュメントの撲滅\nCIでの自動更新により、常に最新の状態を維持。\n\n## 4. オンボーディングコスト削減\n「3ヶ月前の自分は他人」という前提でドキュメントを整備します。",
        _ => "# Not Found\nThe requested CodeWiki page could not be simulated.",
    };
    content.into_response()
}

async fn list_wiki_files(_state: axum::extract::State<Arc<Mutex<HealthMonitor>>>) -> Json<Vec<String>> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir("../../docs") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    files.push(name.to_string());
                }
            }
        }
    }
    // Sort to keep CODE_WIKI at top
    files.sort_by(|a, b| {
        if a == "CODE_WIKI.md" { std::cmp::Ordering::Less }
        else if b == "CODE_WIKI.md" { std::cmp::Ordering::Greater }
        else { a.cmp(b) }
    });
    Json(files)
}

async fn get_wiki_content(
    _state: axum::extract::State<Arc<Mutex<HealthMonitor>>>,
    Path(filename): Path<String>
) -> impl IntoResponse {
    let path = format!("../../docs/{}", filename);
    match fs::read_to_string(path) {
        Ok(content) => content.into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Wiki not found").into_response(),
    }
}

async fn get_health_status(
    axum::extract::State(monitor): axum::extract::State<Arc<Mutex<HealthMonitor>>>,
) -> Json<ResourceStatus> {
    let mut monitor = monitor.lock().await;
    Json(monitor.check())
}
