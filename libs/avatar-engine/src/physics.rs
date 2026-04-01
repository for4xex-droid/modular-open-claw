/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    pub mass: f32,
    pub spring_k: f32,
    pub damping: f32,
}

pub struct PhysicsSimulator {
    config: PhysicsConfig,
    current_value: f32,
    velocity: f32,
}

impl PhysicsSimulator {
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            config,
            current_value: 0.0,
            velocity: 0.0,
        }
    }

    /// 物理演算の 1 ステップを実行
    pub fn step(&mut self, target: f32, dt: f32) -> f32 {
        let force = -self.config.spring_k * (self.current_value - target);
        let acceleration = force / self.config.mass;

        self.velocity += acceleration * dt;
        self.velocity *= 1.0 - (self.config.damping * dt);
        self.current_value += self.velocity * dt;

        self.current_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_oscillation() {
        let config = PhysicsConfig {
            mass: 1.0,
            spring_k: 10.0,
            damping: 0.5,
        };
        let mut sim = PhysicsSimulator::new(config);

        let start = sim.current_value;
        sim.step(1.0, 0.1);
        let next = sim.current_value;

        // Target が 1.0 なので、値が増加するはず
        assert!(next > start);
    }
}
