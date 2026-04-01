/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Returns a shared, globally-configured `reqwest::Client` instance.
/// This prevents new TCP/TLS connections from being opened for every request.
pub fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            // SEC-5 FIX: Global SSRF prevention via redirect blocking
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("❌ Failed to build global reqwest client")
    })
}
