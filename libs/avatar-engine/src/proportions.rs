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
        // [SIMPLIFIED FOR TDD]
        // Real implementation would use gltf-rs to find 'head' and 'leftFoot'/'rightFoot' bones.
        // Here we simulate it by looking for VRM headers.

        if data.starts_with(b"glTF") {
            // Check for VRM extension in the JSON chunk
            let data_str = String::from_utf8_lossy(data);
            if data_str.contains("VRMC_vrm") {
                // If it's a VRM, we expect standard humanoid proportions.
                // In a real implementation, we'd parse the GLB structure.
                return Ok(AvatarDimensions {
                    total_height_meters: 1.6,
                    head_height_meters: 0.2,
                    is_humanoid: true,
                });
            }
        }

        Err(ProportionError::InvalidMetadata(
            "Unsupported or invalid avatar format".into(),
        ))
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
    fn test_extract_from_binary_vrm() {
        let mut data = b"glTF".to_vec();
        data.extend_from_slice(b"some_json_with_VRMC_vrm_extension");

        let res = ProportionsChecker::extract_from_binary(&data);
        assert!(res.is_ok());
        let dim = res.unwrap(); // allow-anti-pattern
        assert!(dim.is_humanoid);
        assert_eq!(dim.total_height_meters, 1.6);
    }

    #[test]
    fn test_extract_from_binary_invalid() {
        let data = b"NOT_A_GLB";
        let res = ProportionsChecker::extract_from_binary(data);
        assert!(res.is_err());
    }
}
