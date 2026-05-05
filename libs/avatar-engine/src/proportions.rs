/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Proportion validation errors
#[derive(Debug, Error)]
pub enum ProportionError {
    #[error(
        "Avatar is too young (proportion rule violation): {0} heads. Minimum 5.5 heads required."
    )]
    TooYoung(f32),
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),
}

/// Avatar dimension metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarDimensions {
    pub total_height_meters: f32,
    pub head_height_meters: f32,
    pub is_humanoid: bool,
}

/// [G-22] ProportionsChecker
/// Enforce maturity standards for avatars to comply with Project NURTURE anti-CSAM protocols.
pub struct ProportionsChecker;

impl ProportionsChecker {
    /// Validate avatar proportions based on head-to-body ratio.
    pub fn validate(dimensions: &AvatarDimensions) -> Result<(), ProportionError> {
        if !dimensions.is_humanoid {
            // Non-humanoid (pets, robots, etc.) have different rules.
            return Ok(());
        }

        if dimensions.head_height_meters <= 0.0 {
            return Err(ProportionError::InvalidMetadata(
                "Head height must be positive".into(),
            ));
        }

        let ratio = dimensions.total_height_meters / dimensions.head_height_meters;

        // Project NURTURE Rule: Minimum 5.5 heads for humanoid avatars.
        if ratio < 5.5 {
            return Err(ProportionError::TooYoung(ratio));
        }

        Ok(())
    }

    /// [G-22] Extract dimensions directly from binary data (VRM/GLB).
    /// This prevents metadata spoofing.
    pub fn extract_from_binary(data: &[u8]) -> Result<AvatarDimensions, ProportionError> {
        let g = gltf::Gltf::from_slice(data)
            .map_err(|e| ProportionError::InvalidMetadata(e.to_string()))?;

        let doc_json = g.document.into_json();
        let extensions = doc_json.extensions.clone().unwrap_or_default();
        let ext_value = serde_json::to_value(&extensions).unwrap_or(serde_json::Value::Null);

        let head_idx = Self::get_human_bone_node(&ext_value, "head");
        let neck_idx = Self::get_human_bone_node(&ext_value, "neck");

        let Some(head_idx) = head_idx else {
            return Err(ProportionError::InvalidMetadata(
                "Not a valid humanoid VRM (missing head bone)".into(),
            ));
        };

        let Some(neck_idx) = neck_idx else {
            return Err(ProportionError::InvalidMetadata(
                "Not a valid humanoid VRM (missing neck bone)".into(),
            ));
        };

        let absolute_y = Self::compute_absolute_y_positions(&doc_json.nodes, &doc_json.scenes);

        let head_y = absolute_y.get(head_idx).copied().unwrap_or(0.0);
        let neck_y = absolute_y.get(neck_idx).copied().unwrap_or(0.0);

        let head_height_meters = (head_y - neck_y).abs() * 2.0;
        if head_height_meters <= 0.0 {
            return Err(ProportionError::InvalidMetadata(
                "Invalid bone translations".into(),
            ));
        }

        // Simplification for TDD: Total height is roughly head_y + (head_size / 2)
        // because the avatar stands at Y=0.
        let total_height_meters = head_y + (head_height_meters / 2.0);

        Ok(AvatarDimensions {
            total_height_meters,
            head_height_meters,
            is_humanoid: true,
        })
    }

    fn get_human_bone_node(ext: &serde_json::Value, bone_name: &str) -> Option<usize> {
        // VRM 1.0 (VRMC_vrm)
        if let Some(vrmc) = ext.get("VRMC_vrm") {
            if let Some(node) = vrmc.pointer(&format!("/humanoid/humanBones/{}/node", bone_name)) {
                return node.as_u64().map(|n| n as usize);
            }
        }
        // VRM 0.0 (VRM)
        if let Some(vrm) = ext.get("VRM") {
            if let Some(bones) = vrm
                .pointer("/humanoid/humanBones")
                .and_then(|b| b.as_array())
            {
                for b in bones {
                    if b.get("bone").and_then(|s| s.as_str()) == Some(bone_name) {
                        return b.get("node").and_then(|n| n.as_u64()).map(|n| n as usize);
                    }
                }
            }
        }
        None
    }

    fn compute_absolute_y_positions(
        nodes: &[gltf::json::Node],
        scenes: &[gltf::json::Scene],
    ) -> Vec<f32> {
        let mut abs_y = vec![0.0; nodes.len()];
        let mut roots = vec![];

        if let Some(scene) = scenes.first() {
            for root_idx in &scene.nodes {
                roots.push(root_idx.value());
            }
        } else if !nodes.is_empty() {
            roots.push(0);
        }

        for root in roots {
            Self::traverse_y(root, 0.0, nodes, &mut abs_y);
        }

        abs_y
    }

    fn traverse_y(
        node_idx: usize,
        parent_y: f32,
        nodes: &[gltf::json::Node],
        abs_y: &mut Vec<f32>,
    ) {
        if node_idx >= nodes.len() {
            return;
        }

        let node = &nodes[node_idx];
        let local_y = if let Some(tr) = node.translation {
            tr[1]
        } else {
            0.0
        };

        let current_y = parent_y + local_y;
        abs_y[node_idx] = current_y;

        if let Some(ref children) = node.children {
            for child in children {
                Self::traverse_y(child.value(), current_y, nodes, abs_y);
            }
        }
    }
}

impl From<ProportionError> for aiome_core_contracts::error::AiomeError {
    fn from(err: ProportionError) -> Self {
        match err {
            ProportionError::TooYoung(_) => Self::SecurityViolation {
                reason: err.to_string(),
            },
            ProportionError::InvalidMetadata(_) => Self::Validation {
                reason: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_adult_avatar() {
        let dim = AvatarDimensions {
            total_height_meters: 1.8,
            head_height_meters: 0.22, // ~8.1 heads
            is_humanoid: true,
        };
        assert!(ProportionsChecker::validate(&dim).is_ok());
    }

    #[test]
    fn test_valid_borderline_avatar() {
        let dim = AvatarDimensions {
            total_height_meters: 1.1,
            head_height_meters: 0.2, // 5.5 heads exactly
            is_humanoid: true,
        };
        assert!(ProportionsChecker::validate(&dim).is_ok());
    }

    #[test]
    fn test_invalid_child_avatar() {
        let dim = AvatarDimensions {
            total_height_meters: 0.8,
            head_height_meters: 0.2, // 4.0 heads (too child-like)
            is_humanoid: true,
        };
        let res = ProportionsChecker::validate(&dim);
        assert!(res.is_err());
        if let Err(ProportionError::TooYoung(r)) = res {
            assert_eq!(r, 4.0);
        } else {
            panic!("Expected TooYoung error");
        }
    }

    #[test]
    fn test_extract_from_binary_adult_vrm() {
        let data = std::fs::read("tests/fixtures/adult.vrm").unwrap();

        let res = ProportionsChecker::extract_from_binary(&data);
        assert!(
            res.is_ok(),
            "Adult VRM should be parsed successfully. Err: {:?}",
            res.err()
        );
        let dim = res.unwrap();
        assert!(dim.is_humanoid);

        let ratio = dim.total_height_meters / dim.head_height_meters;
        assert!(ratio >= 5.5, "Ratio {} should be >= 5.5", ratio);
        assert!(ProportionsChecker::validate(&dim).is_ok());
    }

    #[test]
    fn test_extract_from_binary_child_vrm() {
        let data = std::fs::read("tests/fixtures/child.vrm").unwrap();
        let res = ProportionsChecker::extract_from_binary(&data);
        assert!(res.is_ok(), "Child VRM should be parsed successfully");
        let dim = res.unwrap();

        let res_val = ProportionsChecker::validate(&dim);
        assert!(res_val.is_err(), "Child VRM should fail validation");
        if let Err(ProportionError::TooYoung(r)) = res_val {
            assert!(r < 5.5);
        } else {
            panic!("Expected TooYoung error");
        }
    }

    #[test]
    fn test_proportion_to_aiome_error() {
        use aiome_core_contracts::error::AiomeError;

        let err = ProportionError::TooYoung(4.0);
        let aiome_err: AiomeError = err.into();
        assert!(matches!(aiome_err, AiomeError::SecurityViolation { .. }));

        let err = ProportionError::InvalidMetadata("test".into());
        let aiome_err: AiomeError = err.into();
        assert!(matches!(aiome_err, AiomeError::Validation { .. }));
    }
}
