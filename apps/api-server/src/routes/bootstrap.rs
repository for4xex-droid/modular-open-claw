/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Bootstrap Mode API ルート (Phase 2B-CORE)
//!
//! 初回起動時のセットアップフローに必要なエンドポイントを提供する。
//! これらのエンドポイントは **認証不要** でアクセスできる（セットアップ完了前に使用するため）。

use crate::AppState;
use axum::{extract::State, response::Json};
use serde::Serialize;
use shared::bootstrap_detector::{BootMode, BootstrapDetector};

/// Bootstrap ステータスレスポンス
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BootstrapStatusResponse {
    /// 起動モード ("normal" or "setup")
    pub mode: String,
    /// DB が存在するか
    pub db_exists: bool,
    /// LLM プロバイダが設定済みか
    pub llm_configured: bool,
    /// API サーバーシークレットが設定済みか
    pub api_secret_set: bool,
    /// SOUL.md が存在するか
    pub soul_exists: bool,
    /// 不足している設定項目
    pub missing_items: Vec<String>,
}

/// GET /api/v1/bootstrap/status
///
/// セットアップの完了状態を返す。認証不要。
/// フロントエンドはこのエンドポイントを最初に呼び出し、
/// `mode == "setup"` の場合はセットアップ WebUI に遷移する。
#[utoipa::path(
    get,
    path = "/api/v1/bootstrap/status",
    responses(
        (status = 200, description = "Bootstrap status", body = BootstrapStatusResponse)
    )
)]
pub async fn bootstrap_status(State(state): State<AppState>) -> Json<BootstrapStatusResponse> {
    let config = state.config.get_inner();
    let root = config.resolver.root();

    let api_secret_set = state.api_server_secret.as_opt().is_some();
    let llm_configured = config.gemini_api_key.is_some()
        || config.openai_api_key.is_some()
        || config.anthropic_api_key.is_some()
        || !config.ollama_host.is_empty();

    let diagnosis = BootstrapDetector::diagnose(root, Some(api_secret_set), Some(llm_configured));

    Json(BootstrapStatusResponse {
        mode: match diagnosis.mode {
            BootMode::Normal => "normal".to_string(),
            BootMode::Setup => "setup".to_string(),
        },
        db_exists: diagnosis.db_exists,
        llm_configured: diagnosis.llm_configured,
        api_secret_set: diagnosis.api_secret_set,
        soul_exists: diagnosis.soul_exists,
        missing_items: diagnosis.missing_items,
    })
}

/// Ollama 自動検出の結果
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OllamaDetectionResponse {
    /// Ollama が利用可能か
    pub available: bool,
    /// 検出された URL
    pub url: Option<String>,
    /// インストール済みモデル一覧
    pub models: Vec<String>,
    /// エラーメッセージ（検出失敗時）
    pub error: Option<String>,
}

/// GET /api/v1/bootstrap/detect-ollama
///
/// ローカルの Ollama サーバーを自動検出する。認証不要。
/// Phase 2B-3: Ollama 自動検出
#[utoipa::path(
    get,
    path = "/api/v1/bootstrap/detect-ollama",
    responses(
        (status = 200, description = "Ollama detection result", body = OllamaDetectionResponse)
    )
)]
pub async fn detect_ollama() -> Json<OllamaDetectionResponse> {
    let client = aiome_core::http::get_http_client().clone();

    // 候補 URL を順に試す
    let candidates = ["http://127.0.0.1:11434", "http://localhost:11434"]; // allow-anti-pattern

    for url in &candidates {
        let version_url = format!("{}/api/version", url);
        if let Ok(resp) = client
            .get(&version_url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            if resp.status().is_success() {
                // Ollama が見つかった。モデル一覧を取得
                let mut models = Vec::new();
                let tags_url = format!("{}/api/tags", url);
                if let Ok(tags_resp) = client
                    .get(&tags_url)
                    .timeout(std::time::Duration::from_secs(3))
                    .send()
                    .await
                {
                    if let Ok(json) = tags_resp.json::<serde_json::Value>().await {
                        if let Some(arr) = json.get("models").and_then(|m| m.as_array()) {
                            for m in arr {
                                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                                    models.push(name.to_string());
                                }
                            }
                        }
                    }
                }

                return Json(OllamaDetectionResponse {
                    available: true,
                    url: Some(url.to_string()),
                    models,
                    error: None,
                });
            }
        }
    }

    Json(OllamaDetectionResponse {
        available: false,
        url: None,
        models: vec![],
        error: Some("Ollama server not found on localhost.".to_string()),
    })
}

/// Factory Reset レスポンス
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FactoryResetResponse {
    /// 成功したか
    pub success: bool,
    /// 削除されたファイル
    pub deleted_files: Vec<String>,
    /// 削除されたディレクトリ
    pub deleted_dirs: Vec<String>,
    /// 保持されたファイル
    pub preserved_files: Vec<String>,
    /// エラーメッセージ
    pub errors: Vec<String>,
}

/// POST /api/v1/bootstrap/factory-reset
///
/// Factory Reset を実行する。**System Admin 権限が必要**。
/// Phase 2B-4: Factory Reset
#[utoipa::path(
    post,
    path = "/api/v1/bootstrap/factory-reset",
    responses(
        (status = 200, description = "Factory reset complete", body = FactoryResetResponse),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn factory_reset(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<FactoryResetResponse>, crate::error::AppError> {
    // System Admin 権限チェック
    let agent_id = _auth.agent_id;
    if agent_id != state.system_agent_id {
        return Err(crate::error::AppError::forbidden(
            "Factory reset requires System Admin privileges",
        ));
    }

    let config = state.config.get_inner();
    let root = config.resolver.root();

    match shared::bootstrap_detector::FactoryReset::execute(root) {
        Ok(report) => Ok(Json(FactoryResetResponse {
            success: report.errors.is_empty(),
            deleted_files: report.deleted_files,
            deleted_dirs: report.deleted_dirs,
            preserved_files: report.preserved_files,
            errors: report.errors,
        })),
        Err(e) => Err(crate::error::AppError::internal(format!(
            "Factory reset failed: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_status_response_serialization() {
        let resp = BootstrapStatusResponse {
            mode: "setup".to_string(),
            db_exists: false,
            llm_configured: false,
            api_secret_set: false,
            soul_exists: false,
            missing_items: vec!["LLM provider".to_string()],
        };
        let json = serde_json::to_string(&resp).unwrap(); // allow-anti-pattern
        assert!(json.contains("\"mode\":\"setup\""));
        assert!(json.contains("\"missing_items\":[\"LLM provider\"]"));
    }

    #[test]
    fn test_ollama_detection_response_serialization() {
        let resp = OllamaDetectionResponse {
            available: false,
            url: None,
            models: vec![],
            error: Some("Not found".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap(); // allow-anti-pattern
        assert!(json.contains("\"available\":false"));
        assert!(json.contains("\"error\":\"Not found\""));
    }

    #[test]
    fn test_factory_reset_response_serialization() {
        let resp = FactoryResetResponse {
            success: true,
            deleted_files: vec!["aiome.db".to_string()],
            deleted_dirs: vec!["artifacts".to_string()],
            preserved_files: vec![".env".to_string()],
            errors: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap(); // allow-anti-pattern
        assert!(json.contains("\"success\":true"));
    }
}
