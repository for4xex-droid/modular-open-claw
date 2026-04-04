/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
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
    _auth: crate::auth::Authenticated,
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
    _auth: crate::auth::Authenticated,
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
    _auth: crate::auth::Authenticated,
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
    _auth: crate::auth::Authenticated,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let ingester = state.cortex_ingester.get_inner();
    ingester.delete_document(&id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WikiArticleSummary {
    pub id: String,
    pub title: String,
    pub concepts: Vec<String>,
    pub version: i64,
    pub updated_at: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/cortex/wiki",
    responses(
        (status = 200, description = "List of wiki articles", body = Vec<WikiArticleSummary>)
    ),
    security(("api_key" = []))
)]
pub async fn list_wiki_articles_handler(
    State(state): State<crate::AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?;

    let rows = sqlx::query(
        r#"
        SELECT id, title, concepts, version, updated_at
        FROM cortex_wiki_articles
        ORDER BY updated_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: e.to_string(),
    })?;

    let mut summaries = Vec::new();
    for row in rows {
        use sqlx::Row;
        let concepts_json: String = row.try_get("concepts").unwrap_or_else(|_| "[]".to_string());

        // Ensure id, version and updated_at have fallbacks since they could be null during DB operations, though defined as not null
        let id: String = row.try_get("id").unwrap_or_default();
        let version: i64 = row.try_get("version").unwrap_or(1);

        let updated_at: String = row.try_get("updated_at").unwrap_or_default();

        summaries.push(WikiArticleSummary {
            id,
            title: row.try_get("title").unwrap_or_default(),
            concepts: serde_json::from_str(&concepts_json).unwrap_or_default(),
            version,
            updated_at,
        });
    }

    Ok((StatusCode::OK, Json(summaries)))
}

#[utoipa::path(
    get,
    path = "/api/v1/cortex/wiki/{id}",
    params(
        ("id" = String, Path, description = "Wiki Article ID")
    ),
    responses(
        (status = 200, description = "Wiki article detail", body = infrastructure::cortex_compiler::WikiArticle),
        (status = 404, description = "Wiki article not found")
    ),
    security(("api_key" = []))
)]
pub async fn get_wiki_article_handler(
    State(state): State<crate::AppState>,
    _auth: crate::auth::Authenticated,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?;

    let row = sqlx::query(
        r#"
        SELECT id, title, content_md, concepts, backlinks, source_refs, content_hash, version
        FROM cortex_wiki_articles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: e.to_string(),
    })?;

    if let Some(row) = row {
        use sqlx::Row;
        let concepts_json: String = row.try_get("concepts").unwrap_or_else(|_| "[]".to_string());
        let backlinks_json: String = row
            .try_get("backlinks")
            .unwrap_or_else(|_| "[]".to_string());
        let source_refs_json: String = row
            .try_get("source_refs")
            .unwrap_or_else(|_| "[]".to_string());

        let article = infrastructure::cortex_compiler::WikiArticle {
            id: row.try_get("id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            content_md: row.try_get("content_md").unwrap_or_default(),
            concepts: serde_json::from_str(&concepts_json).unwrap_or_default(),
            backlinks: serde_json::from_str(&backlinks_json).unwrap_or_default(),
            source_refs: serde_json::from_str(&source_refs_json).unwrap_or_default(),
            content_hash: row.try_get("content_hash").unwrap_or_default(),
            version: row.try_get("version").unwrap_or(1),
        };
        Ok(Json(article).into_response())
    } else {
        Ok((StatusCode::NOT_FOUND, "Article not found").into_response())
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct QueryReq {
    pub question: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/cortex/query",
    request_body = QueryReq,
    responses(
        (status = 200, description = "Query answered", body = infrastructure::cortex_query::CortexAnswer)
    ),
    security(("api_key" = []))
)]
pub async fn query_handler(
    State(state): State<crate::AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<QueryReq>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let engine = state.cortex_query.get_inner();
    let ans = engine.query(&req.question).await?;
    Ok((StatusCode::OK, Json(ans)))
}

#[utoipa::path(
    get,
    path = "/api/v1/cortex/suggestions",
    responses(
        (status = 200, description = "Dynamically generated question suggestions based on Cortex concept index. Falls back to a default suggestion when the knowledge base is empty.", body = [String])
    ),
    security(("api_key" = []))
)]
pub async fn suggest_questions_handler(
    State(state): State<crate::AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    let engine = state.cortex_query.get_inner();
    let suggestions = engine.suggest_questions().await?;
    Ok((StatusCode::OK, Json(suggestions)))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SynthReq {
    pub base_model: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/cortex/synth",
    request_body = SynthReq,
    responses(
        (status = 202, description = "Synthetic dataset generation started and LoRA training enqueued")
    ),
    security(("api_key" = []))
)]
pub async fn synth_dataset_handler(
    State(state): State<crate::AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<SynthReq>,
) -> Result<impl IntoResponse, aiome_core::error::AiomeError> {
    // Phase D: Synth -> LoRA integration
    let provider = state.provider.get_inner().clone();
    let pool = state.job_queue.get_pool().clone();

    let synth = infrastructure::cortex_synth::CortexSynthesizer::new(provider, pool, None);

    let jq = state.job_queue.get_inner().clone();
    let base_model = req.base_model;

    tokio::spawn(async move {
        match synth.generate_dataset().await {
            Ok(dataset) => {
                let temp_id = uuid::Uuid::new_v4().to_string();
                let dataset_id = format!("synth_{}.jsonl", temp_id);
                let datasets_dir = shared::app_data::AppDataResolver::new().resolve("datasets");
                if let Err(e) = tokio::fs::create_dir_all(&datasets_dir).await {
                    tracing::error!("Failed to create datasets directory: {}", e);
                    return;
                }

                let out_path = datasets_dir.join(&dataset_id);
                if let Err(e) = synth.export_to_jsonl(&dataset, &out_path) {
                    tracing::error!("Failed to export dataset: {}", e);
                    return;
                }

                use infrastructure::job_queue::CoreOps;
                if let Err(e) = jq
                    .do_enqueue(
                        "LORA_TRAINING",
                        &base_model,
                        &dataset_id,
                        None,
                        None,
                        None,
                        0,
                    )
                    .await
                {
                    tracing::error!("Failed to enqueue LoRA training: {}", e);
                }

                tracing::info!(
                    "Synthetic dataset generated: {} pairs. Enqueued LoRA.",
                    dataset.pairs.len()
                );
            }
            Err(e) => {
                tracing::error!("Synthetic dataset generation failed: {}", e);
            }
        }
    });

    Ok((StatusCode::ACCEPTED, "Generation started"))
}
