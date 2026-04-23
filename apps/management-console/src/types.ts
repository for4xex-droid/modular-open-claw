/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';

export interface AgentStats {
    level: number;
    exp: number;
    resonance: number;
    creativity: number;
    fatigue: number;
}

export interface SidecarHealth {
    name: string;
    status: string;
}


export interface VitalityUIEvent {
    id: number;
    title: string;
    desc: string;
    color: string;
    icon: React.ReactNode;
}

export interface VitalityRawEvent {
    type: 'level_up' | 'karma_update' | 'inspiration' | 'job_started' | 'job_completed' | 'tts_started' | 'tts_completed' | 'skill_loaded' | 'skill_ready' | 'immune_alert' | 'skill_execution' | 'plugin_event' | 'proactive_talk' | 'token_saved' | 'task_awaiting_input' | 'quality_gate';
    data: unknown;
}

export interface SystemBalance {
    id: string;
    current_health: number;
    max_health: number;
    status: string;
}

export interface GraphNode {
    id: string;
    label: string;
    group: string;
}

export interface GraphEdge {
    from: string;
    to: string;
}

export interface ImmuneRule {
    id: string;
    pattern: string;
    severity: number;
    action: string;
    created_at: string;
    approval_status: 'Pending' | 'Approved' | 'Rejected' | 'Quarantined';
    risk?: string;
    active?: boolean;
}

export interface Karma {
    id: string;
    job_id: string;
    node_id: string;
    karma_type: string;
    lesson: string;
    weight: number;
    created_at: string;
}

export interface ChatMessage {
    role: 'user' | 'assistant' | 'system';
    content: string;
    isError?: boolean;
    a2uiEnvelope?: A2uiEnvelope;
}

export interface TreasureItem {
    id: string;
    title: string;
    description: string;
    url: string;
    price_coins?: number;
    category: string;
    score: number;
    disclosure_label: string;
}

export interface TreasureFeedback {
    item_id: string;
    action: string;
    metadata?: Record<string, unknown>;
}

export interface TrajectoryStep {
    step_id: number;
    job_id?: string;
    action: string;
    tool_name?: string;
    input: unknown;
    output: unknown;
    timestamp: string;
    reasoning?: string;
    parent_step_id?: string;
    step_category: string;
    completion_criteria?: string;
}

export interface AgentDiagnosis {
    critical_failure_step: number;
    category: string;
    root_cause: string;
    self_repair_hint: string;
    diagnosed_at: string;
}

export type SoTTrigger = "Manual" | { HighBudgetGig: { threshold: number } } | "SelfEvolution" | "ConstitutionalEscalation";

export type SoTOutcome = "AllCriteriaPassed" | "MaxRoundsReached" | "BudgetExhausted" | "Timeout" | "ConvergedEarly" | "SpectralDivergence" | { Error: string };

export type SoTEventPayload =
    | { type: "SessionStart"; data: { session_id: string; config: unknown; trigger: SoTTrigger } }
    | { type: "RoleStart"; data: { session_id: string; role: string; round: number } }
    | { type: "RoleOutput"; data: { session_id: string; role: string; round: number; content: string; token_count: number } }
    | { type: "Score"; data: { session_id: string; round: number; scores: [string, number][]; all_passed: boolean } }
    | { type: "ThinkerAbstained"; data: { session_id: string; thinker_index: number; round: number } }
    | { type: "ProtocolSelected"; data: { session_id: string; protocol: string; reason: string } }
    | { type: "SessionEnd"; data: { session_id: string; outcome: SoTOutcome; total_tokens: number } };

export interface SoTEvent {
    event: SoTEventPayload;
}

// --- A2UI Types ---
// These MUST match the Rust serde output from infrastructure::a2ui::schema exactly.
// Rust struct Surface { id, version, source, components }
// Rust enum A2uiEnvelope uses #[serde(tag = "type")] with camelCase renames.
export interface A2uiComponent {
    type: string;
    props: Record<string, unknown>;
    children: A2uiComponent[];
}

export interface A2uiSurface {
    id: string;
    version: string;
    source: string;
    components: A2uiComponent[];
}

export type A2uiEnvelope =
    | { type: "createSurface"; surface: A2uiSurface }
    | { type: "updateComponents"; surfaceId: string; components: A2uiComponent[] }
    | { type: "deleteSurface"; surfaceId: string };
