/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::routes::commerce_webhook::relay::attempt_coin_charge_once;
use crate::AppState;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{error, info, warn};

const EVENT_TYPE: &str = "coin_charge_failed";
const EVENT_TYPE_POISON: &str = "coin_charge_failed_poison";
const BATCH_LIMIT: i64 = 10;

/// 1 バッチ分の DLQ 行を処理する。成功した行数を返す（テスト用に pub(crate)）。
pub(crate) async fn process_one_batch(state: &AppState) -> Result<u64, anyhow::Error> {
    let pool = state.db_pool.get_inner();
    let rows: Vec<(String, String)> = infrastructure::sql_fetch_all!(
        &**pool,
        (String, String),
        sqlite: "SELECT id, payload FROM outbox_dead_letters WHERE event_type = ? ORDER BY created_at ASC LIMIT ?",
        pg: "SELECT id, payload FROM outbox_dead_letters WHERE event_type = $1 ORDER BY created_at ASC LIMIT $2",
        EVENT_TYPE,
        BATCH_LIMIT
    )?;

    if rows.is_empty() {
        return Ok(0);
    }

    let nurture_url = match &state.nurture_url {
        Some(url) if !url.is_empty() => url.clone(),
        _ => {
            error!(
                pending = rows.len(),
                "CoinChargeDlq: NURTURE_API_URL unset; {} coin_charge_failed row(s) retained",
                rows.len()
            );
            return Ok(0);
        }
    };
    let secret = match &state.nurture_internal_secret {
        Some(s) if !s.is_empty() => s.clone(),
        _ => {
            error!(
                pending = rows.len(),
                "CoinChargeDlq: NURTURE_INTERNAL_SECRET unset; {} coin_charge_failed row(s) retained",
                rows.len()
            );
            return Ok(0);
        }
    };

    let mut processed = 0u64;

    for (id, payload_str) in rows {
        let payload: serde_json::Value = match serde_json::from_str(&payload_str) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    dlq_id = %id,
                    error = %e,
                    "Quarantining outbox_dead_letters row with invalid JSON payload"
                );
                if let Err(qe) = infrastructure::sql_exec!(
                    &**pool,
                    sqlite: "UPDATE outbox_dead_letters SET event_type = ?, error_reason = ? WHERE id = ?",
                    pg: "UPDATE outbox_dead_letters SET event_type = $1, error_reason = $2 WHERE id = $3",
                    EVENT_TYPE_POISON,
                    &format!("invalid_json: {e}"),
                    &id
                ) {
                    error!(dlq_id = %id, error = %qe, "Failed to quarantine poison DLQ row");
                }
                continue;
            }
        };

        // 行ごとに最新 OXP を読む（バッチ中の poller 更新を反映）
        let oxp = state.oxilean_power.load(Ordering::Relaxed);
        match attempt_coin_charge_once(
            &nurture_url,
            &secret,
            oxp,
            &payload,
            state.nurture_s2s.as_ref(),
        )
        .await
        {
            Ok(()) => {
                infrastructure::sql_exec!(
                    &**pool,
                    sqlite: "DELETE FROM outbox_dead_letters WHERE id = ?",
                    pg: "DELETE FROM outbox_dead_letters WHERE id = $1",
                    &id
                )?;
                processed += 1;
                info!(dlq_id = %id, "Coin charge DLQ row replayed successfully");
            }
            Err(e) => {
                error!(
                    dlq_id = %id,
                    error = %e,
                    "Coin charge DLQ replay failed; row retained"
                );
            }
        }
    }

    Ok(processed)
}

/// `outbox_dead_letters` の coin-charge 失敗行を定期的に再送する。
///
/// `cancel` が発火するまでループする。バッチ失敗はログのみで継続する（oxilean_poller と同型）。
pub async fn run(
    state: AppState,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), anyhow::Error> {
    info!("🔄 Starting Coin Charge DLQ worker...");
    loop {
        if let Err(e) = process_one_batch(&state).await {
            error!("Coin charge DLQ batch failed: {}", e);
        }
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("🛑 Coin Charge DLQ worker shutdown requested");
                return Ok(());
            }
            _ = tokio::time::sleep(Duration::from_secs(60)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use serial_test::serial;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use uuid::Uuid;

    /// commerce.rs と同型: Bearer + OXP≥900 を検証する Nurture mock。
    async fn spawn_oxp_aware_nurture_mock(
        sync_counter: Arc<AtomicUsize>,
        reject_status: Option<axum::http::StatusCode>,
    ) -> String {
        let counter_clone = sync_counter.clone();
        let mock_nurture_app = axum::Router::new().route(
            "/internal/coin-charge",
            axum::routing::post(
                move |headers: axum::http::HeaderMap, _req: axum::extract::Request| {
                    let counter = counter_clone.clone();
                    async move {
                        if let Some(status) = reject_status {
                            return (
                                status,
                                axum::response::Json(serde_json::json!({"error": "rejected"})),
                            )
                                .into_response();
                        }
                        let bearer_ok = headers
                            .get("authorization")
                            .and_then(|h| h.to_str().ok())
                            .map(|v| v == "Bearer mock_secret")
                            .unwrap_or(false);
                        let cert_ok = headers
                            .get("x-oxilean-proof-certificate")
                            .and_then(|h| h.to_str().ok())
                            .map(|b64| {
                                use base64::Engine;
                                base64::engine::general_purpose::STANDARD
                                    .decode(b64)
                                    .ok()
                                    .and_then(|j| {
                                        serde_json::from_slice::<
                                            aiome_core_contracts::oxilean::OxiLeanProofCertificate,
                                        >(&j)
                                        .ok()
                                    })
                                    .map(|c| c.verify("mock_secret") && c.oxp_score >= 900)
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);
                        if !(bearer_ok && cert_ok) {
                            return (
                                axum::http::StatusCode::FORBIDDEN,
                                axum::response::Json(serde_json::json!({"error": "forbidden"})),
                            )
                                .into_response();
                        }
                        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        axum::response::Json(serde_json::json!({"status": "success"}))
                            .into_response()
                    }
                },
            ),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let nurture_url = format!("http://127.0.0.1:{port}");
        tokio::spawn(async move {
            axum::serve(listener, mock_nurture_app).await.unwrap();
        });
        nurture_url
    }

    async fn insert_dlq_row(state: &AppState, id: &str, payload: &str) {
        let pool = state.db_pool.get_inner();
        infrastructure::sql_exec!(
            &**pool,
            sqlite: "INSERT INTO outbox_dead_letters (id, event_type, payload, error_reason) VALUES (?, ?, ?, ?)",
            pg: "INSERT INTO outbox_dead_letters (id, event_type, payload, error_reason) VALUES ($1, $2, $3, $4)",
            id,
            EVENT_TYPE,
            payload,
            "test"
        )
        .unwrap();
    }

    async fn count_dlq_row(state: &AppState, id: &str) -> i64 {
        let pool = state.db_pool.get_inner();
        let remaining: (i64,) = infrastructure::sql_fetch_one!(
            &**pool,
            (i64,),
            sqlite: "SELECT COUNT(*) FROM outbox_dead_letters WHERE id = ?",
            pg: "SELECT COUNT(*) FROM outbox_dead_letters WHERE id = $1",
            id
        )
        .unwrap();
        remaining.0
    }

    #[serial]
    #[tokio::test]
    async fn process_one_batch_replays_and_deletes_on_success() {
        let sync_counter = Arc::new(AtomicUsize::new(0));
        let nurture_url = spawn_oxp_aware_nurture_mock(sync_counter.clone(), None).await;

        let (_server, mut state, _tmp) = crate::api_integration_tests::create_test_server().await;
        state.nurture_url = Some(nurture_url);
        state.nurture_internal_secret = Some("mock_secret".to_string());
        state
            .oxilean_power
            .store(950, std::sync::atomic::Ordering::Relaxed);

        let payload = serde_json::json!({
            "actor_id": Uuid::new_v4(),
            "amount": 500,
            "currency": "coin",
            "stripe_event_id": "ev_dlq_test",
            "idempotency_key": "ev_dlq_test"
        });
        let dlq_id = Uuid::new_v4().to_string();
        insert_dlq_row(&state, &dlq_id, &serde_json::to_string(&payload).unwrap()).await;

        let processed = process_one_batch(&state).await.unwrap();
        assert_eq!(processed, 1);
        assert_eq!(sync_counter.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(count_dlq_row(&state, &dlq_id).await, 0);
    }

    /// N1: mock 500 → 行数不変（再 INSERT なし）
    #[serial]
    #[tokio::test]
    async fn process_one_batch_retains_row_on_http_500() {
        let sync_counter = Arc::new(AtomicUsize::new(0));
        let nurture_url = spawn_oxp_aware_nurture_mock(
            sync_counter.clone(),
            Some(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        )
        .await;

        let (_server, mut state, _tmp) = crate::api_integration_tests::create_test_server().await;
        state.nurture_url = Some(nurture_url);
        state.nurture_internal_secret = Some("mock_secret".to_string());
        state
            .oxilean_power
            .store(950, std::sync::atomic::Ordering::Relaxed);

        let dlq_id = Uuid::new_v4().to_string();
        let payload = serde_json::json!({
            "actor_id": Uuid::new_v4(),
            "amount": 100,
            "currency": "coin",
            "stripe_event_id": "ev_fail",
            "idempotency_key": "ev_fail"
        });
        insert_dlq_row(&state, &dlq_id, &serde_json::to_string(&payload).unwrap()).await;

        let before = count_dlq_row(&state, &dlq_id).await;
        let processed = process_one_batch(&state).await.unwrap();
        assert_eq!(processed, 0);
        assert_eq!(sync_counter.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(count_dlq_row(&state, &dlq_id).await, before);
    }

    /// N1: mock 403（OXP 不足）→ 行数不変
    #[serial]
    #[tokio::test]
    async fn process_one_batch_retains_row_on_oxp_forbidden() {
        let sync_counter = Arc::new(AtomicUsize::new(0));
        let nurture_url = spawn_oxp_aware_nurture_mock(sync_counter.clone(), None).await;

        let (_server, mut state, _tmp) = crate::api_integration_tests::create_test_server().await;
        state.nurture_url = Some(nurture_url);
        state.nurture_internal_secret = Some("mock_secret".to_string());
        state
            .oxilean_power
            .store(0, std::sync::atomic::Ordering::Relaxed);

        let dlq_id = Uuid::new_v4().to_string();
        let payload = serde_json::json!({
            "actor_id": Uuid::new_v4(),
            "amount": 100,
            "currency": "coin",
            "stripe_event_id": "ev_oxp0",
            "idempotency_key": "ev_oxp0"
        });
        insert_dlq_row(&state, &dlq_id, &serde_json::to_string(&payload).unwrap()).await;

        let processed = process_one_batch(&state).await.unwrap();
        assert_eq!(processed, 0);
        assert_eq!(sync_counter.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(count_dlq_row(&state, &dlq_id).await, 1);
    }

    /// N2: 不正 JSON → パニックなし・poison 隔離（再送対象から除外）
    #[serial]
    #[tokio::test]
    async fn process_one_batch_quarantines_invalid_json() {
        let sync_counter = Arc::new(AtomicUsize::new(0));
        let nurture_url = spawn_oxp_aware_nurture_mock(sync_counter.clone(), None).await;

        let (_server, mut state, _tmp) = crate::api_integration_tests::create_test_server().await;
        state.nurture_url = Some(nurture_url);
        state.nurture_internal_secret = Some("mock_secret".to_string());
        state
            .oxilean_power
            .store(950, std::sync::atomic::Ordering::Relaxed);

        let dlq_id = Uuid::new_v4().to_string();
        insert_dlq_row(&state, &dlq_id, "not-json{{{").await;

        let processed = process_one_batch(&state).await.unwrap();
        assert_eq!(processed, 0);
        assert_eq!(sync_counter.load(std::sync::atomic::Ordering::SeqCst), 0);
        // 行は残るが event_type が poison に変わり、再送 SELECT 対象外
        assert_eq!(count_dlq_row(&state, &dlq_id).await, 1);
        let pool = state.db_pool.get_inner();
        let event_type: (String,) = infrastructure::sql_fetch_one!(
            &**pool,
            (String,),
            sqlite: "SELECT event_type FROM outbox_dead_letters WHERE id = ?",
            pg: "SELECT event_type FROM outbox_dead_letters WHERE id = $1",
            &dlq_id
        )
        .unwrap();
        assert_eq!(event_type.0, EVENT_TYPE_POISON);

        // 2 回目バッチでも HTTP は飛ばない（head-of-line 解消）
        let processed2 = process_one_batch(&state).await.unwrap();
        assert_eq!(processed2, 0);
        assert_eq!(sync_counter.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    /// H3: NURTURE_API_URL 未設定でもパニックせず行を保持する
    #[serial]
    #[tokio::test]
    async fn process_one_batch_retains_row_when_nurture_url_unset() {
        let (_server, mut state, _tmp) = crate::api_integration_tests::create_test_server().await;
        state.nurture_url = None;
        state.nurture_internal_secret = Some("mock_secret".to_string());

        let dlq_id = Uuid::new_v4().to_string();
        let payload = serde_json::json!({
            "actor_id": Uuid::new_v4(),
            "amount": 50,
            "currency": "coin",
            "stripe_event_id": "ev_no_url",
            "idempotency_key": "ev_no_url"
        });
        insert_dlq_row(&state, &dlq_id, &serde_json::to_string(&payload).unwrap()).await;

        let processed = process_one_batch(&state).await.unwrap();
        assert_eq!(processed, 0);
        assert_eq!(count_dlq_row(&state, &dlq_id).await, 1);
    }

    /// H3: NURTURE_INTERNAL_SECRET 未設定でも行を保持する
    #[serial]
    #[tokio::test]
    async fn process_one_batch_retains_row_when_nurture_secret_unset() {
        let (_server, mut state, _tmp) = crate::api_integration_tests::create_test_server().await;
        state.nurture_url = Some("http://127.0.0.1:9".to_string());
        state.nurture_internal_secret = None;

        let dlq_id = Uuid::new_v4().to_string();
        let payload = serde_json::json!({
            "actor_id": Uuid::new_v4(),
            "amount": 50,
            "currency": "coin",
            "stripe_event_id": "ev_no_secret",
            "idempotency_key": "ev_no_secret"
        });
        insert_dlq_row(&state, &dlq_id, &serde_json::to_string(&payload).unwrap()).await;

        let processed = process_one_batch(&state).await.unwrap();
        assert_eq!(processed, 0);
        assert_eq!(count_dlq_row(&state, &dlq_id).await, 1);
    }
}
