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
