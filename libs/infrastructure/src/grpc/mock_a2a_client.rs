/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::a2a::{A2aClient, A2aTaskProgress, A2aTaskRequest};
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::{Arc, Mutex};

/// テスト用のダミー A2aClient 実装
#[derive(Clone)]
pub struct MockA2aClient {
    pub responses: Arc<Mutex<Vec<Vec<Result<A2aTaskProgress, AiomeError>>>>>,
}

impl MockA2aClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 次の execute_task 呼び出しで返すプログレスストリームの要素を登録する
    pub fn enqueue_responses(&self, response_stream: Vec<Result<A2aTaskProgress, AiomeError>>) {
        self.responses.lock().unwrap().push(response_stream);
    }
}

#[async_trait]
impl A2aClient for MockA2aClient {
    async fn execute_task(
        &self,
        request: A2aTaskRequest,
    ) -> Result<BoxStream<'static, Result<A2aTaskProgress, AiomeError>>, AiomeError> {
        let mut responses = self.responses.lock().unwrap();
        let stream_items = if !responses.is_empty() {
            responses.remove(0)
        } else {
            // デフォルトの成功レスポンス
            vec![
                Ok(A2aTaskProgress {
                    message: "Starting mock task".into(),
                    percent: 50,
                    is_completed: false,
                    is_failed: false,
                    result: None,
                    error: None,
                    result_hash: None,
                }),
                Ok(A2aTaskProgress {
                    message: "Completed mock task".into(),
                    percent: 100,
                    is_completed: true,
                    is_failed: false,
                    result: Some(format!("Mock result for {}", request.job_id)),
                    error: None,
                    result_hash: Some("mock-hash-1234".into()),
                }),
            ]
        };

        // async-stream で非同期に yield
        let stream = async_stream::stream! {
            for item in stream_items {
                yield item;
            }
        };

        Ok(Box::pin(stream))
    }

    async fn cancel_task(&self, _job_id: &str) -> Result<(), AiomeError> {
        Ok(())
    }
}
