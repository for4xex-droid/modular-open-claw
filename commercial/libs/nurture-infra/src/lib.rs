/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

#![forbid(unsafe_code)]

pub mod a2a;
pub mod b2a;
pub mod csam;
// TaskSupervisor was moved to aiome/libs/infrastructure
pub mod drm;
pub mod economy;
pub mod gift;
pub mod identity;
pub mod marketplace;
pub mod mock_job_queue;
pub mod polar;
pub mod sandbox;
pub mod sidecar;
pub mod storage;
#[cfg(feature = "stripe")]
pub mod stripe;
