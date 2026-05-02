/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod asset_manifest;
pub mod lip_sync;
pub mod loader;
pub mod physics;
pub mod proportions;
pub mod resampler;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Inochi2D (Inox2D) / VRM expression parameters bridging structure.
/// Transmitted over SSE or stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AvatarParameters {
    pub angle_x: f32,
    pub angle_y: f32,
    pub eye_open_l: f32,
    pub eye_open_r: f32,
    pub mouth_open: f32,
    pub eyebrow_y: f32,
    pub body_angle_z: f32,
    pub physics_intensity: f32,
}

/// Maps broad string emotions like "excited" to fine-grained float parameters
pub struct EmotionToParameterMapper {
    mappings: HashMap<String, AvatarParameters>,
}

impl Default for EmotionToParameterMapper {
    fn default() -> Self {
        let mut mappings = HashMap::new();
        mappings.insert(
            "excited".to_string(),
            AvatarParameters {
                eye_open_l: 1.0,
                eye_open_r: 1.0,
                mouth_open: 0.7,
                eyebrow_y: 0.3,
                physics_intensity: 1.5,
                ..Default::default()
            },
        );
        mappings.insert(
            "curious".to_string(),
            AvatarParameters {
                eye_open_l: 0.9,
                eye_open_r: 0.9,
                mouth_open: 0.3,
                eyebrow_y: 0.5,
                angle_x: 15.0,
                physics_intensity: 1.0,
                ..Default::default()
            },
        );
        mappings.insert(
            "sad".to_string(),
            AvatarParameters {
                eye_open_l: 0.4,
                eye_open_r: 0.4,
                mouth_open: 0.1,
                eyebrow_y: -0.5,
                angle_x: -5.0,
                physics_intensity: 0.5,
                ..Default::default()
            },
        );
        mappings.insert(
            "angry".to_string(),
            AvatarParameters {
                eye_open_l: 0.7,
                eye_open_r: 0.7,
                mouth_open: 0.5,
                eyebrow_y: -0.8,
                physics_intensity: 2.0,
                ..Default::default()
            },
        );
        mappings.insert(
            "reflective".to_string(),
            AvatarParameters {
                eye_open_l: 0.6,
                eye_open_r: 0.6,
                mouth_open: 0.0,
                eyebrow_y: 0.0,
                angle_x: 5.0,
                physics_intensity: 0.3,
                ..Default::default()
            },
        );

        Self { mappings }
    }
}

impl EmotionToParameterMapper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn map_emotion(&self, emotion: &str) -> AvatarParameters {
        self.mappings.get(emotion).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_mapping_excited() {
        let mapper = EmotionToParameterMapper::new();
        let params = mapper.map_emotion("excited");
        assert_eq!(params.eye_open_l, 1.0);
        assert_eq!(params.eye_open_r, 1.0);
        assert_eq!(params.mouth_open, 0.7);
        assert_eq!(params.physics_intensity, 1.5);
    }

    #[test]
    fn test_emotion_mapping_unknown() {
        let mapper = EmotionToParameterMapper::new();
        let params = mapper.map_emotion("unknown_emotion");
        // Should return default AvatarParameters (all 0.0)
        assert_eq!(params.eye_open_l, 0.0);
        assert_eq!(params.mouth_open, 0.0);
        assert_eq!(params.physics_intensity, 0.0);
    }

    #[test]
    fn test_emotion_mapping_curious() {
        let mapper = EmotionToParameterMapper::new();
        let params = mapper.map_emotion("curious");
        assert_eq!(params.angle_x, 15.0);
        assert_eq!(params.eyebrow_y, 0.5);
    }
}
