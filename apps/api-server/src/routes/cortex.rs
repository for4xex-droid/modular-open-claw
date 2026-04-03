use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct IngestUrlReq {
    pub url: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct IngestResp {
    pub id: String,
    pub title: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/cortex/ingest",
    request_body = IngestUrlReq,
    responses(
        (status = 201, description = "URL ingrained into Cortex successfully", body = IngestResp)
    ),
    security(("api_key" = []))
)]
pub async fn ingest_url_handler(
    State(state): State<crate::AppState>,
    Json(req): Json<IngestUrlReq>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    let doc = ingester.ingest_url(&req.url).await?;

    Ok((
        StatusCode::CREATED,
        Json(IngestResp {
            id: doc.id,
            title: doc.title,
        }),
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct IngestTextReq {
    pub title: String,
    pub content: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/cortex/ingest/text",
    request_body = IngestTextReq,
    responses(
        (status = 201, description = "Text ingrained into Cortex successfully", body = IngestResp)
    ),
    security(("api_key" = []))
)]
pub async fn ingest_text_handler(
    State(state): State<crate::AppState>,
    Json(req): Json<IngestTextReq>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    let doc = ingester.ingest_text(&req.title, &req.content).await?;

    Ok((
        StatusCode::CREATED,
        Json(IngestResp {
            id: doc.id,
            title: doc.title,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/cortex/documents",
    responses(
        (status = 200, description = "List all documents inside Cortex", body = [infrastructure::cortex_ingester::CortexDocument])
    ),
    security(("api_key" = []))
)]
pub async fn list_documents_handler(
    State(state): State<crate::AppState>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    let docs = ingester.list_documents(100).await?;

    Ok((StatusCode::OK, Json(docs)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/cortex/documents/{id}",
    params(
        ("id" = String, Path, description = "The ID of the document to delete")
    ),
    responses(
        (status = 204, description = "Document deleted successfully")
    ),
    security(("api_key" = []))
)]
pub async fn delete_document_handler(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    ingester.delete_document(&id).await?;

    Ok(StatusCode::NO_CONTENT)
}
