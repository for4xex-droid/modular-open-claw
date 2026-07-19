/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Caller-scoped telemetry helpers for key-proxy (OP-025 / OP-086 Wave B1).
//! Never log Authorization headers, vault secrets, or prompt bodies here.

/// Strip CR/LF so untrusted strings cannot forge multi-line log / span injection.
pub(crate) fn sanitize_for_log(raw: &str) -> String {
    raw.replace(['\n', '\r'], "_")
}

/// Redact credential-shaped substrings from a Display error before logging.
pub(crate) fn redact_display(err: &impl std::fmt::Display) -> String {
    redact_url_secrets(&err.to_string())
}

/// Redact credential-shaped substrings from error/URL strings before logging.
pub(crate) fn redact_url_secrets(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(idx) = rest.find("key=") {
        out.push_str(&rest[..idx]);
        out.push_str("key=REDACTED");
        rest = &rest[idx + 4..];
        if let Some(end) = rest.find(['&', ' ', ')', '"', '\'']) {
            rest = &rest[end..];
        } else {
            rest = "";
        }
    }
    out.push_str(rest);

    fn redact_prefix(input: &str, prefix: &str, replacement: &str) -> String {
        let mut cleaned = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(idx) = rest.find(prefix) {
            cleaned.push_str(&rest[..idx]);
            cleaned.push_str(replacement);
            rest = &rest[idx + prefix.len()..];
            if let Some(end) = rest.find([' ', '"', '\'', ',', ')', '&', '\n', '\r']) {
                rest = &rest[end..];
            } else {
                rest = "";
            }
        }
        cleaned.push_str(rest);
        cleaned
    }

    let out = redact_prefix(&out, "Bearer ", "Bearer REDACTED");
    // Header-style leaks (case variants normalized via lowercase scan on a copy).
    let lower = out.to_lowercase();
    if let Some(idx) = lower.find("x-goog-api-key") {
        let mut cleaned = out[..idx].to_string();
        cleaned.push_str("x-goog-api-key=REDACTED");
        let after = &out[idx + "x-goog-api-key".len()..];
        let after = after.trim_start_matches([' ', ':', '=']);
        if let Some(end) = after.find([' ', '"', '\'', ',', ')', '&', '\n', '\r']) {
            cleaned.push_str(&after[end..]);
        }
        return cleaned;
    }
    out
}

/// Strip CR/LF so caller_id cannot forge multi-line log injection.
pub(crate) fn sanitize_caller_id(raw: &str) -> String {
    sanitize_for_log(raw)
}

/// Record the sanitized caller on the current tracing span (if field exists).
pub(crate) fn record_caller_on_span(safe_caller_id: &str) {
    tracing::Span::current().record("caller_id", tracing::field::display(safe_caller_id));
}

/// Record a sanitized endpoint on the current tracing span (if field exists).
pub(crate) fn record_endpoint_on_span(raw_endpoint: &str) {
    let safe = sanitize_for_log(raw_endpoint);
    tracing::Span::current().record("endpoint", tracing::field::display(safe));
}

pub(crate) fn emit_cost_metric(caller_id: &str, tokens: u64, cost_usd: f64, model: &str) {
    let safe_caller_id = sanitize_caller_id(caller_id);
    tracing::info!(
        target: "key_proxy::metrics",
        caller_id = %safe_caller_id,
        tokens = tokens,
        cost_usd = cost_usd,
        model = %model,
        "💰 [KeyProxy] Cost metric recorded"
    );
}

pub(crate) fn emit_stream_start_metric(caller_id: &str, response_time_ms: u64) {
    let safe_caller_id = sanitize_caller_id(caller_id);
    tracing::info!(
        target: "key_proxy::metrics",
        caller_id = %safe_caller_id,
        response_time_ms = response_time_ms,
        "🌊 [KeyProxy] Streaming response started"
    );
}

pub(crate) fn emit_embed_metric(caller_id: &str, response_time_ms: u64, dims: usize) {
    let safe_caller_id = sanitize_caller_id(caller_id);
    tracing::info!(
        target: "key_proxy::metrics",
        caller_id = %safe_caller_id,
        response_time_ms = response_time_ms,
        embedding_dims = dims,
        "🧬 [KeyProxy] Embed metric recorded"
    );
}

/// Cross-Node Auth Reliability: structured 401 without secret material.
/// Never pass Authorization header values or vault secrets into this helper.
pub(crate) fn emit_unauthorized_access(method: &str, path: &str, has_authorization: bool) {
    let method = sanitize_for_log(method);
    let path = sanitize_for_log(path);
    tracing::warn!(
        method = %method,
        path = %path,
        auth_result = "unauthorized",
        has_authorization = has_authorization,
        "⛔ [KeyProxy] Unauthorized access attempt"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn capture_logs<F: FnOnce()>(f: F) -> String {
        let buf = BufWriter::default();
        let writer = buf.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        String::from_utf8_lossy(&buf.0.lock().unwrap()).into_owned()
    }

    #[test]
    fn sanitize_caller_id_strips_crlf() {
        assert_eq!(sanitize_caller_id("agent\nid"), "agent_id");
        assert_eq!(sanitize_caller_id("a\r\nb"), "a__b");
        assert_eq!(sanitize_for_log("gemini\nINFO"), "gemini_INFO");
    }

    #[test]
    fn redact_url_secrets_strips_api_key_and_bearer() {
        let leaked = "error sending request for url (https://example/v1?key=SUPERSECRET&x=1)";
        let out = redact_url_secrets(leaked);
        assert!(out.contains("key=REDACTED"));
        assert!(!out.contains("SUPERSECRET"));
        let bearer = "status Authorization: Bearer tokensecret fails";
        let out2 = redact_url_secrets(bearer);
        assert!(out2.contains("Bearer REDACTED"));
        assert!(!out2.contains("tokensecret"));
        let goog = "upstream x-goog-api-key: googlesecret failed";
        let out3 = redact_url_secrets(goog);
        assert!(out3.contains("x-goog-api-key=REDACTED"));
        assert!(!out3.contains("googlesecret"));
    }

    #[test]
    fn cost_metric_includes_caller_id() {
        let out = capture_logs(|| {
            emit_cost_metric("shadow-worker", 42, 0.0000063, "gemini-2.0-flash");
        });
        assert!(
            out.contains("caller_id") && out.contains("shadow-worker"),
            "metrics must include caller_id: {out}"
        );
        assert!(out.contains("key_proxy::metrics") || out.contains("Cost metric"));
    }

    #[test]
    fn metrics_never_emit_auth_header_shape() {
        let out = capture_logs(|| {
            emit_cost_metric("api-server", 1, 0.0, "m");
            emit_stream_start_metric("api-server", 12);
            emit_embed_metric("api-server", 9, 3);
        });
        assert!(out.contains("api-server"));
        assert!(
            !out.contains("Bearer ") && !out.to_lowercase().contains("authorization"),
            "metrics must not look like auth material: {out}"
        );
    }

    #[test]
    fn unauthorized_log_is_structured_without_secrets() {
        let out = capture_logs(|| {
            emit_unauthorized_access("POST", "/api/v1/llm/complete", true);
        });
        assert!(
            out.contains("unauthorized") || out.contains("Unauthorized"),
            "expected structured unauthorized log: {out}"
        );
        assert!(out.contains("POST") && out.contains("/api/v1/llm/complete"));
        // Negative: helper API has no secret params; guard against regressions that
        // start logging Authorization / API-key header material.
        assert!(
            !out.contains("Bearer ") && !out.contains("x-goog-api-key"),
            "credential header material must not appear: {out}"
        );
    }
}
