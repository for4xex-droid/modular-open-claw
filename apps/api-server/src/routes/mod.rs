/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub mod a2ui;
pub mod agent;
pub mod artifacts;
pub mod audit;
pub mod auth;
pub mod avatar;
pub mod biome;
pub mod blueprint;
pub mod bootstrap;
pub mod buzz;
pub mod commerce;
pub mod commerce_webhook;
pub mod cortex;
#[cfg(any(debug_assertions, feature = "demo"))]
pub mod demo;
pub mod ekyc;
pub mod expression;
pub mod forecast;
pub mod general;
pub mod gift;
pub mod gig;
pub mod inochi2d;
pub mod jobs;
pub mod karma;
pub mod lora;
pub mod lora_market;
pub mod model_setup;
pub mod proof_verifier;
pub mod quality_gate;
pub mod security;
pub mod settings;
pub mod skill;
pub mod soul;
pub mod syndicate;
pub mod treasure;
pub mod voice;
pub mod watchtower;
pub mod whisper;

#[cfg(test)]
pub mod polar_webhook_tests;
