use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use crate::app_state::AppState;
use crate::auth::Authenticated;
use tracing::info;

/// [POST] /api/v1/voice/upload
/// Phase 9: Voice asset upload stub (G-20 / G-21 preview)
pub async fn upload_voice_handler(
    _auth: Authenticated,
    State(_state): State<AppState>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let size = body.len();
    info!("🎤 [Voice] Received voice asset upload: {} bytes", size);
    
    // Phase 10: Here we will decrypt via VoiceCoreDrm and store in Abyss Vault
    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "received_bytes": size,
            "message": "Voice asset received and queued for DRM processing"
        })),
    )
}
