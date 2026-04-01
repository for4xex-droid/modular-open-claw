/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::a2a::agent_card::{
    AgentCard, Endpoints, PricingConfig, SecurityProfile, SlaConfig, ZtasProfile,
};
use axum::{http::StatusCode, Json};
use infrastructure::auto_profile::AutoProfileEngine;
use std::path::Path;

pub async fn get_agent_card() -> (StatusCode, Json<AgentCard>) {
    let workspace = std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| ".".to_string());
    let detected = AutoProfileEngine::scan_workspace(Path::new(&workspace));

    let mut skills: Vec<String> = detected
        .into_iter()
        .map(|s| format!("{}:{}", s.domain, s.skill))
        .collect();

    // Default fallback skill
    if skills.is_empty() {
        skills.push("autonomous_worker".to_string());
    }

    let card = AgentCard {
        name: "Aiome Node".to_string(),
        version: "1.0".to_string(),
        skills,
        capabilities: vec![
            "gig/publish".to_string(),
            "gig/status".to_string(),
            "gig/capabilities".to_string(),
            "profile/info".to_string(),
        ],
        endpoints: Endpoints {
            grpc: Some("grpc://localhost:50051".to_string()),
            rest: Some("http://localhost:8080".to_string()),
        },
        security: SecurityProfile {
            auth: vec!["mcp".to_string(), "oauth2".to_string()],
            ztas: ZtasProfile {
                did: "did:key:placeholder".to_string(),
            },
        },
        sla: SlaConfig {
            max_latency_ms: 200,
            availability: 0.999,
        },
        pricing: PricingConfig {
            protocol: "x402".to_string(),
            base_rate: "0.001 USDC".to_string(),
        },
    };

    (StatusCode::OK, Json(card))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup_router;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for axum 0.7 body collection

    #[tokio::test]
    async fn test_well_known_agent_card() {
        let app = setup_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/agent.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let card: AgentCard = serde_json::from_slice(&body).expect("Failed to parse AgentCard");

        assert_eq!(card.name, "Aiome Node");
        assert_eq!(card.version, "1.0");
        assert_eq!(card.pricing.protocol, "x402");
    }
}
