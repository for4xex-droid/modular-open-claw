/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentModel {
    pub interaction_count: u64,
    pub style: AttachmentStyle,
    pub master_preference: Option<Vec<f32>>,
    pub separation_anxiety: f64,
}

impl Default for AttachmentModel {
    fn default() -> Self {
        Self {
            interaction_count: 0,
            style: AttachmentStyle::Secure,
            master_preference: None,
            separation_anxiety: 0.1,
        }
    }
}

impl AttachmentModel {
    pub fn update_from_experience(&mut self, valence: f64) {
        self.interaction_count += 1;

        // Positive experiences reduce anxiety, negative ones increase it
        let adjustment = if valence > 0.0 { -0.1 } else { 0.2 };
        self.separation_anxiety = (self.separation_anxiety + adjustment).clamp(0.0, 1.0);

        self.reassess_style();
    }

    pub fn reassess_style(&mut self) {
        // Secure is the base, but high anxiety pushes towards Anxious or Disorganized
        if self.interaction_count < 20 {
            // Genesis phase: mostly secure or hesitant
            if self.separation_anxiety > 0.8 {
                self.style = AttachmentStyle::Anxious;
            } else {
                self.style = AttachmentStyle::Secure;
            }
        } else {
            // Mature phase: more complex transitions
            if self.separation_anxiety > 0.85 {
                self.style = AttachmentStyle::Disorganized;
            } else if self.separation_anxiety > 0.6 {
                self.style = AttachmentStyle::Anxious;
            } else if self.separation_anxiety < 0.2 {
                self.style = AttachmentStyle::Avoidant;
            } else {
                self.style = AttachmentStyle::Secure;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttachmentStyle {
    Secure,
    Anxious,
    Avoidant,
    Disorganized,
}
