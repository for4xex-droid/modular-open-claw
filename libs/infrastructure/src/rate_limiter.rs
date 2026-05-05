/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use governor::clock::DefaultClock;
use governor::{
    state::keyed::DashMapStateStore, state::InMemoryState, Quota, RateLimiter as GovRateLimiter,
};
use std::num::NonZeroU32;
use uuid::Uuid;

/// エージェント別のレート制限を管理する (G-2)
#[derive(Clone)]
pub struct AgentRateLimiter {
    limiter: Arc<GovRateLimiter<Uuid, DashMapStateStore<Uuid>, DefaultClock>>,
}

use std::sync::Arc;

impl AgentRateLimiter {
    pub fn new(requests_per_minute: u32) -> Result<Self, aiome_core::error::AiomeError> {
        let nz = NonZeroU32::new(requests_per_minute).ok_or_else(|| {
            aiome_core::error::AiomeError::Infrastructure {
                reason: "Rate limit must be > 0".to_string(),
            }
        })?;
        let quota = Quota::per_minute(nz);
        Ok(Self {
            limiter: Arc::new(GovRateLimiter::dashmap(quota)),
        })
    }

    /// リクエストを試行し、許可されたかどうかを返す
    pub fn check(&self, agent_id: Uuid) -> Result<(), &'static str> {
        self.limiter
            .check_key(&agent_id)
            .map(|_| ())
            .map_err(|_| "Rate limit exceeded for agent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_agent_rate_limiting() {
        let limiter = AgentRateLimiter::new(2).expect("Constant 2 is valid"); // allow-anti-pattern
        let agent_id = Uuid::new_v4();

        // 1回目 OK
        assert!(limiter.check(agent_id).is_ok());
        // 2回目 OK
        assert!(limiter.check(agent_id).is_ok());
        // 3回目 NG (限度 2/min)
        assert!(limiter.check(agent_id).is_err());

        assert!(limiter.check(Uuid::new_v4()).is_ok());
    }

    #[test]
    fn test_agent_rate_limiting_zero_quota() {
        let result = AgentRateLimiter::new(0);
        assert!(result.is_err());
    }
}
