/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::auth::AuthenticatedUser;
use crate::error::AppError;
use crate::AppState;
use aiome_contracts::traits::{AgentEvolver, JobQueue};
use aiome_contracts::treasure::{TreasureFeedback, TreasureItem};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use uuid::Uuid;

/// [GET] /api/v1/treasure
#[utoipa::path(
    get,
    path = "/api/v1/treasure",
    responses(
        (status = 200, description = "List of current recommendations", body = Vec<TreasureItem>),
        (status = 401, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn get_treasure(
    State(state): State<AppState>,
    Extension(AuthenticatedUser(claims)): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Generate Intent (AS-1.1 - 1.2)
    let intent_gen = state.intent_generator.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Intent Generator not enabled".into(),
        }
    })?;

    // In a real scenario, we'd fetch actual context.
    // Here we use the agent's ID to fetch its "Recent desires"
    let intent = intent_gen.generate_for_agent(claims.agent_id).await?;

    // 2. Fetch recommendations via AffiliateAdapter (AS-1.3)
    let adapter = state.affiliate_adapter.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Affiliate Adapter not enabled".into(),
        }
    })?;

    let bids: Vec<aiome_contracts::gig::GigBid> = adapter.fetch_bids_for_intent(&intent).await?;

    // 3. Map to TreasureItems
    let items: Vec<TreasureItem> = bids
        .into_iter()
        .map(|bid| {
            let (title, category) = match intent.description.as_str() {
                d if d.contains("inner peace") => {
                    ("Serenity & Mindfulness Guide".into(), "Healing")
                }
                d if d.contains("self-reliance") => ("Advanced Automation Toolkit".into(), "Tools"),
                d if d.contains("creative growth") => {
                    ("Artistic Expansion Summit".into(), "Learning")
                }
                _ => (format!("Sense Upgrade: {}", intent.description), "Other"),
            };

            TreasureItem {
                id: bid.id,
                title,
                description: format!("Based on your current sense: {}", intent.description),
                url: "https://example.com/item".into(),
                price_coins: Some(bid.price_coins),
                category: category.into(),
                score: 0.85,
                disclosure_label: "AI推薦 / 広告".into(),
            }
        })
        .collect();

    Ok(Json(items))
}

/// [POST] /api/v1/treasure
#[utoipa::path(
    post,
    path = "/api/v1/treasure",
    request_body = TreasureFeedback,
    responses(
        (status = 200, description = "Feedback recorded and reward calculated"),
        (status = 401, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn record_feedback(
    State(state): State<AppState>,
    Extension(AuthenticatedUser(claims)): Extension<AuthenticatedUser>,
    Json(feedback): Json<TreasureFeedback>,
) -> Result<impl IntoResponse, AppError> {
    // AS-1.7: AgentSense Feedback & Karma Reward
    tracing::info!(
        "💰 [Treasure] Feedback from {}: {} on {}",
        claims.agent_id,
        feedback.action,
        feedback.item_id
    );

    // Reward Karma if action is "buy" or "click"
    if feedback.action == "click" || feedback.action == "buy" {
        let job_queue = state.job_queue.as_opt().ok_or_else(|| {
            aiome_core::error::AiomeError::Infrastructure {
                reason: "Job Queue not enabled".into(),
            }
        })?;

        let weight = if feedback.action == "buy" { 10 } else { 1 };
        job_queue.add_resonance(weight).await?;

        tracing::info!("⭐ [Treasure] Rewarded {} Resonance to agent", weight);
    }

    Ok(StatusCode::OK)
}
