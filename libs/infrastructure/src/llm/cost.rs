/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::sync::Arc;

pub async fn log_evaluation(
    logger: Arc<crate::llm::evaluation_logger::EvaluationLogger>,
    prompt: String,
    system: Option<String>,
    provider: String,
    model: String,
    latency_ms: i64,
    cache_hit: bool,
    token_in: Option<i64>,
    token_out: Option<i64>,
) {
    let cost = calculate_cost_usd(&model, token_in, token_out);

    if let Err(e) = logger
        .log(crate::llm::evaluation_logger::EvaluationLogEntry {
            prompt,
            system,
            provider,
            model,
            latency_ms,
            token_count_in: token_in,
            token_count_out: token_out,
            cost_usd: Some(cost),
            cache_hit,
        })
        .await
    {
        tracing::warn!(
            "Observability: evaluation log write failed (non-fatal): {}",
            e
        );
    }
}

/// Returns (input_cost_per_million, output_cost_per_million) for a given model.
/// Returns None for local/unknown models (cost = 0).
fn model_pricing(model: &str) -> Option<(f64, f64)> {
    match model {
        "gpt-4o" => Some((5.0, 15.0)),
        "gpt-4.1" => Some((2.0, 8.0)),
        "gpt-4.1-mini" => Some((0.4, 1.6)),
        "claude-3-5-sonnet-20241022" | "claude-3-7-sonnet" | "claude-sonnet-4-20250514" => {
            Some((3.0, 15.0))
        }
        "claude-opus-4-20250514" => Some((15.0, 75.0)),
        "gemini-1.5-pro-002" | "gemini-2.0-flash-exp" => Some((1.25, 5.0)),
        "gemini-2.5-flash" => Some((0.15, 0.60)),
        "gemini-2.5-pro" => Some((1.25, 10.0)),
        _ => None, // Local/internal model -> 0 cost
    }
}

pub fn calculate_cost_usd(model: &str, token_in: Option<i64>, token_out: Option<i64>) -> f64 {
    let Some((input_rate, output_rate)) = model_pricing(model) else {
        return 0.0;
    };
    let input_tokens = token_in.unwrap_or(0).max(0) as f64;
    let output_tokens = token_out.unwrap_or(0).max(0) as f64;
    (input_tokens * input_rate / 1_000_000.0) + (output_tokens * output_rate / 1_000_000.0)
}

pub fn calculate_cost_coins(model: &str, token_in: Option<i64>, token_out: Option<i64>) -> u64 {
    let cost_usd = calculate_cost_usd(model, token_in, token_out);
    if !cost_usd.is_finite() || cost_usd <= 0.0 {
        0
    } else {
        let coins = (cost_usd * 1000.0).ceil() as u64;
        coins.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation_per_model() {
        let t_in = Some(1_000_000);
        let t_out = Some(1_000_000);

        // 既存モデルの正常系テスト
        assert_eq!(calculate_cost_usd("gpt-4o", t_in, t_out), 20.0); // $5 + $15
        assert_eq!(calculate_cost_usd("claude-3-7-sonnet", t_in, t_out), 18.0); // $3 + $15

        // 2025-2026 新規モデル
        assert_eq!(calculate_cost_usd("gemini-2.5-flash", t_in, t_out), 0.75); // $0.15 + $0.60
        assert_eq!(calculate_cost_usd("gemini-2.5-pro", t_in, t_out), 11.25); // $1.25 + $10.00
        assert_eq!(calculate_cost_usd("gpt-4.1", t_in, t_out), 10.0); // $2.00 + $8.00
        assert_eq!(calculate_cost_usd("gpt-4.1-mini", t_in, t_out), 2.0); // $0.40 + $1.60
        assert_eq!(
            calculate_cost_usd("claude-sonnet-4-20250514", t_in, t_out),
            18.0
        );
        assert_eq!(
            calculate_cost_usd("claude-opus-4-20250514", t_in, t_out),
            90.0
        );

        // ローカルモデル（Ollama等）は無料
        assert_eq!(calculate_cost_usd("qwen3.5:9b", t_in, t_out), 0.0);
    }

    #[test]
    fn test_cost_calculation_edge_cases() {
        // None トークン → 0 コスト
        assert_eq!(calculate_cost_usd("gpt-4o", None, None), 0.0);
        assert_eq!(calculate_cost_usd("gpt-4o", Some(1000), None), 0.005);
        assert_eq!(calculate_cost_usd("gpt-4o", None, Some(1000)), 0.015);

        // 未知のモデル → 0 コスト
        assert_eq!(
            calculate_cost_usd("unknown-model-v99", Some(1_000_000), Some(1_000_000)),
            0.0
        );

        // 空文字モデル → 0 コスト
        assert_eq!(
            calculate_cost_usd("", Some(1_000_000), Some(1_000_000)),
            0.0
        );
    }

    #[test]
    fn test_calculate_cost_coins() {
        // gpt-4o: $5.0 input, $15.0 output -> 20.0 USD for 1M each -> 20,000 Coins
        assert_eq!(
            calculate_cost_coins("gpt-4o", Some(1_000_000), Some(1_000_000)),
            20000
        );
        // gemini-2.5-flash: $0.15 input, $0.60 output -> 0.75 USD for 1M each -> 750 Coins
        assert_eq!(
            calculate_cost_coins("gemini-2.5-flash", Some(1_000_000), Some(1_000_000)),
            750
        );
        // Edge case: Minimum 1 coin on micro cost
        assert_eq!(
            calculate_cost_coins("gemini-2.5-flash", Some(1), Some(1)),
            1
        );
        // Unknown model or zero tokens
        assert_eq!(
            calculate_cost_coins("ollama-local", Some(1000), Some(1000)),
            0
        );
        assert_eq!(calculate_cost_coins("gpt-4o", None, None), 0);
        // Edge case: Negative tokens should be clamped to 0 (via calculate_cost_usd .max(0))
        assert_eq!(calculate_cost_coins("gpt-4o", Some(-100), Some(-100)), 0);
    }
}
