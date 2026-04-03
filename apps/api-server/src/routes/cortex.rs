use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct IngestUrlReq {
    pub url: String,
}

#[derive(Serialize)]
pub struct IngestResp {
    pub id: String,
    pub title: String,
}

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

#[derive(Deserialize)]
pub struct IngestTextReq {
    pub title: String,
    pub content: String,
}

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

pub async fn list_documents_handler(
    State(state): State<crate::AppState>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    let docs = ingester.list_documents(100).await?;

    Ok((StatusCode::OK, Json(docs)))
}

pub async fn delete_document_handler(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    ingester.delete_document(&id).await?;

    Ok(StatusCode::NO_CONTENT)
}
