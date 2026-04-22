/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use async_trait::async_trait;

/// 形式検証ゲートのインターフェース
///
/// Phase 2以降で、WasmSkillManager等が証明の実行や結果検証を
/// 行うための抽象境界。Shadow-workerのProofVerifier呼び出し等はこの実装でラップされる。
#[async_trait]
pub trait FormalProofGate: Send + Sync {
    /// 与えられたスキル/WASMの形式検証を実行し、結果を返す
    async fn verify_skill(
        &self,
        skill_name: &str,
        proof_spec_b64: &str,
    ) -> Result<bool, AiomeError>;
}
