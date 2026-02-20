use axum::{
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::IntoResponse,
    routing::{get, post},
    Router, Json,
    http::StatusCode,
};
use std::sync::{Arc, Mutex};
use crate::server::telemetry::TelemetryHub;
use crate::orchestrator::ProductionOrchestrator;
use factory_core::contracts::WorkflowRequest;
use factory_core::traits::AgentAct; // Trait import needed for .execute()
use tuning::StyleManager;
use bastion::fs_guard::Jail;
use tower_http::services::ServeDir;
use uuid::Uuid;
use crate::asset_manager::AssetManager;

pub struct AppState {
    pub telemetry: Arc<TelemetryHub>,
    pub orchestrator: Arc<ProductionOrchestrator>,
    pub style_manager: Arc<StyleManager>,
    pub jail: Arc<Jail>,
    pub is_busy: Arc<Mutex<bool>>, // Resource Locking
    pub asset_manager: Arc<AssetManager>,
    pub current_job: Arc<tokio::sync::Mutex<Option<String>>>,
}


use tower_http::cors::CorsLayer;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/remix", post(remix_handler))
        .route("/api/styles", get(styles_handler))
        .route("/api/projects", get(projects_handler))
        .nest_service("/assets", ServeDir::new("workspace")) // Serve static assets
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// --- WebSocket Handler ---

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx_hb = state.telemetry.subscribe_heartbeat();
    let mut rx_log = state.telemetry.subscribe_log();

    loop {
        tokio::select! {
            Ok(hb) = rx_hb.recv() => {
                // Determine active actor based on busy state
                let mut hb_with_state = hb.clone();
                if let Ok(busy) = state.is_busy.lock() {
                     if *busy {
                         hb_with_state.active_actor = Some("ORCHESTRATOR".to_string());
                     }
                }

                if let Ok(msg) = serde_json::to_string(&hb_with_state) {
                    if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
            Ok(log) = rx_log.recv() => {
                if let Ok(msg) = serde_json::to_string(&log) {
                    if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

// --- REST API Handlers ---

async fn remix_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WorkflowRequest>,
) -> impl IntoResponse {
    // 1. Resource Locking (Overzealous Clicker Guard)
    {
        let mut busy = state.is_busy.lock().unwrap();
        if *busy {
             state.telemetry.broadcast_log("WARN", "Rejecting concurrent remix request.");
             return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({
                 "error": "System is busy. Please wait for the current task to finish."
             }))).into_response();
        }
        *busy = true; // Acquire lock
    }

    let job_id = Uuid::new_v4().to_string();
    state.telemetry.broadcast_log("INFO", &format!("Job Accepted: {} (Remix)", job_id));
    
    let orchestrator = state.orchestrator.clone();
    let jail = state.jail.clone();
    let busy_lock = state.is_busy.clone();
    let telemetry = state.telemetry.clone();
    let job_id_clone = job_id.clone();
    
    // 2. Asynchronous Job Creation
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Set current job info
        {
            let mut job_info = state_clone.current_job.lock().await;
            *job_info = Some(format!("Remix: {}", job_id_clone));
        }

        // Execute the heavy task
        match orchestrator.execute(payload.clone(), &jail).await {
            Ok(res) => {
                let msg = format!("Job Completed: {} -> {}", job_id_clone, res.final_video_path);
                println!("{}", msg);
                telemetry.broadcast_log("INFO", &msg);
            }
            Err(e) => {
                let msg = format!("Job Failed: {} -> {}", job_id_clone, e);
                eprintln!("{}", msg);
                telemetry.broadcast_log("ERROR", &msg);
            }
        }

        // Release Lock & Clear job info
        {
            let mut job_info = state_clone.current_job.lock().await;
            *job_info = None;
        }

        if let Ok(mut busy) = busy_lock.lock() {
            *busy = false;
            telemetry.broadcast_log("INFO", "System Ready");
        }
    });

    // 3. Immediate Response (202 Accepted)
    (StatusCode::ACCEPTED, Json(serde_json::json!({ 
        "status": "accepted", 
        "job_id": job_id,
        "job_type": "remix" 
    }))).into_response()
}

async fn styles_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let styles = state.style_manager.list_available_styles();
    Json(styles)
}

async fn projects_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // AssetManager is inside Orchestrator, but Orchestrator fields are private?
    // Wait, ProductionOrchestrator has asset_manager field but is it public? 
    // Let's check orchestrator.rs. Use a getter or access it if public.
    // If not public, I might need to add a getter to Orchestrator or put AssetManager in AppState directly.
    // AppState doesn't have asset_manager.
    // Let's assume for now I need to add it to AppState or make it accessible.
    // Checking main.rs, I put style_manager in AppState. asset_manager is created in main.rs.
    // I should add asset_manager to AppState in main.rs and router.rs.
    
    // For now, I will write the handler assuming state.asset_manager exists.
    let projects = state.asset_manager.list_projects();
    Json(projects)
}
