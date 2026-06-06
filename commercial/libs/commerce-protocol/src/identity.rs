/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub Uuid);

impl From<Uuid> for ActorId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorKind {
    AiAgent,
    HumanOwner,
    Creator,
    Merchant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicActor {
    pub id: ActorId,
    pub kind: ActorKind,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}
