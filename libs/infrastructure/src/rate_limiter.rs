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
    /// 1分あたりの許可リクエスト数を指定して作成
    pub fn new(requests_per_minute: u32) -> Self {
        let quota =
            Quota::per_minute(NonZeroU32::new(requests_per_minute).expect("Limit must be > 0"));
        Self {
            limiter: Arc::new(GovRateLimiter::dashmap(quota)),
        }
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
        let limiter = AgentRateLimiter::new(2);
        let agent_id = Uuid::new_v4();

        // 1回目 OK
        assert!(limiter.check(agent_id).is_ok());
        // 2回目 OK
        assert!(limiter.check(agent_id).is_ok());
        // 3回目 NG (限度 2/min)
        assert!(limiter.check(agent_id).is_err());

        // 別エージェントは OK
        assert!(limiter.check(Uuid::new_v4()).is_ok());
    }
}
