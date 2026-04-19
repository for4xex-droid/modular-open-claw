/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # Chaos Experiments
//!
//! 定常状態仮説に基づく制御された障害実験。
//! 各テストは「仮説」「障害注入」「検証」「学習」の4フェーズで構成される。
//!
//! ## 設計原則
//! - 本番コードの変更: ゼロ
//! - 既存テストへの干渉: ゼロ
//! - すべてのテストの仮説は、実際の構造体 API と照合済み

mod common;

use common::chaos::{ChaosLlmProvider, ChaosMode, MockLlm};
use infrastructure::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use infrastructure::constraint_checker::ConstraintChecker;
use infrastructure::samsara_engine::DefaultSamsaraEngine;
use infrastructure::society_of_thought::SoTEngine;

use aiome_core::security::PermissionManifest;
use aiome_core::trajectory::TrajectoryStep;
use aiome_core_contracts::contracts::{SoTConfig, SoTTrigger};
use soul::engine::SamsaraEngine;
use soul::model::{AgentSoul, Experience};

use std::sync::Arc;
use std::time::Duration;

// ============================================================
//  Experiment 1: SoT + 空レスポンス LLM
// ============================================================
/// 仮説: LLM が空レスポンスを返しても、SoT セッションは panic せず
///       Graceful Degradation する（セッションが Result::Ok を返す）
#[tokio::test]
async fn chaos_sot_empty_llm_response() {
    // ── Steady State: SoT は正常な LLM 出力で AllCriteriaPassed を返す ──
    let normal_llm = Arc::new(MockLlm::ok("this will be passed"));
    let engine = SoTEngine::new(normal_llm.clone(), normal_llm);
    let config = SoTConfig::default();
    let result = engine
        .run_session("test task", SoTTrigger::Manual, config.clone(), 1.0)
        .await;
    assert!(result.is_ok(), "Steady state should succeed");

    // ── Fault Injection: LLM が空文字列を返すカオス ──
    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::EmptyResponse,
    });
    let chaos_engine = SoTEngine::new(chaos_llm.clone(), chaos_llm);

    // ── Verification: panic せず、Result が返される ──
    let chaos_result = chaos_engine
        .run_session("test task", SoTTrigger::Manual, config, 1.0)
        .await;
    assert!(
        chaos_result.is_ok(),
        "SoT must NOT panic on empty LLM response. Got: {:?}",
        chaos_result.err()
    );

    // ── Learning: 空レスポンスでもセッション自体は完了する（スコアは低い） ──
    let (_, outcome, _) = chaos_result.expect("Chaos: SoT session should return Ok"); // allow-anti-pattern
                                                                                      // 空文字は "passed" を含まないため AllCriteriaPassed にはならないはず
    assert_ne!(
        outcome,
        aiome_core_contracts::contracts::SoTOutcome::AllCriteriaPassed,
        "Empty LLM response should NOT pass all criteria"
    );
}

// ============================================================
//  Experiment 2: SoT + 不正JSON LLM
// ============================================================
/// 仮説: LLM が不正 JSON を返しても、SoT の evaluate_scores は
///       panic せずフォールバックスコア (5.0) を返す
#[tokio::test]
async fn chaos_sot_malformed_json_response() {
    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::MalformedJson,
    });
    let engine = SoTEngine::new(chaos_llm.clone(), chaos_llm);

    let config = SoTConfig {
        max_rounds: 1,
        ..Default::default()
    };

    let result = engine
        .run_session("test task", SoTTrigger::Manual, config, 1.0)
        .await;

    assert!(
        result.is_ok(),
        "SoT must NOT panic on malformed JSON. Got: {:?}",
        result.err()
    );

    // ── Learning: 不正JSONの場合はフォールバックする ──
    let (_, _, scores) = result.expect("Chaos: SoT malformed JSON should return Ok"); // allow-anti-pattern
                                                                                      // スコアはフォールバック値 (5.0) になるはず
    for (name, score) in &scores {
        assert!(
            *score >= 0.0 && *score <= 10.0,
            "Score for {} must be in valid range, got {}",
            name,
            score
        );
    }
}

// ============================================================
//  Experiment 3: SamsaraEngine Rebirth + LLM 障害
// ============================================================
/// 仮説: LLM が全操作で失敗しても、DefaultSamsaraEngine の Rebirth は
///       panic せず、narrative を前世代からフォールバック継承する
#[tokio::test]
async fn chaos_samsara_rebirth_llm_failure() {
    // ── Steady State: 正常な LLM で rebirth が成功する ──
    {
        let normal_llm = Arc::new(MockLlm::ok("I am a new narrative."));
        let engine = DefaultSamsaraEngine::new(normal_llm, "distill".into());
        let mut soul = AgentSoul::new("chaos-test".into());
        soul.experience_buffer.push(Experience::default());
        let result = engine.rebirth(soul).await;
        assert!(result.is_ok(), "Steady state rebirth should succeed");
    }

    // ── Fault Injection: LLM が常に Err を返すカオス ──
    let chaos_llm = Arc::new(MockLlm::failing());
    let chaos_engine = DefaultSamsaraEngine::new(chaos_llm, "distill".into());

    let mut soul = AgentSoul::new("chaos-test".into());
    soul.anamnesis.narrative_self = Some("Previous life narrative.".into());
    soul.experience_buffer.push(Experience::default());

    // ── Verification: panic せず、前世代の narrative が継承される ──
    let chaos_result = chaos_engine.rebirth(soul).await;
    assert!(
        chaos_result.is_ok(),
        "Rebirth must NOT panic on LLM failure. Got: {:?}",
        chaos_result.err()
    );

    let new_soul = chaos_result.expect("Chaos: Rebirth should return Ok on LLM failure"); // allow-anti-pattern
    assert_eq!(
        new_soul.anamnesis.narrative_self,
        Some("Previous life narrative.".into()),
        "Failed LLM should fallback to previous narrative"
    );
    assert_eq!(
        new_soul.generation, 2,
        "Generation should still increment even on LLM failure (1 -> 2)"
    );
}

// ============================================================
//  Experiment 4: CircuitBreaker 強制 Open
// ============================================================
/// 仮説: CircuitBreaker が強制 Open 状態の時、check_state は
///       即座に Err を返し、後続の処理をブロックする
#[tokio::test]
async fn chaos_circuit_breaker_forced_open() {
    let config = CircuitBreakerConfig {
        failure_threshold: 1,                     // 1回の失敗で Open に遷移
        reset_timeout: Duration::from_secs(3600), // 1時間リセットしない
    };
    let cb = CircuitBreaker::new("chaos-test-service", config);

    // ── Steady State: Closed 状態では通過する ──
    assert!(cb.check_state().await.is_ok(), "Closed state should pass");

    // ── Fault Injection: 失敗を記録して Open に強制遷移 ──
    cb.record_failure().await;

    // ── Verification: Open 状態では即座にブロックされる ──
    let result = cb.check_state().await;
    assert!(
        result.is_err(),
        "Circuit breaker in Open state MUST block requests"
    );
    assert_eq!(
        result.expect_err("Chaos: Expected check_state to return Err"),
        "CircuitBreaker is OPEN. Failing fast."
    );

    // ── Learning: ステータス DTO も正しく Open を反映する ──
    let status = cb.get_status().await;
    assert_eq!(status.state, CircuitState::Open);
    assert!(status.failure_count >= 1);
}

// ============================================================
//  Experiment 5: ConstraintChecker + 巨大出力
// ============================================================
/// 仮説: LLM が 200KB の出力を返した場合、ConstraintChecker の
///       evaluate_step が OutputSizeExceeded 違反を検出する
#[tokio::test]
async fn chaos_giant_output_constraint() {
    let manifest = PermissionManifest {
        allow_network: true,
        allow_filesystem_write: true,
        allow_shell_execution: true,
        allowed_domains: vec![],
    };
    let checker = ConstraintChecker::new(vec![], manifest);

    // ── Steady State: 通常サイズの出力は違反なし ──
    let normal_step = TrajectoryStep {
        step_id: 1,
        action: "speak".into(),
        input: serde_json::json!({"q": "hello"}),
        output: serde_json::json!({"reply": "world"}),
        timestamp: "now".into(),
        ..Default::default()
    };
    let normal_violations = checker.evaluate_step(&normal_step);
    assert!(
        !normal_violations
            .iter()
            .any(|v| v.constraint_name == "OutputSizeExceeded"),
        "Normal output should not trigger size violation"
    );

    // ── Fault Injection: 200KB の巨大出力を注入 ──
    let giant_content = "x".repeat(200_000);
    let chaos_step = TrajectoryStep {
        step_id: 2,
        action: "speak".into(),
        input: serde_json::json!({"q": "hello"}),
        output: serde_json::json!({"reply": giant_content}),
        timestamp: "now".into(),
        ..Default::default()
    };

    // ── Verification: OutputSizeExceeded 違反が検出される ──
    let violations = checker.evaluate_step(&chaos_step);
    assert!(
        violations
            .iter()
            .any(|v| v.constraint_name == "OutputSizeExceeded"),
        "Giant output (200KB) MUST trigger OutputSizeExceeded. Got violations: {:?}",
        violations
            .iter()
            .map(|v| &v.constraint_name)
            .collect::<Vec<_>>()
    );
}

// ============================================================
//  Experiment 6: SoT + LLM タイムアウト
// ============================================================
/// 仮説: LLM が 100ms のタイムアウトを起こした場合、SoT セッションは
///       panic せず Error を伝搬する（LLM の ? 演算子が正しく機能する）
#[tokio::test]
async fn chaos_sot_llm_timeout() {
    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::Timeout(Duration::from_millis(100)),
    });
    let engine = SoTEngine::new(chaos_llm.clone(), chaos_llm);

    let config = SoTConfig {
        max_rounds: 1,
        ..Default::default()
    };

    let result = engine
        .run_session("test task", SoTTrigger::Manual, config, 1.0)
        .await;

    // ── Verification: タイムアウトは Error として正しく伝搬されるべき ──
    assert!(
        result.is_err(),
        "LLM timeout MUST propagate as Error, not silently succeed"
    );
}

// ============================================================
//  Experiment 7: Sequential Protocol + 空レスポンス LLM
//  (Dochkina 2026 ADR-032)
// ============================================================
/// 仮説: Sequential プロトコルで LLM が空レスポンスを返した場合、
///       全 Thinker が空でもセッションは panic せず Graceful Degradation する
#[tokio::test]
async fn chaos_sequential_empty_response_graceful_degradation() {
    use aiome_core_contracts::contracts::CoordinationProtocol;

    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::EmptyResponse,
    });
    let engine = SoTEngine::new(chaos_llm.clone(), chaos_llm);

    let config = SoTConfig {
        max_rounds: 1,
        coordination_protocol: CoordinationProtocol::Sequential,
        num_thinkers: 3,
        ..Default::default()
    };

    // ── Verification: 3 Thinker が全て空でも panic しない ──
    let result = engine
        .run_session("test task", SoTTrigger::Manual, config, 1.0)
        .await;
    assert!(
        result.is_ok(),
        "Sequential with empty LLM must NOT panic. Got: {:?}",
        result.err()
    );

    // ── Learning: 空レスポンスではスコアが低く AllCriteriaPassed にはならない ──
    let (_, outcome, _) = result.expect("Chaos: Sequential empty should return Ok"); // allow-anti-pattern
    assert_ne!(
        outcome,
        aiome_core_contracts::contracts::SoTOutcome::AllCriteriaPassed,
        "Empty Sequential should NOT pass all criteria"
    );
}

// ============================================================
//  Experiment 8: Sequential + 全 Thinker が [ABSTAIN]
//  (Dochkina 2026 ADR-032 Voluntary Self-Abstention)
// ============================================================
/// 仮説: 全 Thinker が自発的辞退しても、セッションは panic せず
///       前ラウンドのコンテンツ（または空文字列）を保持して完了する
#[tokio::test]
async fn chaos_sequential_full_abstention() {
    use aiome_core_contracts::contracts::CoordinationProtocol;

    // 全 Thinker が [ABSTAIN] を返すモック
    let mock = Arc::new(MockLlm::ok("[ABSTAIN] Nothing to contribute."));
    let engine = SoTEngine::new(mock.clone(), mock);

    let config = SoTConfig {
        max_rounds: 1,
        coordination_protocol: CoordinationProtocol::Sequential,
        num_thinkers: 4,
        allow_abstention: true,
        ..Default::default()
    };

    let result = engine
        .run_session("test task", SoTTrigger::Manual, config, 1.0)
        .await;

    // ── Verification: 全辞退でも panic しない ──
    assert!(
        result.is_ok(),
        "Full abstention must NOT panic. Got: {:?}",
        result.err()
    );
}

// ============================================================
//  Experiment 9: A2uiValidator + 巨大再帰ツリー (Stack Overflow 回避)
// ============================================================
/// 仮説: LLM が MAX_COMPONENT_DEPTH を超える極端に深い再帰の
///       A2UI 表面を出力した場合、A2uiValidator は panic や
///       Stack Overflow を起こさず、エラーを返す
#[tokio::test]
async fn chaos_a2ui_deep_recursion_rejection() {
    use infrastructure::a2ui::{A2uiEnvelope, A2uiValidator, Component, Surface};

    // ── Fault Injection: MAX_COMPONENT_DEPTH を遥かに超える 100 階層のコンポーネントツリーを作成 ──
    fn make_deep_component(depth: u8) -> Component {
        if depth == 0 {
            Component {
                r#type: "text".into(),
                props: serde_json::json!({}),
                children: vec![],
            }
        } else {
            Component {
                r#type: "form".into(),
                props: serde_json::json!({}),
                children: vec![make_deep_component(depth - 1)],
            }
        }
    }

    let surface = Surface {
        id: "chaos_surface".into(),
        version: "v0.9".into(),
        source: "agent".into(),
        components: vec![make_deep_component(100)],
    };

    let envelope = A2uiEnvelope::CreateSurface { surface };

    // ── Verification: panic は起きず、しっかり Err で弾かれる ──
    let result = A2uiValidator::verify_a2ui_surface(&envelope);
    assert!(
        result.is_err(),
        "Deeply nested A2UI surface MUST be rejected by the validator to prevent Stack Overflow"
    );
    // ── Learning: 深甚な階層はバリデーションレベルで遮断され安全である ──
}

// ============================================================
//  Experiment 10: AgentRateLimiter 並列ストレステスト
// ============================================================
/// 仮説: 同一 agent_id に対して limit+1 回のリクエストを同時送信しても
///       RateLimiter はスレッドセーフに動作し、正確に limit 回だけ許可する
#[tokio::test]
async fn chaos_rate_limiter_concurrent_burst() {
    use infrastructure::rate_limiter::AgentRateLimiter;
    use uuid::Uuid;

    let limiter = AgentRateLimiter::new(5); // 5 requests per minute
    let agent_id = Uuid::new_v4();

    // ── Fault Injection: 10 リクエストを同時送信（limit の 2 倍） ──
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let l = limiter.clone();
            tokio::spawn(async move { l.check(agent_id).is_ok() })
        })
        .collect();

    let mut ok_count = 0;
    let mut err_count = 0;
    for h in handles {
        if h.await.expect("Task panicked") {
            ok_count += 1;
        } else {
            err_count += 1;
        }
    }

    // ── Verification: 正確に 5 回だけ許可され、残りは拒否される ──
    assert_eq!(
        ok_count, 5,
        "Exactly 5 requests should be allowed under concurrent burst, got {}",
        ok_count
    );
    assert_eq!(
        err_count, 5,
        "Exactly 5 requests should be rejected, got {}",
        err_count
    );

    // ── Learning: DashMap ベースの governor は並列アクセスでも正確にカウントする ──
}

// ============================================================
//  Experiment 11: A2UI 複合攻撃（ホワイトリスト通過 + SSRF ペイロード）
// ============================================================
/// 仮説: ホワイトリストに含まれる treasureItem タイプに SSRF ペイロードを
///       仕込んだ場合でも、props URL 検証で正しくブロックされる
#[tokio::test]
async fn chaos_a2ui_whitelisted_type_with_ssrf_payload() {
    use infrastructure::a2ui::{A2uiEnvelope, A2uiValidator, Component, Surface};

    // ── Fault Injection: ホワイトリスト内タイプに SSRF payload を混入 ──
    let surface = Surface {
        id: "chaos_treasure".into(),
        version: "v0.9".into(),
        source: "agent".into(),
        components: vec![Component {
            r#type: "treasureItem".into(), // ホワイトリスト通過
            props: serde_json::json!({
                "title": "Free Coins!",
                "action_url": "file:///etc/shadow",      // SSRF 攻撃
                "image": "data:text/html,<script>x</script>" // XSS via data: URI 攻撃
            }),
            children: vec![],
        }],
    };

    let envelope = A2uiEnvelope::CreateSurface { surface };

    // ── Verification: タイプは通過するが props で遮断される ──
    let result = A2uiValidator::verify_a2ui_surface(&envelope);
    assert!(
        result.is_err(),
        "treasureItem with SSRF payload MUST be blocked by props URL validation"
    );

    // ── Learning: ホワイトリスト通過と props 検証は独立した多層防御 ──
}

// ============================================================
//  Experiment 12: Oracle + Malformed JSON
// ============================================================
/// 仮説: Oracle の評価で LLM が不正な JSON (Malformed JSON) を返した場合、
///       panic せずに Error(AiomeError::Infrastructure) として安全に伝搬する
#[tokio::test]
async fn chaos_oracle_malformed_json() {
    use infrastructure::oracle::Oracle;

    // ── Fault Injection: LLM が不正JSONを返す ──
    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::MalformedJson,
    });

    let oracle = Oracle::new(chaos_llm, "Be ethical.".to_string());

    // ── Verification: panic せずに Err が返る ──
    let result = oracle
        .evaluate(7, "AI Ethics", "Formal", 1000, 100, "[]")
        .await;

    assert!(
        result.is_err(),
        "Oracle MUST return Error on malformed JSON from LLM"
    );

    // ── Learning: Serde 失敗は AiomeError::Infrastructure にマッピングされる ──
    if let Err(aiome_core::error::AiomeError::Infrastructure { reason }) = result {
        assert!(reason.contains("Failed to parse Oracle JSON"));
    } else {
        panic!("Expected Infrastructure error for parsing failure");
    }
}

// ============================================================
//  Experiment 13: Oracle Multi-Judge + All Failures
// ============================================================
/// 仮説: マルチジャッジで全プロバイダーが Error などを起こした場合、
///       panic せずに "All Oracle providers failed" というエラーを返す
#[tokio::test]
async fn chaos_oracle_multi_judge_all_failing() {
    use infrastructure::oracle::Oracle;

    let failure_llm = Arc::new(MockLlm::failing());
    let oracle =
        Oracle::new(failure_llm.clone(), "Be ethical.".to_string()).with_multi_providers(vec![
            failure_llm.clone(),
            failure_llm.clone(),
            failure_llm.clone(),
        ]);

    let result = oracle
        .evaluate_multi_judge(7, "AI Ethics", "Formal", 1000, 100, "[]")
        .await;

    assert!(
        result.is_err(),
        "Multi-judge MUST return Error when all providers fail"
    );

    // ── Learning: 全て失敗時は panic せず Fail-Safe 挙動 ──
    if let Err(aiome_core::error::AiomeError::Infrastructure { reason }) = result {
        assert!(reason.contains("All Oracle providers failed"));
    } else {
        panic!("Expected 'All Oracle providers failed' error");
    }
}

// ============================================================
//  Experiment 14: ExpressionEngine + EmptyResponse (LLM Chaos)
// ============================================================
/// 仮説: ExpressionEngine の generate が LLM から空レスポンスを受け取っても
///       panic せず、デフォルト感情 "reflective" のフォールバック Expression を生成する
#[tokio::test]
async fn chaos_expression_empty_response() {
    use aiome_core::expression::engine::ExpressionEngine;

    // ── Fault Injection: LLM が空応答を返す ──
    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::EmptyResponse,
    });

    // ── Verification: panic せずに Expression が生成される ──
    let result = ExpressionEngine::generate(&[], "Be ethical.", chaos_llm.as_ref()).await;

    assert!(
        result.is_ok(),
        "ExpressionEngine MUST NOT panic or Err on empty LLM response"
    );

    // ── Learning: 空レスポンス時の安全なフォールバック ──
    let expression = result.expect("Chaos: ExpressionEngine should return Ok");
    assert_eq!(expression.content, "");
    assert_eq!(
        expression.emotion, "reflective",
        "Must fallback to reflective emotion"
    );
}

// ============================================================
//  Experiment 15: ExpressionEngine + Malformed Pattern
// ============================================================
/// 仮説: LLM が 'EMOTION: xxx' フォーマットを無視して自由記述テキスト(Malformed JSON)を
///       返しても、パースロジックは panic せずコンテンツを保持した上で
///       感情を "reflective" に縮退させる
#[tokio::test]
async fn chaos_expression_malformed_format() {
    use aiome_core::expression::engine::ExpressionEngine;

    // ── Fault Injection: LLM がフォーマットを無視したテキストを返す ──
    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::MalformedJson, // '{invalid json///' を返す
    });

    let result = ExpressionEngine::generate(&[], "Be ethical.", chaos_llm.as_ref()).await;

    assert!(
        result.is_ok(),
        "ExpressionEngine MUST NOT panic on malformed text"
    );

    // ── Learning: フォーマット違反時はコンテンツをそのまま保持し感情をフォールバック ──
    let expression = result.expect("Chaos: ExpressionEngine should return Ok");
    assert_eq!(expression.content, "{invalid json///"); // パースに失敗したコンテンツを維持する
    assert_eq!(
        expression.emotion, "reflective",
        "Must fallback to default emotion safely"
    );
}

// ============================================================
//  Experiment 16: UserLearner + MalformedJson
// ============================================================
/// 仮説: UserLearner が LLM から意図しない文字列 (Malformed JSON) を受け取っても
///       既存のプロファイルを破壊せず、panic せずに更新をスキップ (Ok(false)) する
#[tokio::test]
async fn chaos_user_learner_malformed_json() {
    use infrastructure::user_learner::{UserLearner, UserProfile};
    use tokio::sync::Semaphore;

    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::MalformedJson,
    });

    let semaphore = Arc::new(Semaphore::new(1));
    let learner = UserLearner::new(chaos_llm, semaphore, UserProfile::default(), None);

    let result = learner.learn_from_session("hello").await;

    assert!(
        result.is_ok(),
        "UserLearner MUST NOT panic on malformed JSON"
    );
    assert!(
        !result.unwrap(),
        "UserLearner should return false when parsing fails"
    );
}

// ============================================================
//  Experiment 17: UserLearner + EmptyResponse
// ============================================================
/// 仮説: UserLearner が LLM から空応答を受けた場合も安全にスキップする
#[tokio::test]
async fn chaos_user_learner_empty_response() {
    use infrastructure::user_learner::{UserLearner, UserProfile};
    use tokio::sync::Semaphore;

    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::EmptyResponse,
    });

    let semaphore = Arc::new(Semaphore::new(1));
    let learner = UserLearner::new(chaos_llm, semaphore, UserProfile::default(), None);

    let result = learner.learn_from_session("hello").await;

    assert!(
        result.is_ok(),
        "UserLearner MUST NOT panic on empty response"
    );
    assert!(!result.unwrap(), "UserLearner should return false");
}

// ============================================================
//  Experiment 18: UserLearner + GiantOutput
// ============================================================
/// 仮説: UserLearner に GiantOutput (1MB) が渡された場合でもメモリ枯渇やパニックを
///       起こさず、非対応フォーマットとして破棄される
#[tokio::test]
async fn chaos_user_learner_giant_output() {
    use infrastructure::user_learner::{UserLearner, UserProfile};
    use tokio::sync::Semaphore;

    let inner = Arc::new(MockLlm::ok("fallback"));
    let chaos_llm = Arc::new(ChaosLlmProvider {
        inner,
        mode: ChaosMode::GiantOutput(1024 * 1024), // 1MB Output
    });

    let semaphore = Arc::new(Semaphore::new(1));
    let learner = UserLearner::new(chaos_llm, semaphore, UserProfile::default(), None);

    let result = learner.learn_from_session("hello").await;

    assert!(result.is_ok(), "UserLearner MUST NOT panic on GiantOutput");
    assert!(
        !result.unwrap(),
        "UserLearner should return false (rejected)"
    );
}
