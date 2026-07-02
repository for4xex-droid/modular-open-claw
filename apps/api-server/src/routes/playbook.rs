/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! Agent Playbooks (F-1): 公式業務テンプレートの同梱レジストリと
//! list / install / import API。
//! 公式 Playbook は `include_str!` によるバイナリ同梱であり、
//! `~/.aiome` への書き出しは行わない。

use infrastructure::workflow::playbook::PlaybookManifest;
use tracing::warn;

/// バイナリ同梱の公式 Playbook（id, JSON 本文）
static BUNDLED_PLAYBOOKS: &[(&str, &str)] = &[
    (
        "seo-operations",
        include_str!("../../assets/playbooks/seo-operations.json"),
    ),
    (
        "sns-operations",
        include_str!("../../assets/playbooks/sns-operations.json"),
    ),
    (
        "competitor-research",
        include_str!("../../assets/playbooks/competitor-research.json"),
    ),
    (
        "support-triage",
        include_str!("../../assets/playbooks/support-triage.json"),
    ),
];

/// 同梱 Playbook をすべてパースして返す。
/// パースに失敗したアセットは warn ログを出して除外する（パニックしない）。
pub(crate) fn load_bundled() -> Vec<PlaybookManifest> {
    BUNDLED_PLAYBOOKS
        .iter()
        .filter_map(|(id, raw)| match serde_json::from_str::<PlaybookManifest>(raw) {
            Ok(manifest) => Some(manifest),
            Err(e) => {
                warn!(
                    "⚠️ [Playbooks] Bundled playbook asset {:?} failed to parse and was excluded: {}",
                    id, e
                );
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use infrastructure::workflow::schema::{NodeType, TriggerType};

    /// アセット品質ゲート: 同梱4本すべてがパース・構造検証を通過し、
    /// 全 Start ノードが Manual トリガーであること。
    #[test]
    fn test_bundled_playbooks_all_parse_and_validate() {
        let playbooks = load_bundled();
        assert_eq!(
            playbooks.len(),
            BUNDLED_PLAYBOOKS.len(),
            "all bundled playbook assets must parse"
        );

        for pb in &playbooks {
            pb.validate_structure().unwrap_or_else(|e| {
                panic!("bundled playbook {:?} failed validation: {}", pb.id, e)
            });
            assert!(
                pb.required_skills.is_empty() && pb.required_mcp_servers.is_empty(),
                "official playbooks must run without external dependencies: {:?}",
                pb.id
            );
            for wf in &pb.workflows {
                for node in &wf.nodes {
                    if let NodeType::Start { trigger } = &node.node_type {
                        assert_eq!(
                            *trigger,
                            TriggerType::Manual,
                            "playbook {:?} workflow {:?} must use Manual trigger",
                            pb.id,
                            wf.name
                        );
                    }
                }
            }
        }
    }
}
