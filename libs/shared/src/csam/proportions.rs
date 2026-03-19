/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum LegalStatus {
    /// 全年齢対象 (5.5頭身以上)
    General,
    /// 児童体型判定 (5.5頭身未満) -> 性的表現等の禁止
    Restricted,
    /// 未審査
    Pending,
}

/// 頭身判定器 (NURTURE §5: 5.5頭身ルール)
pub struct ProportionsChecker;

impl ProportionsChecker {
    /// 頭身を計算し、合否を判定する
    /// VRM/Inochi2D のメタデータやボーン情報を入力とする
    pub fn verify_proportions(head_height: f32, total_height: f32) -> LegalStatus {
        if head_height <= 0.0 || total_height <= 0.0 {
            return LegalStatus::Pending;
        }

        let ratio = total_height / head_height;

        // NURTURE §5: 5.5頭身未満は児童判定
        if ratio < 5.5 {
            LegalStatus::Restricted
        } else {
            LegalStatus::General
        }
    }

    /// VRM JSON データからの簡易抽出 (Mock)
    pub fn check_vrm_data(json_data: &str) -> LegalStatus {
        // TODO: 実際の VRM 解析 (gltf-rs 等) を使用して正確なボーン位置を特定
        // 現時点では JSON 内の特定の拡張フィールドを想定
        if json_data.contains("chibi") || json_data.contains("head_ratio_low") {
            LegalStatus::Restricted
        } else {
            LegalStatus::General
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_5_5_rule() {
        // 正常な大人体型 (7頭身)
        assert_eq!(
            ProportionsChecker::verify_proportions(0.25, 1.75),
            LegalStatus::General
        );

        // 児童体型 (4頭身)
        assert_eq!(
            ProportionsChecker::verify_proportions(0.25, 1.0),
            LegalStatus::Restricted
        );

        // 境界値 (5.5頭身)
        assert_eq!(
            ProportionsChecker::verify_proportions(0.2, 1.1),
            LegalStatus::General
        );

        // 5.4頭身
        assert_eq!(
            ProportionsChecker::verify_proportions(0.2, 1.08),
            LegalStatus::Restricted
        );
    }
}
