/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use super::*;
use futures_util::{SinkExt, StreamExt};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

async fn spawn_test_hub() -> (SocketAddr, Arc<HubState>) {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    let db_pool = shared::db::DatabasePool::Sqlite(pool);
    init_hub_db(&db_pool).await.expect("Failed to init db");

    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(HubState {
        pool: db_pool,
        secret: secrecy::SecretString::new("test_secret".to_string()),
        auth_manager: Arc::new(shared::auth::MockAuthManager::new()),
        tx,
        active_connections: std::sync::atomic::AtomicUsize::new(0),
        agent_registry: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        config: shared::config::AiomeConfig::default(),
        metadata_free_channels: Arc::new(
            tokio::sync::RwLock::new(std::collections::HashMap::new()),
        ),
    });

    let app = build_app(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, state)
}

#[tokio::test]
async fn test_ws_authentication_unauthorized() {
    let (addr, _state) = spawn_test_hub().await;
    let ws_url = format!("ws://{}/api/v1/federation/ws", addr);

    let result = connect_async(&ws_url).await;
    // Should fail because no auth header
    assert!(result.is_err(), "Expected connection to fail without auth");
}

#[tokio::test]
async fn test_ws_authentication_authorized_and_ping() {
    let (addr, state) = spawn_test_hub().await;
    let ws_url = format!("ws://{}/api/v1/federation/ws", addr);

    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer test_secret"),
    );

    let (mut ws_stream, _) = connect_async(request).await.expect("Failed to connect");

    assert_eq!(
        state
            .active_connections
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );

    // Send Ping
    use aiome_core::contracts::HubMessage;
    let ping = HubMessage::Ping {
        client_time: chrono::Utc::now().to_rfc3339(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&ping).unwrap().into()))
        .await
        .unwrap();

    // Receive Pong
    loop {
        if let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();
            if msg.is_text() {
                let text = msg.to_text().unwrap();
                if let Ok(HubMessage::Pong { server_time }) =
                    serde_json::from_str::<HubMessage>(text)
                {
                    assert!(!server_time.is_empty());
                    break;
                }
            }
        } else {
            panic!("No Pong message received");
        }
    }

    // Disconnect
    drop(ws_stream);

    // Give it a moment to process disconnect
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        state
            .active_connections
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn test_metadata_free_unicast_relay() {
    let (addr, _state) = spawn_test_hub().await;

    // 1. Connect Client A for channel_A
    let ws_url_a = format!(
        "ws://{}/api/v1/federation/ws?channel_local_id=channel_A",
        addr
    );
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request_a = ws_url_a.into_client_request().unwrap();
    request_a.headers_mut().insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer test_secret"),
    );
    let (mut ws_stream_a, _) = connect_async(request_a).await.expect("Failed to connect A");

    // 2. Connect Client B for channel_B
    let ws_url_b = format!(
        "ws://{}/api/v1/federation/ws?channel_local_id=channel_B",
        addr
    );
    let mut request_b = ws_url_b.into_client_request().unwrap();
    request_b.headers_mut().insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer test_secret"),
    );
    let (mut ws_stream_b, _) = connect_async(request_b).await.expect("Failed to connect B");

    // 3. Post a message to channel_A via HTTP relay
    let http_client = reqwest::Client::new();
    let relay_url = format!("http://{}/api/v1/commune/relay/metadata-free", addr);

    use aiome_core::commune::ZeroMetadataCommuneEnvelope;
    let envelope = ZeroMetadataCommuneEnvelope {
        channel_local_id: "channel_A".to_string(),
        encrypted_payload: "secret_payload_for_a".to_string(),
    };

    let res = http_client
        .post(&relay_url)
        .header("Authorization", "Bearer test_secret")
        .json(&envelope)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::ACCEPTED);

    // Negative Test (Unregistered channel)
    let bad_envelope = ZeroMetadataCommuneEnvelope {
        channel_local_id: "non_existent_channel".to_string(),
        encrypted_payload: "should_fail".to_string(),
    };
    let res_bad = http_client
        .post(&relay_url)
        .header("Authorization", "Bearer test_secret")
        .json(&bad_envelope)
        .send()
        .await
        .unwrap();
    assert_eq!(res_bad.status(), reqwest::StatusCode::NOT_FOUND);

    // Verify Client A received the relayed envelope
    use aiome_core::contracts::HubMessage;
    let msg_a = loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream_a.next())
            .await
            .expect("Timeout waiting for message on Client A")
            .expect("Stream A closed")
            .unwrap();
        if msg.is_text() {
            break msg;
        } else {
            println!("Received non-text message: {:?}", msg);
        }
    };
    let text_a = msg_a.to_text().unwrap();
    let hub_msg: HubMessage = serde_json::from_str(text_a).unwrap();
    if let HubMessage::ZeroMetadataCommuneRelay(env) = hub_msg {
        assert_eq!(env.channel_local_id, "channel_A");
        assert_eq!(env.encrypted_payload, "secret_payload_for_a");
    } else {
        panic!(
            "Expected HubMessage::ZeroMetadataCommuneRelay, got {:?}",
            hub_msg
        );
    }

    // Verify Client B received nothing (no text messages should arrive, though Ping is fine)
    let msg_b_res = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while let Some(Ok(msg)) = ws_stream_b.next().await {
            if msg.is_text() {
                return msg;
            }
        }
        std::future::pending::<tokio_tungstenite::tungstenite::Message>().await
    })
    .await;
    assert!(
        msg_b_res.is_err(),
        "Client B should not have received any text message"
    );
}

/// OP-020-F5 S-1/S-2: opaque relay + pairing gate + no Soul canary in DB.
#[tokio::test]
async fn test_soul_sync_relay_broadcast_and_no_plaintext_in_db() {
    let (addr, state) = spawn_test_hub().await;

    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let ws_url = format!("ws://{}/api/v1/federation/ws", addr);
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer test_secret"),
    );
    let (mut ws_stream, _) = connect_async(request).await.expect("WS connect");

    // Give the hub a moment to subscribe the WS to broadcast.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    const CANARY: &str = "SOUL_PLAINTEXT_CANARY_NEVER_ON_HUB";
    use aiome_core::soul_sync::{EncryptedEnvelope, SoulSyncPairRequest};
    let session_id = "sess-s1-test".to_string();
    let envelope = EncryptedEnvelope {
        session_id: session_id.clone(),
        // Deliberately put a Soul-like canary in the ciphertext field: hub must still
        // treat it as opaque and must not persist it.
        ciphertext: CANARY.to_string(),
    };

    let http_client = reqwest::Client::new();
    let pair_url = format!("http://{}/api/v1/soul-sync/pair", addr);
    let relay_url = format!("http://{}/api/v1/soul-sync/relay", addr);

    // Negative (S-2): unpaired session cannot relay.
    let res_unpaired = http_client
        .post(&relay_url)
        .header("Authorization", "Bearer test_secret")
        .json(&envelope)
        .send()
        .await
        .unwrap();
    assert_eq!(res_unpaired.status(), reqwest::StatusCode::FORBIDDEN);

    let pair = SoulSyncPairRequest {
        session_id: session_id.clone(),
        device_a_pubkey: "pubkey-a-base64".into(),
        device_b_pubkey: "pubkey-b-base64".into(),
    };
    let res_pair = http_client
        .post(&pair_url)
        .header("Authorization", "Bearer test_secret")
        .json(&pair)
        .send()
        .await
        .unwrap();
    assert_eq!(res_pair.status(), reqwest::StatusCode::CREATED);

    let res = http_client
        .post(&relay_url)
        .header("Authorization", "Bearer test_secret")
        .json(&envelope)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::ACCEPTED);

    use aiome_core::contracts::HubMessage;
    let msg = loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
            .await
            .expect("Timeout waiting for SoulSyncRelay")
            .expect("WS closed")
            .unwrap();
        if msg.is_text() {
            break msg;
        }
    };
    let text = msg.to_text().unwrap();
    let hub_msg: HubMessage = serde_json::from_str(text).unwrap();
    match hub_msg {
        HubMessage::SoulSyncRelay(env) => {
            assert_eq!(env.session_id, session_id);
            assert_eq!(env.ciphertext, CANARY);
        }
        other => panic!("Expected SoulSyncRelay, got {:?}", other),
    }

    // Negative: canary must not appear in Soul-bearing tables — paired_devices holds pubkeys only.
    assert_canary_absent_from_sqlite(&state.pool, CANARY).await;

    // Negative: unauthorized relay rejected.
    let res_unauth = http_client
        .post(&relay_url)
        .json(&envelope)
        .send()
        .await
        .unwrap();
    assert_eq!(res_unauth.status(), reqwest::StatusCode::UNAUTHORIZED);

    // Negative: empty ciphertext rejected.
    let res_bad = http_client
        .post(&relay_url)
        .header("Authorization", "Bearer test_secret")
        .json(&EncryptedEnvelope {
            session_id: session_id.clone(),
            ciphertext: String::new(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(res_bad.status(), reqwest::StatusCode::BAD_REQUEST);

    // S-2 Negative: after unpair, relay is forbidden again.
    let unpair_url = format!("http://{}/api/v1/soul-sync/pair/{}", addr, session_id);
    let res_unpair = http_client
        .delete(&unpair_url)
        .header("Authorization", "Bearer test_secret")
        .send()
        .await
        .unwrap();
    assert_eq!(res_unpair.status(), reqwest::StatusCode::OK);

    let res_after = http_client
        .post(&relay_url)
        .header("Authorization", "Bearer test_secret")
        .json(&envelope)
        .send()
        .await
        .unwrap();
    assert_eq!(res_after.status(), reqwest::StatusCode::FORBIDDEN);
}

async fn assert_canary_absent_from_sqlite(pool: &shared::db::DatabasePool, canary: &str) {
    use sqlx::Column;
    use sqlx::Row;
    let shared::db::DatabasePool::Sqlite(p) = pool else {
        panic!("test hub uses sqlite");
    };
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(p)
    .await
    .expect("list tables");
    for (table,) in tables {
        let safe = table.replace('"', "");
        let sql = format!("SELECT * FROM \"{}\"", safe);
        let rows = sqlx::query(&sql).fetch_all(p).await.expect("scan table");
        for row in rows {
            for col in row.columns() {
                let name = col.name();
                if let Ok(v) = row.try_get::<String, _>(name) {
                    assert!(
                        !v.contains(canary),
                        "Soul canary leaked into hub DB {}.{}",
                        safe,
                        name
                    );
                } else if let Ok(v) = row.try_get::<Option<String>, _>(name) {
                    if let Some(v) = v {
                        assert!(
                            !v.contains(canary),
                            "Soul canary leaked into hub DB {}.{}",
                            safe,
                            name
                        );
                    }
                }
            }
        }
    }
}
