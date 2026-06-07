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
            if let Err(e) = sqlx::query(
                "CREATE TABLE IF NOT EXISTS app_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                    level TEXT NOT NULL,
                    target TEXT NOT NULL,
                    message TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await
            {
                eprintln!("Failed to initialize logging database table: {}", e);
            }

            while let Some(entry) = rx.recv().await {
                // Ignore inserts if queue is too large or db fails (silent drop for logging layer)
                if let Err(e) =
                    sqlx::query("INSERT INTO app_logs (level, target, message) VALUES (?, ?, ?)")
                        .bind(entry.level)
                        .bind(entry.target)
                        .bind(entry.message)
                        .execute(&pool)
                        .await
                {
                    eprintln!("Failed to write log to database: {}", e);
                }
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
        let redactor = infrastructure::security::secret_redactor::SecretRedactor::new();
        let mut masked_message = redactor.redact(&visitor.message).into_owned();

        // Phase 2-C: Apply GDPR PII Masking
        masked_message = shared::guardrails::mask_pii(&masked_message);

        let entry = LogEntry {
            level,
            target,
            message: masked_message,
        };

        // DropSafe: 受信側 close またはバッファ満杯による drop は許容
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
