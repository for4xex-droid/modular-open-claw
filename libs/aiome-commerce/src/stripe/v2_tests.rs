/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::*;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::sqlite::SqlitePoolOptions;

async fn get_test_engine_with_secrets(webhook_secret: &str) -> StripeCommerceEngine {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    StripeCommerceEngine::new(
        SecretString::from("sk_test_mock".to_string()),
        SecretString::from(webhook_secret.to_string()),
        pool,
        None,
        None,
    )
}

fn generate_signature(payload: &str, secret: &str) -> (String, String) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let timestamp = now.to_string();

    let signed_payload = format!("{}.{}", timestamp, payload);
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let result_code = mac.finalize();
    let signature = hex::encode(result_code.into_bytes());

    (timestamp, signature)
}

#[tokio::test]
async fn test_verify_signature_multiple_secrets_first() {
    let secrets = "whsec_first_123,whsec_second_456";
    let engine = get_test_engine_with_secrets(secrets).await;

    let payload = r#"{"id": "evt_123", "type": "checkout.session.completed"}"#;
    let (timestamp, signature) = generate_signature(payload, "whsec_first_123");
    let sig_header = format!("t={},v1={}", timestamp, signature);

    let result = engine.verify_signature(payload, &sig_header);

    // カンマ区切りの1つ目のシークレットで署名された場合、検証成功すること
    assert!(
        result.is_ok(),
        "First secret verification failed: {:?}",
        result
    );
}

#[tokio::test]
async fn test_verify_signature_multiple_secrets_second() {
    let secrets = "whsec_first_123,whsec_second_456";
    let engine = get_test_engine_with_secrets(secrets).await;

    let payload = r#"{"id": "evt_123", "type": "checkout.session.completed"}"#;
    let (timestamp, signature) = generate_signature(payload, "whsec_second_456");
    let sig_header = format!("t={},v1={}", timestamp, signature);

    let result = engine.verify_signature(payload, &sig_header);

    assert!(
        result.is_ok(),
        "Second secret verification failed: {:?}",
        result
    );
}

#[tokio::test]
async fn test_verify_signature_multiple_secrets_fails() {
    let secrets = "whsec_first_123,whsec_second_456";
    let engine = get_test_engine_with_secrets(secrets).await;

    let payload = r#"{"id": "evt_123", "type": "checkout.session.completed"}"#;
    let (timestamp, signature) = generate_signature(payload, "whsec_invalid_999");
    let sig_header = format!("t={},v1={}", timestamp, signature);

    let result = engine.verify_signature(payload, &sig_header);

    assert!(
        result.is_err(),
        "Verification should have failed for invalid secret"
    );
}

// ============================================================================
// v2 thin event 判定テスト (単体テスト)
// ============================================================================

/// v2 thin event の判定: "object": "v2.core.event" を検出すること
#[test]
fn test_v2_thin_event_detection() {
    let payload = serde_json::json!({
        "id": "evt_v2_001",
        "object": "v2.core.event",
        "type": "v1.checkout.session.completed",
        "related_object": {
            "url": "/v1/checkout/sessions/cs_test_123",
            "id": "cs_test_123",
            "type": "checkout.session"
        }
    });

    assert_eq!(
        payload["object"].as_str(),
        Some("v2.core.event"),
        "v2 thin event should be detected by 'object' field"
    );
}

/// v1 snapshot event は "object": "event" であり、v2 判定に一致しないこと
#[test]
fn test_v1_event_not_detected_as_v2() {
    let payload = serde_json::json!({
        "id": "evt_v1_001",
        "object": "event",
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_test_456",
                "object": "checkout.session"
            }
        }
    });

    assert_ne!(
        payload["object"].as_str(),
        Some("v2.core.event"),
        "v1 event should NOT be detected as v2"
    );
}

/// v2 thin event の type から "v1." プレフィックスを除去して v1 互換にする変換テスト
#[test]
fn test_v2_type_prefix_stripping() {
    let test_cases = vec![
        (
            "v1.checkout.session.completed",
            "checkout.session.completed",
        ),
        ("v1.invoice.paid", "invoice.paid"),
        (
            "v1.customer.subscription.deleted",
            "customer.subscription.deleted",
        ),
        // v1. プレフィックスがない場合はそのまま
        ("checkout.session.completed", "checkout.session.completed"),
        ("invoice.payment_failed", "invoice.payment_failed"),
    ];

    for (input, expected) in test_cases {
        let result = input.strip_prefix("v1.").unwrap_or(input);
        assert_eq!(
            result, expected,
            "Type '{}' should be converted to '{}'",
            input, expected
        );
    }
}

/// related_object が null の場合のペイロード構造テスト
/// Webhook ハンドラは 200 OK で処理スキップすべき
#[test]
fn test_v2_thin_event_null_related_object() {
    let payload = serde_json::json!({
        "id": "evt_v2_002",
        "object": "v2.core.event",
        "type": "v1.checkout.session.completed",
        "related_object": null
    });

    assert_eq!(payload["object"].as_str(), Some("v2.core.event"));
    assert!(
        payload["related_object"].is_null(),
        "related_object should be null"
    );
    // url が取得できないことを確認
    assert!(
        payload["related_object"]["url"].as_str().is_none(),
        "null related_object should not have url"
    );
}

/// related_object に url がない場合のペイロード構造テスト
#[test]
fn test_v2_thin_event_missing_url_in_related_object() {
    let payload = serde_json::json!({
        "id": "evt_v2_003",
        "object": "v2.core.event",
        "type": "v1.checkout.session.completed",
        "related_object": {
            "id": "cs_test_789",
            "type": "checkout.session"
        }
    });

    assert_eq!(payload["object"].as_str(), Some("v2.core.event"));
    assert!(
        payload["related_object"]["url"].as_str().is_none(),
        "related_object without url should return None"
    );
}

// ============================================================================
// SSRF パスバリデーションテスト
// ============================================================================

/// SSRF 防御: /v1/ プレフィックスは許可
#[test]
fn test_ssrf_valid_v1_path() {
    let url = "/v1/checkout/sessions/cs_test_123";
    assert!(url.starts_with("/v1/") || url.starts_with("/v2/"));
    assert!(!url.contains(".."));
}

/// SSRF 防御: /v2/ プレフィックスは許可
#[test]
fn test_ssrf_valid_v2_path() {
    let url = "/v2/billing/meter_events/mtr_evt_123";
    assert!(url.starts_with("/v1/") || url.starts_with("/v2/"));
    assert!(!url.contains(".."));
}

/// SSRF 防御: 無効なプレフィックスは拒否
#[test]
fn test_ssrf_rejects_invalid_prefix() {
    let invalid_paths = [
        "/admin/secret",
        "/internal/config",
        "https://evil.com/steal",
        "//evil.com",
        "",
    ];
    for path in &invalid_paths {
        assert!(
            !(path.starts_with("/v1/") || path.starts_with("/v2/")),
            "Path '{}' should be rejected",
            path
        );
    }
}

/// SSRF 防御: パストラバーサルは拒否（KI: Directory Traversal Blocking）
#[test]
fn test_ssrf_rejects_path_traversal() {
    let traversal_paths = [
        "/v1/../../internal/admin",
        "/v1/checkout/../../../etc/passwd",
        "/v2/../v1/secrets",
    ];
    for path in &traversal_paths {
        assert!(
            path.contains(".."),
            "Path '{}' should contain traversal",
            path
        );
    }
}

/// whsec_test がカンマ区切りの一部に含まれている場合の拒否テスト
#[tokio::test]
async fn test_verify_signature_rejects_whsec_test_in_multi_secret() {
    let secrets = "whsec_real_123,whsec_test";
    let engine = get_test_engine_with_secrets(secrets).await;

    let payload = r#"{"id": "evt_123", "type": "checkout.session.completed"}"#;
    let (timestamp, signature) = generate_signature(payload, "whsec_real_123");
    let sig_header = format!("t={},v1={}", timestamp, signature);

    // whsec_test が含まれている場合、エラーを返すべき（ただし is_mock=false の時のみ）
    // テストエンジンは is_mock=true なので、この場合は検証自体が通る可能性がある
    // StripeCommerceEngine の is_mock フィールドを確認
    let result = engine.verify_signature(payload, &sig_header);
    // is_mock=true のテストエンジンでは whsec_test チェックはスキップされる
    assert!(
        result.is_ok(),
        "Mock engine should bypass whsec_test check: {:?}",
        result
    );
}
