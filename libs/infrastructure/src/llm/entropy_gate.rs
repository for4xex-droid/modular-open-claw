use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, TokenLogprob};
use async_trait::async_trait;
use std::sync::Arc;

/// Shannon Entropyを計算するユーティリティ関数
///
/// logprob は通常自然対数 (ln(p)) で与えられます。
/// H = - Σ (p_i * log2(p_i))
///
/// `top_logprobs` が利用可能な場合、各トークン位置における確率分布全体から
/// Shannon Entropy を算出します。利用不可な場合は単一トークンの
/// self-information を近似値として使用します。
pub fn calculate_sequence_entropy(logprobs: &[TokenLogprob]) -> f64 {
    if logprobs.is_empty() {
        return 0.0;
    }

    let mut total_entropy = 0.0;
    for lp in logprobs {
        let position_entropy = if let Some(ref tops) = lp.top_logprobs {
            if tops.is_empty() {
                // top_logprobs が空配列の場合はフォールバック
                single_token_entropy(lp.logprob)
            } else {
                // 確率分布全体から Shannon Entropy を計算
                let mut h = 0.0;
                for (_token, lp) in tops {
                    let p = lp.exp();
                    if p > 0.0 {
                        h -= p * p.log2();
                    }
                }
                h
            }
        } else {
            // top_logprobs なし: self-information による近似
            single_token_entropy(lp.logprob)
        };
        total_entropy += position_entropy;
    }

    total_entropy / logprobs.len() as f64
}

/// 単一トークンの logprob から self-information ベースのエントロピー近似値を算出
fn single_token_entropy(logprob: f64) -> f64 {
    let p = logprob.exp();
    if p > 0.0 {
        -(p * p.log2())
    } else {
        0.0
    }
}

/// 不確実性に基づく自己修正ループを管理するミドルウェア
#[derive(Debug, Clone)]
pub struct EntropyGate {
    inner: Arc<dyn LlmProvider>,
    threshold: f64,
    max_re_ask: usize,
}

impl EntropyGate {
    pub fn new(inner: Arc<dyn LlmProvider>, threshold: f64, max_re_ask: usize) -> Self {
        Self {
            inner,
            threshold,
            max_re_ask,
        }
    }

    /// レスポンスのエントロピーを検査し、閾値超過時はリトライするコアロジック
    async fn check_and_retry<F, Fut>(&self, mut invoke: F) -> Result<LlmResponse, AiomeError>
    where
        F: FnMut(usize) -> Fut,
        Fut: std::future::Future<Output = Result<LlmResponse, AiomeError>>,
    {
        let mut retries = 0;

        loop {
            let resp = invoke(retries).await?;

            if let Some(ref logprobs) = resp.logprobs {
                let entropy = calculate_sequence_entropy(logprobs);
                if entropy > self.threshold {
                    if retries < self.max_re_ask {
                        tracing::warn!(
                            "High uncertainty detected (entropy: {:.4}). Re-asking... (try {}/{})",
                            entropy,
                            retries + 1,
                            self.max_re_ask
                        );
                        retries += 1;
                        continue;
                    } else {
                        return Err(AiomeError::LlmResponse {
                            source: anyhow::anyhow!(
                                "High Uncertainty Limit Exceeded (entropy: {:.4})",
                                entropy
                            ),
                        });
                    }
                }
            }

            return Ok(resp);
        }
    }
}

#[async_trait]
impl LlmProvider for EntropyGate {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let mut current_prompt = prompt.to_string();
        let system_owned = system.map(|s| s.to_string());

        self.check_and_retry(|retry_count| {
            // リトライ時はプロンプトを修正してから送信
            if retry_count > 0 {
                current_prompt = format!(
                    "{}\n\n[System Error: High Uncertainty Detected. Please rethink step-by-step and provide a more certain response.]",
                    current_prompt
                );
            }

            let p = current_prompt.clone();
            let s = system_owned.clone();
            let inner = self.inner.clone();

            async move {
                inner.complete(&p, s.as_deref()).await
            }
        }).await
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let mut current_request = request;

        self.check_and_retry(|retry_count| {
            // リトライ時はリクエストを修正してから送信
            if retry_count > 0 {
                if let Some(last_msg) = current_request.messages.last_mut() {
                    last_msg.content = format!(
                        "{}\n\n[System Error: High Uncertainty Detected. Please rethink step-by-step and provide a more certain response.]",
                        last_msg.content
                    );
                }
            }

            let req = current_request.clone();
            let inner = self.inner.clone();

            async move {
                inner.complete_with_cache(req).await
            }
        }).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.inner.test_connection().await
    }

    fn name(&self) -> &str {
        "entropy_gate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_calculate_sequence_entropy_certain() {
        let certain = vec![TokenLogprob {
            token: "a".into(),
            logprob: 0.0,
            top_logprobs: None,
        }];
        assert_eq!(calculate_sequence_entropy(&certain), 0.0);
    }

    #[test]
    fn test_calculate_sequence_entropy_uncertain() {
        let uncertain = vec![TokenLogprob {
            token: "a".into(),
            logprob: -0.6931471805599453,
            top_logprobs: None,
        }];
        let entropy = calculate_sequence_entropy(&uncertain);
        assert!((entropy - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_calculate_sequence_entropy_empty() {
        assert_eq!(calculate_sequence_entropy(&[]), 0.0);
    }

    #[test]
    fn test_calculate_sequence_entropy_with_top_logprobs() {
        // Uniform distribution over 2 tokens: H = 1.0 bit
        let with_tops = vec![TokenLogprob {
            token: "a".into(),
            logprob: -0.6931471805599453, // ln(0.5)
            top_logprobs: Some(vec![
                ("a".to_string(), -0.6931471805599453),
                ("b".to_string(), -0.6931471805599453),
            ]),
        }];
        let entropy = calculate_sequence_entropy(&with_tops);
        assert!(
            (entropy - 1.0).abs() < 1e-5,
            "Uniform binary distribution should have entropy 1.0, got {}",
            entropy
        );
    }

    #[derive(Debug)]
    struct EntropyMockProvider {
        call_count: Arc<AtomicUsize>,
        uncertain_logprob: f64,
    }

    #[async_trait]
    impl LlmProvider for EntropyMockProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(LlmResponse {
                content: "I think so".into(),
                logprobs: Some(vec![TokenLogprob {
                    token: "I".into(),
                    logprob: self.uncertain_logprob,
                    top_logprobs: None,
                }]),
                ..Default::default()
            })
        }
        async fn complete_with_cache(
            &self,
            _request: LlmRequest,
        ) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "entropy_mock"
        }
    }

    #[tokio::test]
    async fn test_entropy_gate_retries_and_fails() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock = Arc::new(EntropyMockProvider {
            call_count: call_count.clone(),
            uncertain_logprob: -0.693147, // entropy ≈ 0.5
        });

        // threshold 0.1, max_re_ask 2回
        let gate = EntropyGate::new(mock, 0.1, 2);

        let result = gate.complete("test prompt", None).await;

        // エントロピー0.5 > 0.1 なのでブロック（またはエラー）される
        assert!(result.is_err());

        // 初回1回 + リトライ2回 = 計3回呼ばれているはず
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    /// リトライ時にプロンプトが正しく修正されることを検証するモック
    #[derive(Debug)]
    struct PromptCaptureMock {
        captured_prompts: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl LlmProvider for PromptCaptureMock {
        async fn complete(
            &self,
            prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            self.captured_prompts
                .lock()
                .unwrap()
                .push(prompt.to_string());
            Ok(LlmResponse {
                content: "uncertain".into(),
                logprobs: Some(vec![TokenLogprob {
                    token: "x".into(),
                    logprob: -0.693147,
                    top_logprobs: None,
                }]),
                ..Default::default()
            })
        }
        async fn complete_with_cache(
            &self,
            _request: LlmRequest,
        ) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "prompt_capture"
        }
    }

    #[tokio::test]
    async fn test_entropy_gate_retry_sends_modified_prompt() {
        let captured = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let mock = Arc::new(PromptCaptureMock {
            captured_prompts: captured.clone(),
        });

        // threshold 0.1, max_re_ask 1回
        let gate = EntropyGate::new(mock, 0.1, 1);
        let _ = gate.complete("original question", None).await;

        let prompts = captured.lock().unwrap();
        assert_eq!(prompts.len(), 2, "Should have initial call + 1 retry");
        assert_eq!(
            prompts[0], "original question",
            "First call should use original prompt"
        );
        assert!(
            prompts[1].contains("original question"),
            "Retry should contain original prompt"
        );
        assert!(
            prompts[1].contains("[System Error: High Uncertainty Detected"),
            "Retry prompt must contain the rethink injection"
        );
    }
}
