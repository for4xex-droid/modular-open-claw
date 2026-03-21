/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

pub mod asset_manifest;
pub mod lip_sync;
pub mod loader;
pub mod physics;

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

        let mut excited = AvatarParameters::default();
        excited.eye_open_l = 1.0;
        excited.eye_open_r = 1.0;
        excited.mouth_open = 0.7;
        excited.eyebrow_y = 0.3;
        excited.physics_intensity = 1.5;
        mappings.insert("excited".to_string(), excited);

        let mut curious = AvatarParameters::default();
        curious.eye_open_l = 0.9;
        curious.eye_open_r = 0.9;
        curious.mouth_open = 0.3;
        curious.eyebrow_y = 0.5;
        curious.angle_x = 15.0; // tilt
        curious.physics_intensity = 1.0;
        mappings.insert("curious".to_string(), curious);

        let mut sad = AvatarParameters::default();
        sad.eye_open_l = 0.4;
        sad.eye_open_r = 0.4;
        sad.mouth_open = 0.1;
        sad.eyebrow_y = -0.5;
        sad.angle_x = -5.0;
        sad.physics_intensity = 0.5;
        mappings.insert("sad".to_string(), sad);

        let mut angry = AvatarParameters::default();
        angry.eye_open_l = 0.7;
        angry.eye_open_r = 0.7;
        angry.mouth_open = 0.5;
        angry.eyebrow_y = -0.8;
        angry.physics_intensity = 2.0;
        mappings.insert("angry".to_string(), angry);

        let mut reflective = AvatarParameters::default();
        reflective.eye_open_l = 0.6;
        reflective.eye_open_r = 0.6;
        reflective.mouth_open = 0.0;
        reflective.eyebrow_y = 0.0;
        reflective.angle_x = 5.0;
        reflective.physics_intensity = 0.3;
        mappings.insert("reflective".to_string(), reflective);

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
