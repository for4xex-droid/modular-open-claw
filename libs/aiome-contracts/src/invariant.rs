/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

/// 境界で検証される不変条件のスコープ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvariantScope {
    /// 事前条件（アクション実行前）
    PreCondition,
    /// 事後条件（アクション実行後）
    PostCondition,
}

/// 境界トートロジー検証で使用される不変条件の型定義
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invariant {
    /// 不変条件の一意識別名 (例: "path_in_sandbox")
    pub name: String,
    /// 人間可読な説明
    pub description: String,
    /// 検証のタイミング
    pub scope: InvariantScope,
}

/// 検証結果の判定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationVerdict {
    /// 検証通過
    Pass,
    /// 検証棄却（違反）
    Reject {
        /// 違反した不変条件の名前
        invariant_name: String,
        /// 期待される状態
        expected: String,
        /// 実際の状態
        actual: String,
    },
}

/// 不変条件付き状態遷移ノード (Phase 48 で使用)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvariantDagNode {
    /// SHA-256 ハッシュ (parent_hash + action + parameters + verified_invariants)
    pub hash: String,
    /// 親ノードのハッシュ ("genesis" は "0")
    pub parent_hash: String,
    /// 対応するステップ ID
    pub step_id: u32,
    /// 対応するジョブ ID
    pub job_id: String,
    /// 実行されたアクション
    pub action: String,
    /// Phase 47 で通過した不変条件名リスト
    pub verified_invariants: Vec<String>,
    /// ISO-8601 タイムスタンプ
    pub timestamp: String,
}
