/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{Event, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

pub struct DbLoggerLayer {
    tx: mpsc::Sender<LogEntry>,
}

#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub target: String,
    pub message: String,
}

impl DbLoggerLayer {
    pub fn new(pool: SqlitePool) -> Self {
        let (tx, mut rx) = mpsc::channel::<LogEntry>(1000);

        tokio::spawn(async move {
            // Ensure table exists
            let _ = sqlx::query(
                "CREATE TABLE IF NOT EXISTS app_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                    level TEXT NOT NULL,
                    target TEXT NOT NULL,
                    message TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await;

            while let Some(entry) = rx.recv().await {
                // Ignore inserts if queue is too large or db fails (silent drop for logging layer)
                let _ =
                    sqlx::query("INSERT INTO app_logs (level, target, message) VALUES (?, ?, ?)")
                        .bind(entry.level)
                        .bind(entry.target)
                        .bind(entry.message)
                        .execute(&pool)
                        .await;
            }
        });

        Self { tx }
    }
}

impl<S: Subscriber> Layer<S> for DbLoggerLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = event.metadata().level().to_string();
        let target = event.metadata().target().to_string();

        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        // SEC-4: Secret Masking (Expert 4 Gap)
        // Simple heuristic to obfuscate common API keys or Stripe secrets from being logged
        let mut masked_message = if visitor.message.to_lowercase().contains("stripe_")
            || visitor.message.to_lowercase().contains("api_key")
            || visitor.message.to_lowercase().contains("sk_")
            || visitor.message.to_lowercase().contains("bearer")
            || visitor.message.to_lowercase().contains("secret")
            || visitor.message.to_lowercase().contains("password")
        {
            // More comprehensive regex for physical filtering of credentials
            let re =
                regex::Regex::new(r"(?i)(sk_(live|test)_|STRIPE_[A-Z_]+|API_KEY|Bearer\s+|secret|password|VAULT_MASTER_PASSWORD)[=: ]*[\x21-\x7E]+")
                    .expect("Invalid regex"); // allow-anti-pattern
            re.replace_all(&visitor.message, "$1***MASKED***")
                .to_string()
        } else {
            visitor.message
        };

        // Phase 2-C: Apply GDPR PII Masking
        masked_message = shared::guardrails::mask_pii(&masked_message);

        let entry = LogEntry {
            level,
            target,
            message: masked_message,
        };

        // Fire and forget (don't block the actual thread emitting log)
        let _ = self.tx.try_send(entry);
    }
}

struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // remove surrounding quotes if any
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }
}
