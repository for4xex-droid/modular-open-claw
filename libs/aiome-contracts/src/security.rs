/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 🛡️ PermissionManifest
///
/// エージェントやプラグインに許可される「物理的操作」の宣言。
/// Contracts 層に定義することで、AIが勝手に権限を昇格できないように「物理ロック」をかける。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PermissionManifest {
    pub allow_network: bool,
    pub allow_filesystem_write: bool,
    pub allow_shell_execution: bool,
    pub allowed_domains: Vec<String>,
}

/// Host allowlist check for [`PermissionManifest::allowed_domains`] (OP-096 / ADR-057).
///
/// Rules (Code Mode `aiome.fetch` base + harden):
/// - empty list / empty host (after trim) → deny
/// - host with control chars or internal whitespace → deny
/// - host is lowercased; a single trailing `.` (FQDN) is stripped
/// - allow entries are lowercased after trim; empty / leading-`.` / trailing-`.` junk ignored
/// - `*` → allow any non-empty normalized host
/// - exact host match (case-insensitive via normalization)
/// - subdomain suffix (`host.ends_with("." + domain)`) only when `domain` contains `.`
///   (prevents `allowed_domains=["com"]` from matching `evil.com`)
pub fn host_permitted(host: &str, allowed_domains: &[String]) -> bool {
    let host = host.trim();
    if host.is_empty() || allowed_domains.is_empty() {
        return false;
    }
    if host.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    let mut host = host.to_lowercase();
    if host.ends_with('.') {
        host.pop();
        if host.is_empty() {
            return false;
        }
    }

    for domain in allowed_domains {
        let domain = domain.trim();
        // Reject empty / dotted-junk entries (leading/trailing '.') — no accidental grants.
        if domain.is_empty() || domain.starts_with('.') || domain.ends_with('.') {
            continue;
        }
        let domain = domain.to_lowercase();
        if domain == "*" || domain == host {
            return true;
        }
        // Require a dot in the allow entry so a bare TLD cannot suffix-match the world.
        if domain.contains('.') && host.ends_with(&format!(".{}", domain)) {
            return true;
        }
    }
    false
}

/// 🛡️ SandboxProfile
///
/// 実行環境に応じたサンドボックスの制限レベル。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxProfile {
    /// デフォルト（通常のシェル実行制限）
    Default,
    /// Python スキルのビルド・検証用（より厳格な隔離）
    PythonForge,
    /// WASM スキルの実行用
    WasmRun,
    /// WASM スキルのビルド用 (SkillForge)
    WasmBuild,
    /// 汎用的なビルド・生成用
    ForgeBuild,
    /// 最も厳格な制限（ネットワーク・ファイル書き込み完全禁止）
    Strict,
    /// LoRA 学習用（HuggingFace などの一部ドメインのみ透過 + 長時間実行可）
    LoraTraining,
    /// MCP サーバー用（ネットワーク許可、ファイル書き込みはマニフェストに従う）
    McpServer,
    /// ブラウザ自律エージェント用（ネットワーク許可、ファイル書き込み禁止、OLLAMAへのアクセス許可）
    BrowserAgent,
}

/// ⛓️ RuntimeJail
///
/// エージェントのアクションを閉じ込める「檻」。
/// 物理的な入出力を行うすべてのコンポーネントは、このインターフェースを介して
/// 検証を受ける。
#[async_trait]
pub trait RuntimeJail: Send + Sync {
    /// シェルコマンドの実行を検証し、許可されていれば実行する
    async fn safe_exec(&self, cmd: &str) -> Result<String, crate::error::AiomeError> {
        self.safe_exec_with_profile(cmd, SandboxProfile::Default)
            .await
    }

    /// プロファイルを指定してシェルコマンドを実行する
    async fn safe_exec_with_profile(
        &self,
        cmd: &str,
        profile: SandboxProfile,
    ) -> Result<String, crate::error::AiomeError>;

    /// ファイル書き込みを検証する
    fn check_fs_write(&self, path: &std::path::Path) -> Result<(), crate::error::AiomeError>;

    /// ネットワーク接続を検証する
    fn check_network(&self, url: &str) -> Result<(), crate::error::AiomeError>;
}

/// 🪝 AgentHook
///
/// エージェントの実行サイクル（LLM呼び出し等）における介入ポイント。
/// BehaviorMonitor や AuditLogger がこれを実装し、実行の監視や停止を行う。
#[async_trait]
pub trait AgentHook: Send + Sync + std::fmt::Debug {
    /// LLM呼び出し「前」に実行されるフック。
    /// Err を返すとその呼び出しは中止される（監視・防御用）。
    async fn on_pre_execute(
        &self,
        request: &crate::llm::LlmRequest,
    ) -> Result<(), crate::error::AiomeError>;

    /// LLM呼び出し「後」に実行されるフック。
    /// レスポンスの検証や、レゾナンスの加算等に使用。
    async fn on_post_execute(
        &self,
        request: &crate::llm::LlmRequest,
        response: &crate::llm::LlmResponse,
    ) -> Result<(), crate::error::AiomeError>;

    /// ジョブ完了時に実行されるフック。
    /// 報酬付与やエスクロー解除等の事後処理に使用される。
    async fn on_job_completed(
        &self,
        _job_id: &str,
        _status: &str,
    ) -> Result<(), crate::error::AiomeError> {
        Ok(())
    }

    /// ユーザーへの実行許可要求 (Ask) 発生時に呼び出されるフック
    async fn on_permission_request(
        &self,
        _tool: &str,
        _reason: &str,
    ) -> Result<bool, crate::error::AiomeError> {
        Ok(true) // デフォルト: 許可
    }

    /// セッション開始時のフック
    async fn on_session_start(&self) -> Result<(), crate::error::AiomeError> {
        Ok(())
    }

    /// 停止時のフック
    async fn on_stop(&self, _reason: &str) -> Result<(), crate::error::AiomeError> {
        Ok(())
    }

    /// 形式検証（OxiLean等）完了時に実行されるフック。
    /// 証明力（Proof Power）のNurture側（KarmaForge等）への伝搬に使用される。
    async fn on_proof_completed(
        &self,
        _skill_name: &str,
        _is_valid: bool,
    ) -> Result<(), crate::error::AiomeError> {
        Ok(())
    }

    /// 決済イベント完了時に実行されるフック。
    /// 決済データから経済的貢献を評価し、KarmaForge等へ伝搬するために使用される。
    async fn on_transaction_completed(
        &self,
        _source: &str,
        _amount_cents: i64,
        _actor_id: &str,
        _transaction_id: &str,
    ) -> Result<(), crate::error::AiomeError> {
        Ok(())
    }
}
