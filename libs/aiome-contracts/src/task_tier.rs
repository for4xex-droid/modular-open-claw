/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

/// LLM タスクの推論強度ティア
///
/// 将来的にタスクディスパッチャー（Task Dispatcher）によって、タスクの難易度やリアルタイム性の要件に応じて、
/// 自動的に最適な推論ティア（ローカルモデル優先の `Fast`、クラウド等の高機能モデル優先の `Smart`）を
/// 選択・ディスパッチするための拡張ポイントとして設計されています。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskTier {
    /// 低推論: 分類、ルーティング、要約、テンプレート埋め
    /// ローカル LLM 優先
    Fast,
    /// 高推論: 複雑な推論、品質判定、長文生成
    /// クラウド LLM 優先
    Smart,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_tier_serialization() {
        let fast = TaskTier::Fast;
        let serialized = serde_json::to_string(&fast).unwrap();
        assert_eq!(serialized, "\"Fast\"");

        let smart = TaskTier::Smart;
        let serialized = serde_json::to_string(&smart).unwrap();
        assert_eq!(serialized, "\"Smart\"");
    }

    #[test]
    fn test_task_tier_deserialization() {
        let deserialized: TaskTier = serde_json::from_str("\"Fast\"").unwrap();
        assert_eq!(deserialized, TaskTier::Fast);

        let deserialized: TaskTier = serde_json::from_str("\"Smart\"").unwrap();
        assert_eq!(deserialized, TaskTier::Smart);
    }
}
