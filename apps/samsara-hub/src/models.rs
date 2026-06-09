/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct FederatedKarmaRecord {
    pub id: String,
    pub karma_type: String,
    pub related_skill: String,
    pub lesson: String,
    pub weight: i64,
    pub soul_version_hash: Option<String>,
    pub lamport_clock: i64,
    pub node_id: String,
    pub signature: Option<String>,
    pub created_at: String,
    pub clone_origin_id: Option<String>,
    pub generation: Option<i64>,
    pub somatic_valence: Option<f64>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct ImmuneRuleRecord {
    pub id: String,
    pub pattern: String,
    pub severity: i64,
    pub action: String,
    pub lamport_clock: i64,
    pub node_id: String,
    pub signature: Option<String>,
    pub created_at: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct ArenaMatchRecord {
    pub id: String,
    pub skill_a: String,
    pub skill_b: String,
    pub topic: String,
    pub output_a: Option<String>,
    pub output_b: Option<String>,
    pub winner: Option<String>,
    pub reasoning: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct TopicRecord {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub summary: Option<String>,
    pub turn_count: i32,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub summary: Option<String>,
}

#[derive(Deserialize)]
pub struct CommuneWsQuery {
    pub node_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct TimelineSyncRequest {
    pub hub_id: String,
    pub automerge_blob: Vec<u8>,
}
