/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;
use tracing::warn;

pub struct Authenticated;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();

        let query_token =
            axum::extract::Query::<std::collections::HashMap<String, String>>::try_from_uri(
                &parts.uri,
            )
            .ok()
            .and_then(|q| q.get("token").cloned());

        let expected_secret = std::env::var("API_SERVER_SECRET").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "dev_secret".to_string()
            } else {
                panic!("🚨 [Auth] FATAL: API_SERVER_SECRET must be set in release builds!");
            }
        });
        let expected_bearer = format!("Bearer {}", expected_secret);

        // SEC: Always perform constant-time comparison regardless of length to prevent timing leaks
        // Sub-clause: We check lengths first but only inside a combined constant-time check logic
        let bearer_match = {
            let a = auth_header.as_bytes();
            let b = expected_bearer.as_bytes();
            let max_len = std::cmp::max(a.len(), b.len());
            let mut a_padded = vec![0u8; max_len];
            let mut b_padded = vec![0u8; max_len];
            a_padded[..a.len()].copy_from_slice(a);
            b_padded[..b.len()].copy_from_slice(b);
            // Length check is combined with ct_eq results
            a.len() == b.len() && bool::from(a_padded.ct_eq(&b_padded))
        };

        let query_match = query_token
        .as_ref()
        .map(|t| {
            let max_len = std::cmp::max(t.len(), expected_secret.len());
            let mut a = vec![0u8; max_len];
            let mut b = vec![0u8; max_len];
            a[..t.len()].copy_from_slice(t.as_bytes());
            b[..expected_secret.len()].copy_from_slice(expected_secret.as_bytes());
            t.len() == expected_secret.len() && bool::from(a.ct_eq(&b))
        })
        .unwrap_or(false);

        let is_valid = bearer_match || query_match;

        if is_valid {
            Ok(Authenticated)
        } else {
            let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            if !auth_header.is_empty() && auth_header.starts_with("Bearer ") {
                use axum::http::HeaderValue;
                resp.headers_mut()
                    .insert("X-Token-Expired", HeaderValue::from_static("true"));
            }
            Err(resp)
        }
    }
}
