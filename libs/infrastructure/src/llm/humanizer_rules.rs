/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::writing_context::WritingContext;
use regex::Regex;
use std::sync::LazyLock;

/// ヒューマナイザールールがマッチした際のアクション定義
#[derive(Debug, Clone, PartialEq)]
pub enum HumanizerAction {
    /// 固定文字列に置換
    Replace(String),
    /// 完全に削除
    Delete,
    /// 置換はせず警告ログのみ（構造の破壊を防ぐためなど）
    LogWarning,
}

/// LLM出力からAI特有のパターンを検出して除去するためのルール
#[derive(Clone)]
pub struct HumanizerRule {
    /// ルールの識別名
    pub name: &'static str,
    /// 検出するための正規表現パターン
    pub pattern: Regex,
    /// マッチした場合のアクション
    pub action: HumanizerAction,
    /// このルールが適用されるコンテキストのリスト。空の場合は全コンテキストに適用。
    pub active_contexts: Vec<WritingContext>,
}

/// 日本語に特化したデフォルトのAIくささ除去ルールセット
pub fn default_rules_ja() -> Vec<HumanizerRule> {
    vec![
        // 1. emダッシュ・全角ダッシュの乱用
        HumanizerRule {
            name: "em_dash_replacement",
            pattern: match Regex::new(r"——|—|─") {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile Em dash regex: {}", e);
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::Replace("、".to_string()),
            active_contexts: vec![
                WritingContext::Chat,
                WritingContext::TechLog,
                WritingContext::Default,
            ],
        },
        // 2. チャットボット残留表現
        HumanizerRule {
            name: "chatbot_artifacts",
            pattern: match Regex::new(
                r"(?x)
                お役に立てれば幸いです[！!]*|
                ご不明な点がございましたら.*?お知らせください[！!]*|
                その他に[も]?お手伝いできることはありますか[？?]*|
                お気軽にお申し付けください[！!]*
            ",
            ) {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile chatbot artifacts regex: {}", e);
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::Delete,
            active_contexts: vec![], // All contexts
        },
        // 3. 追従的トーン
        HumanizerRule {
            name: "sycophantic_tone",
            pattern: match Regex::new(
                r"(?x)
                素晴らしい[ご]?質問ですね[！!]*|
                おっしゃる通りです[！!]*|
                その通りです[。]*
            ",
            ) {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile sycophantic tone regex: {}", e);
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::Delete,
            active_contexts: vec![], // All contexts
        },
        // 4. 過剰ヘッジング
        HumanizerRule {
            name: "excessive_hedging",
            pattern: match Regex::new(r"かもしれない可能性がある") {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile excessive hedging regex: {}", e);
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::Replace("だろう".to_string()),
            active_contexts: vec![
                WritingContext::TechLog,
                WritingContext::Chat,
                WritingContext::Default,
            ],
        },
        // 5. フィラー句
        HumanizerRule {
            name: "filler_phrases",
            pattern: match Regex::new(r"〜という事実により|であるため、結果として")
            {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile filler phrases regex: {}", e);
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::LogWarning, // 機械的な置換は文を壊す恐れがあるためログのみ
            active_contexts: vec![],
        },
        // 6. 意義の過剰強調
        HumanizerRule {
            name: "inflated_significance",
            pattern: match Regex::new(r"の重要性を(さらに)?(強調|浮き彫りに)して(い|おり)ます")
            {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!(
                        "FATAL: Failed to compile inflated significance regex: {}",
                        e
                    );
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::Replace("を示しています".to_string()),
            active_contexts: vec![WritingContext::TechLog, WritingContext::Default],
        },
        // 7. AI頻出接続詞の連続（簡易版: 文頭の「さらに」「加えて」が多すぎる場合）
        // ここでは単純な単語ベースルールに留める
        HumanizerRule {
            name: "ai_vocabulary",
            pattern: match Regex::new(r"^(さらに|加えて)、") {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile AI vocabulary regex: {}", e);
                    std::process::exit(1);
                }
            },
            action: HumanizerAction::LogWarning,
            active_contexts: vec![],
        },
    ]
}
