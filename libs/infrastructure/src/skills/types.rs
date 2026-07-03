/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! スキル関連の型定義（TypeState パターン / メタデータ / 成熟度）。

use super::WasmSkillManager;
use contracts::requires;

#[derive(Debug, Clone)]
pub struct UnverifiedSkill {
    /// name
    pub name: String,
    /// input_test_payload
    pub input_test_payload: String,
}

/// 状態: 確定的検証をパスした安全なSkill (TypeState Pattern)
#[derive(Debug, Clone)]
pub struct VerifiedSkill {
    name: String,
}

impl VerifiedSkill {
    /// Internal constructor for the infrastructure crate to promote unverified skills.
    /// This ensures mathematical safety of the TypeState pattern.
    pub(crate) fn promote_internal(name: String) -> Self {
        Self { name }
    }

    /// TEST ONLY: Create a verified skill without dry-run.
    /// This is used for integration tests.
    pub fn new_for_test<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }

    /// `name` を実行する
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl UnverifiedSkill {
    /// 契約プログラミングにより、検証を通過したものだけが型を昇格できる
    #[requires(self.input_test_payload.len() < 50_000, "Payload limits exceeded")]
    // #[ensures] is removed here because verification failure (Err) is a valid, expected state machine outcome for malicious skills.
    pub async fn verify(
        self,
        manager: &WasmSkillManager,
    ) -> Result<VerifiedSkill, Box<dyn std::error::Error + Send + Sync>> {
        let is_safe = manager
            .dry_run_skill(&self.name, &self.input_test_payload)
            .await?;
        if is_safe {
            Ok(VerifiedSkill::promote_internal(self.name))
        } else {
            Err(format!(
                "Skill {} failed the deterministic dry-run quarantine",
                self.name
            )
            .into())
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// `SkillMetadata` 構造体
pub struct SkillMetadata {
    /// name
    pub name: String,
    /// description
    pub description: String,
    /// capabilities
    pub capabilities: Vec<String>,
    /// inputs
    pub inputs: Vec<String>,
    /// outputs
    pub outputs: Vec<String>,
    #[serde(default)]
    /// allowed_hosts
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    /// permissions
    pub permissions: crate::security::PermissionManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkillMaturity {
    Quarantined, // dry-run未通過
    Probation,   // 通過済みだが実績 < 5
    Trusted,     // 実績 >= 5 & 成功率 > 80%
    Veteran,     // 実績 >= 50 & 成功率 > 95%
}

impl std::fmt::Display for SkillMaturity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillMaturity::Quarantined => write!(f, "Quarantined"),
            SkillMaturity::Probation => write!(f, "Probation"),
            SkillMaturity::Trusted => write!(f, "Trusted"),
            SkillMaturity::Veteran => write!(f, "Veteran"),
        }
    }
}
