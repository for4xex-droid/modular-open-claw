use axum::{
    extract::Path, http::StatusCode, response::IntoResponse, routing::get, Extension, Router,
};
use commerce_protocol::identity::ActorId;
use tracing::{error, info};
use uuid::Uuid;

use crate::state::SharedState;

pub fn asset_routes() -> Router {
    Router::new().route("/:id/download/:buyer_id", get(download_asset))
}

async fn download_asset(
    Path((asset_id, buyer_id)): Path<(Uuid, Uuid)>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    let buyer = ActorId(buyer_id);
    let asset_id_str = asset_id.to_string();

    if !asset_id_str
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-')
    {
        return (StatusCode::BAD_REQUEST, "Invalid asset_id format").into_response();
    }

    // 1. Get the item to find the creator_id
    let item = match state.marketplace.get_item(&asset_id).await {
        Ok(item) => item,
        Err(e) => {
            error!("❌ [Asset/Download] Item not found {}: {}", asset_id, e);
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    // 2. Check License
    let license = match state.license_store.get_license(&buyer, &asset_id).await {
        Ok(Some(l)) => {
            info!(
                "✅ [Asset/Download] Valid license found for buyer {} and asset {}",
                buyer_id, asset_id
            );
            l
        }
        Ok(None) => {
            error!(
                "❌ [Asset/Download] No valid license for buyer {} on asset {}",
                buyer_id, asset_id
            );
            return StatusCode::FORBIDDEN.into_response();
        }
        Err(e) => {
            error!("❌ [Asset/Download] License verification failed: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // 3. Fetch from AssetStorage
    match state
        .asset_storage
        .get_asset(&item.creator_id, &asset_id)
        .await
    {
        Ok(data) => {
            // Note: In a real environment, we'd stream this. For now, returning bytes.
            info!(
                "✅ [Asset/Download] Asset {} successfully downloaded",
                asset_id
            );
            let mut res = data.into_response();
            res.headers_mut().insert(
                "x-nurture-drm-key",
                axum::http::HeaderValue::from_str(&license.decryption_key)
                    .unwrap_or(axum::http::HeaderValue::from_static("error")),
            );
            res
        }
        Err(e) => {
            error!("❌ [Asset/Download] Failed to fetch asset data: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
