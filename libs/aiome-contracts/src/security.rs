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

    /// 形式検証（OxiLean等）完了時に実行されるフック。
    /// 証明力（Proof Power）のNurture側（KarmaForge等）への伝搬に使用される。
    async fn on_proof_completed(
        &self,
        _skill_name: &str,
        _is_valid: bool,
    ) -> Result<(), crate::error::AiomeError> {
        Ok(())
    }
}
